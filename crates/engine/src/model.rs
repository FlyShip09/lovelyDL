//! Every type that crosses IPC. The TypeScript side is generated from these.
//!
//! Byte counts and timestamps are `f64` because TypeScript numbers are; both stay exact
//! far beyond any real file size or date.

use serde::{Deserialize, Serialize};
use specta::Type;

pub type JobId = String;

/// The default output template: title capped at 200 bytes, then the id.
pub const DEFAULT_TEMPLATE: &str = "%(title).200B [%(id)s].%(ext)s";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    Paste,
    Typed,
    Drop,
    DeepLink,
    Clipboard,
    Playlist,
    Again,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum JobState {
    Queued,
    Probing,
    Downloading,
    PostProcessing,
    Completed,
    Paused,
    Failed,
    Canceled,
}

/// What the job is doing right now. The UI turns this into words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Waiting,
    Probing,
    Preparing,
    Video,
    Audio,
    Media,
    Subtitles,
    Merging,
    ExtractingAudio,
    Converting,
    EmbeddingMetadata,
    EmbeddingThumbnail,
    EmbeddingSubtitles,
    SponsorBlock,
    Fixing,
    Moving,
    WaitingForLive,
    Done,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub stage: Stage,
    /// 1-based position of the current stage, and how many stages are expected.
    pub stage_index: u32,
    pub stage_count: u32,
    /// Overall progress from 0 to 1, when it can be known.
    pub fraction: Option<f64>,
    pub downloaded: f64,
    pub total: Option<f64>,
    pub speed: Option<f64>,
    pub eta: Option<f64>,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            stage: Stage::Waiting,
            stage_index: 0,
            stage_count: 0,
            fraction: None,
            downloaded: 0.0,
            total: None,
            speed: None,
            eta: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AudioFormat {
    M4a,
    Mp3,
    Opus,
    Flac,
}

