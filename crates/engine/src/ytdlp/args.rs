//! Builds yt-dlp command lines. Pure functions: everything is an argv element, nothing is
//! ever passed through a shell, and the URL always comes after `--`.

use std::path::{Path, PathBuf};

use crate::model::{AudioFormat, DownloadRequest, Options, PlaylistChoice, Preset, Settings, SponsorBlock};
use crate::ytdlp::parse::{DOWNLOAD_TEMPLATE, FILE_TEMPLATE, POSTPROCESS_TEMPLATE, REAL_TEMPLATE};

/// Absolute paths of the tools a run needs. Nothing here is looked up on PATH at run time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tools {
    pub ytdlp: PathBuf,
    pub ffmpeg_dir: Option<PathBuf>,
    /// Runtime name (`deno`, `node`, …) and the path to its binary.
    pub js_runtime: Option<(String, PathBuf)>,
}

/// What exact preset resolution prints: the chosen format and its parts, as one JSON line.
pub const RESOLVE_TEMPLATE: &str = concat!(
    r#"{"top":%(.{format_id,ext,width,height,fps,vcodec,acodec,filesize,filesize_approx,tbr,duration})j,"#,
    r#""parts":%(requested_formats.:.{format_id,filesize,filesize_approx,tbr,vcodec,acodec,height,fps}|null)j}"#
);

pub const COOKIE_BROWSERS: [&str; 8] = ["firefox", "chrome", "edge", "brave", "opera", "vivaldi", "chromium", "whale"];

pub const SPONSORBLOCK_CATEGORIES: [&str; 8] =
    ["sponsor", "selfpromo", "interaction", "intro", "outro", "preview", "music_offtopic", "filler"];

fn s(v: &str) -> String {
    v.to_string()
}

