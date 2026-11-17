//! The engine end to end, offline: every yt-dlp run is `fake-ytdlp` replaying a recorded
//! session (`tests/fixtures/sessions`). The test's view of the state is built only from the
//! changes the engine emits, the same way the UI builds its view.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{Harness, Setup, fake_url, gone};
use engine::model::*;
use engine::state::can_transition;

fn every_transition_is_legal(h: &Harness, id: &str) {
    let states = h.states_of(id);
    for pair in states.windows(2) {
        assert!(can_transition(pair[0], pair[1]), "{:?} -> {:?} in {states:?}", pair[0], pair[1]);
    }
}

async fn ready(setup: Setup) -> Harness {
    let mut h = Harness::new(setup).await;
    h.wait_for("tools ready", |h| h.components.ytdlp.ready() && h.components.ffmpeg.ready()).await;
    h
}

#[tokio::test]
async fn a_link_downloads_to_a_file_and_lands_in_history() {
    let mut h = ready(Setup::default()).await;
    let url = fake_url("generic_direct");
    h.engine.add_links(format!("look: {url}"), Source::Paste).await.unwrap();
    h.wait_for("completed", |h| h.state("generic_direct") == Some(JobState::Completed)).await;

    let job = h.job("generic_direct").clone();
    assert_eq!(job.output, h.downloads().join("ok [ok].mp4").to_string_lossy());
    assert!(std::path::Path::new(&job.output).is_file());
    assert_eq!(job.progress.fraction, Some(1.0));
    assert_eq!(job.title, "ok");
    every_transition_is_legal(&h, &job.id);
    assert!(h.states_of(&job.id).contains(&JobState::Downloading));
    // Partial files and the cached probe (which can hold personal data) are gone.
    assert!(gone(&h.partial(&job.id)).await);
    h.settle(200).await;
    assert!(h.probe_cache().is_empty(), "{:?}", h.probe_cache());
    // The download reused the probe instead of extracting the page twice.
    assert_eq!(h.modes("generic_direct"), ["probe", "download"]);
    let (_, _, args) = h.log().into_iter().find(|(m, ..)| m == "download").unwrap();
    assert!(args.contains(&"--load-info-json".to_string()), "{args:?}");
    assert!(matches!(h.notices().as_slice(), [Notice::Finished { title, .. }] if title == "ok"));

    let page = h.engine.history(String::new(), HistoryFilter::All, 0, 50).await;
    assert_eq!(page.items.iter().map(|j| j.id.as_str()).collect::<Vec<_>>(), [job.id.as_str()]);
    assert!(!page.items[0].file_missing);
    std::fs::remove_file(&job.output).unwrap();
    let page = h.engine.history("ok".into(), HistoryFilter::Video, 0, 50).await;
    assert!(page.items[0].file_missing, "a deleted file is noticed");
}

#[tokio::test]
async fn video_and_audio_streams_report_stages_and_progress_that_never_goes_back() {
    // Slow enough that the 100 ms batching cannot fold the stages together.
    let mut h = ready(Setup { line_ms: 150, ..Setup::default() }).await;
    h.engine.add_links(fake_url("video_audio"), Source::Paste).await.unwrap();
    h.wait_for("completed", |h| h.state("video_audio") == Some(JobState::Completed)).await;

    let id = h.job("video_audio").id.clone();
    let mut stages = Vec::new();
    let mut last = 0.0;
    for change in &h.seen {
        let progress = match change {
            Change::Progress { id: pid, progress } if *pid == id => progress,
            Change::JobUpdated(job) if job.id == id => &job.progress,
            _ => continue,
        };
        if stages.last() != Some(&progress.stage) {
            stages.push(progress.stage);
        }
        if let Some(f) = progress.fraction.filter(|_| matches!(progress.stage, Stage::Video | Stage::Audio)) {
            assert!(f >= last, "progress went back from {last} to {f}");
            last = f;
        }
    }
    let video = stages.iter().position(|s| *s == Stage::Video).expect("a video stage");
    let audio = stages.iter().position(|s| *s == Stage::Audio).expect("an audio stage");
    let merge = stages.iter().position(|s| *s == Stage::Merging).expect("a merge stage");
    assert!(video < audio && audio < merge, "{stages:?}");
    assert!(stages.contains(&Stage::EmbeddingMetadata), "{stages:?}");
    every_transition_is_legal(&h, &id);
}

