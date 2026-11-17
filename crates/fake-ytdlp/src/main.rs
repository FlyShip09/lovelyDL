//! A test double for yt-dlp.
//!
//! **Replay** (the default): prints a recorded session. The fixture is chosen by the link:
//! `https://fake.test/<name>` plays `$FAKE_YTDLP_FIXTURES/<name>/<mode>.txt`, where `<mode>`
//! is `probe`, `simulate`, `filename` or `download`, worked out from the arguments. It
//! creates the files a real run would (partial files in the temp folder, the final file).
//!
//! **Record**: with `FAKE_YTDLP_RECORD=<fixtures dir>`, `FAKE_YTDLP_FIXTURE=<name>` and
//! `FAKE_YTDLP_REAL=<path to yt-dlp>` it runs the real tool with the same arguments, passes
//! the output through, and saves a scrubbed copy. See `scripts/capture-fixture.sh`.
//!
//! `FAKE_YTDLP_LINE_MS` slows replay down (a pause after every line) and `FAKE_YTDLP_LOG`
//! names a file that gets one line per run: mode, fixture and the arguments as JSON.
//!
//! Fixture lines: `out <text>`, `err <text>`, `sleep <ms>`, `exit <code>`, `hang`, `crash`,
//! `# comment`. In text, `{home}`, `{temp}` and `{url}` stand for this run's folders and link.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::{Arc, Mutex};

use serde_json::Value;

const VERSION: &str = "2026.10.26";

/// A setting: an environment variable, or a `NAME=value` line in `fake-ytdlp.env` next to
/// this executable. The file lets each test configure its own copy without touching the
/// process environment that parallel tests share.
fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty()).or_else(|| {
        let beside = std::env::current_exe().ok()?.with_file_name("fake-ytdlp.env");
        let text = std::fs::read_to_string(beside).ok()?;
        text.lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix('=').map(|v| v.trim().to_string()))
            .filter(|v| !v.is_empty())
    })
}

fn mode(args: &[String]) -> &'static str {
    let has = |flag: &str| args.iter().any(|a| a == flag);
    if has("-version") {
        "ffmpeg"
    } else if has("--version") {
        "version"
    } else if has("-j") || has("-J") {
        "probe"
    } else if has("--simulate") && args.windows(2).any(|w| w[0] == "--print" && w[1] == "filename") {
        "filename"
    } else if has("--simulate") {
        "simulate"
    } else {
        "download"
    }
}

/// The value given to `-P home:<dir>` or `-P temp:<dir>`.
fn path_arg(args: &[String], kind: &str) -> Option<String> {
    args.windows(2).find_map(|w| (w[0] == "-P").then(|| w[1].strip_prefix(kind)).flatten().map(String::from))
}

fn value_after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].as_str())
}

/// The link this run is about: the argument after `--`, or the one the cached info names.
fn link(args: &[String]) -> Option<String> {
    if let Some(at) = args.iter().position(|a| a == "--") {
        return args.get(at + 1).cloned();
    }
    let info = std::fs::read_to_string(value_after(args, "--load-info-json")?).ok()?;
    let json: Value = serde_json::from_str(&info).ok()?;
    json.get("original_url").and_then(Value::as_str).map(String::from)
}

/// `https://fake.test/<name>/…` names its fixture; any other link uses `$FAKE_YTDLP_FIXTURE`.
fn fixture_name(link: Option<&str>) -> String {
    link.and_then(|l| l.strip_prefix("https://fake.test/"))
        .map(|rest| rest.split(['/', '?', '#']).next().unwrap_or_default().to_string())
        .filter(|name| !name.is_empty())
        .or_else(|| env("FAKE_YTDLP_FIXTURE"))
        .unwrap_or_else(|| "default".into())
}

/// A path as it appears inside a JSON string.
fn json_escaped(path: &str) -> String {
    let quoted = serde_json::to_string(path).unwrap_or_default();
    quoted.trim_matches('"').to_string()
}

fn touch(path: &str, bytes: usize) {
    let path = Path::new(path);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, vec![b'x'; bytes]);
}

/// Does on disk what the real tool would have done by the time it printed this line.
fn side_effects(text: &str) {
    if let Some(json) = text.strip_prefix("@@lovelydl@@FILE ") {
        if let Ok(file) = serde_json::from_str::<String>(json) {
            touch(&file, 1024);
        }
    } else if let Some(json) = text.strip_prefix("@@lovelydl@@DL ")
        && let Ok(line) = serde_json::from_str::<Value>(json)
    {
        let progress = &line["p"];
        let name = progress["tmpfilename"].as_str().or_else(|| progress["filename"].as_str());
        if let Some(name) = name {
            touch(name, 16);
        }
    }
}

