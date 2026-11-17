//! Downloads one link from the command line with live progress, using the real engine.
//!
//! ```text
//! cargo run -p engine --example dl -- <url> [--preset mp4-720] [--format 137+140] [--dir <folder>] [--install]
//! ```
//!
//! Tools come from PATH unless `--install` is given, which downloads managed copies into
//! `.dev/bin` first. `LOVELYDL_YTDLP` overrides the yt-dlp binary (used to record fixtures).

use std::io::Write;
use std::path::PathBuf;

use engine::model::{Change, FormatPick, JobState, OnAdd, Preset, Source, Stage, ToolStatus};
use engine::{Config, Dirs, Engine};

fn size(bytes: f64) -> String {
    match bytes {
        b if b >= 1e9 => format!("{:.2} GB", b / 1e9),
        b if b >= 1e6 => format!("{:.1} MB", b / 1e6),
        b => format!("{:.0} KB", b / 1e3),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (mut url, mut preset, mut format, mut dir, mut install) = (None, None, None::<String>, None, false);
    let (mut subs, mut section, mut sponsor, mut rate) =
        (None::<String>, None::<String>, None::<String>, None::<String>);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--subs" => subs = args.next(),
            "--section" => section = args.next(),
            "--sponsorblock" => sponsor = args.next(),
            "--rate" => rate = args.next(),
            "--preset" => preset = args.next(),
            "--format" => format = args.next(),
            "--dir" => dir = args.next(),
            "--install" => install = true,
            _ => url = Some(arg),
        }
    }
    let Some(url) = url else {
        eprintln!(
            "usage: dl <url> [--preset best|mp4|mp4-720|mp3|…] [--format 137+140] [--subs en,fr] [--section 1:30-2:00] [--sponsorblock mark|remove] [--rate 500K] [--dir <folder>] [--install]"
        );
        std::process::exit(2);
    };

    let root = std::env::var("LOVELYDL_DEV_DIR").map_or_else(|_| PathBuf::from(".dev"), PathBuf::from);
    let root = std::path::absolute(root)?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<Change>>();
    let engine = Engine::start(Config {
        dirs: Dirs::under(&root),
        sink: Box::new(move |changes| {
            let _ = tx.send(changes);
        }),
        user_agent: concat!("lovelyDL/", env!("CARGO_PKG_VERSION"), " (dl example)").into(),
        api: None,
        retry_backoff: Config::DEFAULT_RETRY_BACKOFF,
    })?;

    let mut settings = engine.snapshot().await?.settings;
    settings.on_add = OnAdd::Download;
    settings.use_system_tools = !install;
    settings.custom_ytdlp = std::env::var("LOVELYDL_YTDLP").unwrap_or_default();
    if let Some(dir) = dir {
        settings.download_dir = std::path::absolute(dir)?.to_string_lossy().into_owned();
    }
    if let Some(name) = &preset {
        settings.default_request.preset = Preset::from_name(name).ok_or("unknown preset name")?;
    }
    if let Some(format) = &format {
        let (video, audio) = format.split_once('+').map_or((format.as_str(), None), |(v, a)| (v, Some(a)));
        settings.default_request.formats =
            Some(FormatPick { video: Some(video.into()), audio: audio.map(String::from) });
    }
    let options = &mut settings.default_request.options;
    if let Some(langs) = subs {
        options.subtitles = true;
        options.sub_langs = langs.split(',').map(String::from).collect();
    }
    if let Some((start, end)) = section.as_deref().and_then(|s| s.split_once('-')) {
        options.section = Some(engine::model::Section { start: start.into(), end: end.into() });
    }
    match sponsor.as_deref() {
        Some("mark") => options.sponsorblock = engine::model::SponsorBlock::Mark,
        Some("remove") => options.sponsorblock = engine::model::SponsorBlock::Remove,
        _ => {}
    }
    if let Some(rate) = rate {
        settings.rate_limit = rate;
    }
    engine.settings_set(settings).await?;

    if install {
        println!("Installing tools into {} …", root.join("bin").display());
        engine.components_install_missing().await?;
    }
    engine.add_links(url, Source::Typed).await?;

    let mut announced = false;
    let mut last_stage = Stage::Waiting;
    let mut outcome = None;
    while outcome.is_none() {
        let Some(changes) = rx.recv().await else { break };
        for change in changes {
            match change {
                Change::Components(c) if !announced => {
                    for tool in [&c.ytdlp, &c.ffmpeg, &c.deno] {
                        match &tool.status {
                            ToolStatus::Ready => {
                                println!("{:?} {} ({:?}) {}", tool.id, tool.version, tool.origin, tool.path)
                            }
                            ToolStatus::Installing { fraction } => {
                                print!("\r{:?}: installing {:>3.0}%   ", tool.id, fraction * 100.0)
                            }
                            other => println!("{:?}: {other:?}", tool.id),
                        }
                    }
                    announced = [&c.ytdlp, &c.ffmpeg, &c.deno].iter().all(|t| t.status == ToolStatus::Ready);
                }
                Change::Preview(Some(preview)) => {
                    if let engine::model::PreviewStatus::Failed { error } = &preview.status {
                        outcome = Some(Err(format!("{:?}: {}", error.kind, error.detail)));
                    }
                }
                Change::JobAdded(job) | Change::JobUpdated(job) => {
                    match job.state {
                        JobState::Probing => println!("Probing {}", job.url),
                        JobState::Downloading if job.progress.stage == Stage::Preparing => {
                            let plan = job.planned.as_ref();
                            println!(
                                "{} — {} · planned {} {}",
                                job.title,
                                job.uploader,
                                plan.map_or(String::new(), |p| p.format_ids.join("+")),
                                plan.and_then(|p| p.bytes).map_or(String::new(), size),
                            );
                        }
                        JobState::Completed => {
                            outcome = Some(Ok(format!("{} ({})", job.output, job.bytes.map_or("?".into(), size))))
                        }
                        JobState::Failed => {
                            outcome = Some(Err(job.error.map_or("failed".into(), |e| {
                                format!("{:?} (fix: {:?}): {}", e.kind, e.fix, e.detail)
                            })));
                        }
                        _ => {}
                    }
                    if job.progress.stage != last_stage && job.state == JobState::PostProcessing {
                        last_stage = job.progress.stage;
                        println!("\n{:?}", job.progress.stage);
                    }
                }
                Change::Progress { progress, .. } => {
                    if progress.stage != last_stage {
                        last_stage = progress.stage;
                        println!("\n{:?} (stage {} of {})", progress.stage, progress.stage_index, progress.stage_count);
                    }
                    print!(
                        "\r{:>5.1}%  {:>10}  {:>10}/s  eta {:>4.0}s   ",
                        progress.fraction.unwrap_or(0.0) * 100.0,
                        size(progress.downloaded),
                        progress.speed.map_or("-".into(), size),
                        progress.eta.unwrap_or(0.0),
                    );
                    let _ = std::io::stdout().flush();
                }
                _ => {}
            }
        }
    }
    engine.shutdown().await;
    match outcome {
        Some(Ok(file)) => {
            println!("\nSaved {file}");
            Ok(())
        }
        Some(Err(error)) => Err(format!("\nFailed: {error}").into()),
        None => Err("the engine stopped".into()),
    }
}