#[tokio::test]
async fn five_links_at_a_concurrency_of_two() {
    let mut h = ready(Setup { line_ms: 60, settings: Box::new(|s| s.max_concurrent = 2), ..Setup::default() }).await;
    let links: Vec<String> = (1..=5).map(|n| format!("{}?n={n}", fake_url("generic_direct"))).collect();
    let added = h.engine.add_links(links.join("\n"), Source::Paste).await.unwrap();
    assert_eq!(added, 5);
    h.wait_for("five jobs completed", |h| h.jobs.len() == 5 && h.jobs.iter().all(|j| j.state == JobState::Completed))
        .await;
    assert_eq!(h.peak_active, 2, "never more than two at once, and both slots used");
    // The queue runs in order: the first link starts first.
    let first_started = h.jobs.iter().min_by(|a, b| a.started_at.partial_cmp(&b.started_at).unwrap()).unwrap();
    assert!(first_started.url.ends_with("?n=1"));
    assert_eq!(h.notices().len(), 5);
}

#[tokio::test]
async fn the_per_site_limit_holds_back_a_busy_site() {
    let mut h = ready(Setup {
        line_ms: 60,
        settings: Box::new(|s| {
            s.max_concurrent = 4;
            s.max_per_site = 1;
        }),
        ..Setup::default()
    })
    .await;
    let links: Vec<String> = (1..=3).map(|n| format!("{}?n={n}", fake_url("generic_direct"))).collect();
    h.engine.add_links(links.join(" "), Source::Paste).await.unwrap();
    h.wait_for("all completed", |h| h.jobs.len() == 3 && h.jobs.iter().all(|j| j.state == JobState::Completed)).await;
    assert_eq!(h.peak_per_site, 1);
}

#[tokio::test]
async fn pause_keeps_partial_files_and_resume_runs_the_same_command() {
    let mut h = ready(Setup { line_ms: 400, ..Setup::default() }).await;
    h.engine.add_links(fake_url("generic_direct"), Source::Paste).await.unwrap();
    h.wait_for("bytes arriving", |h| {
        h.find("generic_direct").is_some_and(|j| j.state == JobState::Downloading && j.progress.downloaded > 0.0)
    })
    .await;
    let id = h.job("generic_direct").id.clone();
    h.engine.job_pause(id.clone());
    h.wait_for("paused", |h| h.state("generic_direct") == Some(JobState::Paused)).await;
    h.settle(300).await;
    let part = h.partial(&id).join("ok [ok].mp4.part");
    assert!(part.is_file(), "the partial file survives a pause");

    h.engine.job_resume(id.clone());
    h.wait_for("completed", |h| h.state("generic_direct") == Some(JobState::Completed)).await;
    let downloads: Vec<Vec<String>> =
        h.log().into_iter().filter(|(m, ..)| m == "download").map(|(.., args)| args).collect();
    assert_eq!(downloads.len(), 2);
    assert_eq!(downloads[0], downloads[1], "resume reruns the identical command, so yt-dlp continues the .part file");
    every_transition_is_legal(&h, &id);
}

#[tokio::test]
async fn cancel_stops_a_hung_download_and_removes_its_files() {
    let mut h = ready(Setup::default()).await;
    h.engine.add_links(fake_url("hang"), Source::Paste).await.unwrap();
    h.wait_for("downloading", |h| h.find("hang").is_some_and(|j| j.progress.downloaded > 0.0)).await;
    let id = h.job("hang").id.clone();
    let partial = h.partial(&id);
    assert!(partial.join("ok [ok].mp4.part").is_file());

    h.engine.job_cancel(id.clone());
    h.wait_for("canceled", |h| h.state("hang") == Some(JobState::Canceled)).await;
    assert!(gone(&partial).await, "the temp folder is deleted once the process has exited");

    // Retry puts it back in line.
    h.engine.job_retry(id.clone());
    h.wait_for("running again", |h| h.find("hang").is_some_and(|j| j.attempts == 1 && j.progress.downloaded > 0.0))
        .await;
    h.engine.job_cancel(id);
    h.wait_for("canceled again", |h| h.state("hang") == Some(JobState::Canceled)).await;
}