fn replay(args: &[String]) -> ExitCode {
    let mode = mode(args);
    let link = link(args);
    let name = fixture_name(link.as_deref());
    // Tests read this to see what was run, and with which arguments.
    if let Some(log) = env("FAKE_YTDLP_LOG")
        && let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(log)
    {
        let _ = writeln!(file, "{mode} {name} {}", serde_json::to_string(args).unwrap_or_default());
    }
    let root = PathBuf::from(env("FAKE_YTDLP_FIXTURES").unwrap_or_else(|| "fixtures".into()));
    let script = match std::fs::read_to_string(root.join(&name).join(format!("{mode}.txt"))) {
        Ok(script) => script,
        // Sensible answers for the short questions, so a fixture only needs what it tests.
        Err(_) => match mode {
            "version" => format!("out {}\n", env("FAKE_YTDLP_VERSION").unwrap_or_else(|| VERSION.into())),
            "ffmpeg" => "out ffmpeg version 9.0.2-fake Copyright (c) the FFmpeg developers\n".into(),
            "simulate" => String::new(),
            "filename" => "out Fake title [fake].mp4\n".into(),
            _ => format!("err ERROR: fake-ytdlp has no fixture {name}/{mode}.txt\nexit 1\n"),
        },
    };
    let line_delay = env("FAKE_YTDLP_LINE_MS").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    let home = json_escaped(&path_arg(args, "home:").unwrap_or_default());
    let temp = json_escaped(&path_arg(args, "temp:").unwrap_or_default());
    let url = link.unwrap_or_default();
    let (stdout, stderr) = (std::io::stdout(), std::io::stderr());
    for line in script.lines() {
        let (verb, rest) = line.split_once(' ').unwrap_or((line, ""));
        let text = || rest.replace("{home}", &home).replace("{temp}", &temp).replace("{url}", &url);
        match verb {
            "out" | "err" => {
                let text = text();
                side_effects(&text);
                // Flush every line: the engine reads these as they arrive.
                if verb == "out" {
                    let mut out = stdout.lock();
                    let _ = writeln!(out, "{text}").and_then(|()| out.flush());
                } else {
                    let _ = writeln!(stderr.lock(), "{text}");
                }
                std::thread::sleep(std::time::Duration::from_millis(line_delay));
            }
            "sleep" => std::thread::sleep(std::time::Duration::from_millis(rest.parse().unwrap_or(0))),
            "exit" => return ExitCode::from(rest.parse::<u8>().unwrap_or(1)),
            "hang" => loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
            },
            "crash" => std::process::abort(),
            _ => {}
        }
    }
    ExitCode::SUCCESS
}

/// Keys that carry signed links, addresses or bulk that a fixture does not need.
const DROP: [&str; 14] = [
    "http_headers",
    "heatmap",
    "fragments",
    "downloader_options",
    "__x_forwarded_for_ip",
    "description",
    "tags",
    "comment_count",
    "like_count",
    "view_count",
    "channel_follower_count",
    "average_rating",
    "epoch",
    "available_at",
];

