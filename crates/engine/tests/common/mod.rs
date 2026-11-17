//! Shared test harness: an engine in a temp folder, wired to `fake-ytdlp`, plus a mirror of
//! its state built only from the changes it emits (the same thing the UI does).

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use engine::model::*;
use engine::{Config, Dirs, Engine};
use tokio::sync::mpsc::UnboundedReceiver;

pub const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

/// Builds the fake once per test run and returns the path to its binary. Cargo reports the
/// path itself: the target folder layout changes between cargo versions.
pub fn fake() -> PathBuf {
    static FAKE: OnceLock<PathBuf> = OnceLock::new();
    FAKE.get_or_init(|| {
        let output = std::process::Command::new(env!("CARGO"))
            .args(["build", "-q", "-p", "fake-ytdlp", "--message-format=json"])
            .output()
            .expect("cargo runs");
        assert!(output.status.success(), "could not build fake-ytdlp: {}", String::from_utf8_lossy(&output.stderr));
        let path = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|msg| msg["target"]["name"] == "fake-ytdlp")
            .find_map(|msg| msg["executable"].as_str().map(PathBuf::from))
            .expect("cargo names the fake-ytdlp executable");
        assert!(path.is_file(), "fake-ytdlp is not at {}", path.display());
        path
    })
    .clone()
}

pub fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

pub struct Harness {
    pub root: PathBuf,
    /// Keeps the temp folder alive; `None` after `restart` hands it to the next harness.
    guard: Option<tempfile::TempDir>,
    pub engine: Engine,
    rx: UnboundedReceiver<Vec<Change>>,
    /// Every change received, in order.
    pub seen: Vec<Change>,
    /// Live jobs in queue order, rebuilt from changes alone.
    pub jobs: Vec<Job>,
    pub preview: Option<Preview>,
    pub settings: Settings,
    pub components: Components,
    /// The largest number of jobs that were active at the same moment.
    pub peak_active: usize,
    pub peak_per_site: usize,
}

pub struct Setup {
    /// Pause after every line the fake prints, to keep downloads in flight long enough to act on.
    pub line_ms: u64,
    /// Fixture for links that are not `https://fake.test/<name>`.
    pub default_fixture: &'static str,
    pub settings: Box<dyn FnOnce(&mut Settings)>,
}

impl Default for Setup {
    fn default() -> Self {
        Self { line_ms: 0, default_fixture: "generic_direct", settings: Box::new(|_| {}) }
    }
}

fn start(root: &Path) -> (Engine, UnboundedReceiver<Vec<Change>>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let engine = Engine::start(Config {
        dirs: Dirs::under(root),
        sink: Box::new(move |changes| {
            let _ = tx.send(changes);
        }),
        user_agent: "lovelyDL/test".into(),
        // Nothing in these tests may reach the real GitHub.
        api: Some("http://127.0.0.1:9".into()),
        // Tests must not sit through the real wait between automatic retries, but it has to
        // outlast the change window (100 ms) so the waiting state is actually seen.
        retry_backoff: Duration::from_millis(250),
    })
    .expect("engine starts");
    (engine, rx)
}