#[tokio::test]
async fn a_crash_without_a_word_is_reported_as_one() {
    let mut h = ready(Setup::default()).await;
    h.engine.add_links(fake_url("crash"), Source::Paste).await.unwrap();
    h.wait_for("failed", |h| h.state("crash") == Some(JobState::Failed)).await;
    let error = h.job("crash").error.clone().unwrap();
    assert_eq!((error.kind, error.fix, error.retryable), (ErrorKind::Crashed, Some(Fix::Retry), true));
    assert!(matches!(h.notices().as_slice(), [Notice::Failed { .. }]));
}

#[tokio::test]
async fn errors_are_classified_from_real_output() {
    let mut h = ready(Setup::default()).await;
    let cases = [
        ("unavailable", ErrorKind::Unavailable, None, false),
        ("format_unavailable", ErrorKind::FormatUnavailable, Some(Fix::ChooseFormat), false),
        ("http404", ErrorKind::Unavailable, None, false),
        ("http429", ErrorKind::Http429, Some(Fix::Retry), true),
        ("http500", ErrorKind::Network, Some(Fix::Retry), true),
        ("http403_probe", ErrorKind::Http403, Some(Fix::UpdateYtdlp), true),
        ("drm", ErrorKind::Drm, None, false),
    ];
    let links: Vec<String> = cases.iter().map(|(name, ..)| fake_url(name)).collect();
    h.engine.add_links(links.join("\n"), Source::Paste).await.unwrap();
    h.wait_for("all failed", |h| h.jobs.len() == cases.len() && h.jobs.iter().all(|j| j.state == JobState::Failed))
        .await;
    for (name, kind, fix, retryable) in cases {
        let error = h.job(&format!("/{name}")).error.clone().unwrap();
        assert_eq!((error.kind, error.fix, error.retryable), (kind, fix, retryable), "{name}: {}", error.detail);
        assert!(!error.detail.is_empty());
    }
    // A DRM item stops at the probe: nothing is downloaded.
    assert_eq!(h.modes("drm"), ["probe"]);

    // Retry Failed skips what retrying cannot fix.
    h.engine.retry_failed();
    h.wait_for("retryable ones queued again", |h| h.jobs.iter().filter(|j| j.attempts == 1).count() == 3).await;
    assert_eq!(h.job("/unavailable").attempts, 0);
}

#[tokio::test]
async fn an_expired_cached_probe_is_probed_again_once_on_403() {
    let mut h = ready(Setup::default()).await;
    h.engine.add_links(fake_url("http403_download"), Source::Paste).await.unwrap();
    h.wait_for("failed", |h| h.state("http403_download") == Some(JobState::Failed)).await;
    assert_eq!(h.modes("http403_download"), ["probe", "download", "probe", "download"]);
    assert_eq!(h.job("http403_download").error.as_ref().map(|e| e.kind), Some(ErrorKind::Http403));
    every_transition_is_legal(&h, &h.job("http403_download").id.clone());
}

#[tokio::test]
async fn quitting_pauses_downloads_and_relaunching_resumes_them() {
    let mut h = ready(Setup { line_ms: 400, ..Setup::default() }).await;
    h.engine.add_links(fake_url("generic_direct"), Source::Paste).await.unwrap();
    h.wait_for("bytes arriving", |h| h.find("generic_direct").is_some_and(|j| j.progress.downloaded > 0.0)).await;
    let id = h.job("generic_direct").id.clone();
    assert_eq!(h.engine.active_count().await, 1);

    let mut h = h.restart().await;
    assert_eq!(h.jobs.len(), 1);
    assert!(h.partial(&id).join("ok [ok].mp4.part").is_file(), "partial files survive the quit");
    h.wait_for("tools ready again", |h| h.components.ytdlp.ready()).await;
    h.wait_for("completed after relaunch", |h| h.state("generic_direct") == Some(JobState::Completed)).await;
    assert_eq!(h.job("generic_direct").id, id);
}

