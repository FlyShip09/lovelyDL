//! Reads yt-dlp's info JSON: one full object for a video, one flat entry per playlist item.

use serde_json::Value;

use crate::model::{AudioFormat, DownloadRequest, Format, LiveStatus, MediaInfo, PlaylistEntry, Preset, Resolved};

#[allow(clippy::large_enum_variant, reason = "one value per output line, consumed at once")]
pub enum Probed {
    Media(Box<MediaInfo>),
    Entry { entry: PlaylistEntry, playlist_title: String, playlist_uploader: String, archive_key: String },
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn num(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn int(v: &Value, key: &str) -> Option<u32> {
    num(v, key).filter(|n| *n >= 0.0).map(|n| n as u32)
}

/// yt-dlp writes a missing codec as `"none"` or null.
fn codec(v: &Value, key: &str) -> String {
    match v.get(key).and_then(Value::as_str) {
        Some("none") | None => String::new(),
        Some(c) => c.to_string(),
    }
}

/// The smallest thumbnail that is at least 480 px wide: sharp in the preview, light in rows.
fn thumbnail(v: &Value) -> String {
    let sized = v
        .get("thumbnails")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|t| Some((int(t, "width")?, t.get("url")?.as_str()?)));
    let mut best: Option<(u32, &str)> = None;
    let mut largest: Option<(u32, &str)> = None;
    for (w, url) in sized {
        if w >= 480 && best.is_none_or(|(bw, _)| w < bw) {
            best = Some((w, url));
        }
        if largest.is_none_or(|(lw, _)| w > lw) {
            largest = Some((w, url));
        }
    }
    best.or(largest).map(|(_, url)| url.to_string()).unwrap_or_else(|| text(v, "thumbnail"))
}

fn format(f: &Value) -> Option<Format> {
    let (vcodec, acodec) = (codec(f, "vcodec"), codec(f, "acodec"));
    let ext = text(f, "ext");
    // Storyboards and other image-only "formats" are not downloadable media.
    if ext == "mhtml" || (vcodec.is_empty() && acodec.is_empty() && f.get("vcodec").is_some()) {
        return None;
    }
    let exact = num(f, "filesize");
    Some(Format {
        id: text(f, "format_id"),
        ext,
        width: int(f, "width"),
        height: int(f, "height"),
        fps: num(f, "fps"),
        vcodec,
        acodec,
        tbr: num(f, "tbr"),
        bytes: exact.or_else(|| num(f, "filesize_approx")),
        bytes_approx: exact.is_none(),
        note: text(f, "format_note"),
        dynamic_range: text(f, "dynamic_range"),
        language: text(f, "language"),
        protocol: text(f, "protocol"),
    })
}

fn media(v: &Value) -> MediaInfo {
    let formats: Vec<Format> =
        v.get("formats").and_then(Value::as_array).into_iter().flatten().filter_map(format).collect();
    let live = match v.get("live_status").and_then(Value::as_str) {
        Some("is_live") => LiveStatus::Live,
        Some("is_upcoming") => LiveStatus::Upcoming,
        Some("was_live" | "post_live") => LiveStatus::WasLive,
        _ if v.get("is_live").and_then(Value::as_bool) == Some(true) => LiveStatus::Live,
        _ => LiveStatus::NotLive,
    };
    let mut subtitles: Vec<String> = v
        .get("subtitles")
        .and_then(Value::as_object)
        .map(|m| m.keys().filter(|k| *k != "live_chat").cloned().collect())
        .unwrap_or_default();
    subtitles.sort();
    let site = text(v, "webpage_url_domain");
    MediaInfo {
        id: text(v, "id"),
        title: text(v, "title"),
        uploader: [text(v, "uploader"), text(v, "channel")].into_iter().find(|s| !s.is_empty()).unwrap_or_default(),
        duration: num(v, "duration"),
        site: if site.is_empty() { text(v, "extractor") } else { site },
        thumbnail: thumbnail(v),
        webpage_url: text(v, "webpage_url"),
        live,
        release_at: num(v, "release_timestamp"),
        age_restricted: num(v, "age_limit").is_some_and(|a| a >= 18.0),
        members_only: matches!(v.get("availability").and_then(Value::as_str), Some("subscriber_only" | "premium_only")),
        drm: v.get("_has_drm").and_then(Value::as_bool) == Some(true),
        formats,
        subtitles,
        auto_captions: v.get("automatic_captions").and_then(Value::as_object).is_some_and(|m| !m.is_empty()),
        chapters: v.get("chapters").and_then(Value::as_array).map_or(0, |c| c.len() as u32),
    }
}

/// Parses one line of `-j --flat-playlist` output.
pub fn parse_probe_line(line: &str) -> Option<Probed> {
    let v: Value = serde_json::from_str(line.trim()).ok()?;
    if v.get("formats").is_some_and(Value::is_array) || v.get("_type").and_then(Value::as_str) == Some("video") {
        return Some(Probed::Media(Box::new(media(&v))));
    }
    let url = [text(&v, "url"), text(&v, "webpage_url")].into_iter().find(|u| u.starts_with("http"))?;
    let id = text(&v, "id");
    Some(Probed::Entry {
        // The key yt-dlp writes to its download archive: lower-cased extractor, then the id.
        archive_key: format!("{} {id}", text(&v, "ie_key").to_lowercase()),
        entry: PlaylistEntry {
            title: [text(&v, "title"), url.clone()].into_iter().find(|s| !s.is_empty()).unwrap_or_default(),
            uploader: [text(&v, "uploader"), text(&v, "channel")]
                .into_iter()
                .find(|s| !s.is_empty())
                .unwrap_or_default(),
            duration: num(&v, "duration"),
            thumbnail: thumbnail(&v),
            downloaded: false,
            id,
            url,
        },
        playlist_title: text(&v, "playlist_title"),
        playlist_uploader: [text(&v, "playlist_uploader"), text(&v, "playlist_channel")]
            .into_iter()
            .find(|s| !s.is_empty())
            .unwrap_or_default(),
    })
}

fn bytes_of(v: &Value, duration: Option<f64>) -> (Option<f64>, bool) {
    if let Some(exact) = num(v, "filesize") {
        return (Some(exact), false);
    }
    let estimate = num(v, "filesize_approx").or_else(|| Some(num(v, "tbr")? * duration? * 125.0)).filter(|b| *b > 0.0);
    (estimate, true)
}

/// Audio that gets re-encoded ends up a different size than its source.
fn reencodes(preset: &Preset, acodec: &str) -> bool {
    match preset {
        Preset::Audio { format: AudioFormat::M4a } => !(acodec.starts_with("mp4a") || acodec.starts_with("aac")),
        Preset::Audio { format: AudioFormat::Opus } => acodec != "opus",
        Preset::Audio { format: AudioFormat::Mp3 } => !acodec.starts_with("mp3"),
        Preset::Audio { format: AudioFormat::Flac } => acodec != "flac",
        _ => false,
    }
}

/// Parses the line printed by [`RESOLVE_TEMPLATE`](crate::ytdlp::args::RESOLVE_TEMPLATE).
pub fn parse_resolved(line: &str, req: &DownloadRequest, source_height: Option<u32>) -> Option<Resolved> {
    let v: Value = serde_json::from_str(line.trim()).ok()?;
    let top = v.get("top")?;
    let duration = num(top, "duration");
    let parts: Vec<&Value> = v.get("parts").and_then(Value::as_array).into_iter().flatten().collect();
    let (bytes, approx) = if parts.is_empty() {
        bytes_of(top, duration)
    } else {
        let sized: Vec<(Option<f64>, bool)> = parts.iter().map(|p| bytes_of(p, duration)).collect();
        match sized.iter().map(|(b, _)| *b).sum::<Option<f64>>() {
            Some(total) => (Some(total), sized.iter().any(|(_, a)| *a)),
            None => bytes_of(top, duration),
        }
    };
    let (vcodec, acodec) = (codec(top, "vcodec"), codec(top, "acodec"));
    let audio_only = vcodec.is_empty() || req.preset.is_audio();
    Some(Resolved {
        exact: true,
        format_ids: text(top, "format_id").split('+').filter(|s| !s.is_empty()).map(String::from).collect(),
        height: if audio_only { None } else { int(top, "height") },
        fps: if audio_only { None } else { num(top, "fps") },
        ext: match &req.preset {
            Preset::Audio { format } => format.ext().to_string(),
            _ => text(top, "ext"),
        },
        bytes,
        bytes_approx: approx || reencodes(&req.preset, &acodec),
        vcodec: if audio_only { String::new() } else { vcodec },
        acodec,
        source_height,
    })
}

fn is_h264(f: &Format) -> bool {
    f.vcodec.starts_with("avc") || f.vcodec.starts_with("h264")
}

fn is_aac(f: &Format) -> bool {
    f.acodec.starts_with("mp4a") || f.acodec.starts_with("aac")
}

fn codec_rank(vcodec: &str) -> u32 {
    match vcodec.split('.').next().unwrap_or_default() {
        "av01" => 4,
        "vp09" | "vp9" => 3,
        "hev1" | "hvc1" | "h265" => 2,
        _ => 1,
    }
}

/// Plain HTTP streams sort ahead of HLS variants of the same quality, as in yt-dlp.
fn direct(f: &Format) -> u32 {
    u32::from(!f.protocol.contains("m3u8"))
}

pub fn source_height(info: &MediaInfo) -> Option<u32> {
    info.formats.iter().filter(|f| !f.vcodec.is_empty()).filter_map(|f| f.height).max()
}

/// A quick guess at what a request selects, good enough to show while yt-dlp works out the
/// exact answer. It mirrors yt-dlp's sort order only roughly, so the result is marked inexact.
pub fn approx_resolve(info: &MediaInfo, req: &DownloadRequest) -> Resolved {
    let find = |id: &Option<String>| id.as_ref().and_then(|id| info.formats.iter().find(|f| &f.id == id));
    let picked = req.formats.as_ref().map(|p| (find(&p.video), find(&p.audio)));
    let target_aac = matches!(req.preset, Preset::Mp4 { .. } | Preset::Audio { format: AudioFormat::M4a });
    let target_opus = matches!(req.preset, Preset::Audio { format: AudioFormat::Opus });
    let best_audio = || {
        info.formats.iter().filter(|f| f.vcodec.is_empty() && !f.acodec.is_empty() && !f.id.contains("-drc")).max_by(
            |a, b| {
                let key = |f: &Format| {
                    let wanted = (target_aac && is_aac(f)) || (target_opus && f.acodec == "opus");
                    (u32::from(wanted), direct(f), f.tbr.unwrap_or(0.0))
                };
                key(a).partial_cmp(&key(b)).unwrap_or(std::cmp::Ordering::Equal)
            },
        )
    };
    let best_video = || {
        let (cap, want_h264) = match &req.preset {
            Preset::Best { max_height } => (*max_height, false),
            Preset::Mp4 { max_height } => (*max_height, true),
            Preset::Audio { .. } => return None,
        };
        let videos: Vec<&Format> = info.formats.iter().filter(|f| !f.vcodec.is_empty()).collect();
        let under: Vec<&Format> = match cap {
            Some(cap) if videos.iter().any(|f| f.height.is_some_and(|h| h <= cap)) => {
                videos.iter().copied().filter(|f| f.height.is_some_and(|h| h <= cap)).collect()
            }
            // Nothing under the cap: yt-dlp falls back to the smallest above it.
            Some(_) => videos.iter().copied().min_by_key(|f| f.height.unwrap_or(u32::MAX)).into_iter().collect(),
            None => videos,
        };
        let pool: Vec<&Format> = if want_h264 && under.iter().any(|f| is_h264(f)) {
            under.into_iter().filter(|f| is_h264(f)).collect()
        } else {
            under
        };
        pool.into_iter().max_by(|a, b| {
            let key = |f: &Format| {
                (
                    f.height.unwrap_or(0),
                    f.fps.unwrap_or(0.0).round() as u32,
                    codec_rank(&f.vcodec),
                    direct(f),
                    f.tbr.unwrap_or(0.0),
                )
            };
            key(a).partial_cmp(&key(b)).unwrap_or(std::cmp::Ordering::Equal)
        })
    };
    let (video, audio) = match picked {
        Some((v, a)) if v.is_some() || a.is_some() => (v, a),
        _ => {
            let video = best_video();
            // A stream that already carries audio needs no second one.
            let audio = if video.is_some_and(|v| !v.acodec.is_empty()) { None } else { best_audio() };
            (video, audio.or(if video.is_none() { info.formats.iter().rfind(|f| !f.acodec.is_empty()) } else { None }))
        }
    };
    let parts: Vec<&Format> = [video, audio].into_iter().flatten().collect();
    let duration = info.duration;
    let size = |f: &Format| f.bytes.or_else(|| Some(f.tbr? * duration? * 125.0));
    let acodec = audio.or(video).map(|f| f.acodec.clone()).unwrap_or_default();
    let ext = match (&req.preset, video, audio) {
        (Preset::Audio { format }, ..) => format.ext().to_string(),
        (Preset::Mp4 { .. }, ..) => "mp4".to_string(),
        (_, Some(v), Some(a)) if v.ext == "mp4" && a.ext == "m4a" => "mp4".to_string(),
        (_, Some(v), Some(a)) if v.ext == "webm" && a.ext == "webm" => "webm".to_string(),
        (_, Some(_), Some(_)) => "mkv".to_string(),
        (_, Some(v), None) => v.ext.clone(),
        (_, None, Some(a)) => a.ext.clone(),
        (_, None, None) => String::new(),
    };
    Resolved {
        exact: false,
        format_ids: parts.iter().map(|f| f.id.clone()).collect(),
        height: video.and_then(|v| v.height),
        fps: video.and_then(|v| v.fps),
        vcodec: video.map(|v| v.vcodec.clone()).unwrap_or_default(),
        acodec,
        ext,
        bytes: if parts.is_empty() { None } else { parts.iter().map(|f| size(f)).sum() },
        bytes_approx: true,
        source_height: source_height(info),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FormatPick;

    const INFO: &str = include_str!("../../tests/fixtures/probe/youtube_video.json");
    const PLAYLIST: &str = include_str!("../../tests/fixtures/probe/youtube_playlist.jsonl");

    fn info() -> MediaInfo {
        match parse_probe_line(INFO) {
            Some(Probed::Media(m)) => *m,
            _ => panic!("fixture is not a video"),
        }
    }

    fn req(preset: Preset) -> DownloadRequest {
        DownloadRequest { preset, ..DownloadRequest::default() }
    }

    #[test]
    fn reads_a_real_video() {
        let m = info();
        assert_eq!(
            (m.id.as_str(), m.title.as_str(), m.uploader.as_str()),
            ("SkVqJ1SGeL0", "Caminandes 3: Llamigos", "Blender")
        );
        assert_eq!((m.duration, m.site.as_str(), m.live), (Some(150.0), "youtube.com", LiveStatus::NotLive));
        assert!(!m.drm && !m.age_restricted && !m.members_only && m.auto_captions);
        // Storyboards are dropped; the 1080p AV1 stream is there with an exact size.
        assert!(m.formats.iter().all(|f| f.ext != "mhtml"));
        let f399 = m.formats.iter().find(|f| f.id == "399").unwrap_or_else(|| panic!("no 399"));
        assert_eq!((f399.height, f399.bytes, f399.bytes_approx), (Some(1080), Some(23_972_841.0), false));
        assert!(f399.acodec.is_empty() && f399.vcodec.starts_with("av01"));
        assert!(m.thumbnail.starts_with("https://") && m.thumbnail.contains("SkVqJ1SGeL0"));
        assert_eq!(source_height(&m), Some(1080));
    }

    #[test]
    fn reads_real_playlist_entries() {
        let entries: Vec<_> = PLAYLIST.lines().filter_map(parse_probe_line).collect();
        assert_eq!(entries.len(), 17);
        let Probed::Entry { entry, playlist_title, playlist_uploader, archive_key } = &entries[9] else {
            panic!("not an entry")
        };
        assert_eq!(entry.id, "SkVqJ1SGeL0");
        assert_eq!(entry.url, "https://www.youtube.com/watch?v=SkVqJ1SGeL0");
        assert_eq!((entry.title.as_str(), entry.duration), ("Caminandes 3: Llamigos", Some(151.0)));
        assert_eq!((playlist_title.as_str(), playlist_uploader.as_str()), ("Official Blender Open Movies", "Blender"));
        assert_eq!(archive_key, "youtube SkVqJ1SGeL0");
    }

    /// The estimate must agree with what yt-dlp really selected for the same fixture
    /// (recorded with `--load-info-json … --simulate`, see docs/DECISIONS.md).
    #[test]
    fn the_estimate_matches_yt_dlp_on_the_fixture() {
        let m = info();
        let ids = |p| approx_resolve(&m, &req(p)).format_ids.join("+");
        assert_eq!(ids(Preset::Best { max_height: None }), "399+251");
        assert_eq!(ids(Preset::Mp4 { max_height: None }), "137+140");
        assert_eq!(ids(Preset::Mp4 { max_height: Some(720) }), "136+140");
        assert_eq!(ids(Preset::Best { max_height: Some(720) }), "398+251");
        assert_eq!(ids(Preset::Best { max_height: Some(2160) }), "399+251");
        assert_eq!(ids(Preset::Audio { format: AudioFormat::M4a }), "140");
        assert_eq!(ids(Preset::Audio { format: AudioFormat::Opus }), "251");
        assert_eq!(ids(Preset::Audio { format: AudioFormat::Mp3 }), "251");
        let r = approx_resolve(&m, &req(Preset::Mp4 { max_height: None }));
        assert_eq!((r.height, r.ext.as_str(), r.bytes), (Some(1080), "mp4", Some(37_577_764.0 + 2_430_774.0)));
        assert!(!r.exact && r.bytes_approx && r.vcodec.starts_with("avc1"));
    }

    #[test]
    fn a_cap_below_everything_falls_back_to_the_smallest() {
        let m = info();
        let r = approx_resolve(&m, &req(Preset::Mp4 { max_height: Some(100) }));
        assert_eq!(r.height, Some(144));
    }

    #[test]
    fn an_explicit_pick_is_used_as_is() {
        let m = info();
        let mut request = req(Preset::Best { max_height: None });
        request.formats = Some(FormatPick { video: Some("160".into()), audio: Some("139".into()) });
        let r = approx_resolve(&m, &request);
        assert_eq!((r.format_ids.join("+"), r.height), ("160+139".to_string(), Some(144)));
    }

    #[test]
    fn parses_what_yt_dlp_prints_for_a_resolution() {
        let line = r#"{"top":{"format_id": "136+140", "ext": "mp4", "width": 1280, "height": 720, "fps": 24, "vcodec": "avc1.4D401F", "acodec": "mp4a.40.2", "filesize_approx": 14017680, "tbr": 747.319, "duration": 150},"parts":[{"format_id": "136", "filesize": 11586906, "filesize_approx": 11586897, "tbr": 617.799, "vcodec": "avc1.4D401F", "acodec": "none", "height": 720, "fps": 24}, {"format_id": "140", "filesize": 2430774, "filesize_approx": 2430766, "tbr": 129.52, "vcodec": "none", "acodec": "mp4a.40.2"}]}"#;
        let r = parse_resolved(line, &req(Preset::Mp4 { max_height: Some(720) }), Some(1080))
            .unwrap_or_else(|| panic!("unparsed"));
        assert_eq!(r.format_ids, ["136", "140"]);
        assert_eq!((r.height, r.ext.as_str(), r.bytes, r.bytes_approx), (Some(720), "mp4", Some(14_017_680.0), false));
        assert!(r.exact && r.source_height == Some(1080));

        // Audio: yt-dlp still reports the video's stale height; the preset decides the extension.
        let line = r#"{"top":{"format_id": "251", "ext": "webm", "width": 1920, "height": 1080, "fps": 24, "vcodec": "none", "acodec": "opus", "filesize": 2469546, "tbr": 131.602, "duration": 150},"parts":[]}"#;
        let r = parse_resolved(line, &req(Preset::Audio { format: AudioFormat::Mp3 }), None)
            .unwrap_or_else(|| panic!("unparsed"));
        assert_eq!((r.height, r.ext.as_str(), r.vcodec.as_str()), (None, "mp3", ""));
        assert!(r.bytes_approx, "mp3 from opus is re-encoded, so the size is a guess");
        assert!(parse_resolved("NA", &req(Preset::Best { max_height: None }), None).is_none());
    }
}
