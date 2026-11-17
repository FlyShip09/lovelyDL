//! Turns yt-dlp's log into an error kind and one fix.
//!
//! The patterns come from output captured with yt-dlp 2026.10.26 (see `tests/fixtures/errors`).
//! Messages that cannot be produced on demand are taken from yt-dlp's source and marked so.

use crate::model::{ErrorKind, Fix, JobError};

use ErrorKind::*;

/// Checked in order: the first kind with a matching pattern wins. Specific messages come
/// before the generic sign-in hint that yt-dlp appends to many of them.
const RULES: &[(ErrorKind, &[&str])] = &[
    (Drm, &["drm protect", "known to use drm"]),
    (
        Cookies,
        &["cookies database", "could not copy chrome cookie", "failed to decrypt with dpapi", "failed to load cookies"],
    ),
    (MembersOnly, &["members-only", "join this channel"]),
    (AgeRestricted, &["confirm your age", "age-restricted", "inappropriate for some users"]),
    (Private, &["private video", "video is private"]),
    (GeoBlocked, &["in your country", "geo restrict", "not available from your location", "ip address is blocked"]),
    (LiveNotStarted, &["live event will begin", "premieres in"]),
    (Http429, &["http error 429", "too many requests", "rate-limited"]),
    (SignIn, &["not a bot", "only available for registered users", "cookies for the authentication", "login required"]),
    (UnsupportedUrl, &["unsupported url"]),
    (FormatUnavailable, &["requested format is not available"]),
    (
        JsRuntime,
        &[
            "no supported javascript runtime",
            "challenge solving failed",
            "nsig extraction failed",
            "signature extraction failed",
        ],
    ),
    (
        FfmpegMissing,
        &["ffmpeg not found", "ffmpeg is not installed", "ffprobe and ffmpeg not found", "continuing without ffmpeg"],
    ),
    (DiskFull, &["no space left on device", "not enough space on the disk", "errno 28"]),
    (
        PermissionDenied,
        &["access is denied", "permission denied", "operation not permitted", "winerror 5]", "errno 13"],
    ),
    (Http403, &["http error 403"]),
    (
        Unavailable,
        &[
            "video unavailable",
            "video is unavailable",
            "has been removed",
            "has been terminated",
            "http error 404",
            "http error 410",
        ],
    ),
    (
        Network,
        &[
            "failed to resolve",
            "getaddrinfo failed",
            "name or service not known",
            "nodename nor servname",
            "name resolution",
            "timed out",
            "actively refused",
            "connection refused",
            "unable to connect to proxy",
            "connection reset",
            "connection aborted",
            "network is unreachable",
            "remote end closed connection",
            "http error 5",
        ],
    ),
];

/// Signs that yt-dlp's extractor is out of date rather than that the link is bad.
const BREAKAGE: [&str; 3] = ["unable to extract", "please report this issue", "confirm you are on the latest version"];

fn fix_for(kind: ErrorKind) -> (Option<Fix>, bool) {
    match kind {
        SignIn | AgeRestricted | Private | MembersOnly | Cookies => (Some(Fix::UseCookies), false),
        FormatUnavailable => (Some(Fix::ChooseFormat), false),
        Http403 => (Some(Fix::UpdateYtdlp), true),
        JsRuntime | FfmpegMissing | YtdlpMissing => (Some(Fix::InstallComponents), true),
        DiskFull | PermissionDenied => (Some(Fix::ChooseFolder), true),
        LiveNotStarted => (Some(Fix::WaitForLive), true),
        Playlist => (Some(Fix::ChooseVideos), false),
        Http429 | Network | Crashed => (Some(Fix::Retry), true),
        Drm | GeoBlocked | Unavailable | UnsupportedUrl => (None, false),
        Unknown => (Some(Fix::Retry), true),
    }
}

pub fn error(kind: ErrorKind, detail: impl Into<String>) -> JobError {
    let (fix, retryable) = fix_for(kind);
    JobError { kind, fix, retryable, detail: detail.into() }
}

fn lines_with<'a>(log: &'a [String], prefix: &str) -> Vec<&'a str> {
    log.iter().map(|l| l.trim_start()).filter(|l| l.starts_with(prefix)).collect()
}

