//! The real yt-dlp and ffmpeg, end to end, without the internet: a small video served from a
//! local HTTP server goes through yt-dlp's generic extractor. Skipped when the tools are not
//! on PATH (CI installs them).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use engine::model::*;
use engine::toolchain::{exe, on_path};
use engine::{Config, Dirs, Engine};

const VIDEO: &[u8] = include_bytes!("fixtures/media/tiny.mp4");

#[tokio::test]
async fn a_local_video_goes_through_the_real_yt_dlp() {
    if on_path(&exe(ToolId::Ytdlp)).is_none() || on_path(&exe(ToolId::Ffmpeg)).is_none() {
        eprintln!("skipped: yt-dlp or ffmpeg is not on PATH");
        return;
    }
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let link = format!("http://{}/clip.mp4", server.server_addr().to_ip().unwrap());
    std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let header = tiny_http::Header::from_bytes("Content-Type", "video/mp4").unwrap();
            let _ = request.respond(tiny_http::Response::from_data(VIDEO).with_header(header));
        }
    });

    let dir = tempfile::tempdir().unwrap();
    let mut settings = Settings {
        onboarding_done: true,
        on_add: OnAdd::Download,
        use_system_tools: true,
        download_dir: dir.path().join("out").to_string_lossy().into_owned(),
        last_tool_check: engine::engine::now_ms(),
        ..Settings::default()
    };
    settings.default_request.preset = Preset::Best { max_height: None };
    engine::settings::save(&dir.path().join("data").join("settings.json"), &settings).unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let engine = Engine::start(Config {
        dirs: Dirs::under(dir.path()),
        sink: Box::new(move |changes| {
            let _ = tx.send(changes);
        }),
        user_agent: "lovelyDL/test".into(),
        api: Some("http://127.0.0.1:9".into()),
        retry_backoff: Config::DEFAULT_RETRY_BACKOFF,
    })
    .unwrap();
    engine.add_links(link, Source::Paste).await.unwrap();

    let finished = tokio::time::timeout(Duration::from_secs(120), async {
        loop {
            for change in rx.recv().await.expect("engine runs") {
                match change {
                    Change::JobUpdated(job) if job.state == JobState::Completed => return Ok(*job),
                    Change::JobUpdated(job) if job.state == JobState::Failed => return Err(job.error),
                    Change::Preview(Some(p)) => {
                        if let PreviewStatus::Failed { error } = p.status {
                            return Err(Some(error));
                        }
                    }
                    _ => {}
                }
            }
        }
    })
    .await
    .expect("finishes within two minutes");
    let job = finished.unwrap_or_else(|e| panic!("download failed: {e:?}"));
    assert_eq!(job.title, "clip");
    let saved = std::fs::metadata(&job.output).unwrap();
    // Metadata is embedded, so the file is the source plus a little.
    assert!(saved.len() as usize >= VIDEO.len(), "{} bytes", saved.len());
    assert!(job.output.ends_with("clip [clip].mp4"), "{}", job.output);
    engine.shutdown().await;
}