impl Harness {
    pub async fn new(setup: Setup) -> Self {
        let guard = tempfile::tempdir().unwrap();
        let root = guard.path().to_path_buf();
        let tools = root.join("tools");
        std::fs::create_dir_all(&tools).unwrap();
        for name in ["yt-dlp", "ffmpeg", "deno"] {
            std::fs::copy(fake(), tools.join(exe(name))).unwrap();
        }
        std::fs::write(
            tools.join("fake-ytdlp.env"),
            format!(
                "FAKE_YTDLP_FIXTURES={FIXTURES}/sessions\nFAKE_YTDLP_LINE_MS={}\nFAKE_YTDLP_LOG={}\nFAKE_YTDLP_FIXTURE={}\n",
                setup.line_ms,
                root.join("fake.log").display(),
                setup.default_fixture,
            ),
        )
        .unwrap();
        let mut settings = Settings {
            onboarding_done: true,
            download_dir: root.join("downloads").to_string_lossy().into_owned(),
            on_add: OnAdd::Download,
            custom_ytdlp: tools.join(exe("yt-dlp")).to_string_lossy().into_owned(),
            custom_ffmpeg_dir: tools.to_string_lossy().into_owned(),
            custom_js_runtime: format!("deno:{}", tools.join(exe("deno")).display()),
            // Recent enough that no test triggers an update check.
            last_tool_check: engine::engine::now_ms(),
            ..Settings::default()
        };
        (setup.settings)(&mut settings);
        engine::settings::save(&root.join("data").join("settings.json"), &settings).unwrap();
        let (engine, rx) = start(&root);
        let mut harness = Self {
            root,
            guard: Some(guard),
            engine,
            rx,
            seen: Vec::new(),
            jobs: Vec::new(),
            preview: None,
            settings,
            components: Components::default(),
            peak_active: 0,
            peak_per_site: 0,
        };
        harness.sync().await;
        harness
    }

    /// Replaces the mirror with a fresh snapshot, as a reloaded UI would.
    pub async fn sync(&mut self) {
        let snapshot = self.engine.snapshot().await.unwrap();
        self.jobs = snapshot.jobs;
        self.preview = snapshot.preview;
        self.settings = snapshot.settings;
        self.components = snapshot.components;
    }

    /// Shuts the engine down and starts a new one on the same folders: quit and relaunch.
    pub async fn restart(mut self) -> Self {
        self.engine.shutdown().await;
        let (engine, rx) = start(&self.root);
        let mut next = Self {
            root: self.root.clone(),
            guard: self.guard.take(),
            engine,
            rx,
            seen: Vec::new(),
            jobs: Vec::new(),
            preview: None,
            settings: self.settings.clone(),
            components: Components::default(),
            peak_active: 0,
            peak_per_site: 0,
        };
        next.sync().await;
        next
    }

    fn apply(&mut self, change: Change) {
        match &change {
            Change::JobAdded(job) => self.jobs.push((**job).clone()),
            Change::JobUpdated(job) => match self.jobs.iter_mut().find(|j| j.id == job.id) {
                Some(slot) => *slot = (**job).clone(),
                None => panic!("update for a job that was never added: {}", job.id),
            },
            Change::JobRemoved(id) => self.jobs.retain(|j| j.id != *id),
            Change::Progress { id, progress } => {
                if let Some(job) = self.jobs.iter_mut().find(|j| j.id == *id) {
                    job.progress = progress.clone();
                }
            }
            Change::Order(ids) => self.jobs.sort_by_key(|j| ids.iter().position(|id| *id == j.id)),
            Change::Preview(preview) => self.preview = preview.as_deref().cloned(),
            Change::PlaylistEntries { preview_id, entries } => {
                if let Some(Preview { id, status: PreviewStatus::Playlist { entries: all, .. }, .. }) =
                    &mut self.preview
                    && id == preview_id
                {
                    all.extend(entries.iter().cloned());
                }
            }
            Change::Settings(settings) => self.settings = (**settings).clone(),
            Change::Components(components) => self.components = (**components).clone(),
            _ => {}
        }
        let active: Vec<&Job> = self.jobs.iter().filter(|j| engine::state::is_active(j.state)).collect();
        self.peak_active = self.peak_active.max(active.len());
        let mut per_site: HashMap<String, usize> = HashMap::new();
        for job in &active {
            let host = url::Url::parse(&job.url).ok().and_then(|u| u.host_str().map(String::from)).unwrap_or_default();
            *per_site.entry(host).or_default() += 1;
        }
        self.peak_per_site = self.peak_per_site.max(per_site.values().copied().max().unwrap_or(0));
        self.seen.push(change);
    }