fn scrub_json(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.retain(|key, _| !DROP.contains(&key.as_str()));
            if let Some(Value::Object(captions)) = map.get_mut("automatic_captions") {
                let keep: Vec<String> = captions.keys().take(2).cloned().collect();
                captions.retain(|key, _| keep.contains(key));
            }
            for (key, child) in map.iter_mut() {
                let signed = matches!(key.as_str(), "url" | "manifest_url" | "fragment_base_url");
                match child {
                    // Media links are signed for the recording machine. Thumbnails and page links are public.
                    Value::String(s) if signed && !s.contains("ytimg.com") && !s.contains("/watch?") => {
                        *s = "https://example.invalid/scrubbed".into();
                    }
                    other => scrub_json(other),
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(scrub_json),
        _ => {}
    }
}

/// Replaces this machine's folders, link and user name in one recorded line.
fn scrub_line(text: &str, args: &[String], probing: bool) -> String {
    let mut text = text.to_string();
    if probing
        && text.starts_with('{')
        && let Ok(mut json) = serde_json::from_str::<Value>(&text)
    {
        scrub_json(&mut json);
        text = json.to_string();
    }
    let mut swap = |real: Option<String>, placeholder: &str| {
        if let Some(real) = real.filter(|r| r.len() > 3) {
            text = text.replace(&json_escaped(&real), placeholder).replace(&real, placeholder);
        }
    };
    swap(path_arg(args, "home:"), "{home}");
    swap(path_arg(args, "temp:"), "{temp}");
    swap(link(args), "{url}");
    swap(env("USERPROFILE").or_else(|| env("HOME")), r"C:\Users\USER");
    text
}

fn record(args: &[String], real: &str, root: &str) -> ExitCode {
    let name = env("FAKE_YTDLP_FIXTURE").unwrap_or_else(|| "recorded".into());
    let mode = mode(args);
    let spawned =
        Command::new(real).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            eprintln!("ERROR: fake-ytdlp could not run {real}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let lines: Arc<Mutex<Vec<(&'static str, String)>>> = Arc::default();
    let tee = |reader: Box<dyn std::io::Read + Send>, verb: &'static str| {
        let lines = lines.clone();
        std::thread::spawn(move || {
            for bytes in BufReader::new(reader).split(b'\n').map_while(Result::ok) {
                let line = String::from_utf8_lossy(&bytes).trim_end_matches('\r').to_string();
                if verb == "out" {
                    println!("{line}");
                } else {
                    eprintln!("{line}");
                }
                if let Ok(mut lines) = lines.lock() {
                    lines.push((verb, line));
                }
            }
        })
    };
    let readers = [
        child.stdout.take().map(|out| tee(Box::new(out), "out")),
        child.stderr.take().map(|err| tee(Box::new(err), "err")),
    ];
    let code = child.wait().ok().and_then(|status| status.code()).unwrap_or(1);
    for reader in readers.into_iter().flatten() {
        let _ = reader.join();
    }
    // Version checks are not worth a fixture; replay answers them itself.
    if !matches!(mode, "version" | "ffmpeg") {
        let mut script = format!("# Recorded from yt-dlp by fake-ytdlp. Mode: {mode}.\n");
        if let Ok(lines) = lines.lock() {
            for (verb, line) in lines.iter() {
                script.push_str(&format!("{verb} {}\n", scrub_line(line, args, mode == "probe")));
            }
        }
        script.push_str(&format!("exit {code}\n"));
        let dir = Path::new(root).join(&name);
        let saved =
            std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(dir.join(format!("{mode}.txt")), script));
        if let Err(error) = saved {
            eprintln!("ERROR: fake-ytdlp could not save the fixture: {error}");
        }
    }
    ExitCode::from(code.clamp(0, 255) as u8)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match (env("FAKE_YTDLP_RECORD"), env("FAKE_YTDLP_REAL")) {
        (Some(root), Some(real)) => record(&args, &real, &root),
        _ => replay(&args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn modes_from_arguments() {
        assert_eq!(mode(&args(&["--version"])), "version");
        assert_eq!(mode(&args(&["-version"])), "ffmpeg");
        assert_eq!(mode(&args(&["-j", "--flat-playlist", "--", "https://fake.test/a"])), "probe");
        assert_eq!(mode(&args(&["--simulate", "--print", "{}", "--load-info-json", "x"])), "simulate");
        assert_eq!(mode(&args(&["--simulate", "--print", "filename", "-o", "t"])), "filename");
        assert_eq!(mode(&args(&["--newline", "-P", "home:C:\\x", "--", "https://fake.test/a"])), "download");
    }

    #[test]
    fn fixture_from_the_link() {
        assert_eq!(fixture_name(Some("https://fake.test/video_audio")), "video_audio");
        assert_eq!(fixture_name(Some("https://fake.test/hang/extra?x=1")), "hang");
        assert_eq!(fixture_name(Some("https://www.youtube.com/watch?v=1")), "default");
        assert_eq!(path_arg(&args(&["-P", "home:C:\\out", "-P", "temp:D:\\t"]), "temp:").as_deref(), Some("D:\\t"));
    }

    #[test]
    fn recorded_lines_lose_this_machine() {
        let a = args(&[
            "-P",
            r"home:C:\Users\me\Videos",
            "-P",
            r"temp:C:\cache\j1",
            "--",
            "https://real.example/watch?v=1",
        ]);
        let line = r#"@@lovelydl@@FILE "C:\\Users\\me\\Videos\\Title [1].mp4""#;
        assert_eq!(scrub_line(line, &a, false), r#"@@lovelydl@@FILE "{home}\\Title [1].mp4""#);
        let probe = r#"{"id":"1","original_url":"https://real.example/watch?v=1","formats":[{"format_id":"18","url":"https://cdn.example/v?ip=1.2.3.4&sig=abc","http_headers":{"Cookie":"x"}}],"thumbnails":[{"url":"https://i.ytimg.com/vi/1/hq.jpg"}],"__x_forwarded_for_ip":"1.2.3.4"}"#;
        let scrubbed = scrub_line(probe, &a, true);
        assert!(scrubbed.contains(r#""original_url":"{url}""#), "{scrubbed}");
        assert!(scrubbed.contains("example.invalid/scrubbed") && scrubbed.contains("i.ytimg.com"));
        for leaked in ["1.2.3.4", "sig=abc", "Cookie", "cdn.example"] {
            assert!(!scrubbed.contains(leaked), "{leaked} survived: {scrubbed}");
        }
    }
}
