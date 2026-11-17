//! What the shell does with the engine's changes besides passing them to the UI: the taskbar
//! progress and badge, keeping the computer awake, and notifications when the window is not
//! in front.

use std::collections::HashMap;

use engine::model::{Change, JobId, JobState, Notice, Settings};
use engine::state::is_active;
use tauri::window::{ProgressBarState, ProgressBarStatus};
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::platform;
use crate::strings;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, specta::Type, tauri_specta::Event)]
pub struct Update(pub Vec<Change>);

struct Mirror {
    state: JobState,
    fraction: Option<f64>,
    title: String,
    /// Arrival order, close enough to queue order for the tray's short list.
    seq: u64,
}

pub struct Shell {
    jobs: HashMap<JobId, Mirror>,
    seq: u64,
    settings: Settings,
    awake: platform::Awake,
    shown: (u32, Option<u64>, bool),
    toast_id: String,
    /// What the tray last showed: its layout, its full text, and when it was rebuilt.
    tray: (String, String, std::time::Instant),
}

impl Shell {
    pub fn new(settings: Settings, identifier: &str) -> Self {
        Self {
            jobs: HashMap::new(),
            seq: 0,
            settings,
            awake: platform::Awake::new(),
            shown: (0, None, false),
            toast_id: platform::toast_app_id(identifier),
            tray: (String::new(), String::new(), std::time::Instant::now()),
        }
    }

    fn remember(&mut self, job: &engine::model::Job) {
        self.seq += 1;
        let seq = self.jobs.get(&job.id).map_or(self.seq, |m| m.seq);
        let title = if job.title.is_empty() { job.url.clone() } else { job.title.clone() };
        self.jobs.insert(job.id.clone(), Mirror { state: job.state, fraction: job.progress.fraction, title, seq });
    }

    /// Passes a batch to the UI and updates everything outside the window.
    pub fn handle(&mut self, app: &AppHandle, changes: Vec<Change>) {
        let mut notices = Vec::new();
        for change in &changes {
            match change {
                Change::JobAdded(job) | Change::JobUpdated(job) => self.remember(job),
                Change::Progress { id, progress } => {
                    if let Some(entry) = self.jobs.get_mut(id) {
                        entry.fraction = progress.fraction;
                    }
                }
                Change::JobRemoved(id) => {
                    self.jobs.remove(id);
                }
                Change::Settings(settings) => self.settings = (**settings).clone(),
                Change::Notice(notice) => notices.push(notice.clone()),
                _ => {}
            }
        }
        if let Err(error) = Update(changes).emit(app) {
            tracing::error!(%error, "could not send changes to the window");
        }
        self.update_taskbar(app);
        self.update_tray(app);
        for notice in notices {
            self.notify(app, notice);
        }
    }

    /// Seeds the mirror from the jobs loaded at start-up.
    pub fn seed(&mut self, jobs: &[engine::model::Job]) {
        jobs.iter().for_each(|job| self.remember(job));
    }

    /// Keeps the tray icon in step. Its layout is rebuilt at once; percentages at most once
    /// a second, so a busy queue does not rebuild a menu ten times a second.
    fn update_tray(&mut self, app: &AppHandle) {
        let enabled = self.settings.tray_icon;
        if let Some(state) = app.try_state::<crate::AppState>() {
            state.tray.store(enabled, std::sync::atomic::Ordering::Relaxed);
        }
        let mut live: Vec<(&JobId, &Mirror)> =
            self.jobs.iter().filter(|(_, m)| is_active(m.state) || m.state == JobState::Queued).collect();
        live.sort_by_key(|(_, m)| (m.state == JobState::Queued, m.seq));
        let entries: Vec<crate::tray::Entry> = live
            .iter()
            .map(|(id, m)| crate::tray::Entry {
                id: (*id).clone(),
                title: m.title.clone(),
                percent: m.fraction.map(|f| (f * 100.0) as u32),
                waiting: m.state == JobState::Queued,
            })
            .collect();
        let paused = self.jobs.values().any(|m| m.state == JobState::Paused);
        let layout = format!(
            "{enabled}{paused}{}",
            entries.iter().map(|e| format!("{}{}", e.id, e.waiting)).collect::<String>()
        );
        let text = format!("{layout}{:?}", entries.iter().map(|e| e.percent).collect::<Vec<_>>());
        let (last_layout, last_text, at) = &self.tray;
        let quiet = !enabled && last_layout.is_empty();
        if quiet || text == *last_text || (layout == *last_layout && at.elapsed().as_millis() < 1000) {
            return;
        }
        crate::tray::sync(app, enabled, &entries, paused);
        self.tray = (layout, text, std::time::Instant::now());
    }