#[tokio::test]
async fn the_queue_order_is_the_users_and_survives_a_relaunch() {
    let mut h = ready(Setup { settings: Box::new(|s| s.max_concurrent = 1), line_ms: 200, ..Setup::default() }).await;
    let links: Vec<String> = (1..=3).map(|n| format!("{}?n={n}", fake_url("hang"))).collect();
    h.engine.add_links(links.join(" "), Source::Paste).await.unwrap();
    h.wait_for("three jobs", |h| h.jobs.len() == 3).await;
    let ids: Vec<String> = h.jobs.iter().map(|j| j.id.clone()).collect();
    h.engine.pause_all();
    h.wait_for("all paused", |h| h.jobs.iter().all(|j| j.state == JobState::Paused)).await;
    h.engine.jobs_reorder(vec![ids[2].clone(), ids[0].clone(), ids[1].clone()]);
    h.wait_for("reordered", |h| h.jobs[0].id == ids[2]).await;

    let h = h.restart().await;
    let order: Vec<&str> = h.jobs.iter().map(|j| j.id.as_str()).collect();
    assert_eq!(order, [ids[2].as_str(), ids[0].as_str(), ids[1].as_str()]);
    assert!(h.jobs.iter().all(|j| j.state == JobState::Paused), "paused stays paused");
}

#[tokio::test]
async fn the_preview_shows_what_a_preset_really_produces() {
    let mut h = ready(Setup { settings: Box::new(|s| s.on_add = OnAdd::Preview), ..Setup::default() }).await;
    h.engine.add_links(fake_url("video_audio"), Source::Paste).await.unwrap();
    h.wait_for(
        "exact resolution",
        |h| matches!(&h.preview, Some(Preview { status: PreviewStatus::Ready { resolved, .. }, .. }) if resolved.exact),
    )
    .await;
    let preview = h.preview.clone().unwrap();
    let PreviewStatus::Ready { info, resolved } = &preview.status else { unreachable!() };
    assert_eq!(info.title, "Caminandes 3: Llamigos");
    // From the recorded `--simulate` run, not from the estimate.
    assert_eq!(resolved.format_ids, ["136", "140"]);
    assert_eq!((resolved.height, resolved.source_height, resolved.bytes), (Some(720), Some(1080), Some(14_017_680.0)));
    assert!(h.jobs.is_empty(), "nothing downloads until the user says so");
    let (_, _, args) = h.log().into_iter().find(|(m, ..)| m == "simulate").unwrap();
    assert!(args.contains(&"--load-info-json".to_string()) && !args.contains(&"--".to_string()));

    h.engine.preview_start(preview.id.clone(), preview.request.clone(), vec![], false).await.unwrap();
    h.wait_for("completed", |h| h.state("video_audio") == Some(JobState::Completed)).await;
    assert!(h.preview.is_none());
    assert_eq!(h.job("video_audio").planned.as_ref().map(|p| p.exact), Some(true));
    assert_eq!(h.modes("video_audio"), ["probe", "simulate", "download"], "no second probe");
}

