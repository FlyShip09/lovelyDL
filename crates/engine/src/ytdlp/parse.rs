//! Parses yt-dlp's output. Only lines that start with the sentinel are parsed; everything
//! else is log text.

use serde::Deserialize;

use crate::model::{Progress, Resolved, Stage};

pub const SENTINEL: &str = "@@lovelydl@@";

// `|null` matters: a missing field would otherwise print as a bare `NA` and break the JSON.
pub const DOWNLOAD_TEMPLATE: &str = concat!(
    "download:@@lovelydl@@DL ",
    r#"{"fid":%(info.format_id|null)j,"vcodec":%(info.vcodec|null)j,"acodec":%(info.acodec|null)j,"p":%(progress)j}"#
);
pub const POSTPROCESS_TEMPLATE: &str = "postprocess:@@lovelydl@@PP %(progress)j";
pub const FILE_TEMPLATE: &str = "after_move:@@lovelydl@@FILE %(filepath)j";
/// `false` when yt-dlp handed back a file that already existed instead of downloading. Its own
/// message for that is hidden by the quiet mode `--print` turns on.
pub const REAL_TEMPLATE: &str = "after_move:@@lovelydl@@REAL %(__real_download)j";

#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    Download(DlLine),
    Post {
        name: String,
        started: bool,
    },
    File(String),
    /// The file existed already, so nothing was downloaded.
    Existing,
    Log(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DlLine {
    pub fid: String,
    pub has_video: bool,
    pub has_audio: bool,
    pub finished: bool,
    pub downloaded: f64,
    pub total: Option<f64>,
    pub speed: Option<f64>,
    pub eta: Option<f64>,
    pub filename: String,
    pub fragment_index: Option<f64>,
    pub fragment_count: Option<f64>,
}

#[derive(Deserialize)]
struct RawDl {
    fid: Option<String>,
    vcodec: Option<String>,
    acodec: Option<String>,
    p: RawProgress,
}

// Every field can be missing or null, and totals can be floats.
#[derive(Deserialize, Default)]
struct RawProgress {
    status: Option<String>,
    downloaded_bytes: Option<f64>,
    total_bytes: Option<f64>,
    total_bytes_estimate: Option<f64>,
    speed: Option<f64>,
    eta: Option<f64>,
    filename: Option<String>,
    fragment_index: Option<f64>,
    fragment_count: Option<f64>,
    postprocessor: Option<String>,
}

/// Python prints `NaN` and `Infinity`, which are not JSON.
fn from_python_json<'a, T: Deserialize<'a>>(text: &'a str, scratch: &'a mut String) -> Option<T> {
    if let Ok(v) = serde_json::from_str(text) {
        return Some(v);
    }
    *scratch = text.replace(": NaN", ": null").replace(": -Infinity", ": null").replace(": Infinity", ": null");
    serde_json::from_str(scratch).ok()
}

fn present(codec: &Option<String>) -> bool {
    codec.as_deref().is_some_and(|c| !c.is_empty() && c != "none")
}

pub fn parse_line(line: &str) -> Line {
    let line = line.trim_end_matches(['\r', '\n']);
    let Some(rest) = line.trim_start().strip_prefix(SENTINEL) else {
        return Line::Log(line.to_string());
    };
    let mut scratch = String::new();
    let parsed = if let Some(json) = rest.strip_prefix("DL ") {
        from_python_json::<RawDl>(json, &mut scratch).map(|raw| {
            let p = raw.p;
            Line::Download(DlLine {
                fid: raw.fid.unwrap_or_default(),
                has_video: present(&raw.vcodec),
                has_audio: present(&raw.acodec),
                finished: p.status.as_deref() == Some("finished"),
                downloaded: p.downloaded_bytes.unwrap_or(0.0),
                total: p.total_bytes.or(p.total_bytes_estimate).filter(|t| *t > 0.0),
                speed: p.speed.filter(|s| s.is_finite() && *s > 0.0),
                eta: p.eta.filter(|e| e.is_finite() && *e >= 0.0),
                filename: p.filename.unwrap_or_default(),
                fragment_index: p.fragment_index,
                fragment_count: p.fragment_count,
            })
        })
    } else if let Some(json) = rest.strip_prefix("PP ") {
        from_python_json::<RawProgress>(json, &mut scratch)
            .and_then(|p| Some(Line::Post { name: p.postprocessor?, started: p.status.as_deref() == Some("started") }))
    } else if let Some(json) = rest.strip_prefix("FILE ") {
        serde_json::from_str::<String>(json).ok().map(Line::File)
    } else {
        rest.strip_prefix("REAL ")
            .map(|real| if real.trim() == "false" { Line::Existing } else { Line::Log(line.to_string()) })
    };
    // A sentinel line we cannot read is kept as log text rather than dropped.
    parsed.unwrap_or_else(|| Line::Log(line.to_string()))
}

const SUBTITLE_EXTS: [&str; 10] = ["vtt", "srt", "ass", "ssa", "lrc", "ttml", "srv1", "srv2", "srv3", "json3"];

fn is_subtitle(filename: &str) -> bool {
    filename.rsplit_once('.').is_some_and(|(_, ext)| SUBTITLE_EXTS.contains(&ext))
}

fn post_stage(name: &str) -> Stage {
    match name {
        "Merger" => Stage::Merging,
        "ExtractAudio" => Stage::ExtractingAudio,
        "Metadata" => Stage::EmbeddingMetadata,
        "EmbedThumbnail" => Stage::EmbeddingThumbnail,
        "EmbedSubtitle" => Stage::EmbeddingSubtitles,
        "SponsorBlock" | "ModifyChapters" => Stage::SponsorBlock,
        "MoveFiles" => Stage::Moving,
        "VideoRemuxer" | "VideoConvertor" | "SubtitlesConvertor" | "ThumbnailsConvertor" => Stage::Converting,
        _ => Stage::Fixing,
    }
}

/// Turns the stream of parsed lines for one job into overall progress.
///
/// Progress restarts for every stream. When the total size is known up front the fraction is
/// weighted by bytes; otherwise it is "stream i of n".
#[derive(Debug)]
pub struct Tracker {
    stream_count: u32,
    planned_total: Option<f64>,
    current_fid: String,
    streams_seen: u32,
    finished_bytes: f64,
    current_bytes: f64,
    /// True once a media stream has reported progress; post-processors before that are setup.
    pub downloading: bool,
    pub in_post: bool,
    pub progress: Progress,
}

impl Tracker {
    pub fn new(planned: Option<&Resolved>) -> Self {
        Self {
            stream_count: planned.map_or(1, |r| r.format_ids.len().max(1) as u32),
            planned_total: planned.and_then(|r| r.bytes).filter(|b| *b > 0.0),
            current_fid: String::new(),
            streams_seen: 0,
            finished_bytes: 0.0,
            current_bytes: 0.0,
            downloading: false,
            in_post: false,
            progress: Progress { stage: Stage::Preparing, ..Progress::default() },
        }
    }

    pub fn on_download(&mut self, d: &DlLine) {
        self.in_post = false;
        if is_subtitle(&d.filename) {
            self.progress.stage = Stage::Subtitles;
            return;
        }
        if !self.downloading || d.fid != self.current_fid {
            // A new stream starts: bank the previous one.
            self.finished_bytes += self.current_bytes;
            self.current_bytes = 0.0;
            self.current_fid = d.fid.clone();
            self.streams_seen += 1;
            self.stream_count = self.stream_count.max(self.streams_seen);
            self.downloading = true;
        }
        self.current_bytes = d.downloaded;
        let stage = match (d.has_video, d.has_audio) {
            (true, false) => Stage::Video,
            (false, true) => Stage::Audio,
            _ => Stage::Media,
        };
        let stream_fraction = match (d.total, d.fragment_index, d.fragment_count) {
            _ if d.finished => Some(1.0),
            (Some(total), _, _) => Some(d.downloaded / total),
            (None, Some(i), Some(n)) if n > 0.0 => Some(i / n),
            _ => None,
        };
        let done = self.finished_bytes + self.current_bytes;
        let fraction = match self.planned_total {
            Some(total) => {
                // The estimate can be low; never let the bar pass the end before the end.
                let stream_total = self.finished_bytes + d.total.unwrap_or(self.current_bytes);
                Some(done / total.max(stream_total))
            }
            None => stream_fraction.map(|f| (f64::from(self.streams_seen - 1) + f) / f64::from(self.stream_count)),
        }
        .map(|f| f.clamp(0.0, 1.0));
        let same_stage = self.progress.stage == stage;
        self.progress = Progress {
            stage,
            stage_index: self.streams_seen,
            stage_count: self.stream_count,
            // Within a stage the bar never moves backwards.
            fraction: match (fraction, self.progress.fraction) {
                (Some(new), Some(old)) if same_stage => Some(new.max(old)),
                (new, _) => new,
            },
            downloaded: done,
            total: self.planned_total.or(if self.stream_count == 1 { d.total } else { None }),
            speed: if d.finished { None } else { d.speed },
            eta: if d.finished { None } else { d.eta },
        };
    }

    /// Returns `true` when this post-processor marks the start of post-processing proper.
    pub fn on_post(&mut self, name: &str, started: bool) -> bool {
        if !started || !self.downloading {
            return false;
        }
        self.in_post = true;
        self.progress.stage = post_stage(name);
        self.progress.fraction = Some(1.0);
        self.progress.speed = None;
        self.progress.eta = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DL: &str = r#"@@lovelydl@@DL {"fid":"160","vcodec":"avc1.4D400C","acodec":"none","p":{"status": "downloading", "downloaded_bytes": 1047552, "total_bytes": 1095577, "tmpfilename": "C:\\t\\a.f160.mp4.part", "filename": "C:\\t\\a.f160.mp4", "eta": 0, "speed": 8103771.66, "elapsed": 0.51, "ctx_id": null, "_percent_str": " 95.6%"}}"#;

    fn dl(line: &str) -> DlLine {
        match parse_line(line) {
            Line::Download(d) => d,
            other => panic!("not a download line: {other:?}"),
        }
    }

    #[test]
    fn download_line() {
        let d = dl(DL);
        assert_eq!(d.fid, "160");
        assert!(d.has_video && !d.has_audio && !d.finished);
        assert_eq!(d.downloaded, 1_047_552.0);
        assert_eq!(d.total, Some(1_095_577.0));
        assert_eq!(d.eta, Some(0.0));
    }

    #[test]
    fn nulls_estimates_and_python_floats() {
        let d = dl(
            r#"@@lovelydl@@DL {"fid":"230","vcodec":"avc1","acodec":null,"p":{"status": "downloading", "downloaded_bytes": 10, "total_bytes": null, "total_bytes_estimate": 99.5, "speed": NaN, "eta": Infinity, "fragment_index": 2, "fragment_count": 8}}"#,
        );
        assert_eq!(d.total, Some(99.5));
        assert_eq!((d.speed, d.eta), (None, None));
        assert_eq!((d.fragment_index, d.fragment_count), (Some(2.0), Some(8.0)));
        // Unknown size, no fragments: nothing to divide by.
        let d = dl(r#"@@lovelydl@@DL {"fid":null,"vcodec":null,"acodec":null,"p":{"downloaded_bytes": 5}}"#);
        assert_eq!((d.total, d.fid.as_str()), (None, ""));
    }

    #[test]
    fn post_file_and_log_lines() {
        assert_eq!(
            parse_line(
                r#"@@lovelydl@@PP {"status": "started", "postprocessor": "Merger", "_default_template": "Merger started"}"#
            ),
            Line::Post { name: "Merger".into(), started: true }
        );
        assert_eq!(
            parse_line("@@lovelydl@@FILE \"C:\\\\out\\\\Caminandes 3\\uff1a Llamigos [x].mp4\"\r\n"),
            Line::File("C:\\out\\Caminandes 3\u{ff1a} Llamigos [x].mp4".into())
        );
        // What yt-dlp 2026.10.26 prints for a skipped existing file, and for a real download.
        assert_eq!(parse_line("@@lovelydl@@REAL false"), Line::Existing);
        assert!(matches!(parse_line("@@lovelydl@@REAL true"), Line::Log(_)));
        for log in ["ERROR: [youtube] x: This video is unavailable", "@@lovelydl@@DL {broken", ""] {
            assert_eq!(parse_line(log), Line::Log(log.into()));
        }
    }

    fn line(fid: &str, video: bool, done: f64, total: f64, finished: bool) -> DlLine {
        DlLine {
            fid: fid.into(),
            has_video: video,
            has_audio: !video,
            finished,
            downloaded: done,
            total: Some(total),
            filename: format!("a.f{fid}.mp4"),
            ..DlLine::default()
        }
    }

    fn plan(ids: &[&str], bytes: Option<f64>) -> Resolved {
        Resolved {
            exact: true,
            format_ids: ids.iter().map(|s| s.to_string()).collect(),
            height: None,
            fps: None,
            vcodec: String::new(),
            acodec: String::new(),
            ext: "mp4".into(),
            bytes,
            bytes_approx: false,
            source_height: None,
        }
    }

    #[test]
    fn weighted_by_bytes_when_the_total_is_known() {
        let mut t = Tracker::new(Some(&plan(&["137", "140"], Some(1000.0))));
        t.on_download(&line("137", true, 450.0, 900.0, false));
        assert_eq!((t.progress.stage, t.progress.fraction), (Stage::Video, Some(0.45)));
        t.on_download(&line("137", true, 900.0, 900.0, true));
        t.on_download(&line("140", false, 50.0, 100.0, false));
        assert_eq!((t.progress.stage, t.progress.fraction), (Stage::Audio, Some(0.95)));
        assert_eq!((t.progress.stage_index, t.progress.stage_count), (2, 2));
    }

    #[test]
    fn stage_i_of_n_when_it_is_not() {
        let mut t = Tracker::new(Some(&plan(&["137", "140"], None)));
        t.on_download(&line("137", true, 50.0, 100.0, false));
        assert_eq!(t.progress.fraction, Some(0.25));
        t.on_download(&line("140", false, 50.0, 100.0, false));
        assert_eq!(t.progress.fraction, Some(0.75));
    }

    #[test]
    fn never_backwards_within_a_stage_and_low_estimates_do_not_overflow() {
        let mut t = Tracker::new(Some(&plan(&["18"], Some(100.0))));
        t.on_download(&line("18", true, 80.0, 400.0, false));
        assert_eq!(t.progress.fraction, Some(0.2));
        // A shrinking estimate must not pull the bar back.
        t.on_download(&line("18", true, 81.0, 800.0, false));
        assert_eq!(t.progress.fraction, Some(0.2));
    }

    #[test]
    fn post_processors_before_the_download_are_setup() {
        let mut t = Tracker::new(None);
        assert!(!t.on_post("ThumbnailsConvertor", true));
        assert_eq!(t.progress.stage, Stage::Preparing);
        t.on_download(&line("18", true, 10.0, 10.0, true));
        assert!(t.on_post("Merger", true));
        assert_eq!((t.progress.stage, t.progress.fraction), (Stage::Merging, Some(1.0)));
        assert!(!t.on_post("Merger", false));
    }

    #[test]
    fn subtitles_do_not_count_as_a_stream() {
        let mut t = Tracker::new(Some(&plan(&["18"], Some(100.0))));
        let mut sub = line("18", true, 5.0, 5.0, true);
        sub.filename = "a.en.vtt".into();
        t.on_download(&sub);
        assert_eq!((t.progress.stage, t.progress.downloaded), (Stage::Subtitles, 0.0));
    }
}
