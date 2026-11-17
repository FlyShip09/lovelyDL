//! The engine: one actor task that owns every piece of state.
//!
//! Callers hold an [`Engine`] handle and send it closures; the closures run one at a time on
//! the actor with `&mut Core`. Child processes and other async work report back the same way,
//! so every state change, including every job state transition, happens in one place.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, oneshot};

use crate::model::*;
use crate::state::{can_transition, is_active};
use crate::store::{Store, StoreError};
use crate::toolchain::{self, Toolchain};
use crate::ytdlp::args::{self, Tools};
use crate::ytdlp::errors::{self, classify};
use crate::ytdlp::parse::{Line, Tracker, parse_line};
use crate::ytdlp::probe::{self, Probed};
use crate::ytdlp::run::{self, Stop};
use crate::{redact, settings};

/// All pending changes go out as one batch at most this often.
const FLUSH_WINDOW: Duration = Duration::from_millis(100);
/// How long a probe result may be reused for the download.
const PROBE_TTL: Duration = Duration::from_secs(30 * 60);
const DAY_MS: f64 = 24.0 * 60.0 * 60.0 * 1000.0;
const LOG_LINES: usize = 300;
const MAX_THUMBNAIL: usize = 8 * 1024 * 1024;

/// A built-in item for previewing file name templates when no link is open.
const SAMPLE_INFO: &str = r#"{"id":"SkVqJ1SGeL0","title":"Caminandes 3: Llamigos","uploader":"Blender","channel":"Blender","upload_date":"20160130","duration":150,"ext":"mp4","extractor":"youtube","extractor_key":"Youtube","webpage_url":"https://www.youtube.com/watch?v=SkVqJ1SGeL0","_type":"video","formats":[{"format_id":"18","url":"https://example.invalid/sample.mp4","ext":"mp4","vcodec":"avc1.42001E","acodec":"mp4a.40.2","width":640,"height":360,"protocol":"https"}]}"#;

pub struct Dirs {
    /// Database, download archive and settings.
    pub data: PathBuf,
    /// Probe cache, thumbnails and partial downloads.
    pub cache: PathBuf,
    /// Managed tools.
    pub bin: PathBuf,
    /// Where downloads go until the user chooses a folder.
    pub downloads: PathBuf,
    /// The user's home folder, replaced with `~` in logs.
    pub home: Option<PathBuf>,
}

impl Dirs {
    /// Everything under one folder. For tests and the command-line example.
    pub fn under(root: &Path) -> Self {
        Self {
            data: root.join("data"),
            cache: root.join("cache"),
            bin: root.join("bin"),
            downloads: root.join("downloads"),
            home: None,
        }
    }
}

pub type Sink = Box<dyn Fn(Vec<Change>) + Send + Sync>;

pub struct Config {
    pub dirs: Dirs,
    /// Receives every batch of changes.
    pub sink: Sink,
    /// `lovelyDL/<version>`; sent to GitHub and shown in copied details.
    pub user_agent: String,
    /// GitHub API base URL override, for tests.
    pub api: Option<String>,
    /// Wait before the first automatic retry; it doubles with each attempt. Short in tests.
    pub retry_backoff: Duration,
}

impl Config {
    pub const DEFAULT_RETRY_BACKOFF: Duration = Duration::from_secs(5);
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("the engine has stopped")]
    Stopped,
}

type Call = Box<dyn FnOnce(&mut Core) + Send>;

struct Shared {
    dirs: Dirs,
    store: Store,
    toolchain: Arc<Toolchain>,
    /// For thumbnails: addresses a page chose, so public internet only.
    web: reqwest::Client,
}

#[derive(Clone)]
pub struct Engine {
    tx: mpsc::UnboundedSender<Call>,
    shared: Arc<Shared>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Probe,
    Download,
}

struct Run {
    seq: u64,
    stop: Stop,
    phase: Phase,
    tracker: Tracker,
    /// The download used cached probe info, so a 403 may just mean it went stale.
    cached: bool,
    file: Option<String>,
    media: Option<(Box<MediaInfo>, String)>,
    saw_entries: bool,
    /// yt-dlp found a finished file with this name and skipped the download.
    name_taken: bool,
}

#[derive(Default)]
struct JobLog {
    args: Vec<String>,
    lines: VecDeque<String>,
}

struct PreviewRun {
    stop: Option<Stop>,
    args: Vec<String>,
    log: Vec<String>,
    archive: HashSet<String>,
    auto_start: bool,
}

struct Core {
    me: mpsc::UnboundedSender<Call>,
    shared: Arc<Shared>,
    sink: Sink,
    user_agent: String,
    settings: Settings,
    /// Settings waiting to be written by the one writer task.
    settings_out: tokio::sync::watch::Sender<Option<Settings>>,
    components: Components,
    tool_paths: HashMap<ToolId, PathBuf>,
    /// False while a health check is under way; nothing starts on stale tool state.
    tools_known: bool,
    /// Only the newest health check's answer is used.
    refresh_seq: u64,
    installing: HashSet<ToolId>,
    checking_updates: bool,
    /// Jobs to retry once the tools they were missing are in place.
    waiting_for_tools: Vec<JobId>,
    jobs: Vec<Job>,
    runs: HashMap<JobId, Run>,
    /// Processes that were told to stop and have not exited yet, by run number.
    stopping: HashMap<u64, JobId>,
    /// When each job's cached probe info was written.
    probed: HashMap<JobId, Instant>,
    /// Jobs that already got their one re-probe after an HTTP 403.
    reprobed: HashSet<JobId>,
    /// The waiting download whose title and thumbnail are being fetched ahead of its turn.
    peeking: Option<JobId>,
    /// Waiting downloads already looked at, so a link that will not load is tried once.
    peeked: HashSet<JobId>,
    /// Jobs saving under a numbered name because the plain one was taken: id → number.
    renamed: HashMap<JobId, u32>,
    /// Queued jobs waiting out an automatic retry, and when they may start.
    retry_at: HashMap<JobId, Instant>,
    retry_backoff: Duration,
    logs: HashMap<JobId, JobLog>,
    preview: Option<Preview>,
    preview_run: Option<PreviewRun>,
    /// A preview probe asked for before the first tool check finished; it starts right after.
    preview_waiting: Option<(String, PlaylistChoice)>,
    resolve_seq: u64,
    pending: Vec<Change>,
    flush_armed: bool,
    last_flush: Instant,
    needs_pump: bool,
    seq: u64,
    shutdown: Option<oneshot::Sender<()>>,
    shutting_down: bool,
}

pub fn now_ms() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_millis() as f64)
}

/// Every http(s) link in a piece of text, in order, without repeats.
pub fn extract_urls(text: &str) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    for token in text.split(|c: char| c.is_whitespace() || c == '<' || c == '>' || c == '"') {
        let Ok(url) = url::Url::parse(token) else { continue };
        if matches!(url.scheme(), "http" | "https") && url.host_str().is_some() && !urls.iter().any(|u| u == token) {
            urls.push(token.to_string());
        }
    }
    urls
}

/// A watch link that also names a playlist: the user has to say which one they mean.
// shortcut: recognizes the `v` + `list` query pair (YouTube and YouTube Music). Other sites
// with such links download just the video; widen this when one is reported.
fn is_ambiguous(link: &str) -> bool {
    url::Url::parse(link).is_ok_and(|u| {
        let has = |key: &str| u.query_pairs().any(|(k, v)| k == key && !v.is_empty());
        has("v") && has("list")
    })
}

fn host_of(link: &str) -> String {
    url::Url::parse(link)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_string()))
        .unwrap_or_default()
}

/// A folder name Windows accepts, from a playlist title.
fn folder_name(title: &str) -> String {
    let cleaned: String =
        title.chars().map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { ' ' } else { c }).collect();
    let joined = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed: String = joined.trim_matches(|c| c == '.' || c == ' ').chars().take(120).collect();
    if trimmed.is_empty() { "Playlist".into() } else { trimmed }
}

fn same_volume(a: &Path, b: &Path) -> bool {
    use std::path::Component::Prefix;
    match (a.components().next(), b.components().next()) {
        (Some(Prefix(x)), Some(Prefix(y))) => x.as_os_str().eq_ignore_ascii_case(y.as_os_str()),
        // shortcut: without drive prefixes (not Windows) everything counts as one volume;
        // compare device ids when another system is supported.
        _ => true,
    }
}

/// What kind of image this is, from its first bytes. `None` if it is not one we show.
fn image_type(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0xFF, 0xD8, ..] => Some("image/jpeg"),
        [0x89, b'P', b'N', b'G', ..] => Some("image/png"),
        [b'G', b'I', b'F', b'8', ..] => Some("image/gif"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("image/webp"),
        [_, _, _, _, b'f', b't', b'y', b'p', b'a', b'v', b'i', b'f', ..] => Some("image/avif"),
        _ => None,
    }
}

async fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        tokio::fs::create_dir_all(dir).await?;
    }
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, path).await
}

/// Creates a job's temp folder. One on the destination drive lives in a hidden folder there.
async fn prepare_temp(temp: PathBuf, hidden_parent: bool) {
    let _ = tokio::fs::create_dir_all(&temp).await;
    #[cfg(windows)]
    if hidden_parent && let Some(parent) = temp.parent() {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, SetFileAttributesW};
        let wide: Vec<u16> = parent.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: `wide` is a NUL-terminated UTF-16 path that outlives the call.
        unsafe { SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_HIDDEN) };
    }
    #[cfg(not(windows))]
    let _ = hidden_parent;
}