#[tokio::test]
async fn a_playlist_offers_its_entries_and_downloads_a_selection_into_its_own_folder() {
    let mut h = ready(Setup { settings: Box::new(|s| s.on_add = OnAdd::Download), ..Setup::default() }).await;
    std::fs::write(h.root.join("data").join("archive.txt"), "youtube SkVqJ1SGeL0\n").unwrap();
    h.engine.add_links(fake_url("playlist"), Source::Paste).await.unwrap();
    h.wait_for("all entries", |h| {
        matches!(&h.preview, Some(Preview { status: PreviewStatus::Playlist { complete: true, .. }, .. }))
    })
    .await;
    let preview = h.preview.clone().unwrap();
    let PreviewStatus::Playlist { title, entries, .. } = &preview.status else { unreachable!() };
    assert_eq!((title.as_str(), entries.len()), ("Official Blender Open Movies", 17));
    assert!(h.jobs.is_empty(), "a playlist always asks first, even in download-now mode");
    let downloaded: Vec<&str> = entries.iter().filter(|e| e.downloaded).map(|e| e.id.as_str()).collect();
    assert_eq!(downloaded, ["SkVqJ1SGeL0"], "the download archive marks what is already saved");

    let pick = vec![entries[0].id.clone(), entries[9].id.clone(), entries[16].id.clone()];
    h.engine.preview_start(preview.id, DownloadRequest::default(), pick, true).await.unwrap();
    h.wait_for("three completed", |h| h.jobs.len() == 3 && h.jobs.iter().all(|j| j.state == JobState::Completed)).await;

    let folder = h.downloads().join("Official Blender Open Movies");
    for job in &h.jobs {
        assert!(job.output.starts_with(&*folder.to_string_lossy()), "{}", job.output);
        assert_eq!(job.group.as_ref().map(|g| g.count), Some(17));
    }
    let templates: Vec<String> = h
        .log()
        .into_iter()
        .filter(|(m, ..)| m == "download")
        .map(|(.., args)| {
            assert!(args.contains(&"--download-archive".to_string()));
            args.windows(2).find(|w| w[0] == "-o").map(|w| w[1].clone()).unwrap()
        })
        .collect();
    for prefix in ["01 - ", "10 - ", "17 - "] {
        assert!(templates.iter().any(|t| t.starts_with(prefix)), "{prefix} in {templates:?}");
    }
    // Fifty finished items would be fifty notifications; a playlist gets one.
    match h.notices().as_slice() {
        [Notice::GroupFinished { completed: 3, failed: 0, title, .. }] => {
            assert_eq!(title, "Official Blender Open Movies")
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_link_that_is_both_a_video_and_a_playlist_asks_which() {
    let mut h = ready(Setup { settings: Box::new(|s| s.on_add = OnAdd::Preview), ..Setup::default() }).await;
    h.engine.add_links(format!("{}?v=SkVqJ1SGeL0&list=PL1", fake_url("video_audio")), Source::Paste).await.unwrap();
    h.wait_for("the question", |h| matches!(&h.preview, Some(p) if p.status == PreviewStatus::Ambiguous)).await;
    assert!(h.log().is_empty(), "nothing is probed before the user answers");
    let id = h.preview.as_ref().unwrap().id.clone();
    h.engine.preview_choose(id, PlaylistChoice::Video);
    h.wait_for("the video", |h| matches!(&h.preview, Some(Preview { status: PreviewStatus::Ready { .. }, .. }))).await;
    let (_, _, args) = h.log().into_iter().find(|(m, ..)| m == "probe").unwrap();
    assert!(args.contains(&"--no-playlist".to_string()));

    h.engine.add_links(format!("{}?v=x&list=PL1", fake_url("playlist")), Source::Paste).await.unwrap();
    h.wait_for("the question again", |h| matches!(&h.preview, Some(p) if p.status == PreviewStatus::Ambiguous)).await;
    let id = h.preview.as_ref().unwrap().id.clone();
    h.engine.preview_choose(id, PlaylistChoice::Playlist);
    h.wait_for("the playlist", |h| matches!(&h.preview, Some(Preview { status: PreviewStatus::Playlist { .. }, .. })))
        .await;
    let (_, _, args) = h.log().into_iter().rfind(|(m, ..)| m == "probe").unwrap();
    assert!(args.contains(&"--yes-playlist".to_string()));
}

#[tokio::test]
async fn deep_links_only_open_a_preview_and_only_take_known_presets() {
    let mut h = ready(Setup { settings: Box::new(|s| s.on_add = OnAdd::Download), ..Setup::default() }).await;
    h.engine.open_link(fake_url("video_audio"), Some("mp3".into()), Source::DeepLink).await.unwrap();
    h.wait_for("preview", |h| matches!(&h.preview, Some(Preview { status: PreviewStatus::Ready { .. }, .. }))).await;
    h.settle(300).await;
    assert!(h.jobs.is_empty(), "a deep link never starts a download, whatever the setting");
    assert_eq!(h.preview.as_ref().unwrap().request.preset, Preset::Audio { format: AudioFormat::Mp3 });

    let first = h.preview.as_ref().unwrap().id.clone();
    h.engine.open_link(fake_url("video_audio"), Some("--exec calc".into()), Source::DeepLink).await.unwrap();
    h.wait_for(
        "second preview",
        |h| matches!(&h.preview, Some(Preview { id, status: PreviewStatus::Ready { .. }, .. }) if *id != first),
    )
    .await;
    assert_eq!(h.preview.as_ref().unwrap().request, h.settings.default_request, "an unknown preset is ignored");
    assert!(h.engine.open_link("file:///C:/Windows/win.ini".into(), None, Source::DeepLink).await.is_err());
}

#[tokio::test]
async fn several_links_and_bad_input() {
    let mut h = ready(Setup::default()).await;
    assert!(h.engine.add_links("no links here, just words".into(), Source::Paste).await.is_err());
    assert!(h.engine.add_links("javascript:alert(1) file:///etc/passwd".into(), Source::Paste).await.is_err());
    let n = h
        .engine
        .add_links(format!("{0}?a=1\n{0}?a=2 {0}?a=1", fake_url("generic_direct")), Source::Drop)
        .await
        .unwrap();
    assert_eq!(n, 2, "duplicates in one paste count once");
    h.wait_for("both completed", |h| h.jobs.len() == 2 && h.jobs.iter().all(|j| j.state == JobState::Completed)).await;
    assert!(h.jobs.iter().all(|j| j.source == Source::Drop));

    h.engine.clear_completed();
    h.wait_for("cleared", |h| h.jobs.is_empty()).await;
    let page = h.engine.history(String::new(), HistoryFilter::All, 0, 10).await;
    assert_eq!(page.total, 2, "cleared from Downloads, still in History");
    let first = page.items[0].id.clone();
    h.engine.history_remove(vec![first]);
    h.settle(200).await;
    assert_eq!(h.engine.history(String::new(), HistoryFilter::All, 0, 10).await.total, 1);
}

#[tokio::test]
async fn missing_tools_say_so_and_installing_fixes_the_job() {
    let mut h = Harness::new(Setup {
        settings: Box::new(|s| s.custom_ytdlp = r"C:\definitely\not\here\yt-dlp.exe".into()),
        ..Setup::default()
    })
    .await;
    h.wait_for("components checked", |h| h.seen.iter().any(|c| matches!(c, Change::Components(_)))).await;
    assert_eq!(h.components.ytdlp.status, ToolStatus::Missing);
    assert!(h.components.ffmpeg.ready());
    h.engine.add_links(fake_url("generic_direct"), Source::Paste).await.unwrap();
    h.wait_for("preview fails", |h| matches!(&h.preview, Some(Preview { status: PreviewStatus::Failed { .. }, .. })))
        .await;
    let Some(Preview { status: PreviewStatus::Failed { error }, .. }) = &h.preview else { unreachable!() };
    assert_eq!((error.kind, error.fix), (ErrorKind::YtdlpMissing, Some(Fix::InstallComponents)));

    // Pointing the setting at a working yt-dlp is picked up without a restart.
    let mut settings = h.settings.clone();
    settings.custom_ytdlp = h.root.join("tools").join(common::exe("yt-dlp")).to_string_lossy().into_owned();
    h.engine.settings_set(settings).await.unwrap();
    h.wait_for("ready", |h| h.components.ytdlp.ready()).await;
    assert_eq!(h.components.ytdlp.origin, ToolOrigin::Custom);
    assert_eq!(h.components.ytdlp.version, "2026.10.26");
}

#[tokio::test]
async fn copy_details_has_versions_and_the_command_but_no_secrets() {
    let mut h = ready(Setup {
        settings: Box::new(|s| s.proxy = "http://bob:hunter2@127.0.0.1:3128".into()),
        ..Setup::default()
    })
    .await;
    // One link fails in the preview; two go to the queue and fail there.
    h.engine.add_links(format!("{}?token=s3cret", fake_url("http429")), Source::Paste).await.unwrap();
    h.wait_for("preview failed", |h| matches!(&h.preview, Some(Preview { status: PreviewStatus::Failed { .. }, .. })))
        .await;
    let details = h.engine.job_details(h.preview.as_ref().unwrap().id.clone()).await.unwrap();
    assert!(details.contains("Http429") && details.contains("Command: yt-dlp"), "{details}");
    assert!(!details.contains("s3cret") && !details.contains("hunter2"), "{details}");

    h.engine.add_links(format!("{0}?token=s3cret {0}?n=2", fake_url("http429")), Source::Paste).await.unwrap();
    h.wait_for("failed", |h| h.jobs.len() == 2 && h.jobs.iter().all(|j| j.state == JobState::Failed)).await;
    let details = h.engine.job_details(h.job("s3cret").id.clone()).await.unwrap();
    assert!(details.contains("yt-dlp 2026.10.26 (custom)"), "{details}");
    assert!(details.contains("Http429"), "{details}");
    assert!(details.contains("--ignore-config"), "{details}");
    for secret in ["hunter2", "s3cret"] {
        assert!(!details.contains(secret), "{secret} leaked:\n{details}");
    }
}