    fn update_taskbar(&mut self, app: &AppHandle) {
        let active: Vec<Option<f64>> = self.jobs.values().filter(|m| is_active(m.state)).map(|m| m.fraction).collect();
        let waiting = self.jobs.values().filter(|m| m.state == JobState::Queued).count();
        let count = (active.len() + waiting) as u32;
        // Each running download counts the same; one with no known size counts as half done.
        let progress = (!active.is_empty())
            .then(|| (active.iter().map(|f| f.unwrap_or(0.5)).sum::<f64>() / active.len() as f64 * 100.0) as u64);
        let awake = !active.is_empty() && self.settings.keep_awake;
        if (count, progress, awake) == self.shown {
            return;
        }
        let Some(window) = app.get_webview_window("main") else { return };
        if count != self.shown.0 {
            let icon = (count > 0).then(|| {
                tauri::image::Image::new_owned(crate::badge::render(count), crate::badge::size(), crate::badge::size())
            });
            if let Err(error) = window.set_overlay_icon(icon) {
                tracing::debug!(%error, "no taskbar badge");
            }
        }
        if progress != self.shown.1 {
            let state = match progress {
                Some(value) => ProgressBarState { status: Some(ProgressBarStatus::Normal), progress: Some(value) },
                None => ProgressBarState { status: Some(ProgressBarStatus::None), progress: None },
            };
            if let Err(error) = window.set_progress_bar(state) {
                tracing::debug!(%error, "no taskbar progress");
            }
        }
        if awake != self.shown.2 {
            self.awake.set(awake);
        }
        self.shown = (count, progress, awake);
    }

    fn notify(&self, app: &AppHandle, notice: Notice) {
        // Tracked from window events: asking the window here would wait on the UI thread.
        let focused =
            app.try_state::<crate::AppState>().is_some_and(|s| s.focused.load(std::sync::atomic::Ordering::Relaxed));
        if focused || !self.settings.notifications {
            return;
        }
        let handle = app.clone();
        match notice {
            Notice::Finished { title, path, .. } => {
                platform::toast(&self.toast_id, strings::DOWNLOADED, &title, move || reveal(&handle, &path));
            }
            Notice::Failed { job_id, title } => {
                platform::toast(&self.toast_id, strings::FAILED, &title, move || {
                    show_main(&handle);
                    let _ = Update(vec![Change::ShowJob(job_id.clone())]).emit(&handle);
                });
            }
            Notice::GroupFinished { title, completed, failed, folder } => {
                let body = strings::group_body(&title, completed, failed);
                platform::toast(&self.toast_id, strings::PLAYLIST_DONE, &body, move || reveal(&handle, &folder));
            }
        }
    }
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn reveal(app: &AppHandle, path: &str) {
    use tauri_plugin_opener::OpenerExt;
    let target = std::path::Path::new(path);
    let result = if target.is_dir() {
        app.opener().open_path(path, None::<&str>)
    } else {
        app.opener().reveal_item_in_dir(target)
    };
    if let Err(error) = result {
        tracing::warn!(%error, "could not show the file in Explorer");
    }
}