impl Engine {
    /// Loads settings and the queue, then starts the actor. Call from inside a tokio runtime.
    /// Reads two small files synchronously; this is start-up, before anything is on screen.
    pub fn start(config: Config) -> Result<Self, EngineError> {
        let dirs = config.dirs;
        let mut loaded = settings::load(&dirs.data.join("settings.json"));
        if loaded.download_dir.is_empty() {
            loaded.download_dir = dirs.downloads.to_string_lossy().into_owned();
        }
        let (store, mut jobs) = Store::open(&dirs.data.join("lovelydl.sqlite3"))?;
        // Whatever was running when the app last closed goes back in line and resumes.
        for (pos, job) in jobs.iter_mut().enumerate() {
            if is_active(job.state) {
                job.state = JobState::Queued;
                job.progress.speed = None;
                job.progress.eta = None;
                store.save(job, pos);
            }
        }
        let mut toolchain = Toolchain::new(dirs.bin.clone(), &config.user_agent);
        if let Some(api) = config.api {
            toolchain.api = api;
        }
        let web = crate::web::public_client(&config.user_agent);
        let shared = Arc::new(Shared { dirs, store, toolchain: Arc::new(toolchain), web });
        let (tx, mut rx) = mpsc::unbounded_channel::<Call>();
        let (settings_out, mut settings_in) = tokio::sync::watch::channel(None::<Settings>);
        let settings_path = shared.dirs.data.join("settings.json");
        tokio::spawn(async move {
            // One writer that always takes the newest settings: saves never overlap, and an
            // older copy can never land after a newer one.
            while settings_in.changed().await.is_ok() {
                let Some(latest) = settings_in.borrow_and_update().clone() else { continue };
                let path = settings_path.clone();
                let saved = tokio::task::spawn_blocking(move || settings::save(&path, &latest)).await;
                if let Ok(Err(error)) = saved {
                    tracing::error!(%error, "could not save settings");
                }
            }
        });
        let mut core = Core {
            me: tx.clone(),
            shared: shared.clone(),
            sink: config.sink,
            user_agent: config.user_agent,
            settings: loaded,
            settings_out,
            components: Components::default(),
            tool_paths: HashMap::new(),
            tools_known: false,
            refresh_seq: 0,
            installing: HashSet::new(),
            checking_updates: false,
            waiting_for_tools: Vec::new(),
            jobs,
            runs: HashMap::new(),
            stopping: HashMap::new(),
            probed: HashMap::new(),
            reprobed: HashSet::new(),
            peeking: None,
            peeked: HashSet::new(),
            renamed: HashMap::new(),
            retry_at: HashMap::new(),
            retry_backoff: config.retry_backoff,
            logs: HashMap::new(),
            preview: None,
            preview_run: None,
            preview_waiting: None,
            resolve_seq: 0,
            pending: Vec::new(),
            flush_armed: false,
            last_flush: Instant::now() - FLUSH_WINDOW,
            needs_pump: false,
            seq: 0,
            shutdown: None,
            shutting_down: false,
        };
        tokio::spawn(async move {
            core.housekeeping();
            core.refresh_components();
            while let Some(call) = rx.recv().await {
                call(&mut core);
                if std::mem::take(&mut core.needs_pump) {
                    core.pump();
                }
                // Shut down and every process gone: send what is left and stop.
                if core.shutting_down && core.shutdown.is_none() && core.stopping.is_empty() {
                    core.flush();
                    break;
                }
            }
        });
        Ok(Self { tx, shared })
    }

    fn cast(&self, f: impl FnOnce(&mut Core) + Send + 'static) {
        let _ = self.tx.send(Box::new(f));
    }