fn path(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

pub fn is_rate(v: &str) -> bool {
    let digits = v.trim_end_matches(['K', 'M', 'G', 'k', 'm', 'g']);
    !digits.is_empty() && v.len() - digits.len() <= 1 && digits.parse::<f64>().is_ok_and(|n| n > 0.0)
}

pub fn is_proxy(v: &str) -> bool {
    url::Url::parse(v).is_ok_and(|u| {
        ["http", "https", "socks4", "socks4a", "socks5", "socks5h"].contains(&u.scheme()) && u.host_str().is_some()
    })
}

/// A time for `--download-sections`: seconds or `h:mm:ss`, digits only.
fn is_time(v: &str) -> bool {
    !v.is_empty() && v.chars().all(|c| c.is_ascii_digit() || c == ':' || c == '.')
}

fn is_lang(v: &str) -> bool {
    !v.is_empty() && !v.starts_with("--") && v.chars().all(|c| c.is_ascii_alphanumeric() || "-_.*".contains(c))
}

/// Flags every invocation shares. Without `--encoding utf-8` yt-dlp writes the console code
/// page to pipes on Windows and silently drops characters it cannot encode (verified).
fn base() -> Vec<String> {
    vec![s("--ignore-config"), s("--color"), s("never"), s("--encoding"), s("utf-8")]
}

/// Flags every network-facing invocation shares.
fn common(settings: &Settings, tools: &Tools) -> Vec<String> {
    let mut a = base();
    if let Some((name, bin)) = &tools.js_runtime {
        if name != "deno" {
            // Any runtime other than Deno has to be enabled explicitly.
            a.push(s("--no-js-runtimes"));
        }
        a.extend([s("--js-runtimes"), format!("{name}:{}", path(bin))]);
    }
    if let Some(dir) = &tools.ffmpeg_dir {
        a.extend([s("--ffmpeg-location"), path(dir)]);
    }
    if COOKIE_BROWSERS.contains(&settings.cookies_browser.as_str()) {
        a.extend([s("--cookies-from-browser"), settings.cookies_browser.clone()]);
    }
    if is_proxy(&settings.proxy) {
        a.extend([s("--proxy"), settings.proxy.clone()]);
    }
    if settings.pace_requests {
        a.extend([s("-t"), s("sleep")]);
    }
    a
}

/// Format selection for a request. Built from preset aliases and sort order, not `-f` strings.
pub fn format_args(req: &DownloadRequest) -> Vec<String> {
    let mut a = Vec::new();
    let pick = req.formats.as_ref().and_then(|p| match (&p.video, &p.audio) {
        (Some(v), Some(au)) => Some(format!("{v}+{au}")),
        (Some(one), None) | (None, Some(one)) => Some(one.clone()),
        (None, None) => None,
    });
    let picked = pick.is_some();
    if let Some(pick) = pick {
        a.extend([s("-f"), pick]);
    }
    // A height cap must come after `-t mp4`: a later `-S` is prepended to the sort order.
    let cap = |h: &Option<u32>| h.map(|h| vec![s("-S"), format!("res:{h}")]).unwrap_or_default();
    match &req.preset {
        Preset::Best { max_height } if !picked => a.extend(cap(max_height)),
        Preset::Mp4 { max_height } if !picked => {
            a.extend([s("-t"), s("mp4")]);
            // H.264 stays first: a bare `res:` cap would outrank it and pick VP9 or AV1 at 1440p
            // and 2160p, which the remux then wraps in MP4 (checked with 2026.10.26).
            if let Some(h) = max_height {
                a.extend([s("-S"), format!("vcodec:h264,res:{h},acodec:aac")]);
            }
            // The alias only remuxes, which fails when no H.264 exists (an MPEG-2 or VP9 source).
            // Re-encoding happens only when the result is not already MP4 (verified with 2026.10.26).
            a.extend([s("--recode-video"), s("mp4")]);
        }
        Preset::Best { .. } | Preset::Mp4 { .. } => {}
        Preset::Audio { format } => {
            match format {
                AudioFormat::Mp3 if !picked => a.extend([s("-t"), s("mp3")]),
                _ => a.extend([s("-x"), s("--audio-format"), s(format.ext())]),
            }
            match format {
                // Prefer a source that is already in the target codec, so nothing is re-encoded.
                AudioFormat::M4a if !picked => a.extend([s("-S"), s("acodec:aac")]),
                AudioFormat::Opus if !picked => a.extend([s("-S"), s("acodec:opus")]),
                AudioFormat::Mp3 => a.extend([s("--audio-quality"), s("0")]),
                _ => {}
            }
        }
    }
    a
}

fn option_args(o: &Options, audio: bool) -> Vec<String> {
    let mut a = Vec::new();
    if o.embed_metadata {
        a.push(s("--embed-metadata"));
    }
    if o.embed_thumbnail {
        a.extend([s("--embed-thumbnail"), s("--convert-thumbnails"), s("jpg")]);
    }
    if o.embed_chapters {
        a.push(s("--embed-chapters"));
    }
    let langs: Vec<&str> = o.sub_langs.iter().map(String::as_str).filter(|l| is_lang(l)).collect();
    if o.subtitles && !audio && !langs.is_empty() {
        a.push(s("--write-subs"));
        if o.auto_subs {
            a.push(s("--write-auto-subs"));
        }
        a.extend([s("--sub-langs"), langs.join(",")]);
        if o.embed_subs {
            // With an explicit --write-subs yt-dlp keeps the subtitle file beside the video;
            // no-keep-subs deletes it once embedded (yt-dlp 2026.10.26, __init__.py).
            a.extend([s("--embed-subs"), s("--compat-options"), s("no-keep-subs")]);
        }
    }
    let categories: Vec<&str> =
        o.sponsorblock_categories.iter().map(String::as_str).filter(|c| SPONSORBLOCK_CATEGORIES.contains(c)).collect();
    if !categories.is_empty() {
        match o.sponsorblock {
            SponsorBlock::Off => {}
            SponsorBlock::Remove => a.extend([s("--sponsorblock-remove"), categories.join(",")]),
            SponsorBlock::Mark => a.extend([s("--sponsorblock-mark"), categories.join(",")]),
        }
    }
    if let Some(section) = &o.section {
        let start = if is_time(&section.start) { section.start.as_str() } else { "0" };
        let end = if is_time(&section.end) { section.end.as_str() } else { "inf" };
        if (start, end) != ("0", "inf") {
            a.extend([s("--download-sections"), format!("*{start}-{end}")]);
            if o.precise_cuts {
                a.push(s("--force-keyframes-at-cuts"));
            }
        }
    }
    if o.live_from_start {
        a.push(s("--live-from-start"));
    }
    if o.wait_for_live {
        a.extend([s("--wait-for-video"), s("60")]);
    }
    a
}

/// Probe a link. Single videos print one full JSON object; playlists print one flat entry
/// per line as they are found.
pub fn probe_args(url: &str, choice: PlaylistChoice, settings: &Settings, tools: &Tools) -> Vec<String> {
    let mut a = common(settings, tools);
    a.extend([s("-j"), s("--flat-playlist"), s("--lazy-playlist")]);
    a.push(s(match choice {
        PlaylistChoice::Video => "--no-playlist",
        PlaylistChoice::Playlist => "--yes-playlist",
    }));
    a.extend([s("--"), s(url)]);
    a
}

/// Ask yt-dlp what a request selects from cached info. Makes no network calls.
pub fn resolve_args(info_json: &Path, req: &DownloadRequest) -> Vec<String> {
    let mut a = base();
    a.push(s("--no-playlist"));
    a.extend(format_args(req));
    a.extend([s("--simulate"), s("--print"), s(RESOLVE_TEMPLATE)]);
    a.extend([s("--load-info-json"), path(info_json)]);
    a
}

/// Ask yt-dlp what file name a template produces for cached info. Makes no network calls.
pub fn filename_args(info_json: &Path, req: &DownloadRequest, template: &str) -> Vec<String> {
    let mut a = base();
    a.push(s("--no-playlist"));
    a.extend(format_args(req));
    a.extend([s("--simulate"), s("--print"), s("filename"), s("-o"), s(template)]);
    a.extend([s("--load-info-json"), path(info_json)]);
    a
}

pub struct Download<'a> {
    pub url: &'a str,
    pub request: &'a DownloadRequest,
    pub settings: &'a Settings,
    pub tools: &'a Tools,
    pub dest: &'a Path,
    pub temp: &'a Path,
    /// Output template, already including any playlist index prefix.
    pub template: &'a str,
    /// Fresh cached probe result; skips a second extraction.
    pub info_json: Option<&'a Path>,
    /// Download archive, for playlist items that should be skipped when already saved.
    pub archive: Option<&'a Path>,
}

