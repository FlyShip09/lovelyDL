//! Tool installs against a local stand-in for the GitHub API: checksums, atomic switch,
//! rollback, and archives that try to write outside their folder.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

use common::exe;
use engine::model::{Channel, ToolId};
use engine::toolchain::{ToolError, Toolchain};
use sha2::{Digest, Sha256};

type Routes = Arc<Mutex<HashMap<String, Vec<u8>>>>;

/// Serves whatever is in `routes` by path; anything else is a 404.
fn serve() -> (String, Routes) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let base = format!("http://{}", server.server_addr().to_ip().unwrap());
    let routes: Routes = Arc::default();
    let shared = routes.clone();
    std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let body = shared.lock().unwrap().get(request.url()).cloned();
            let _ = match body {
                Some(body) => request.respond(tiny_http::Response::from_data(body)),
                None => request.respond(tiny_http::Response::empty(404)),
            };
        }
    });
    (base, routes)
}

fn zip_with(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in files {
        writer.start_file(*name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// Publishes a yt-dlp release the way yt-dlp does: the zip plus a `SHA2-256SUMS` file.
fn publish_ytdlp(base: &str, routes: &Routes, version: &str, zip: &[u8], published_sum: &str) {
    let mut r = routes.lock().unwrap();
    let release = serde_json::json!({
        "tag_name": version,
        "assets": [
            { "name": "SHA2-256SUMS", "browser_download_url": format!("{base}/dl/{version}/SHA2-256SUMS"), "size": 100, "digest": null },
            { "name": "yt-dlp_win.zip", "browser_download_url": format!("{base}/dl/{version}/yt-dlp_win.zip"), "size": zip.len(), "digest": null },
            { "name": "yt-dlp_macos", "browser_download_url": format!("{base}/dl/x"), "size": 1, "digest": null },
        ]
    });
    r.insert("/repos/yt-dlp/yt-dlp/releases/latest".into(), release.to_string().into_bytes());
    r.insert(
        format!("/dl/{version}/SHA2-256SUMS"),
        format!("{published_sum}  yt-dlp_win.zip\n0000  yt-dlp\n").into_bytes(),
    );
    r.insert(format!("/dl/{version}/yt-dlp_win.zip"), zip.to_vec());
}

fn toolchain(base: &str) -> (tempfile::TempDir, Toolchain) {
    let dir = tempfile::tempdir().unwrap();
    let mut toolchain = Toolchain::new(dir.path().join("bin"), "lovelyDL/test");
    toolchain.api = base.to_string();
    (dir, toolchain)
}

#[tokio::test]
async fn installs_verify_switch_atomically_and_roll_back() {
    let (base, routes) = serve();
    let (_dir, tc) = toolchain(&base);
    let binary = std::fs::read(common::fake()).unwrap();
    let zip =
        zip_with(&[(exe("yt-dlp").as_str(), binary.as_slice()), ("_internal/base_library.zip", &b"python stdlib"[..])]);

    publish_ytdlp(&base, &routes, "2026.10.26", &zip, &sha(&zip));
    let release = tc.latest(ToolId::Ytdlp, Channel::Stable).await.unwrap();
    assert_eq!((release.version.as_str(), release.sha256.clone()), ("2026.10.26", sha(&zip)));
    let fractions = Arc::new(Mutex::new(Vec::new()));
    let seen = fractions.clone();
    let first = tc.install(ToolId::Ytdlp, &release, move |f| seen.lock().unwrap().push(f)).await.unwrap();
    assert!(
        first.ends_with(format!("2026.10.26/{}", exe("yt-dlp")))
            || first.ends_with(format!("2026.10.26\\{}", exe("yt-dlp")))
    );
    assert!(first.with_file_name("_internal").join("base_library.zip").is_file(), "the whole folder is unpacked");
    assert_eq!(fractions.lock().unwrap().last().copied(), Some(1.0));
    assert_eq!(tc.managed(ToolId::Ytdlp), Some(first.clone()));

    // A second version becomes current; the first is kept for rollback.
    publish_ytdlp(&base, &routes, "2026.11.30", &zip, &sha(&zip));
    let release = tc.latest(ToolId::Ytdlp, Channel::Stable).await.unwrap();
    let second = tc.install(ToolId::Ytdlp, &release, |_| {}).await.unwrap();
    assert_eq!(tc.managed(ToolId::Ytdlp), Some(second.clone()));
    assert!(first.is_file());
    assert_eq!(tc.rollback(ToolId::Ytdlp).await.unwrap(), first);
    assert_eq!(tc.managed(ToolId::Ytdlp), Some(first));

    // No leftovers from staging or downloading.
    let names: Vec<String> = std::fs::read_dir(tc.bin.join("yt-dlp"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().all(|n| !n.starts_with('.')), "{names:?}");
}

#[tokio::test]
async fn a_download_that_does_not_match_its_checksum_is_thrown_away() {
    let (base, routes) = serve();
    let (_dir, tc) = toolchain(&base);
    let zip = zip_with(&[(exe("yt-dlp").as_str(), &b"tampered"[..])]);
    publish_ytdlp(&base, &routes, "2026.10.26", &zip, &"ab".repeat(32));
    let release = tc.latest(ToolId::Ytdlp, Channel::Stable).await.unwrap();
    let error = tc.install(ToolId::Ytdlp, &release, |_| {}).await.unwrap_err();
    assert!(matches!(error, ToolError::Checksum { .. }), "{error}");
    assert_eq!(tc.managed(ToolId::Ytdlp), None);
    assert!(std::fs::read_dir(tc.bin.join("yt-dlp")).unwrap().next().is_none(), "nothing is left behind");
}

#[tokio::test]
async fn a_tool_that_does_not_run_is_not_installed() {
    let (base, routes) = serve();
    let (_dir, tc) = toolchain(&base);
    let zip = zip_with(&[(exe("yt-dlp").as_str(), &b"MZ this is not a program"[..])]);
    publish_ytdlp(&base, &routes, "2026.10.26", &zip, &sha(&zip));
    let release = tc.latest(ToolId::Ytdlp, Channel::Stable).await.unwrap();
    let error = tc.install(ToolId::Ytdlp, &release, |_| {}).await.unwrap_err();
    assert!(matches!(error, ToolError::Unhealthy(_)), "{error}");
    assert_eq!(tc.managed(ToolId::Ytdlp), None);
}

#[tokio::test]
async fn entries_that_escape_the_folder_are_skipped() {
    let (base, routes) = serve();
    let (dir, tc) = toolchain(&base);
    let binary = std::fs::read(common::fake()).unwrap();
    let zip = zip_with(&[
        ("../../escaped.txt", &b"x"[..]),
        ("/abs.txt", &b"x"[..]),
        (exe("yt-dlp").as_str(), binary.as_slice()),
    ]);
    publish_ytdlp(&base, &routes, "1.0", &zip, &sha(&zip));
    let release = tc.latest(ToolId::Ytdlp, Channel::Stable).await.unwrap();
    tc.install(ToolId::Ytdlp, &release, |_| {}).await.unwrap();
    assert!(!dir.path().join("escaped.txt").exists() && !tc.bin.join("escaped.txt").exists());
}

#[tokio::test]
async fn deno_and_ffmpeg_use_the_digest_github_publishes_and_ffmpeg_keeps_two_binaries() {
    let (base, routes) = serve();
    let (_dir, tc) = toolchain(&base);
    let binary = std::fs::read(common::fake()).unwrap();
    let zip = zip_with(&[
        (&format!("ffmpeg-9.0.2-essentials_build/bin/{}", exe("ffmpeg")), binary.as_slice()),
        (&format!("ffmpeg-9.0.2-essentials_build/bin/{}", exe("ffprobe")), binary.as_slice()),
        (&format!("ffmpeg-9.0.2-essentials_build/bin/{}", exe("ffplay")), binary.as_slice()),
        ("ffmpeg-9.0.2-essentials_build/doc/ffmpeg.html", &b"docs"[..]),
    ]);
    let release = |asset: &str, digest: Option<String>| {
        serde_json::json!({ "tag_name": "9.0.2", "assets": [{
            "name": asset, "browser_download_url": format!("{base}/dl/{asset}"), "size": zip.len(),
            "digest": digest.map(|d| format!("sha256:{d}")),
        }]})
        .to_string()
        .into_bytes()
    };
    {
        let mut r = routes.lock().unwrap();
        r.insert(
            "/repos/GyanD/codexffmpeg/releases/latest".into(),
            release("ffmpeg-9.0.2-essentials_build.zip", Some(sha(&zip))),
        );
        r.insert("/dl/ffmpeg-9.0.2-essentials_build.zip".into(), zip.clone());
        r.insert("/repos/denoland/deno/releases/latest".into(), release("deno-x86_64-pc-windows-msvc.zip", None));
    }
    let ff = tc.latest(ToolId::Ffmpeg, Channel::Stable).await.unwrap();
    let path = tc.install(ToolId::Ffmpeg, &ff, |_| {}).await.unwrap();
    let mut kept: Vec<String> = std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    kept.sort();
    assert_eq!(kept, [exe("ffmpeg"), exe("ffprobe")]);
    // No checksum published at all: refuse rather than install unverified.
    assert!(matches!(tc.latest(ToolId::Deno, Channel::Stable).await, Err(ToolError::NoChecksum)));
}

#[tokio::test]
async fn an_unreachable_or_limiting_server_is_reported_plainly() {
    let (base, routes) = serve();
    let (_dir, tc) = toolchain("http://127.0.0.1:9");
    assert!(matches!(tc.latest(ToolId::Ytdlp, Channel::Stable).await, Err(ToolError::Network(_))));
    let (_dir, tc) = toolchain(&base);
    drop(routes);
    assert!(
        matches!(tc.latest(ToolId::Ytdlp, Channel::Nightly).await, Err(ToolError::Network(_))),
        "404 is a network error"
    );
}