    async fn call<R: Send + 'static>(&self, f: impl FnOnce(&mut Core) -> R + Send + 'static) -> Result<R, EngineError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Box::new(move |core| {
                let _ = tx.send(f(core));
            }))
            .map_err(|_| EngineError::Stopped)?;
        rx.await.map_err(|_| EngineError::Stopped)
    }

    /// Runs `f` on the actor and flattens its own error into a message for the UI.
    async fn ask<R: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Core) -> Result<R, String> + Send + 'static,
    ) -> Result<R, String> {
        self.call(f).await.map_err(|e| e.to_string())?
    }

    pub async fn snapshot(&self) -> Result<Snapshot, EngineError> {
        self.call(|core| Snapshot {
            jobs: core.jobs.clone(),
            preview: core.preview.clone(),
            settings: core.settings.clone(),
            components: core.components.clone(),
            system: SystemInfo::default(),
        })
        .await
    }

    /// Adds every link found in `text`. One link opens the preview (or downloads straight
    /// away, by the setting); several go to the queue. Returns how many links were found.
    pub async fn add_links(&self, text: String, source: Source) -> Result<u32, String> {
        self.ask(move |core| core.add_links(&text, source)).await
    }

    /// Opens the preview for a link from outside the app. Never starts a download.
    pub async fn open_link(&self, link: String, preset: Option<String>, source: Source) -> Result<(), String> {
        self.ask(move |core| {
            let url = extract_urls(&link).into_iter().next().ok_or("That is not a web link.")?;
            let mut request = core.settings.default_request.clone();
            if let Some(preset) = preset.as_deref().and_then(Preset::from_name) {
                request.preset = preset;
            }
            core.open_preview(url, source, request, false);
            Ok(())
        })
        .await
    }

    pub fn preview_update(&self, id: String, request: DownloadRequest) {
        self.cast(move |core| core.preview_update(&id, request));
    }

    pub fn preview_choose(&self, id: String, choice: PlaylistChoice) {
        self.cast(move |core| core.preview_choose(&id, choice));
    }

    pub fn preview_dismiss(&self, id: String) {
        self.cast(move |core| {
            if core.preview.as_ref().is_some_and(|p| p.id == id) {
                core.close_preview(true);
            }
        });
    }

    /// Turns the preview into downloads. `selection` lists the playlist entry ids to take.
    pub async fn preview_start(
        &self,
        id: String,
        request: DownloadRequest,
        selection: Vec<String>,
        skip_downloaded: bool,
    ) -> Result<(), String> {
        self.ask(move |core| core.preview_start(&id, request, &selection, skip_downloaded)).await
    }

    pub fn job_pause(&self, id: JobId) {
        self.cast(move |core| core.pause(&id));
    }

    pub fn job_resume(&self, id: JobId) {
        self.cast(move |core| {
            core.transition(&id, JobState::Queued);
        });
    }

    pub fn job_cancel(&self, id: JobId) {
        self.cast(move |core| core.cancel(&id));
    }

    pub fn job_retry(&self, id: JobId) {
        self.cast(move |core| core.retry(&id));
    }

    /// Takes jobs off the Downloads list. Finished ones stay in History.
    pub fn job_remove(&self, ids: Vec<JobId>) {
        self.cast(move |core| ids.iter().for_each(|id| core.remove(id, false)));
    }

    pub fn job_apply_fix(&self, id: JobId, fix: Fix) {
        self.cast(move |core| core.apply_fix(&id, fix));
    }

    /// Changes what a job that is not running will download, then queues it again.
    pub async fn job_update_request(&self, id: JobId, request: DownloadRequest) -> Result<(), String> {
        self.ask(move |core| {
            let i = core.index(&id).ok_or("That download is no longer in the list.")?;
            if is_active(core.jobs[i].state) {
                return Err("Pause the download before changing it.".into());
            }
            let job = &mut core.jobs[i];
            job.kind = if request.preset.is_audio() { MediaKind::Audio } else { MediaKind::Video };
            job.request = request;
            job.planned = None;
            core.changed(i);
            core.retry(&id);
            Ok(())
        })
        .await
    }

    /// The formats a queued or failed job offers, for the format picker.
    pub async fn job_formats(&self, id: JobId) -> Result<MediaInfo, String> {
        let (ytdlp, args, info_path) = self
            .ask(move |core| {
                let i = core.index(&id).ok_or("That download is no longer in the list.")?;
                let tools = core.tools().ok_or("yt-dlp is not installed yet.")?;
                let args = args::probe_args(&core.jobs[i].url, PlaylistChoice::Video, &core.settings, &tools);
                Ok((tools.ytdlp, args, core.cache_path(&id)))
            })
            .await?;
        let out = run::output(&ytdlp, &args, 120).await.map_err(|e| e.to_string())?;
        let found = out.stdout.lines().find_map(|line| match probe::parse_probe_line(line) {
            Some(Probed::Media(media)) => Some((media, line.to_string())),
            _ => None,
        });
        let Some((media, raw)) = found else {
            let log: Vec<String> = out.stderr.lines().map(String::from).collect();
            return Err(classify(&log, out.code).detail);
        };
        if let (Some(path), Some(raw)) = (info_path, probe::cacheable(&raw)) {
            let _ = write_atomic(&path, raw.as_bytes()).await;
        }
        Ok(*media)
    }

    pub fn jobs_reorder(&self, ids: Vec<JobId>) {
        self.cast(move |core| core.reorder(&ids));
    }

    pub fn pause_all(&self) {
        self.cast(|core| {
            let ids: Vec<JobId> = core
                .jobs
                .iter()
                .filter(|j| is_active(j.state) || j.state == JobState::Queued)
                .map(|j| j.id.clone())
                .collect();
            ids.iter().for_each(|id| core.pause(id));
        });
    }

    pub fn resume_all(&self) {
        self.cast(|core| {
            for id in core.ids_in(JobState::Paused) {
                core.transition(&id, JobState::Queued);
            }
        });
    }

    pub fn retry_failed(&self) {
        self.cast(|core| {
            let ids: Vec<JobId> = core
                .jobs
                .iter()
                .filter(|j| j.state == JobState::Failed && j.error.as_ref().is_none_or(|e| e.retryable))
                .map(|j| j.id.clone())
                .collect();
            ids.iter().for_each(|id| core.retry(id));
        });
    }

    pub fn clear_completed(&self) {
        self.cast(|core| {
            for id in core.ids_in(JobState::Completed) {
                core.remove(&id, false);
            }
        });
    }

    pub async fn history(&self, query: String, filter: HistoryFilter, offset: u32, limit: u32) -> HistoryPage {
        self.shared.store.history(query, filter, offset, limit.clamp(1, 500)).await
    }

    /// Deletes history rows. The files stay where they are.
    pub fn history_remove(&self, ids: Vec<JobId>) {
        self.cast(move |core| ids.iter().for_each(|id| core.remove(id, true)));
    }

    /// Moves the downloaded files to the Recycle Bin (so it can be undone) and deletes the rows.
    pub async fn history_trash(&self, ids: Vec<JobId>) -> Result<(), String> {
        let mut failed = None;
        for id in ids {
            let Some(job) = self.shared.store.get(id.clone()).await else { continue };
            if !job.output.is_empty() {
                let path = PathBuf::from(&job.output);
                let trashed = tokio::task::spawn_blocking(move || {
                    if path.exists() { trash::delete(&path).map_err(|e| e.to_string()) } else { Ok(()) }
                })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);
                if let Err(error) = trashed {
                    failed = Some(format!("Could not move “{}” to the Recycle Bin: {error}", job.title));
                    continue;
                }
            }
            self.cast(move |core| core.remove(&id, true));
        }
        failed.map_or(Ok(()), Err)
    }

    /// Queues a fresh download of something from history.
    pub async fn download_again(&self, id: JobId) -> Result<(), String> {
        let old = self.shared.store.get(id).await.ok_or("That item is no longer in history.")?;
        self.ask(move |core| {
            let mut job = core.new_job(old.url, Source::Again, old.request);
            job.title = old.title;
            job.uploader = old.uploader;
            job.duration = old.duration;
            job.site = old.site;
            job.thumbnail = old.thumbnail;
            core.add_job(job);
            Ok(())
        })
        .await
    }

    /// The saved file of a finished download, for revealing or opening it.
    pub async fn output_path(&self, id: JobId) -> Result<PathBuf, String> {
        let job = self.shared.store.get(id).await.ok_or("That download is no longer in the list.")?;
        if job.output.is_empty() {
            return Err("This download has no file yet.".into());
        }
        let path = PathBuf::from(job.output);
        match tokio::fs::try_exists(&path).await {
            Ok(true) => Ok(path),
            _ => Err("The file was moved or deleted.".into()),
        }
    }

    pub async fn job_url(&self, id: JobId) -> Result<String, String> {
        self.shared.store.get(id).await.map(|j| j.url).ok_or_else(|| "That download is no longer in the list.".into())
    }

    /// Sanitized text for "Copy details" (a job, or the open preview by its id): versions,
    /// command line, error and log tail.
    pub async fn job_details(&self, id: JobId) -> Result<String, String> {
        let stored = self.shared.store.get(id.clone()).await;
        self.ask(move |core| {
            if let Some(details) = core.preview_details(&id) {
                return Ok(details);
            }
            let job = core
                .index(&id)
                .map(|i| core.jobs[i].clone())
                .or(stored)
                .ok_or("That download is no longer in the list.")?;
            Ok(core.details(&job))
        })
        .await
    }

    pub async fn settings_set(&self, new: Settings) -> Result<(), String> {
        self.ask(move |core| {
            core.set_settings(new);
            Ok(())
        })
        .await
    }

    /// Checks that a folder can be written to, creating it if needed.
    pub async fn check_folder(&self, folder: String) -> Result<(), String> {
        tokio::task::spawn_blocking(move || {
            let dir = PathBuf::from(&folder);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let probe = dir.join(".lovelydl-write-test");
            std::fs::write(&probe, b"").map_err(|e| e.to_string())?;
            std::fs::remove_file(&probe).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// The file name a template produces, for the open preview or a built-in sample.
    pub async fn template_preview(&self, template: String) -> Result<String, String> {
        if template.trim().is_empty() {
            return Err("The template is empty.".into());
        }
        let sample = self.shared.dirs.cache.join("probe").join("sample.json");
        let (ytdlp, info, request) = self
            .ask(move |core| {
                let tools = core.tools().ok_or("yt-dlp is not installed yet.")?;
                let open = core.preview.as_ref().filter(|p| matches!(p.status, PreviewStatus::Ready { .. }));
                let info = open.map(|p| core.info_path(&p.id));
                let request = open.map_or_else(|| core.settings.default_request.clone(), |p| p.request.clone());
                Ok((tools.ytdlp, info, request))
            })
            .await?;
        let info = match info {
            Some(path) => path,
            None => {
                write_atomic(&sample, SAMPLE_INFO.as_bytes()).await.map_err(|e| e.to_string())?;
                sample
            }
        };
        let out = run::output(&ytdlp, &args::filename_args(&info, &request, &template), 60)
            .await
            .map_err(|e| e.to_string())?;
        match out.stdout.lines().map(str::trim).find(|l| !l.is_empty()) {
            Some(name) if out.code == Some(0) => Ok(name.to_string()),
            _ => Err(classify(&out.stderr.lines().map(String::from).collect::<Vec<_>>(), out.code).detail),
        }
    }

    pub fn components_refresh(&self) {
        self.cast(|core| core.refresh_components());
    }

    /// Installs or updates one managed tool. Resolves when it is done.
    pub async fn component_install(&self, id: ToolId) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.cast(move |core| core.install(id, Some(tx)));
        rx.await.map_err(|_| EngineError::Stopped.to_string())?
    }

    /// Installs every tool that is not usable yet. Resolves when all are done.
    pub async fn components_install_missing(&self) -> Result<(), String> {
        let missing: Vec<ToolId> = self
            .call(|core| {
                [ToolId::Ytdlp, ToolId::Ffmpeg, ToolId::Deno]
                    .into_iter()
                    .filter(|id| !core.components.tool(*id).ready())
                    .collect()
            })
            .await
            .map_err(|e| e.to_string())?;
        let mut result = Ok(());
        let installs: Vec<_> = missing
            .into_iter()
            .map(|id| {
                tokio::spawn({
                    let me = self.clone();
                    async move { me.component_install(id).await }
                })
            })
            .collect();
        for install in installs {
            if let Err(error) = install.await.map_err(|e| e.to_string()).and_then(|r| r) {
                result = Err(error);
            }
        }
        result
    }

    /// Goes back to the previous version of a managed tool.
    pub async fn component_rollback(&self, id: ToolId) -> Result<(), String> {
        self.shared.toolchain.rollback(id).await.map_err(|e| e.to_string())?;
        self.components_refresh();
        Ok(())
    }

    /// A thumbnail's bytes and media type, fetched once and cached. Only http(s) images.
    pub async fn thumbnail(&self, link: &str) -> Result<(Vec<u8>, &'static str), String> {
        let parsed = url::Url::parse(link).map_err(|e| e.to_string())?;
        if !matches!(parsed.scheme(), "http" | "https") || !crate::web::public_host(&parsed) {
            return Err("not a public web address".into());
        }
        let name: String = Sha256::digest(link.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
        let path = self.shared.dirs.cache.join("thumbs").join(name);
        if let Ok(bytes) = tokio::fs::read(&path).await
            && let Some(kind) = image_type(&bytes)
        {
            return Ok((bytes, kind));
        }
        let response = self.shared.web.get(parsed).send().await.map_err(|e| e.without_url().to_string())?;
        let mut response = response.error_for_status().map_err(|e| e.without_url().to_string())?;
        if response.content_length().is_some_and(|n| n as usize > MAX_THUMBNAIL) {
            return Err("the image is too large".into());
        }
        // Read in pieces, so a reply without a length cannot fill memory before the check.
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| e.without_url().to_string())? {
            if bytes.len() + chunk.len() > MAX_THUMBNAIL {
                return Err("the image is too large".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let kind = image_type(&bytes).ok_or("not an image")?;
        let _ = write_atomic(&path, &bytes).await;
        Ok((bytes, kind))
    }

    /// How many downloads are running right now, for the quit confirmation.
    pub async fn active_count(&self) -> u32 {
        self.call(|core| core.jobs.iter().filter(|j| is_active(j.state)).count() as u32).await.unwrap_or(0)
    }

    /// Stops every running download so it resumes on the next launch, and waits until the
    /// processes are gone and the database is written.
    pub async fn shutdown(&self) {
        let (tx, rx) = oneshot::channel();
        self.cast(move |core| core.begin_shutdown(tx));
        let _ = rx.await;
        self.shared.store.flush().await;
    }
}

impl Core {
    fn next_id(&mut self, prefix: &str) -> String {
        self.seq += 1;
        format!("{prefix}{:x}{:03x}", now_ms() as u64, self.seq % 0x1000)
    }

    fn index(&self, id: &str) -> Option<usize> {
        self.jobs.iter().position(|j| j.id == id)
    }

    fn ids_in(&self, state: JobState) -> Vec<JobId> {
        self.jobs.iter().filter(|j| j.state == state).map(|j| j.id.clone()).collect()
    }

    fn home(&self) -> Option<String> {
        self.shared.dirs.home.as_ref().map(|h| h.to_string_lossy().into_owned())
    }

    fn info_path(&self, id: &str) -> PathBuf {
        self.shared.dirs.cache.join("probe").join(format!("{id}.json"))
    }

    /// Where to cache a probe, or `None` while browser cookies are in use: what a site sends
    /// back for a signed-in session (auth headers, signed links) must not be written to disk.
    fn cache_path(&self, id: &str) -> Option<PathBuf> {
        self.settings.cookies_browser.is_empty().then(|| self.info_path(id))
    }

    // ----- changes ---------------------------------------------------------------------

    /// Queues a change for the next batch, replacing older changes it makes redundant.
    fn push(&mut self, change: Change) {
        match &change {
            Change::Progress { id, .. } => {
                self.pending.retain(|c| !matches!(c, Change::Progress { id: other, .. } if other == id));
            }
            // An older update is dropped only if it says nothing new about the state: the UI
            // animates state changes, so every one of them has to arrive.
            Change::JobUpdated(job) => self.pending.retain(|c| match c {
                Change::JobUpdated(other) => other.id != job.id || other.state != job.state,
                Change::Progress { id, .. } => *id != job.id,
                _ => true,
            }),
            Change::Preview(_) => self.pending.retain(|c| !matches!(c, Change::Preview(_))),
            Change::Settings(_) => self.pending.retain(|c| !matches!(c, Change::Settings(_))),
            Change::Components(_) => self.pending.retain(|c| !matches!(c, Change::Components(_))),
            Change::Order(_) => self.pending.retain(|c| !matches!(c, Change::Order(_))),
            Change::HistoryChanged => self.pending.retain(|c| !matches!(c, Change::HistoryChanged)),
            _ => {}
        }
        if let (
            Change::PlaylistEntries { preview_id, entries },
            Some(Change::PlaylistEntries { preview_id: last, entries: batch }),
        ) = (&change, self.pending.last_mut())
            && last == preview_id
        {
            batch.extend(entries.iter().cloned());
        } else {
            self.pending.push(change);
        }
        if !self.flush_armed {
            self.flush_armed = true;
            // The first change after a quiet spell goes out at once; later ones wait for the window.
            let wait = FLUSH_WINDOW.saturating_sub(self.last_flush.elapsed());
            let me = self.me.clone();
            tokio::spawn(async move {
                tokio::time::sleep(wait).await;
                let _ = me.send(Box::new(|core| core.flush()));
            });
        }
    }

    fn flush(&mut self) {
        self.flush_armed = false;
        self.last_flush = Instant::now();
        if !self.pending.is_empty() {
            (self.sink)(std::mem::take(&mut self.pending));
        }
    }

    /// Saves a job and tells the UI about it.
    fn changed(&mut self, i: usize) {
        self.shared.store.save(&self.jobs[i], i);
        self.push(Change::JobUpdated(Box::new(self.jobs[i].clone())));
    }

    /// The only way a job changes state. Illegal transitions are refused and logged.
    fn transition(&mut self, id: &str, to: JobState) -> bool {
        let Some(i) = self.index(id) else { return false };
        let from = self.jobs[i].state;
        if !can_transition(from, to) {
            tracing::warn!(job = id, ?from, ?to, "refused an illegal state transition");
            return false;
        }
        let job = &mut self.jobs[i];
        job.state = to;
        match to {
            JobState::Probing | JobState::Downloading if job.started_at.is_none() => job.started_at = Some(now_ms()),
            JobState::Completed | JobState::Failed | JobState::Canceled => job.finished_at = Some(now_ms()),
            JobState::Queued => {
                job.finished_at = None;
                job.progress.stage = Stage::Waiting;
            }
            _ => {}
        }
        if !is_active(to) {
            job.progress.speed = None;
            job.progress.eta = None;
        }
        self.changed(i);
        if matches!(to, JobState::Completed | JobState::Failed) {
            self.push(Change::HistoryChanged);
        }
        self.needs_pump = true;
        true
    }

    // ----- scheduling ------------------------------------------------------------------

    /// The per-site limit counts by the link's host: it is known before probing and, unlike
    /// the extractor's site name, does not change once the probe finishes.
    fn site_of(job: &Job) -> String {
        host_of(&job.url)
    }

    /// Starts queued jobs, in order, while the global and per-site limits allow.
    fn pump(&mut self) {
        if self.shutting_down || !self.tools_known {
            return;
        }
        let mut active = self.jobs.iter().filter(|j| is_active(j.state)).count();
        let mut per_site: HashMap<String, u32> = HashMap::new();
        for job in self.jobs.iter().filter(|j| is_active(j.state)) {
            *per_site.entry(Self::site_of(job)).or_default() += 1;
        }
        let stopping: HashSet<&JobId> = self.stopping.values().collect();
        let now = Instant::now();
        let mut start = Vec::new();
        for job in self.jobs.iter().filter(|j| j.state == JobState::Queued) {
            if active >= self.settings.max_concurrent as usize {
                break;
            }
            let site = per_site.entry(Self::site_of(job)).or_default();
            // A job whose old process is still exiting waits, so two never share partial files.
            // One being looked at ahead of its turn waits for that answer instead of asking twice.
            // One waiting out an automatic retry keeps its place but does not start yet.
            if *site >= self.settings.max_per_site
                || stopping.contains(&job.id)
                || self.peeking.as_ref() == Some(&job.id)
                || self.retry_at.get(&job.id).is_some_and(|at| *at > now)
            {
                continue;
            }
            *site += 1;
            active += 1;
            start.push(job.id.clone());
        }
        for id in start {
            self.start(&id);
        }
        self.peek_ahead();
        if active == 0 {
            self.check_for_tool_updates();
        }
    }

    /// Fetches the title and thumbnail of the next waiting download that still shows only its
    /// link, one at a time, so the queue reads as a list of videos. The answer is cached, and
    /// the download later starts from it without asking the site again.
    fn peek_ahead(&mut self) {
        if self.peeking.is_some() || self.shutdown.is_some() || !self.components.ytdlp.ready() {
            return;
        }
        let Some(tools) = self.tools() else { return };
        // One waiting out a retry is left alone: it already knows the site, and asking again
        // would be one more request at exactly the wrong moment.
        let waiting = |j: &&Job| {
            j.state == JobState::Queued
                && j.title.is_empty()
                && !self.peeked.contains(&j.id)
                && !self.retry_at.contains_key(&j.id)
        };
        let Some((id, url)) = self.jobs.iter().find(waiting).map(|j| (j.id.clone(), j.url.clone())) else { return };
        self.peeked.insert(id.clone());
        self.peeking = Some(id.clone());
        let args = args::probe_args(&url, PlaylistChoice::Video, &self.settings, &tools);
        let (me, path) = (self.me.clone(), self.cache_path(&id));
        tokio::spawn(async move {
            let out = run::output(&tools.ytdlp, &args, 60).await.ok();
            let found = out.and_then(|out| {
                out.stdout.lines().find_map(|line| match probe::parse_probe_line(line) {
                    Some(Probed::Media(media)) => Some((media, line.to_string())),
                    _ => None,
                })
            });
            // A protected item is never cached: it is refused below, before any download.
            let cached = match (&found, path) {
                (Some((media, raw)), Some(path)) if !media.drm => match probe::cacheable(raw) {
                    Some(clean) => write_atomic(&path, clean.as_bytes()).await.is_ok(),
                    None => false,
                },
                _ => false,
            };
            let _ = me.send(Box::new(move |core| {
                core.peeking = None;
                core.needs_pump = true;
                let (Some((media, _)), Some(i)) = (found, core.index(&id)) else { return };
                if core.jobs[i].state != JobState::Queued {
                    return;
                }
                let job = &mut core.jobs[i];
                job.title = media.title.clone();
                job.uploader = media.uploader.clone();
                job.duration = media.duration;
                job.site = media.site.clone();
                if !media.thumbnail.is_empty() {
                    job.thumbnail = media.thumbnail.clone();
                }
                job.planned = Some(probe::approx_resolve(&media, &job.request));
                if media.drm {
                    return core.fail(&id, errors::error(ErrorKind::Drm, "This video is DRM protected"));
                }
                if cached {
                    core.probed.insert(id.clone(), Instant::now());
                }
                core.changed(i);
            }));
        });
    }

    fn tools(&self) -> Option<Tools> {
        let runtime = self.tool_paths.get(&ToolId::Deno).map(|path| {
            let name = toolchain::custom_runtime(&self.settings.custom_js_runtime)
                .map_or_else(|| "deno".to_string(), |(name, _)| name);
            (name, path.clone())
        });
        Some(Tools {
            ytdlp: self.tool_paths.get(&ToolId::Ytdlp)?.clone(),
            ffmpeg_dir: self.tool_paths.get(&ToolId::Ffmpeg).and_then(|p| p.parent().map(Path::to_path_buf)),
            js_runtime: runtime,
        })
    }

    fn start(&mut self, id: &str) {
        if !self.components.ytdlp.ready() {
            return self.fail(id, errors::error(ErrorKind::YtdlpMissing, "yt-dlp is not installed"));
        }
        if !self.components.ffmpeg.ready() {
            return self.fail(id, errors::error(ErrorKind::FfmpegMissing, "ffmpeg is not installed"));
        }
        if self.probed.get(id).is_some_and(|at| at.elapsed() < PROBE_TTL) {
            self.start_download(id, true);
        } else {
            self.start_probe(id);
        }
    }

    fn start_probe(&mut self, id: &str) {
        let (Some(i), Some(tools)) = (self.index(id), self.tools()) else { return };
        let args = args::probe_args(&self.jobs[i].url, PlaylistChoice::Video, &self.settings, &tools);
        self.jobs[i].progress = Progress { stage: Stage::Probing, ..Progress::default() };
        if self.transition(id, JobState::Probing) {
            self.spawn_run(id, Phase::Probe, &tools.ytdlp, args, Tracker::new(None), false);
        }
    }

    fn destination(&self, job: &Job) -> PathBuf {
        let base = PathBuf::from(job.request.destination.as_deref().unwrap_or(&self.settings.download_dir));
        match &job.group {
            Some(group) => base.join(folder_name(&group.title)),
            None => base,
        }
    }

    /// Partial files go to the cache folder when it shares a drive with the destination, so
    /// the final move is a rename; otherwise to a hidden folder on the destination's drive.
    fn temp_dir(&self, job: &Job) -> (PathBuf, bool) {
        let dest = self.destination(job);
        let cache = self.shared.dirs.cache.join("partial");
        if same_volume(&cache, &dest) {
            (cache.join(&job.id), false)
        } else {
            (dest.join(".lovelydl-partial").join(&job.id), true)
        }
    }

    fn template(&self, job: &Job) -> String {
        let template = numbered(&self.settings.filename_template, self.renamed.get(&job.id).copied());
        match &job.group {
            Some(group) if self.settings.playlist_index => {
                let width = group.count.to_string().len().max(2);
                let prefix = format!("{:0width$} - ", group.index);
                // The number belongs on the file name, not on a folder in the template.
                match template.rfind(['/', '\\']) {
                    Some(at) => format!("{}{prefix}{}", &template[..=at], &template[at + 1..]),
                    None => format!("{prefix}{template}"),
                }
            }
            _ => template,
        }
    }

    fn start_download(&mut self, id: &str, cached: bool) {
        let (Some(i), Some(tools)) = (self.index(id), self.tools()) else { return };
        let job = &self.jobs[i];
        let dest = self.destination(job);
        let (temp, hidden) = self.temp_dir(job);
        let info = cached.then(|| self.info_path(id));
        let archive =
            job.group.as_ref().filter(|g| g.skip_downloaded).map(|_| self.shared.dirs.data.join("archive.txt"));
        let args = args::download_args(&args::Download {
            url: &job.url,
            request: &job.request,
            settings: &self.settings,
            tools: &tools,
            dest: &dest,
            temp: &temp,
            template: &self.template(job),
            info_json: info.as_deref(),
            archive: archive.as_deref(),
        });
        let tracker = Tracker::new(job.planned.as_ref());
        self.jobs[i].progress = Progress { stage: Stage::Preparing, ..Progress::default() };
        if !self.transition(id, JobState::Downloading) {
            return;
        }
        let (me, id) = (self.me.clone(), id.to_string());
        tokio::spawn(async move {
            prepare_temp(temp, hidden).await;
            let _ = me.send(Box::new(move |core| {
                // The job may have been paused or canceled while the folder was being made.
                let still = core.index(&id).is_some_and(|i| core.jobs[i].state == JobState::Downloading);
                if still && !core.runs.contains_key(&id) {
                    core.spawn_run(&id, Phase::Download, &tools.ytdlp, args, tracker, cached);
                }
            }));
        });
    }

    fn spawn_run(&mut self, id: &str, phase: Phase, program: &Path, args: Vec<String>, tracker: Tracker, cached: bool) {
        self.seq += 1;
        let seq = self.seq;
        let home = self.home();
        let log = self.logs.entry(id.to_string()).or_default();
        log.args = redact::redact_args(&args, home.as_deref());
        log.lines.clear();
        let (me_line, id_line) = (self.me.clone(), id.to_string());
        let (me_exit, id_exit) = (self.me.clone(), id.to_string());
        let started = run::start(
            program,
            &args,
            move |line| {
                let id = id_line.clone();
                let _ = me_line.send(Box::new(move |core| core.on_line(&id, seq, line)));
            },
            move |code| {
                let _ = me_exit.send(Box::new(move |core| core.on_exit(&id_exit, seq, code)));
            },
        );
        match started {
            Ok(stop) => {
                self.runs.insert(
                    id.to_string(),
                    Run {
                        seq,
                        stop,
                        phase,
                        tracker,
                        cached,
                        file: None,
                        media: None,
                        saw_entries: false,
                        name_taken: false,
                    },
                );
            }
            Err(error) => {
                self.fail(id, errors::error(ErrorKind::YtdlpMissing, format!("yt-dlp could not be started: {error}")))
            }
        }
    }

    fn log_line(&mut self, id: &str, line: &str) {
        let line = redact::redact(line, self.home().as_deref());
        let log = self.logs.entry(id.to_string()).or_default();
        if log.lines.len() == LOG_LINES {
            log.lines.pop_front();
        }
        log.lines.push_back(line);
    }

    fn on_line(&mut self, id: &str, seq: u64, line: String) {
        let Some(phase) = self.runs.get(id).filter(|r| r.seq == seq).map(|r| r.phase) else { return };
        if phase == Phase::Probe {
            let parsed = line.starts_with('{').then(|| probe::parse_probe_line(&line)).flatten();
            match (parsed, self.runs.get_mut(id)) {
                (Some(Probed::Media(media)), Some(run)) => run.media = Some((media, line)),
                (Some(Probed::Entry { .. }), Some(run)) => run.saw_entries = true,
                _ => self.log_line(id, &line),
            }
            return;
        }
        let (Some(i), Some(run)) = (self.jobs.iter().position(|j| j.id == id), self.runs.get_mut(id)) else { return };
        let job = &mut self.jobs[i];
        // What this line changes: the job's state, or just its progress.
        let (mut next, mut moved) = (None, false);
        match parse_line(&line) {
            Line::Download(d) => {
                run.tracker.on_download(&d);
                job.progress = run.tracker.progress.clone();
                moved = true;
                if job.state == JobState::PostProcessing {
                    next = Some(JobState::Downloading);
                }
            }
            Line::Post { name, started } => {
                if run.tracker.on_post(&name, started) {
                    job.progress = run.tracker.progress.clone();
                    moved = true;
                    if job.state == JobState::Downloading {
                        next = Some(JobState::PostProcessing);
                    }
                }
            }
            Line::File(path) => run.file = Some(path),
            Line::Existing => run.name_taken = true,
            Line::Log(text) => {
                // With --wait-for-video, yt-dlp warns and waits instead of failing.
                let lower = text.to_lowercase();
                if lower.contains("will begin in") || lower.contains("premieres in") {
                    job.progress.stage = Stage::WaitingForLive;
                    moved = true;
                }
                self.log_line(id, &text);
            }
        }
        match next {
            Some(state) => {
                self.transition(id, state);
            }
            None if moved => {
                self.push(Change::Progress { id: id.to_string(), progress: self.jobs[i].progress.clone() })
            }
            None => {}
        }
    }

    fn on_exit(&mut self, id: &str, seq: u64, code: Option<i32>) {
        self.needs_pump = true;
        if self.stopping.remove(&seq).is_some() {
            if self.stopping.is_empty()
                && let Some(done) = self.shutdown.take()
            {
                let _ = done.send(());
            }
            return;
        }
        if self.runs.get(id).is_none_or(|r| r.seq != seq) {
            return;
        }
        let Some(run) = self.runs.remove(id) else { return };
        let log: Vec<String> = self.logs.get(id).map(|l| l.lines.iter().cloned().collect()).unwrap_or_default();
        let Some(i) = self.index(id) else { return };
        match run.phase {
            Phase::Probe => match run.media {
                Some((media, raw)) => self.probe_finished(id, *media, raw),
                None if run.saw_entries => self.fail(id, errors::error(ErrorKind::Playlist, "this link is a playlist")),
                None => self.fail(id, classify(&log, code)),
            },
            Phase::Download => match (code, run.file) {
                // Another file already has this name, and yt-dlp would hand back that old file
                // (a 480p copy, a 30-second clip). Download again under a numbered name instead.
                (Some(0), Some(_)) if run.name_taken && self.renamed.get(id).is_none_or(|n| *n < 99) => {
                    *self.renamed.entry(id.to_string()).or_insert(1) += 1;
                    self.transition(id, JobState::Queued);
                }
                (Some(0), Some(file)) => {
                    let (me, id) = (self.me.clone(), id.to_string());
                    tokio::spawn(async move {
                        let size = tokio::fs::metadata(&file).await.ok().map(|m| m.len() as f64);
                        let _ = me.send(Box::new(move |core| core.complete(&id, file, size)));
                    });
                }
                // An item that is already in the download archive is skipped silently.
                (Some(0), None) if self.jobs[i].group.as_ref().is_some_and(|g| g.skip_downloaded) => {
                    self.complete(id, String::new(), None);
                }
                (code, _) => {
                    let mut error = classify(&log, code);
                    if code == Some(0) && error.kind == ErrorKind::Crashed {
                        error.detail = "yt-dlp finished without producing a file".into();
                    }
                    // Cached info with expired links fails with 403: probe again, once.
                    if error.kind == ErrorKind::Http403 && run.cached && self.reprobed.insert(id.to_string()) {
                        self.probed.remove(id);
                        self.start_probe(id);
                    } else {
                        self.fail(id, error);
                    }
                }
            },
        }
    }

    fn probe_finished(&mut self, id: &str, media: MediaInfo, raw: String) {
        let Some(i) = self.index(id) else { return };
        let job = &mut self.jobs[i];
        job.title = media.title.clone();
        job.uploader = media.uploader.clone();
        job.duration = media.duration;
        job.site = media.site.clone();
        if !media.thumbnail.is_empty() {
            job.thumbnail = media.thumbnail.clone();
        }
        job.planned = Some(probe::approx_resolve(&media, &job.request));
        if media.drm {
            return self.fail(id, errors::error(ErrorKind::Drm, "This video is DRM protected"));
        }
        self.changed(i);
        let (me, id, path) = (self.me.clone(), id.to_string(), self.cache_path(id));
        tokio::spawn(async move {
            let cached = match (path, probe::cacheable(&raw)) {
                (Some(path), Some(raw)) => write_atomic(&path, raw.as_bytes()).await.is_ok(),
                _ => false,
            };
            let _ = me.send(Box::new(move |core| {
                if cached {
                    core.probed.insert(id.clone(), Instant::now());
                }
                // Still probing unless the user paused or canceled meanwhile.
                if core.index(&id).is_some_and(|i| core.jobs[i].state == JobState::Probing) {
                    core.start_download(&id, cached);
                }
            }));
        });
    }

    fn forget_files(&mut self, id: &str, remove_temp: bool) {
        self.probed.remove(id);
        self.reprobed.remove(id);
        let info = self.info_path(id);
        let temp = self.index(id).filter(|_| remove_temp).map(|i| self.temp_dir(&self.jobs[i]).0);
        tokio::spawn(async move {
            let _ = tokio::fs::remove_file(info).await;
            if let Some(temp) = temp {
                let _ = tokio::fs::remove_dir_all(temp).await;
            }
        });
    }

    fn complete(&mut self, id: &str, file: String, size: Option<f64>) {
        let Some(i) = self.index(id) else { return };
        let job = &mut self.jobs[i];
        job.output = file;
        job.bytes = size;
        job.error = None;
        job.progress = Progress {
            stage: Stage::Done,
            fraction: Some(1.0),
            downloaded: size.unwrap_or(0.0),
            total: size,
            ..Progress::default()
        };
        if !self.transition(id, JobState::Completed) {
            return;
        }
        // The probe cache can hold personal data; it goes as soon as it has served its purpose.
        self.forget_files(id, true);
        self.logs.remove(id);
        self.notify(id, true);
    }

    fn fail(&mut self, id: &str, error: JobError) {
        let Some(i) = self.index(id) else { return };
        tracing::info!(job = id, kind = ?error.kind, detail = error.detail, "download failed");
        if self.auto_retry_in(i, &error) {
            let job = &mut self.jobs[i];
            job.error = Some(error);
            job.attempts += 1;
            let wait = self.retry_backoff * 2u32.pow(self.jobs[i].attempts.min(4) - 1);
            tracing::info!(job = id, attempt = self.jobs[i].attempts, ?wait, "trying again by itself");
            self.retry_at.insert(id.to_string(), Instant::now() + wait);
            if self.transition(id, JobState::Queued) {
                // After the move to Queued, which would otherwise read as plain waiting.
                self.jobs[i].progress = Progress { stage: Stage::Retrying, ..Progress::default() };
                self.changed(i);
                // Wake the queue when the wait is over; the scheduler holds it back until then.
                let (me, id) = (self.me.clone(), id.to_string());
                tokio::spawn(async move {
                    tokio::time::sleep(wait).await;
                    let _ = me.send(Box::new(move |core| {
                        core.retry_at.remove(&id);
                        core.needs_pump = true;
                    }));
                });
            }
            return;
        }
        self.retry_at.remove(id);
        self.jobs[i].error = Some(error);
        if self.transition(id, JobState::Failed) {
            self.notify(id, false);
        }
    }

    /// Whether a failure is worth another go without asking: a passing network fault, a few
    /// times over, and only when the user has left that on.
    fn auto_retry_in(&self, i: usize, error: &JobError) -> bool {
        const MAX_ATTEMPTS: u32 = 3;
        self.settings.auto_retry
            && self.jobs[i].attempts < MAX_ATTEMPTS
            && self.jobs[i].group.is_none()
            && matches!(error.kind, ErrorKind::Network | ErrorKind::Http429 | ErrorKind::Crashed)
    }

    /// One notice per finished download; a playlist gets a single notice when its last item ends.
    fn notify(&mut self, id: &str, completed: bool) {
        let Some(job) = self.index(id).map(|i| &self.jobs[i]) else { return };
        let notice = match &job.group {
            None if completed => {
                Notice::Finished { job_id: job.id.clone(), title: job.title.clone(), path: job.output.clone() }
            }
            None => Notice::Failed {
                job_id: job.id.clone(),
                title: if job.title.is_empty() { job.url.clone() } else { job.title.clone() },
            },
            Some(group) => {
                let members: Vec<&Job> =
                    self.jobs.iter().filter(|j| j.group.as_ref().is_some_and(|g| g.id == group.id)).collect();
                if members.iter().any(|j| !crate::state::is_finished(j.state)) {
                    return;
                }
                let count = |state| members.iter().filter(|j| j.state == state).count() as u32;
                Notice::GroupFinished {
                    title: group.title.clone(),
                    completed: count(JobState::Completed),
                    failed: count(JobState::Failed),
                    folder: self.destination(job).to_string_lossy().into_owned(),
                }
            }
        };
        self.push(Change::Notice(notice));
    }

    // ----- user actions on jobs --------------------------------------------------------

    /// Stops a job's process, if it has one. The process is tracked until it has exited.
    fn stop_run(&mut self, id: &str, cleanup: Option<PathBuf>) -> bool {
        match self.runs.remove(id) {
            Some(run) => {
                self.stopping.insert(run.seq, id.to_string());
                run.stop.stop(cleanup);
                true
            }
            None => false,
        }
    }

    /// Pause stops the process and keeps the partial files; resuming runs the same command.
    fn pause(&mut self, id: &str) {
        let Some(i) = self.index(id) else { return };
        if is_active(self.jobs[i].state) || self.jobs[i].state == JobState::Queued {
            self.stop_run(id, None);
            self.transition(id, JobState::Paused);
        }
    }

    fn cancel(&mut self, id: &str) {
        let Some(i) = self.index(id) else { return };
        let temp = self.temp_dir(&self.jobs[i]).0;
        if !can_transition(self.jobs[i].state, JobState::Canceled) {
            return;
        }
        // A running process must exit before its files can be deleted.
        let running = self.stop_run(id, Some(temp));
        self.forget_files(id, !running);
        self.transition(id, JobState::Canceled);
    }

    fn retry(&mut self, id: &str) {
        self.retry_at.remove(id);
        let Some(i) = self.index(id) else { return };
        if !matches!(self.jobs[i].state, JobState::Failed | JobState::Canceled) {
            return;
        }
        let job = &mut self.jobs[i];
        job.error = None;
        job.attempts += 1;
        job.progress = Progress::default();
        self.reprobed.remove(id);
        self.transition(id, JobState::Queued);
    }

    /// Takes a job off the list. `forget` also deletes it from history.
    fn remove(&mut self, id: &str, forget: bool) {
        let live = self.index(id);
        if let Some(i) = live {
            let finished = matches!(self.jobs[i].state, JobState::Completed | JobState::Failed);
            let temp = self.temp_dir(&self.jobs[i]).0;
            let running = self.stop_run(id, Some(temp));
            self.forget_files(id, !running);
            let mut job = self.jobs.remove(i);
            self.logs.remove(id);
            self.push(Change::JobRemoved(id.to_string()));
            if finished && !forget {
                // Still part of history, just no longer on the Downloads list.
                job.cleared = true;
                self.shared.store.save(&job, i);
            } else {
                self.shared.store.delete(vec![id.to_string()]);
            }
            self.needs_pump = true;
        } else if forget {
            self.shared.store.delete(vec![id.to_string()]);
        }
        if forget || live.is_some() {
            self.push(Change::HistoryChanged);
        }
    }

    fn reorder(&mut self, ids: &[JobId]) {
        // Jobs named in `ids` come first, in that order; anything not named keeps its place after.
        let rank = |job: &Job| ids.iter().position(|id| *id == job.id).unwrap_or(usize::MAX);
        self.jobs.sort_by_key(rank);
        let order: Vec<JobId> = self.jobs.iter().map(|j| j.id.clone()).collect();
        self.shared.store.save_order(order.clone());
        self.push(Change::Order(order));
        self.needs_pump = true;
    }

    fn apply_fix(&mut self, id: &str, fix: Fix) {
        match fix {
            Fix::Retry => self.retry(id),
            Fix::WaitForLive => {
                if let Some(i) = self.index(id) {
                    self.jobs[i].request.options.wait_for_live = true;
                    self.retry(id);
                }
            }
            Fix::UpdateYtdlp => {
                self.waiting_for_tools.push(id.to_string());
                self.install(ToolId::Ytdlp, None);
            }
            Fix::InstallComponents => {
                self.waiting_for_tools.push(id.to_string());
                let missing: Vec<ToolId> = [ToolId::Ytdlp, ToolId::Ffmpeg, ToolId::Deno]
                    .into_iter()
                    .filter(|t| !self.components.tool(*t).ready())
                    .collect();
                if missing.is_empty() {
                    self.refresh_components();
                }
                missing.into_iter().for_each(|tool| self.install(tool, None));
            }
            Fix::ChooseVideos => {
                let Some(i) = self.index(id) else { return };
                let (url, request) = (self.jobs[i].url.clone(), self.jobs[i].request.clone());
                self.remove(id, true);
                self.open_preview(url, Source::Paste, request, false);
                // A link that is also a single video was already tried as one: take the playlist.
                if let Some(preview) = self.preview.as_ref().filter(|p| p.status == PreviewStatus::Ambiguous) {
                    let id = preview.id.clone();
                    self.preview_choose(&id, PlaylistChoice::Playlist);
                }
            }
            // These need the user to choose something first; the UI does that, then retries.
            Fix::UseCookies | Fix::ChooseFormat | Fix::ChooseFolder => {}
        }
    }

    fn new_job(&mut self, url: String, source: Source, request: DownloadRequest) -> Job {
        Job {
            id: self.next_id("j"),
            site: host_of(&url),
            url,
            source,
            title: String::new(),
            uploader: String::new(),
            duration: None,
            thumbnail: String::new(),
            kind: if request.preset.is_audio() { MediaKind::Audio } else { MediaKind::Video },
            request,
            state: JobState::Queued,
            progress: Progress::default(),
            planned: None,
            output: String::new(),
            file_missing: false,
            bytes: None,
            error: None,
            attempts: 0,
            group: None,
            cleared: false,
            created_at: now_ms(),
            started_at: None,
            finished_at: None,
        }
    }

    fn add_job(&mut self, job: Job) {
        self.shared.store.save(&job, self.jobs.len());
        self.push(Change::JobAdded(Box::new(job.clone())));
        self.jobs.push(job);
        self.needs_pump = true;
    }

    fn add_links(&mut self, text: &str, source: Source) -> Result<u32, String> {
        let urls = extract_urls(text);
        match urls.as_slice() {
            [] => Err("No link found. Paste a web address that starts with http or https.".into()),
            [one] => {
                let auto = self.settings.on_add == OnAdd::Download;
                self.open_preview(one.clone(), source, self.settings.default_request.clone(), auto);
                Ok(1)
            }
            many => {
                for url in many {
                    let job = self.new_job(url.clone(), source, self.settings.default_request.clone());
                    self.add_job(job);
                }
                Ok(many.len() as u32)
            }
        }
    }

    // ----- preview ---------------------------------------------------------------------

    fn set_preview_status(&mut self, status: PreviewStatus) {
        if let Some(preview) = &mut self.preview {
            preview.status = status;
        }
        self.push_preview();
    }

    fn push_preview(&mut self) {
        // While a link is being checked for an immediate download there is nothing to show.
        let hidden = self.preview_run.as_ref().is_some_and(|r| r.auto_start)
            && self.preview.as_ref().is_some_and(|p| matches!(p.status, PreviewStatus::Ready { .. }));
        if !hidden {
            self.push(Change::Preview(self.preview.clone().map(Box::new)));
        }
    }

    fn close_preview(&mut self, delete_cache: bool) {
        if let Some(run) = self.preview_run.take()
            && let Some(stop) = run.stop
        {
            stop.stop(None);
        }
        if let Some(preview) = self.preview.take()
            && delete_cache
        {
            let info = self.info_path(&preview.id);
            tokio::spawn(async move {
                let _ = tokio::fs::remove_file(info).await;
            });
        }
        self.resolve_seq += 1;
        self.push(Change::Preview(None));
    }

    fn open_preview(&mut self, url: String, source: Source, request: DownloadRequest, auto_start: bool) {
        self.close_preview(true);
        let id = self.next_id("p");
        let ambiguous = is_ambiguous(&url);
        self.preview = Some(Preview {
            id: id.clone(),
            url,
            source,
            request,
            status: if ambiguous { PreviewStatus::Ambiguous } else { PreviewStatus::Probing },
        });
        self.preview_run =
            Some(PreviewRun { stop: None, args: Vec::new(), log: Vec::new(), archive: HashSet::new(), auto_start });
        self.push_preview();
        if !ambiguous {
            self.probe_preview(&id, PlaylistChoice::Video);
        }
    }

    fn preview_choose(&mut self, id: &str, choice: PlaylistChoice) {
        if self.preview.as_ref().is_some_and(|p| p.id == id && p.status == PreviewStatus::Ambiguous) {
            self.set_preview_status(PreviewStatus::Probing);
            self.probe_preview(id, choice);
        }
    }

    fn probe_preview(&mut self, id: &str, choice: PlaylistChoice) {
        let Some(url) = self.preview.as_ref().filter(|p| p.id == id).map(|p| p.url.clone()) else { return };
        if !self.tools_known {
            // A link can arrive in the first moments after launch. It waits for the tool check.
            self.preview_waiting = Some((id.to_string(), choice));
            return;
        }
        let Some(tools) = self.tools() else {
            let error = errors::error(ErrorKind::YtdlpMissing, "yt-dlp is not installed");
            return self.set_preview_status(PreviewStatus::Failed { error });
        };
        let args = args::probe_args(&url, choice, &self.settings, &tools);
        let archive = self.shared.dirs.data.join("archive.txt");
        let (me, id) = (self.me.clone(), id.to_string());
        tokio::spawn(async move {
            // Read first, so entries can be marked as already downloaded as they arrive.
            let known: HashSet<String> = tokio::fs::read_to_string(archive)
                .await
                .unwrap_or_default()
                .lines()
                .map(|l| l.trim().to_string())
                .collect();
            let _ = me.send(Box::new(move |core| core.spawn_preview_probe(id, tools.ytdlp, args, known)));
        });
    }

    fn spawn_preview_probe(&mut self, id: String, ytdlp: PathBuf, args: Vec<String>, archive: HashSet<String>) {
        if self.preview.as_ref().is_none_or(|p| p.id != id) {
            return;
        }
        let (me_line, id_line) = (self.me.clone(), id.clone());
        let (me_exit, id_exit) = (self.me.clone(), id.clone());
        let started = run::start(
            &ytdlp,
            &args,
            move |line| {
                let id = id_line.clone();
                let _ = me_line.send(Box::new(move |core| core.on_preview_line(&id, line)));
            },
            move |code| {
                let _ = me_exit.send(Box::new(move |core| core.on_preview_exit(&id_exit, code)));
            },
        );
        let shown = redact::redact_args(&args, self.home().as_deref());
        match (started, &mut self.preview_run) {
            (Ok(stop), Some(run)) => {
                run.stop = Some(stop);
                run.archive = archive;
                run.args = shown;
            }
            (Ok(stop), None) => stop.stop(None),
            (Err(error), _) => self.set_preview_status(PreviewStatus::Failed {
                error: errors::error(ErrorKind::YtdlpMissing, format!("yt-dlp could not be started: {error}")),
            }),
        }
    }

    fn on_preview_line(&mut self, id: &str, line: String) {
        let Some(preview) = self.preview.as_mut().filter(|p| p.id == id) else { return };
        let Some(run) = self.preview_run.as_mut() else { return };
        match line.starts_with('{').then(|| probe::parse_probe_line(&line)).flatten() {
            Some(Probed::Media(info)) => {
                // yt-dlp has said all it will and exits by itself; there is nothing left to stop.
                run.stop = None;
                let resolved = probe::approx_resolve(&info, &preview.request);
                preview.status = PreviewStatus::Ready { info, resolved };
                let (me, id, path) = (self.me.clone(), id.to_string(), self.cache_path(id));
                self.push_preview();
                tokio::spawn(async move {
                    let cached = match (path, probe::cacheable(&line)) {
                        (Some(path), Some(raw)) => write_atomic(&path, raw.as_bytes()).await.is_ok(),
                        _ => false,
                    };
                    let _ = me.send(Box::new(move |core| core.preview_cached(&id, cached)));
                });
            }
            Some(Probed::Entry { mut entry, playlist_title, playlist_uploader, archive_key }) => {
                entry.downloaded = run.archive.contains(&archive_key);
                if let PreviewStatus::Playlist { entries, .. } = &mut preview.status {
                    entries.push(entry.clone());
                    self.push(Change::PlaylistEntries { preview_id: id.to_string(), entries: vec![entry] });
                } else {
                    preview.status = PreviewStatus::Playlist {
                        title: playlist_title,
                        uploader: playlist_uploader,
                        entries: vec![entry],
                        complete: false,
                    };
                    // A playlist always needs the selection sheet, even in download-now mode.
                    run.auto_start = false;
                    self.push_preview();
                }
            }
            None => run.log.push(redact::redact(&line, self.shared.dirs.home.as_deref().and_then(Path::to_str))),
        }
    }

    /// The info file is on disk: the exact preset resolution can run, or the download start.
    fn preview_cached(&mut self, id: &str, cached: bool) {
        let Some(preview) = self.preview.as_ref().filter(|p| p.id == id) else { return };
        if cached {
            self.probed.insert(id.to_string(), Instant::now());
        }
        if self.preview_run.as_ref().is_some_and(|r| r.auto_start) {
            let request = preview.request.clone();
            if let Err(error) = self.preview_start(id, request, &[], false) {
                // Nothing was started (DRM, for one): show the preview so the reason is visible.
                tracing::info!(error, "could not start the download straight away");
                if let Some(run) = &mut self.preview_run {
                    run.auto_start = false;
                }
                self.push_preview();
            }
        } else if cached {
            self.resolve_exact();
        }
    }

    fn on_preview_exit(&mut self, id: &str, code: Option<i32>) {
        let Some(preview) = self.preview.as_mut().filter(|p| p.id == id) else { return };
        // The log stays with the preview run for Copy Details.
        let log = self.preview_run.as_mut().map(|r| {
            r.stop = None;
            r.log.clone()
        });
        match &mut preview.status {
            PreviewStatus::Playlist { complete, .. } => *complete = true,
            PreviewStatus::Probing => {
                preview.status = PreviewStatus::Failed { error: classify(&log.unwrap_or_default(), code) };
                if let Some(run) = &mut self.preview_run {
                    run.auto_start = false;
                }
            }
            _ => return,
        }
        self.push_preview();
    }

    fn preview_update(&mut self, id: &str, request: DownloadRequest) {
        let Some(preview) = self.preview.as_mut().filter(|p| p.id == id) else { return };
        preview.request = request;
        if let PreviewStatus::Ready { info, resolved } = &mut preview.status {
            *resolved = probe::approx_resolve(info, &preview.request);
            self.push_preview();
            self.resolve_exact();
        }
    }

    /// Asks yt-dlp what the open preview's request really selects. Only the newest answer counts.
    fn resolve_exact(&mut self) {
        let Some(preview) = &self.preview else { return };
        let PreviewStatus::Ready { info, .. } = &preview.status else { return };
        let Some(ytdlp) = self.tool_paths.get(&ToolId::Ytdlp).cloned() else { return };
        self.resolve_seq += 1;
        let seq = self.resolve_seq;
        let args = args::resolve_args(&self.info_path(&preview.id), &preview.request);
        let (me, request, height) = (self.me.clone(), preview.request.clone(), probe::source_height(info));
        tokio::spawn(async move {
            let Ok(out) = run::output(&ytdlp, &args, 60).await else { return };
            let Some(exact) = out.stdout.lines().find_map(|l| probe::parse_resolved(l, &request, height)) else {
                return;
            };
            let _ = me.send(Box::new(move |core| {
                if core.resolve_seq != seq {
                    return;
                }
                if let Some(Preview { status: PreviewStatus::Ready { resolved, .. }, .. }) = &mut core.preview {
                    *resolved = exact;
                    core.push_preview();
                }
            }));
        });
    }

    fn preview_start(
        &mut self,
        id: &str,
        request: DownloadRequest,
        selection: &[String],
        skip_downloaded: bool,
    ) -> Result<(), String> {
        let preview = self.preview.clone().filter(|p| p.id == id).ok_or("That link is no longer open.")?;
        match preview.status {
            PreviewStatus::Ready { info, resolved } => {
                if info.drm {
                    return Err(
                        "This video is protected by DRM. lovelyDL does not download DRM-protected media.".into()
                    );
                }
                let mut job = self.new_job(preview.url, preview.source, request.clone());
                job.title = info.title.clone();
                job.uploader = info.uploader.clone();
                job.duration = info.duration;
                job.site = info.site.clone();
                job.thumbnail = info.thumbnail.clone();
                // The shown resolution belongs to the request the preview had; a changed one is re-estimated.
                job.planned =
                    Some(if request == preview.request { resolved } else { probe::approx_resolve(&info, &request) });
                // Hand the cached probe over to the job so it is not extracted twice.
                let (from, to) = (self.info_path(id), self.info_path(&job.id));
                if let Some(at) = self.probed.remove(id) {
                    self.probed.insert(job.id.clone(), at);
                }
                self.preview = None;
                self.close_preview(false);
                let (me, held) = (self.me.clone(), job.clone());
                tokio::spawn(async move {
                    let _ = tokio::fs::rename(from, to).await;
                    let _ = me.send(Box::new(move |core| core.add_job(held)));
                });
                Ok(())
            }
            PreviewStatus::Playlist { title, entries, .. } => {
                let chosen: Vec<(usize, &PlaylistEntry)> =
                    entries.iter().enumerate().filter(|(_, e)| selection.contains(&e.id)).collect();
                if chosen.is_empty() {
                    return Err("Select at least one video.".into());
                }
                let group_id = self.next_id("g");
                let total = entries.len() as u32;
                for (position, entry) in chosen {
                    let mut job = self.new_job(entry.url.clone(), Source::Playlist, request.clone());
                    job.title = entry.title.clone();
                    job.uploader = entry.uploader.clone();
                    job.duration = entry.duration;
                    job.thumbnail = entry.thumbnail.clone();
                    job.group = Some(Group {
                        id: group_id.clone(),
                        title: if title.is_empty() { "Playlist".into() } else { title.clone() },
                        index: position as u32 + 1,
                        count: total,
                        skip_downloaded,
                    });
                    self.add_job(job);
                }
                self.close_preview(true);
                Ok(())
            }
            _ => Err("This link is not ready to download yet.".into()),
        }
    }

    // ----- settings and tools ----------------------------------------------------------

    fn set_settings(&mut self, new: Settings) {
        let mut new = settings::sanitize(new);
        if new.download_dir.trim().is_empty() {
            new.download_dir = self.settings.download_dir.clone();
        }
        // Not a preference: only the engine moves this.
        new.last_tool_check = self.settings.last_tool_check;
        let tools_changed = (&new.custom_ytdlp, &new.custom_ffmpeg_dir, &new.custom_js_runtime, new.use_system_tools)
            != (
                &self.settings.custom_ytdlp,
                &self.settings.custom_ffmpeg_dir,
                &self.settings.custom_js_runtime,
                self.settings.use_system_tools,
            );
        let channel_changed = new.ytdlp_channel != self.settings.ytdlp_channel;
        self.settings = new;
        self.save_settings();
        if tools_changed {
            self.refresh_components();
        }
        if channel_changed && self.components.ytdlp.origin == ToolOrigin::Managed {
            self.install(ToolId::Ytdlp, None);
        }
        self.needs_pump = true;
    }

    fn save_settings(&mut self) {
        self.settings_out.send_replace(Some(self.settings.clone()));
        self.push(Change::Settings(Box::new(self.settings.clone())));
    }

    /// Finds every tool and runs its version command. Tools being installed are left alone.
    fn refresh_components(&mut self) {
        let (toolchain, settings, me) = (self.shared.toolchain.clone(), self.settings.clone(), self.me.clone());
        let skip = self.installing.clone();
        self.refresh_seq += 1;
        let seq = self.refresh_seq;
        self.tools_known = false;
        tokio::spawn(async move {
            let check = |id: ToolId| {
                let (toolchain, settings) = (toolchain.clone(), settings.clone());
                async move {
                    let located =
                        tokio::task::spawn_blocking(move || toolchain.locate(id, &settings)).await.ok().flatten();
                    match located {
                        None => (id, None, ToolOrigin::Managed, Err(String::new())),
                        Some((path, origin)) => {
                            let health = toolchain::health(id, &path).await;
                            (id, Some(path), origin, health)
                        }
                    }
                }
            };
            let results = tokio::join!(check(ToolId::Ytdlp), check(ToolId::Ffmpeg), check(ToolId::Deno));
            let _ = me.send(Box::new(move |core| {
                // A newer check is running (settings changed, or an install finished): let it answer.
                if core.refresh_seq != seq {
                    return;
                }
                for (id, path, origin, health) in [results.0, results.1, results.2] {
                    if skip.contains(&id) && core.installing.contains(&id) {
                        continue;
                    }
                    let tool = core.components.tool_mut(id);
                    tool.origin = origin;
                    tool.path = path.as_ref().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
                    match (path, health) {
                        (Some(path), Ok(version)) => {
                            if tool.latest == version {
                                tool.latest.clear();
                            }
                            tool.version = version;
                            tool.status = ToolStatus::Ready;
                            core.tool_paths.insert(id, path);
                        }
                        (found, health) => {
                            tool.version.clear();
                            tool.status = match (found, health) {
                                (Some(_), Err(message)) => ToolStatus::Broken { message },
                                _ => ToolStatus::Missing,
                            };
                            core.tool_paths.remove(&id);
                        }
                    }
                }
                core.tools_known = true;
                core.push(Change::Components(Box::new(core.components.clone())));
                if let Some((id, choice)) = core.preview_waiting.take() {
                    core.probe_preview(&id, choice);
                }
                if core.components.ytdlp.ready() && core.components.ffmpeg.ready() {
                    for id in std::mem::take(&mut core.waiting_for_tools) {
                        core.retry(&id);
                    }
                }
                core.needs_pump = true;
            }));
        });
    }

    fn install(&mut self, id: ToolId, reply: Option<oneshot::Sender<Result<(), String>>>) {
        if !self.installing.insert(id) {
            // Already on its way; the caller sees the result through the components state.
            if let Some(reply) = reply {
                let _ = reply.send(Ok(()));
            }
            return;
        }
        self.components.tool_mut(id).status = ToolStatus::Installing { fraction: 0.0 };
        self.push(Change::Components(Box::new(self.components.clone())));
        let (toolchain, channel, me) = (self.shared.toolchain.clone(), self.settings.ytdlp_channel, self.me.clone());
        tokio::spawn(async move {
            let progress_me = me.clone();
            let last = std::sync::atomic::AtomicU32::new(0);
            let result = async {
                let release = toolchain.latest(id, channel).await?;
                toolchain
                    .install(id, &release, move |fraction| {
                        // One update per percent is plenty.
                        let percent = (fraction * 100.0) as u32;
                        if last.swap(percent, std::sync::atomic::Ordering::Relaxed) != percent {
                            let _ = progress_me.send(Box::new(move |core| {
                                if core.installing.contains(&id) {
                                    core.components.tool_mut(id).status = ToolStatus::Installing { fraction };
                                    core.push(Change::Components(Box::new(core.components.clone())));
                                }
                            }));
                        }
                    })
                    .await
            }
            .await;
            let _ = me.send(Box::new(move |core| {
                core.installing.remove(&id);
                if let Err(error) = &result {
                    tracing::warn!(tool = ?id, %error, "tool install failed");
                }
                core.refresh_components();
                if let Some(reply) = reply {
                    let _ = reply.send(result.map(|_| ()).map_err(|e| e.to_string()));
                }
            }));
        });
    }

    /// At most once a day, and only while nothing is downloading: look for newer managed
    /// tools and, if the user allows it, install them.
    fn check_for_tool_updates(&mut self) {
        let due = now_ms() - self.settings.last_tool_check >= DAY_MS;
        if !due || self.checking_updates || !self.runs.is_empty() || !self.installing.is_empty() {
            return;
        }
        let managed: Vec<(ToolId, String)> = [ToolId::Ytdlp, ToolId::Ffmpeg, ToolId::Deno]
            .into_iter()
            .map(|id| (id, self.components.tool(id)))
            .filter(|(_, tool)| tool.ready() && tool.origin == ToolOrigin::Managed)
            .map(|(id, tool)| (id, tool.version.clone()))
            .collect();
        self.settings.last_tool_check = now_ms();
        self.save_settings();
        if managed.is_empty() {
            return;
        }
        self.checking_updates = true;
        let (toolchain, channel, me) = (self.shared.toolchain.clone(), self.settings.ytdlp_channel, self.me.clone());
        tokio::spawn(async move {
            let mut newer = Vec::new();
            for (id, current) in managed {
                match toolchain.latest(id, channel).await {
                    Ok(release) if release.version != current => {
                        let held = toolchain.skipped(id).is_some_and(|v| v == release.version);
                        newer.push((id, release.version, held));
                    }
                    Ok(_) => {}
                    Err(error) => tracing::info!(tool = ?id, %error, "update check failed"),
                }
            }
            let _ = me.send(Box::new(move |core| {
                core.checking_updates = false;
                for (id, version, held) in newer {
                    core.components.tool_mut(id).latest = version;
                    // Never swap a tool under a running download, or undo a rollback.
                    if core.settings.auto_update_tools && core.runs.is_empty() && !held {
                        core.install(id, None);
                    }
                }
                core.push(Change::Components(Box::new(core.components.clone())));
            }));
        });
    }

    /// Start-up cleaning: probe results from the last session and month-old thumbnails.
    fn housekeeping(&self) {
        let cache = self.shared.dirs.cache.clone();
        tokio::task::spawn_blocking(move || {
            let _ = std::fs::remove_dir_all(cache.join("probe"));
            let month = Duration::from_secs(30 * 24 * 60 * 60);
            for entry in std::fs::read_dir(cache.join("thumbs")).into_iter().flatten().flatten() {
                let old = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .is_ok_and(|at| at.elapsed().is_ok_and(|age| age > month));
                if old {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        });
    }

    fn preview_details(&self, id: &str) -> Option<String> {
        let preview = self.preview.as_ref().filter(|p| p.id == id)?;
        let error = match &preview.status {
            PreviewStatus::Failed { error } => Some(error),
            _ => None,
        };
        let (args, lines) = self.preview_run.as_ref().map_or((&[][..], &[][..]), |r| (&r.args[..], &r.log[..]));
        Some(self.details_text(&preview.url, "Preview", error, args, lines.iter()))
    }

    fn details(&self, job: &Job) -> String {
        let (args, lines) = match self.logs.get(&job.id) {
            Some(log) => (&log.args[..], Some(log.lines.iter())),
            None => (&[][..], None),
        };
        self.details_text(&job.url, &format!("{:?}", job.state), job.error.as_ref(), args, lines.into_iter().flatten())
    }

    fn details_text<'a>(
        &self,
        url: &str,
        state: &str,
        error: Option<&JobError>,
        args: &[String],
        lines: impl Iterator<Item = &'a String>,
    ) -> String {
        let home = self.home();
        let tool = |id: ToolId| {
            let t = self.components.tool(id);
            if t.version.is_empty() {
                "not installed".to_string()
            } else {
                format!("{} ({:?})", t.version, t.origin).to_lowercase()
            }
        };
        let mut out = format!(
            "{}\nyt-dlp {}\nffmpeg {}\nJavaScript runtime {}\n\nLink: {}\nState: {state}\n",
            self.user_agent,
            tool(ToolId::Ytdlp),
            tool(ToolId::Ffmpeg),
            tool(ToolId::Deno),
            redact::redact(url, home.as_deref()),
        );
        if let Some(error) = error {
            out.push_str(&format!("Error: {:?}: {}\n", error.kind, error.detail));
        }
        if !args.is_empty() {
            let quoted: Vec<String> = args
                .iter()
                .map(|a| if a.contains([' ', '"']) { format!("\"{}\"", a.replace('"', "\\\"")) } else { a.clone() })
                .collect();
            out.push_str(&format!("\nCommand: yt-dlp {}\n", quoted.join(" ")));
        }
        let mut lines = lines.peekable();
        if lines.peek().is_some() {
            out.push_str("\nLog:\n");
            for line in lines {
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    }

    fn begin_shutdown(&mut self, done: oneshot::Sender<()>) {
        self.shutting_down = true;
        self.close_preview(true);
        let active: Vec<JobId> = self.jobs.iter().filter(|j| is_active(j.state)).map(|j| j.id.clone()).collect();
        for id in active {
            self.stop_run(&id, None);
            // Back in line: it resumes from its partial files on the next launch.
            self.transition(&id, JobState::Queued);
        }
        if self.stopping.is_empty() {
            let _ = done.send(());
        } else {
            self.shutdown = Some(done);
        }
    }
}

/// The template with " (n)" before the extension, for a name that is already taken.
fn numbered(template: &str, n: Option<u32>) -> String {
    let Some(n) = n else { return template.to_string() };
    match template.strip_suffix(".%(ext)s") {
        Some(stem) => format!("{stem} ({n}).%(ext)s"),
        None => format!("{template} ({n})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_taken_name_gets_a_number_before_the_extension() {
        assert_eq!(numbered("%(title)s [%(id)s].%(ext)s", None), "%(title)s [%(id)s].%(ext)s");
        assert_eq!(numbered("%(title)s [%(id)s].%(ext)s", Some(2)), "%(title)s [%(id)s] (2).%(ext)s");
        assert_eq!(numbered("%(uploader)s/%(title)s", Some(3)), "%(uploader)s/%(title)s (3)");
    }

    #[test]
    fn links_are_found_in_messy_text() {
        let text = "watch this https://a.test/v?x=1 and <https://b.test/>\nhttps://a.test/v?x=1 ftp://c.test/ notalink javascript:alert(1) \"http://d.test\"";
        assert_eq!(extract_urls(text), ["https://a.test/v?x=1", "https://b.test/", "http://d.test"]);
        assert!(extract_urls("--exec=calc file:///C:/x").is_empty());
    }

    #[test]
    fn video_and_playlist_links() {
        assert!(is_ambiguous("https://www.youtube.com/watch?v=SkVqJ1SGeL0&list=PL6B3937A5D230E335"));
        assert!(!is_ambiguous("https://www.youtube.com/watch?v=SkVqJ1SGeL0"));
        assert!(!is_ambiguous("https://www.youtube.com/playlist?list=PL6B3937A5D230E335"));
        assert!(!is_ambiguous("https://www.youtube.com/watch?v=&list=x"));
    }

    #[test]
    fn folder_names_are_valid_on_windows() {
        assert_eq!(folder_name("Official Blender Open Movies"), "Official Blender Open Movies");
        assert_eq!(folder_name("a/b\\c: d? <e> |f| \"g\"*"), "a b c d e f g");
        assert_eq!(folder_name("  trailing dots... "), "trailing dots");
        assert_eq!(folder_name("???"), "Playlist");
        assert_eq!(folder_name(&"x".repeat(300)).len(), 120);
    }

    #[test]
    fn volumes_and_hosts() {
        assert!(same_volume(Path::new(r"C:\a"), Path::new(r"c:\b\c")));
        assert_eq!(cfg!(windows), !same_volume(Path::new(r"C:\a"), Path::new(r"D:\a")));
        assert_eq!(host_of("https://www.youtube.com/watch?v=1"), "youtube.com");
        assert_eq!(host_of("nonsense"), "");
    }

    #[test]
    fn only_real_images_are_served() {
        assert_eq!(image_type(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(image_type(b"RIFF\x10\x00\x00\x00WEBPVP8 "), Some("image/webp"));
        assert_eq!(image_type(b"<html><script>"), None);
        assert_eq!(image_type(b""), None);
    }
}