pub fn download_args(d: &Download) -> Vec<String> {
    let mut a = common(d.settings, d.tools);
    a.extend([s("--newline"), s("--no-playlist")]);
    a.extend(format_args(d.request));
    a.extend(option_args(&d.request.options, d.request.preset.is_audio()));
    if is_rate(&d.settings.rate_limit) {
        a.extend([s("--limit-rate"), d.settings.rate_limit.clone()]);
    }
    if d.settings.concurrent_fragments > 1 {
        a.extend([s("--concurrent-fragments"), d.settings.concurrent_fragments.to_string()]);
    }
    if let Some(archive) = d.archive {
        a.extend([s("--download-archive"), path(archive)]);
    }
    a.extend([s("-P"), format!("home:{}", path(d.dest))]);
    a.extend([s("-P"), format!("temp:{}", path(d.temp))]);
    a.extend([s("-o"), s(d.template)]);
    // `--print` implies `--quiet`; `--progress` brings the progress lines back.
    a.extend([s("--progress"), s("--progress-delta"), s("0.25")]);
    a.extend([s("--progress-template"), s(DOWNLOAD_TEMPLATE)]);
    a.extend([s("--progress-template"), s(POSTPROCESS_TEMPLATE)]);
    a.extend([s("--print"), s(FILE_TEMPLATE), s("--print"), s(REAL_TEMPLATE)]);
    if d.settings.custom_args_enabled {
        a.extend(d.settings.custom_args.iter().filter(|x| !x.trim().is_empty() && *x != "--").cloned());
    }
    match d.info_json {
        Some(info) => a.extend([s("--load-info-json"), path(info)]),
        None => a.extend([s("--"), s(d.url)]),
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FormatPick, Section};

    fn tools() -> Tools {
        Tools {
            ytdlp: PathBuf::from(r"C:\bin\yt-dlp.exe"),
            ffmpeg_dir: Some(PathBuf::from(r"C:\bin\ffmpeg")),
            js_runtime: Some(("deno".into(), PathBuf::from(r"C:\bin\deno.exe"))),
        }
    }

    fn plain() -> DownloadRequest {
        DownloadRequest {
            preset: Preset::Best { max_height: None },
            formats: None,
            options: Options {
                embed_thumbnail: false,
                embed_metadata: false,
                embed_chapters: false,
                ..Options::default()
            },
            destination: None,
        }
    }

    fn download(req: &DownloadRequest, settings: &Settings, info: Option<&Path>) -> Vec<String> {
        download_args(&Download {
            url: "https://example.test/watch?v=1",
            request: req,
            settings,
            tools: &tools(),
            dest: Path::new(r"D:\Videos"),
            temp: Path::new(r"D:\Videos\.lovelydl-partial\j1"),
            template: "%(title).200B [%(id)s].%(ext)s",
            info_json: info,
            archive: None,
        })
    }

    #[test]
    fn golden_default_download() {
        let got = download(&DownloadRequest::default(), &Settings::default(), None);
        let want = [
            "--ignore-config",
            "--color",
            "never",
            "--encoding",
            "utf-8",
            "--js-runtimes",
            r"deno:C:\bin\deno.exe",
            "--ffmpeg-location",
            r"C:\bin\ffmpeg",
            "--newline",
            "--no-playlist",
            "-t",
            "mp4",
            "--recode-video",
            "mp4",
            "--embed-metadata",
            "--embed-thumbnail",
            "--convert-thumbnails",
            "jpg",
            "--embed-chapters",
            "--concurrent-fragments",
            "4",
            "-P",
            r"home:D:\Videos",
            "-P",
            r"temp:D:\Videos\.lovelydl-partial\j1",
            "-o",
            "%(title).200B [%(id)s].%(ext)s",
            "--progress",
            "--progress-delta",
            "0.25",
            "--progress-template",
            DOWNLOAD_TEMPLATE,
            "--progress-template",
            POSTPROCESS_TEMPLATE,
            "--print",
            FILE_TEMPLATE,
            "--print",
            REAL_TEMPLATE,
            "--",
            "https://example.test/watch?v=1",
        ];
        assert_eq!(got, want);
    }

    #[test]
    fn golden_everything_on() {
        let req = DownloadRequest {
            preset: Preset::Mp4 { max_height: Some(720) },
            formats: None,
            options: Options {
                subtitles: true,
                sub_langs: vec!["en".into(), "fr".into(), "--exec".into()],
                auto_subs: true,
                embed_subs: true,
                embed_thumbnail: true,
                embed_metadata: true,
                embed_chapters: true,
                sponsorblock: SponsorBlock::Remove,
                sponsorblock_categories: vec!["sponsor".into(), "intro".into(), "bogus".into()],
                section: Some(Section { start: "1:30".into(), end: "".into() }),
                precise_cuts: true,
                live_from_start: true,
                wait_for_live: true,
            },
            destination: None,
        };
        let settings = Settings {
            rate_limit: "2M".into(),
            proxy: "socks5://127.0.0.1:1080".into(),
            pace_requests: true,
            cookies_browser: "firefox".into(),
            concurrent_fragments: 4,
            custom_args_enabled: true,
            custom_args: vec!["--no-mtime".into(), "--".into(), " ".into()],
            ..Settings::default()
        };
        let got = download(&req, &settings, Some(Path::new(r"C:\cache\probe\j1.json")));
        let want = [
            "--ignore-config",
            "--color",
            "never",
            "--encoding",
            "utf-8",
            "--js-runtimes",
            r"deno:C:\bin\deno.exe",
            "--ffmpeg-location",
            r"C:\bin\ffmpeg",
            "--cookies-from-browser",
            "firefox",
            "--proxy",
            "socks5://127.0.0.1:1080",
            "-t",
            "sleep",
            "--newline",
            "--no-playlist",
            "-t",
            "mp4",
            "-S",
            "vcodec:h264,res:720,acodec:aac",
            "--recode-video",
            "mp4",
            "--embed-metadata",
            "--embed-thumbnail",
            "--convert-thumbnails",
            "jpg",
            "--embed-chapters",
            "--write-subs",
            "--write-auto-subs",
            "--sub-langs",
            "en,fr",
            "--embed-subs",
            "--compat-options",
            "no-keep-subs",
            "--sponsorblock-remove",
            "sponsor,intro",
            "--download-sections",
            "*1:30-inf",
            "--force-keyframes-at-cuts",
            "--live-from-start",
            "--wait-for-video",
            "60",
            "--limit-rate",
            "2M",
            "--concurrent-fragments",
            "4",
            "-P",
            r"home:D:\Videos",
            "-P",
            r"temp:D:\Videos\.lovelydl-partial\j1",
            "-o",
            "%(title).200B [%(id)s].%(ext)s",
            "--progress",
            "--progress-delta",
            "0.25",
            "--progress-template",
            DOWNLOAD_TEMPLATE,
            "--progress-template",
            POSTPROCESS_TEMPLATE,
            "--print",
            FILE_TEMPLATE,
            "--print",
            REAL_TEMPLATE,
            "--no-mtime",
            "--load-info-json",
            r"C:\cache\probe\j1.json",
        ];
        assert_eq!(got, want);
    }

    fn formats(preset: Preset) -> String {
        format_args(&DownloadRequest { preset, ..plain() }).join(" ")
    }

    #[test]
    fn golden_presets() {
        assert_eq!(formats(Preset::Best { max_height: None }), "");
        assert_eq!(formats(Preset::Best { max_height: Some(1080) }), "-S res:1080");
        assert_eq!(formats(Preset::Mp4 { max_height: None }), "-t mp4 --recode-video mp4");
        assert_eq!(
            formats(Preset::Mp4 { max_height: Some(480) }),
            "-t mp4 -S vcodec:h264,res:480,acodec:aac --recode-video mp4"
        );
        let audio = |format| formats(Preset::Audio { format });
        assert_eq!(audio(AudioFormat::M4a), "-x --audio-format m4a -S acodec:aac");
        assert_eq!(audio(AudioFormat::Mp3), "-t mp3 --audio-quality 0");
        assert_eq!(audio(AudioFormat::Opus), "-x --audio-format opus -S acodec:opus");
        assert_eq!(audio(AudioFormat::Flac), "-x --audio-format flac");
    }

    #[test]
    fn an_explicit_pick_replaces_the_preset_selection() {
        let pick = |video: Option<&str>, audio: Option<&str>, preset| {
            format_args(&DownloadRequest {
                preset,
                formats: Some(FormatPick { video: video.map(s), audio: audio.map(s) }),
                ..plain()
            })
            .join(" ")
        };
        let mp4 = Preset::Mp4 { max_height: Some(720) };
        assert_eq!(pick(Some("137"), Some("140"), mp4.clone()), "-f 137+140");
        assert_eq!(pick(Some("18"), None, mp4.clone()), "-f 18");
        assert_eq!(pick(None, None, mp4), "-t mp4 -S vcodec:h264,res:720,acodec:aac --recode-video mp4");
        let mp3 = Preset::Audio { format: AudioFormat::Mp3 };
        assert_eq!(pick(None, Some("251"), mp3), "-f 251 -x --audio-format mp3 --audio-quality 0");
    }

    #[test]
    fn the_url_is_always_last_and_after_a_double_dash() {
        // A hostile "URL" cannot become an option.
        let a = probe_args("--exec=calc", PlaylistChoice::Video, &Settings::default(), &tools());
        assert_eq!(&a[a.len() - 2..], ["--", "--exec=calc"]);
        assert!(a.contains(&s("--no-playlist")) && a.contains(&s("--lazy-playlist")));
        let a = probe_args("https://x.test/", PlaylistChoice::Playlist, &Settings::default(), &tools());
        assert!(a.contains(&s("--yes-playlist")) && !a.contains(&s("--no-playlist")));
    }

    #[test]
    fn bad_setting_values_are_dropped_not_passed() {
        let settings = Settings {
            rate_limit: "fast".into(),
            proxy: "javascript:alert(1)".into(),
            cookies_browser: "safari --exec".into(),
            custom_args_enabled: false,
            custom_args: vec!["--exec".into(), "calc".into()],
            ..Settings::default()
        };
        let a = download(&plain(), &settings, None);
        for flag in ["--limit-rate", "--proxy", "--cookies-from-browser", "--exec", "calc"] {
            assert!(!a.contains(&s(flag)), "{flag}");
        }
        assert!(is_rate("500K") && is_rate("1.5M") && is_rate("50000"));
        assert!(!is_rate("") && !is_rate("M") && !is_rate("2MM") && !is_rate("-1M"));
    }

    #[test]
    fn a_non_deno_runtime_is_enabled_explicitly() {
        let mut t = tools();
        t.js_runtime = Some(("node".into(), PathBuf::from(r"C:\node\node.exe")));
        let a = common(&Settings::default(), &t).join(" ");
        assert!(a.contains(r"--no-js-runtimes --js-runtimes node:C:\node\node.exe"), "{a}");
    }

    #[test]
    fn resolving_and_naming_never_touch_the_network() {
        let info = Path::new("i.json");
        for a in [resolve_args(info, &plain()), filename_args(info, &plain(), "%(id)s.%(ext)s")] {
            assert!(a.contains(&s("--simulate")) && a.contains(&s("--load-info-json")));
            assert!(!a.contains(&s("--")));
        }
    }
}