/// Classifies a failed run from its log. `exit_code` is `None` when the process was killed.
pub fn classify(log: &[String], exit_code: Option<i32>) -> JobError {
    // Option problems come from yt-dlp's argument parser as "yt-dlp: error: …" (or yt-dlp.exe).
    let usage: Vec<&str> = log
        .iter()
        .filter_map(|l| l.trim_start().split_once(": error: ").filter(|(head, _)| head.starts_with("yt-dlp")))
        .map(|(_, message)| message)
        .collect();
    let errors = if usage.is_empty() { lines_with(log, "ERROR:") } else { usage };
    let warnings = lines_with(log, "WARNING:");
    // Errors decide. With none, warnings explain why nothing was produced.
    let decisive = if errors.is_empty() { &warnings } else { &errors };
    let haystack = decisive.join("\n").to_lowercase();
    let warned = warnings.join("\n").to_lowercase();

    let mut kind = RULES
        .iter()
        .find(|(_, patterns)| patterns.iter().any(|p| haystack.contains(p)))
        .map_or(Unknown, |(kind, _)| *kind);
    // A missing JavaScript runtime surfaces as missing formats or a 403 further down.
    if matches!(kind, Unknown | FormatUnavailable | Http403) && warned.contains("no supported javascript runtime") {
        kind = JsRuntime;
    }
    if kind == Unknown && decisive.is_empty() {
        kind = Crashed;
    }

    let detail = match decisive.last() {
        Some(line) => line.trim_start_matches("ERROR:").trim_start_matches("WARNING:").trim().to_string(),
        None => match exit_code {
            Some(code) => format!("yt-dlp stopped with exit code {code} and no error message"),
            None => "yt-dlp stopped unexpectedly".to_string(),
        },
    };
    let mut error = error(kind, detail);
    if kind == Unknown && BREAKAGE.iter().any(|p| haystack.contains(p)) {
        error.fix = Some(Fix::UpdateYtdlp);
    }
    error
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind_named(name: &str) -> ErrorKind {
        serde_json::from_value(serde_json::Value::String(name.to_string()))
            .unwrap_or_else(|_| panic!("fixture name {name:?} is not an ErrorKind"))
    }

    /// Every file in `tests/fixtures/errors` is named `<kind>__<what>.txt`.
    #[test]
    fn classifies_every_captured_error() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/errors");
        let mut seen = 0;
        for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.unwrap_or_else(|e| panic!("{e}")).path();
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
            let Some((kind, _)) = name.split_once("__") else { continue };
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{e}"));
            let log: Vec<String> = text.lines().filter(|l| !l.starts_with('#')).map(String::from).collect();
            let got = classify(&log, Some(1));
            assert_eq!(got.kind, kind_named(kind), "{name}: {}", got.detail);
            assert!(!got.detail.is_empty(), "{name}");
            seen += 1;
        }
        assert!(seen >= 18, "only {seen} error fixtures found");
    }

    #[test]
    fn fixes_and_retryability() {
        let e =
            classify(&["ERROR: [youtube] x: Requested format is not available. Use --list-formats".into()], Some(1));
        assert_eq!((e.fix, e.retryable), (Some(Fix::ChooseFormat), false));
        let e = classify(
            &["ERROR: [youtube] x: Unable to extract player response; please report this issue".into()],
            Some(1),
        );
        assert_eq!((e.kind, e.fix), (Unknown, Some(Fix::UpdateYtdlp)));
        let e = classify(&["ERROR: something nobody has seen before".into()], Some(1));
        assert_eq!((e.kind, e.fix, e.detail.as_str()), (Unknown, Some(Fix::Retry), "something nobody has seen before"));
    }

    #[test]
    fn option_errors_from_the_argument_parser() {
        let log =
            [r#"yt-dlp.exe: error: invalid default output template "%(title)s/%(nonsense": incomplete format key"#
                .to_string()];
        let e = classify(&log, Some(2));
        assert_eq!(e.kind, Unknown);
        assert_eq!(e.detail, r#"invalid default output template "%(title)s/%(nonsense": incomplete format key"#);
    }

    #[test]
    fn silence_is_a_crash() {
        let e = classify(&["[debug] nothing useful".into()], Some(-1073741819));
        assert_eq!(e.kind, Crashed);
        assert!(e.detail.contains("-1073741819"));
        assert_eq!(classify(&[], None).kind, Crashed);
    }

    #[test]
    fn a_missing_js_runtime_is_the_root_cause_of_a_format_error() {
        let log = [
            "WARNING: [youtube] No supported JavaScript runtime could be found. Only deno is enabled by default"
                .to_string(),
            "ERROR: [youtube] x: Requested format is not available. Use --list-formats for a list".to_string(),
        ];
        assert_eq!(classify(&log, Some(1)).kind, JsRuntime);
    }
}