    /// Applies changes until `done` holds. Panics with `what` after 30 seconds.
    pub async fn wait_for(&mut self, what: &str, done: impl Fn(&Self) -> bool) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        while !done(self) {
            match tokio::time::timeout_at(deadline, self.rx.recv()).await {
                Ok(Some(batch)) => batch.into_iter().for_each(|change| self.apply(change)),
                Ok(None) => panic!("engine stopped while waiting for: {what}"),
                Err(_) => panic!(
                    "timed out waiting for: {what}\njobs: {:#?}\npreview: {:#?}\nlog: {:#?}",
                    self.jobs.iter().map(|j| (&j.url, j.state, &j.error)).collect::<Vec<_>>(),
                    self.preview.as_ref().map(|p| &p.status),
                    self.log(),
                ),
            }
        }
    }

    /// Applies whatever arrives during `ms` milliseconds.
    pub async fn settle(&mut self, ms: u64) {
        let deadline = tokio::time::Instant::now() + Duration::from_millis(ms);
        while let Ok(Some(batch)) = tokio::time::timeout_at(deadline, self.rx.recv()).await {
            batch.into_iter().for_each(|change| self.apply(change));
        }
    }

    pub async fn wait_tools(&mut self) {
        self.wait_for("tools to be checked", |h| {
            h.components.ytdlp.status != ToolStatus::Missing
                || h.seen.iter().any(|c| matches!(c, Change::Components(_)))
        })
        .await;
    }

    pub fn job(&self, url_part: &str) -> &Job {
        self.jobs.iter().find(|j| j.url.contains(url_part)).unwrap_or_else(|| panic!("no job for {url_part}"))
    }

    pub fn find(&self, url_part: &str) -> Option<&Job> {
        self.jobs.iter().find(|j| j.url.contains(url_part))
    }

    pub fn state(&self, url_part: &str) -> Option<JobState> {
        self.find(url_part).map(|j| j.state)
    }

    /// Every state a job has been reported in, without repeats, in order.
    pub fn states_of(&self, id: &str) -> Vec<JobState> {
        let mut states: Vec<JobState> = Vec::new();
        for change in &self.seen {
            if let Change::JobAdded(job) | Change::JobUpdated(job) = change
                && job.id == id
                && states.last() != Some(&job.state)
            {
                states.push(job.state);
            }
        }
        states
    }

    pub fn notices(&self) -> Vec<&Notice> {
        self.seen.iter().filter_map(|c| if let Change::Notice(n) = c { Some(n) } else { None }).collect()
    }

    /// What the fake was asked to do: (mode, fixture, arguments), skipping version checks.
    pub fn log(&self) -> Vec<(String, String, Vec<String>)> {
        std::fs::read_to_string(self.root.join("fake.log"))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, ' ');
                let (mode, fixture, args) = (parts.next()?, parts.next()?, parts.next()?);
                Some((mode.to_string(), fixture.to_string(), serde_json::from_str(args).ok()?))
            })
            .filter(|(mode, ..)| mode != "version" && mode != "ffmpeg")
            .collect()
    }

    /// The modes run for one fixture, in order: `["probe", "download"]`.
    pub fn modes(&self, fixture: &str) -> Vec<String> {
        self.log().into_iter().filter(|(_, f, _)| f == fixture).map(|(mode, ..)| mode).collect()
    }

    pub fn downloads(&self) -> PathBuf {
        self.root.join("downloads")
    }

    pub fn partial(&self, id: &str) -> PathBuf {
        self.root.join("cache").join("partial").join(id)
    }

    pub fn probe_cache(&self) -> Vec<String> {
        std::fs::read_dir(self.root.join("cache").join("probe"))
            .map(|dir| dir.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
            .unwrap_or_default()
    }
}

pub fn fake_url(fixture: &str) -> String {
    format!("https://fake.test/{fixture}")
}

/// Waits until a path stops existing; file deletion on Windows trails the process exit.
pub async fn gone(path: &Path) -> bool {
    for _ in 0..100 {
        if !path.exists() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}
