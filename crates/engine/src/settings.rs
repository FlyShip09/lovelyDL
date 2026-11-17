//! Settings: one versioned JSON file, written atomically.

use std::io;
use std::path::Path;

use crate::model::{DEFAULT_TEMPLATE, SETTINGS_VERSION, Settings};

/// Loads settings. A missing file gives defaults. A file that cannot be read is moved
/// aside (`settings.json.bad`) instead of being overwritten, and defaults are used.
pub fn load(path: &Path) -> Settings {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return Settings::default(),
    };
    match serde_json::from_str::<Settings>(&text) {
        Ok(settings) => sanitize(migrate(settings)),
        Err(error) => {
            tracing::warn!(%error, "settings file is unreadable; keeping it as settings.json.bad");
            let _ = std::fs::rename(path, path.with_extension("json.bad"));
            Settings::default()
        }
    }
}

/// Brings settings written by an older version up to date. Each new version adds one step.
fn migrate(mut settings: Settings) -> Settings {
    // 1 → 2: four fragments at a time is much faster on most sites, and version 1 could only
    // ever have held yt-dlp's old default of one. Anyone who chose a number keeps it.
    if settings.version < 2 && settings.concurrent_fragments == 1 {
        settings.concurrent_fragments = 4;
    }
    settings.version = SETTINGS_VERSION;
    settings
}

/// Forces every value into its legal range, whatever wrote it.
pub fn sanitize(mut s: Settings) -> Settings {
    s.version = SETTINGS_VERSION;
    s.max_concurrent = s.max_concurrent.clamp(1, 8);
    s.max_per_site = s.max_per_site.clamp(1, 8);
    s.concurrent_fragments = s.concurrent_fragments.clamp(1, 16);
    if s.filename_template.trim().is_empty() {
        s.filename_template = DEFAULT_TEMPLATE.into();
    }
    s.rate_limit = s.rate_limit.trim().to_string();
    s.proxy = s.proxy.trim().to_string();
    // "127.0.0.1:8080" means an http proxy; without the scheme yt-dlp would never get it.
    if !s.proxy.is_empty() && !s.proxy.contains("://") {
        s.proxy = format!("http://{}", s.proxy);
    }
    s
}

pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    // Rename over the old file so a crash never leaves half a file behind.
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_clamping() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let path = dir.path().join("nested").join("settings.json");
        assert_eq!(load(&path), Settings::default());

        let wild = Settings {
            max_concurrent: 99,
            max_per_site: 0,
            concurrent_fragments: 0,
            filename_template: "  ".into(),
            proxy: " http://127.0.0.1:8080 ".into(),
            ..Settings::default()
        };
        save(&path, &wild).unwrap_or_else(|e| panic!("{e}"));
        let got = load(&path);
        assert_eq!((got.max_concurrent, got.max_per_site, got.concurrent_fragments), (8, 1, 1));
        assert_eq!(got.filename_template, DEFAULT_TEMPLATE);
        assert_eq!(got.proxy, "http://127.0.0.1:8080");
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn an_old_file_gets_the_faster_fragment_default() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"version":1,"concurrentFragments":1,"maxConcurrent":3}"#)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&path).concurrent_fragments, 4);
        // A number the user picked is theirs, and so is anything already on version 2.
        std::fs::write(&path, r#"{"version":1,"concurrentFragments":8}"#).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&path).concurrent_fragments, 8);
        std::fs::write(&path, r#"{"version":2,"concurrentFragments":1}"#).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&path).concurrent_fragments, 1);
    }

    #[test]
    fn a_bare_proxy_address_means_http() {
        let s = sanitize(Settings { proxy: " 127.0.0.1:8080 ".into(), ..Settings::default() });
        assert_eq!(s.proxy, "http://127.0.0.1:8080");
        assert!(crate::ytdlp::args::is_proxy(&s.proxy));
        let s = sanitize(Settings { proxy: "socks5://h:1080".into(), ..Settings::default() });
        assert_eq!(s.proxy, "socks5://h:1080");
    }

    #[test]
    fn a_corrupt_file_is_kept_aside_not_destroyed() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{ not json").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&path), Settings::default());
        let bad = std::fs::read_to_string(path.with_extension("json.bad")).unwrap_or_default();
        assert_eq!(bad, "{ not json");
    }

    #[test]
    fn an_old_or_future_file_loads_with_what_it_has() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"version":0,"maxConcurrent":2,"somethingNew":true}"#)
            .unwrap_or_else(|e| panic!("{e}"));
        let got = load(&path);
        assert_eq!((got.version, got.max_concurrent), (SETTINGS_VERSION, 2));
    }
}