impl AudioFormat {
    pub fn ext(self) -> &'static str {
        match self {
            Self::M4a => "m4a",
            Self::Mp3 => "mp3",
            Self::Opus => "opus",
            Self::Flac => "flac",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Preset {
    /// Best quality in whatever container that needs.
    #[serde(rename_all = "camelCase")]
    Best {
        max_height: Option<u32>,
    },
    /// H.264 and AAC in MP4, for players that only take that.
    #[serde(rename_all = "camelCase")]
    Mp4 {
        max_height: Option<u32>,
    },
    Audio {
        format: AudioFormat,
    },
}

impl Preset {
    pub fn is_audio(&self) -> bool {
        matches!(self, Self::Audio { .. })
    }

    /// Preset names accepted from deep links: `best`, `mp4`, `best-1080`, `mp4-720`, `mp3`…
    pub fn from_name(name: &str) -> Option<Self> {
        let (head, cap) = match name.split_once('-') {
            Some((h, c)) => (h, Some(c.parse::<u32>().ok()?)),
            None => (name, None),
        };
        if let Some(h) = cap
            && ![2160, 1440, 1080, 720, 480].contains(&h)
        {
            return None;
        }
        Some(match (head, cap) {
            ("best", max_height) => Self::Best { max_height },
            ("mp4", max_height) => Self::Mp4 { max_height },
            ("m4a", None) => Self::Audio { format: AudioFormat::M4a },
            ("mp3", None) => Self::Audio { format: AudioFormat::Mp3 },
            ("opus", None) => Self::Audio { format: AudioFormat::Opus },
            ("flac", None) => Self::Audio { format: AudioFormat::Flac },
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SponsorBlock {
    #[default]
    Off,
    Remove,
    Mark,
}

/// A time range, as yt-dlp takes it: `1:30`, `90`, `0:01:30`. Empty means open-ended.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct Options {
    pub subtitles: bool,
    pub sub_langs: Vec<String>,
    pub auto_subs: bool,
    pub embed_subs: bool,
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub embed_chapters: bool,
    pub sponsorblock: SponsorBlock,
    pub sponsorblock_categories: Vec<String>,
    pub section: Option<Section>,
    pub precise_cuts: bool,
    pub live_from_start: bool,
    pub wait_for_live: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            subtitles: false,
            sub_langs: vec!["en".into()],
            auto_subs: false,
            embed_subs: true,
            embed_thumbnail: true,
            embed_metadata: true,
            embed_chapters: true,
            sponsorblock: SponsorBlock::Off,
            sponsorblock_categories: vec!["sponsor".into()],
            section: None,
            precise_cuts: false,
            live_from_start: false,
            wait_for_live: false,
        }
    }
}

/// An explicit pair from the Advanced picker. Overrides the preset's selection.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FormatPick {
    pub video: Option<String>,
    pub audio: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub preset: Preset,
    #[serde(default)]
    pub formats: Option<FormatPick>,
    #[serde(default)]
    pub options: Options,
    /// Destination folder for this download only. `None` uses the setting.
    #[serde(default)]
    pub destination: Option<String>,
}

impl Default for DownloadRequest {
    fn default() -> Self {
        Self { preset: Preset::Mp4 { max_height: None }, formats: None, options: Options::default(), destination: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Video,
    Audio,
}

/// What a request will actually produce for one item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Resolved {
    /// `true` when yt-dlp's own format selection produced this; `false` for the quick estimate.
    pub exact: bool,
    pub format_ids: Vec<String>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub vcodec: String,
    pub acodec: String,
    /// Final container or audio format.
    pub ext: String,
    pub bytes: Option<f64>,
    /// The size is an estimate: a guess at the selection, a stream without an exact size,
    /// or audio that will be re-encoded.
    pub bytes_approx: bool,
    /// Tallest video the source offers, so the UI can say when a preset tops out lower.
    pub source_height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Format {
    pub id: String,
    pub ext: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    /// Empty when the stream has no video (or no audio).
    pub vcodec: String,
    pub acodec: String,
    /// Total bitrate in kbit/s.
    pub tbr: Option<f64>,
    pub bytes: Option<f64>,
    pub bytes_approx: bool,
    pub note: String,
    pub dynamic_range: String,
    pub language: String,
    pub protocol: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LiveStatus {
    NotLive,
    Live,
    Upcoming,
    WasLive,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub id: String,
    pub title: String,
    pub uploader: String,
    pub duration: Option<f64>,
    pub site: String,
    pub thumbnail: String,
    pub webpage_url: String,
    pub live: LiveStatus,
    /// Unix seconds at which an upcoming stream is scheduled to start.
    pub release_at: Option<f64>,
    pub age_restricted: bool,
    pub members_only: bool,
    pub drm: bool,
    pub formats: Vec<Format>,
    pub subtitles: Vec<String>,
    pub auto_captions: bool,
    pub chapters: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistEntry {
    pub id: String,
    pub url: String,
    pub title: String,
    pub uploader: String,
    pub duration: Option<f64>,
    pub thumbnail: String,
    /// Already in the download archive.
    pub downloaded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    SignIn,
    AgeRestricted,
    Private,
    Unavailable,
    GeoBlocked,
    MembersOnly,
    Drm,
    UnsupportedUrl,
    FormatUnavailable,
    Http403,
    Http429,
    JsRuntime,
    FfmpegMissing,
    YtdlpMissing,
    DiskFull,
    PermissionDenied,
    Network,
    Cookies,
    LiveNotStarted,
    /// The link is a playlist; the user has to choose which videos to take.
    Playlist,
    Crashed,
    Unknown,
}

/// The one primary fix offered for an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Fix {
    Retry,
    UseCookies,
    ChooseFormat,
    UpdateYtdlp,
    InstallComponents,
    ChooseFolder,
    WaitForLive,
    ChooseVideos,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JobError {
    pub kind: ErrorKind,
    pub fix: Option<Fix>,
    pub retryable: bool,
    /// The last error line from yt-dlp, redacted.
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub title: String,
    /// 1-based position in the playlist.
    pub index: u32,
    pub count: u32,
    pub skip_downloaded: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: JobId,
    pub url: String,
    pub source: Source,
    pub title: String,
    pub uploader: String,
    pub duration: Option<f64>,
    pub site: String,
    pub thumbnail: String,
    pub kind: MediaKind,
    pub request: DownloadRequest,
    pub state: JobState,
    pub progress: Progress,
    pub planned: Option<Resolved>,
    /// Final file path, empty until the download completes.
    pub output: String,
    /// Set when history is queried: the file is no longer where it was saved.
    #[serde(default)]
    pub file_missing: bool,
    pub bytes: Option<f64>,
    pub error: Option<JobError>,
    pub attempts: u32,
    pub group: Option<Group>,
    /// Hidden from the Downloads list, still in History.
    pub cleared: bool,
    pub created_at: f64,
    pub started_at: Option<f64>,
    pub finished_at: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PlaylistChoice {
    Video,
    Playlist,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PreviewStatus {
    Probing,
    /// The link names both a video and a playlist; the user has to say which.
    Ambiguous,
    #[serde(rename_all = "camelCase")]
    Ready {
        info: Box<MediaInfo>,
        resolved: Resolved,
    },
    #[serde(rename_all = "camelCase")]
    Playlist {
        title: String,
        uploader: String,
        entries: Vec<PlaylistEntry>,
        /// `false` while entries are still arriving.
        complete: bool,
    },
    Failed {
        error: JobError,
    },
}

/// The link the user is looking at before it becomes a download.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub id: String,
    pub url: String,
    pub source: Source,
    pub request: DownloadRequest,
    pub status: PreviewStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum OnAdd {
    Preview,
    Download,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Channel {
    Stable,
    Nightly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    pub onboarding_done: bool,
    pub download_dir: String,
    pub on_add: OnAdd,
    pub default_request: DownloadRequest,
    pub max_concurrent: u32,
    pub max_per_site: u32,
    pub filename_template: String,
    pub playlist_index: bool,
    pub skip_downloaded: bool,
    /// yt-dlp rate syntax such as `2M` or `500K`. Empty means unlimited.
    pub rate_limit: String,
    pub proxy: String,
    pub pace_requests: bool,
    /// Browser to read cookies from. Empty means none.
    pub cookies_browser: String,
    pub ytdlp_channel: Channel,
    pub use_system_tools: bool,
    pub auto_update_tools: bool,
    pub custom_ytdlp: String,
    pub custom_ffmpeg_dir: String,
    /// `name:path`, for example `node:C:\Program Files\nodejs\node.exe`.
    pub custom_js_runtime: String,
    pub custom_args_enabled: bool,
    /// One yt-dlp argument per entry. Used only when `custom_args_enabled` is set.
    pub custom_args: Vec<String>,
    pub concurrent_fragments: u32,
    pub notifications: bool,
    pub notice_copied_links: bool,
    pub tray_icon: bool,
    pub keep_awake: bool,
    pub auto_update_app: bool,
    pub sidebar_visible: bool,
    /// Unix milliseconds of the last tool update check.
    pub last_tool_check: f64,
}

pub const SETTINGS_VERSION: u32 = 1;

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            onboarding_done: false,
            download_dir: String::new(),
            on_add: OnAdd::Preview,
            default_request: DownloadRequest::default(),
            max_concurrent: 3,
            max_per_site: 2,
            filename_template: DEFAULT_TEMPLATE.into(),
            playlist_index: true,
            skip_downloaded: true,
            rate_limit: String::new(),
            proxy: String::new(),
            pace_requests: false,
            cookies_browser: String::new(),
            ytdlp_channel: Channel::Stable,
            use_system_tools: false,
            auto_update_tools: true,
            custom_ytdlp: String::new(),
            custom_ffmpeg_dir: String::new(),
            custom_js_runtime: String::new(),
            custom_args_enabled: false,
            custom_args: Vec::new(),
            concurrent_fragments: 1,
            notifications: true,
            notice_copied_links: false,
            tray_icon: false,
            keep_awake: true,
            auto_update_app: true,
            sidebar_visible: true,
            last_tool_check: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ToolId {
    Ytdlp,
    Ffmpeg,
    Deno,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ToolOrigin {
    /// Installed and updated by lovelyDL.
    Managed,
    /// Found on PATH.
    System,
    /// A path the user set.
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ToolStatus {
    Missing,
    Checking,
    /// Downloading or unpacking; `fraction` runs from 0 to 1.
    Installing {
        fraction: f64,
    },
    Ready,
    Broken {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub id: ToolId,
    pub status: ToolStatus,
    pub version: String,
    /// Newer version available for a managed tool, if a check found one.
    pub latest: String,
    pub path: String,
    pub origin: ToolOrigin,
}

impl Tool {
    pub fn missing(id: ToolId) -> Self {
        Self {
            id,
            status: ToolStatus::Missing,
            version: String::new(),
            latest: String::new(),
            path: String::new(),
            origin: ToolOrigin::Managed,
        }
    }

    pub fn ready(&self) -> bool {
        self.status == ToolStatus::Ready
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Components {
    pub ytdlp: Tool,
    pub ffmpeg: Tool,
    pub deno: Tool,
}

impl Components {
    pub fn tool(&self, id: ToolId) -> &Tool {
        match id {
            ToolId::Ytdlp => &self.ytdlp,
            ToolId::Ffmpeg => &self.ffmpeg,
            ToolId::Deno => &self.deno,
        }
    }

    pub fn tool_mut(&mut self, id: ToolId) -> &mut Tool {
        match id {
            ToolId::Ytdlp => &mut self.ytdlp,
            ToolId::Ffmpeg => &mut self.ffmpeg,
            ToolId::Deno => &mut self.deno,
        }
    }
}

impl Default for Components {
    fn default() -> Self {
        Self {
            ytdlp: Tool::missing(ToolId::Ytdlp),
            ffmpeg: Tool::missing(ToolId::Ffmpeg),
            deno: Tool::missing(ToolId::Deno),
        }
    }
}

/// Something worth telling the user about outside the list itself. The shell turns it into
/// a toast when the window is not focused; the UI announces it to screen readers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Notice {
    #[serde(rename_all = "camelCase")]
    Finished { job_id: JobId, title: String, path: String },
    #[serde(rename_all = "camelCase")]
    Failed { job_id: JobId, title: String },
    #[serde(rename_all = "camelCase")]
    GroupFinished { title: String, completed: u32, failed: u32, folder: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum HistoryFilter {
    All,
    Video,
    Audio,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub items: Vec<Job>,
    pub total: u32,
}

/// System facts the UI styles itself with. Filled in by the shell, not the engine.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    /// The user's accent color as `#rrggbb`, or empty to use the fallback.
    pub accent: String,
    /// Whether a real system material is behind the window.
    pub material: bool,
    pub app_version: String,
}

/// A request from outside the web view (tray, deep link, second launch) that the UI acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum UiCommand {
    NewDownload,
    ShowDownloads,
    ShowFailed,
    ConfirmQuit,
    CheckClipboard,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub jobs: Vec<Job>,
    pub preview: Option<Preview>,
    pub settings: Settings,
    pub components: Components,
    pub system: SystemInfo,
}

/// One state change. The shell delivers these to the UI in batches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum Change {
    JobAdded(Box<Job>),
    JobUpdated(Box<Job>),
    JobRemoved(JobId),
    Progress {
        id: JobId,
        progress: Progress,
    },
    /// The ids of all live jobs, in queue order.
    Order(Vec<JobId>),
    Preview(Option<Box<Preview>>),
    #[serde(rename_all = "camelCase")]
    PlaylistEntries {
        preview_id: String,
        entries: Vec<PlaylistEntry>,
    },
    Settings(Box<Settings>),
    Components(Box<Components>),
    System(SystemInfo),
    Notice(Notice),
    Command(UiCommand),
    /// History rows changed; a visible history list should query again.
    HistoryChanged,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_names() {
        assert_eq!(Preset::from_name("best"), Some(Preset::Best { max_height: None }));
        assert_eq!(Preset::from_name("mp4-720"), Some(Preset::Mp4 { max_height: Some(720) }));
        assert_eq!(Preset::from_name("flac"), Some(Preset::Audio { format: AudioFormat::Flac }));
        for bad in ["", "mp4-719", "mp3-720", "best-", "--exec", "mp4 -x"] {
            assert_eq!(Preset::from_name(bad), None, "{bad}");
        }
    }

    #[test]
    fn settings_tolerate_missing_and_unknown_fields() {
        let s: Settings =
            serde_json::from_str(r#"{"maxConcurrent":5,"fromTheFuture":1}"#).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(s.max_concurrent, 5);
        assert_eq!(s.max_per_site, 2);
    }
}
