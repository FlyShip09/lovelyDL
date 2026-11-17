//! Commands the UI can call. Each one is a thin wrapper around the engine or the platform.

use std::sync::atomic::Ordering;

use engine::model::*;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;

use crate::{AppState, shell, strings, windows};

type Res<T> = Result<T, String>;

#[tauri::command]
#[specta::specta]
pub async fn get_snapshot(state: State<'_, AppState>) -> Res<Snapshot> {
    let mut snapshot = state.engine.snapshot().await.map_err(|e| e.to_string())?;
    snapshot.system = state.system();
    Ok(snapshot)
}

#[tauri::command]
#[specta::specta]
pub async fn add_links(state: State<'_, AppState>, text: String, source: Source) -> Res<u32> {
    state.engine.add_links(text, source).await
}

#[tauri::command]
#[specta::specta]
pub fn preview_update(state: State<'_, AppState>, id: String, request: DownloadRequest) {
    state.engine.preview_update(id, request);
}

#[tauri::command]
#[specta::specta]
pub fn preview_choose(state: State<'_, AppState>, id: String, choice: PlaylistChoice) {
    state.engine.preview_choose(id, choice);
}

#[tauri::command]
#[specta::specta]
pub fn preview_dismiss(state: State<'_, AppState>, id: String) {
    state.engine.preview_dismiss(id);
}

#[tauri::command]
#[specta::specta]
pub async fn preview_start(
    state: State<'_, AppState>,
    id: String,
    request: DownloadRequest,
    selection: Vec<String>,
    skip_downloaded: bool,
) -> Res<()> {
    state.engine.preview_start(id, request, selection, skip_downloaded).await
}

#[tauri::command]
#[specta::specta]
pub fn job_pause(state: State<'_, AppState>, id: JobId) {
    state.engine.job_pause(id);
}

#[tauri::command]
#[specta::specta]
pub fn job_resume(state: State<'_, AppState>, id: JobId) {
    state.engine.job_resume(id);
}

#[tauri::command]
#[specta::specta]
pub fn job_cancel(state: State<'_, AppState>, id: JobId) {
    state.engine.job_cancel(id);
}

#[tauri::command]
#[specta::specta]
pub fn job_retry(state: State<'_, AppState>, id: JobId) {
    state.engine.job_retry(id);
}

#[tauri::command]
#[specta::specta]
pub fn job_remove(state: State<'_, AppState>, ids: Vec<JobId>) {
    state.engine.job_remove(ids);
}

#[tauri::command]
#[specta::specta]
pub fn job_apply_fix(state: State<'_, AppState>, id: JobId, fix: Fix) {
    state.engine.job_apply_fix(id, fix);
}

#[tauri::command]
#[specta::specta]
pub async fn job_update_request(state: State<'_, AppState>, id: JobId, request: DownloadRequest) -> Res<()> {
    state.engine.job_update_request(id, request).await
}

#[tauri::command]
#[specta::specta]
pub async fn job_formats(state: State<'_, AppState>, id: JobId) -> Res<MediaInfo> {
    state.engine.job_formats(id).await
}

#[tauri::command]
#[specta::specta]
pub fn jobs_reorder(state: State<'_, AppState>, ids: Vec<JobId>) {
    state.engine.jobs_reorder(ids);
}

#[tauri::command]
#[specta::specta]
pub fn pause_all(state: State<'_, AppState>) {
    state.engine.pause_all();
}

#[tauri::command]
#[specta::specta]
pub fn resume_all(state: State<'_, AppState>) {
    state.engine.resume_all();
}

#[tauri::command]
#[specta::specta]
pub fn retry_failed(state: State<'_, AppState>) {
    state.engine.retry_failed();
}

#[tauri::command]
#[specta::specta]
pub fn clear_completed(state: State<'_, AppState>) {
    state.engine.clear_completed();
}

#[tauri::command]
#[specta::specta]
pub async fn history_query(
    state: State<'_, AppState>,
    query: String,
    filter: HistoryFilter,
    offset: u32,
    limit: u32,
) -> Res<HistoryPage> {
    Ok(state.engine.history(query, filter, offset, limit).await)
}

#[tauri::command]
#[specta::specta]
pub fn history_remove(state: State<'_, AppState>, ids: Vec<JobId>) {
    state.engine.history_remove(ids);
}

#[tauri::command]
#[specta::specta]
pub async fn history_trash(state: State<'_, AppState>, ids: Vec<JobId>) -> Res<()> {
    state.engine.history_trash(ids).await
}

#[tauri::command]
#[specta::specta]
pub async fn download_again(state: State<'_, AppState>, id: JobId) -> Res<()> {
    state.engine.download_again(id).await
}

#[tauri::command]
#[specta::specta]
pub async fn reveal(app: AppHandle, state: State<'_, AppState>, id: JobId) -> Res<()> {
    let path = state.engine.output_path(id).await?;
    shell::reveal(&app, &path.to_string_lossy());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn open_file(app: AppHandle, state: State<'_, AppState>, id: JobId) -> Res<()> {
    let path = state.engine.output_path(id).await?;
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn open_folder(app: AppHandle, state: State<'_, AppState>) -> Res<()> {
    let folder = state.engine.snapshot().await.map_err(|e| e.to_string())?.settings.download_dir;
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    app.opener().open_path(folder, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn copy_link(app: AppHandle, state: State<'_, AppState>, id: JobId) -> Res<()> {
    let url = state.engine.job_url(id).await?;
    app.clipboard().write_text(url).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn copy_text(app: AppHandle, text: String) -> Res<()> {
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn job_details(state: State<'_, AppState>, id: JobId) -> Res<String> {
    state.engine.job_details(id).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set(state: State<'_, AppState>, settings: Settings) -> Res<()> {
    state.engine.settings_set(settings).await
}

/// Opens the system folder picker. `None` when the user cancels.
#[tauri::command]
#[specta::specta]
pub async fn choose_folder(app: AppHandle, start: String) -> Res<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = app.dialog().file();
        if !start.is_empty() {
            dialog = dialog.set_directory(start);
        }
        dialog.blocking_pick_folder()
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(picked.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
#[specta::specta]
pub async fn check_folder(state: State<'_, AppState>, path: String) -> Res<()> {
    state.engine.check_folder(path).await
}

#[tauri::command]
#[specta::specta]
pub async fn template_preview(state: State<'_, AppState>, template: String) -> Res<String> {
    state.engine.template_preview(template).await
}

#[tauri::command]
#[specta::specta]
pub fn components_refresh(state: State<'_, AppState>) {
    state.engine.components_refresh();
}

#[tauri::command]
#[specta::specta]
pub async fn component_install(state: State<'_, AppState>, tool: ToolId) -> Res<()> {
    state.engine.component_install(tool).await
}

#[tauri::command]
#[specta::specta]
pub async fn components_install_missing(state: State<'_, AppState>) -> Res<()> {
    state.engine.components_install_missing().await
}

#[tauri::command]
#[specta::specta]
pub async fn component_rollback(state: State<'_, AppState>, tool: ToolId) -> Res<()> {
    state.engine.component_rollback(tool).await
}

/// Adds the link that is on the clipboard. Only called after the user accepted the offer.
#[tauri::command]
#[specta::specta]
pub async fn clipboard_take_link(app: AppHandle, state: State<'_, AppState>) -> Res<u32> {
    let text = app.clipboard().read_text().map_err(|e| e.to_string())?;
    state.engine.add_links(text, Source::Clipboard).await
}

#[tauri::command]
#[specta::specta]
pub fn clipboard_dismiss(state: State<'_, AppState>) {
    // The offer is not repeated for the same clipboard contents.
    state.clipboard_seen.store(crate::platform::clipboard_sequence(), Ordering::Relaxed);
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WindowAction {
    Close,
    Minimize,
    Zoom,
    /// Shows the window once the UI has painted, so there is never a blank frame.
    Show,
}

#[tauri::command]
#[specta::specta]
pub fn window_action(window: WebviewWindow, action: WindowAction) -> Res<()> {
    let result = match action {
        WindowAction::Close => window.close(),
        WindowAction::Minimize => window.minimize(),
        WindowAction::Zoom => match window.is_maximized() {
            Ok(true) => window.unmaximize(),
            _ => window.maximize(),
        },
        WindowAction::Show => window.show().and_then(|()| window.set_focus()),
    };
    result.map_err(|e| e.to_string())
}

/// Stops running downloads so they resume on the next launch, then quits.
#[tauri::command]
#[specta::specta]
pub async fn quit(app: AppHandle) -> Res<()> {
    crate::quit(app).await;
    Ok(())
}

/// Async on purpose: building a window inside a synchronous command deadlocks on Windows.
#[tauri::command]
#[specta::specta]
pub async fn open_settings(app: AppHandle) -> Res<()> {
    windows::open_settings(&app).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Link {
    SupportedSites,
    ReportIssue,
    Project,
    PrivacySettings,
    Bookmarklet,
}

#[tauri::command]
#[specta::specta]
pub async fn open_link(app: AppHandle, state: State<'_, AppState>, link: Link) -> Res<()> {
    let url = match link {
        Link::SupportedSites => "https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md".to_string(),
        Link::Project => "https://github.com/FlyShip09/lovelyDL".to_string(),
        Link::Bookmarklet => "https://github.com/FlyShip09/lovelyDL#bookmarklet".to_string(),
        Link::PrivacySettings => "ms-settings:privacy-broadfilesystemaccess".to_string(),
        Link::ReportIssue => {
            // App and tool versions only. Links and paths are left for the user to add.
            let components = state.engine.snapshot().await.map_err(|e| e.to_string())?.components;
            let tool = |t: &Tool| if t.version.is_empty() { "not installed".into() } else { t.version.clone() };
            let body = format!(
                "**What happened**\n\n\n**What you expected**\n\n\n---\nlovelyDL {}\nWindows build {}\nyt-dlp {}\nffmpeg {}\nJavaScript runtime {}\n",
                env!("CARGO_PKG_VERSION"),
                crate::os_build(),
                tool(&components.ytdlp),
                tool(&components.ffmpeg),
                tool(&components.deno),
            );
            let mut url =
                url::Url::parse("https://github.com/FlyShip09/lovelyDL/issues/new").map_err(|e| e.to_string())?;
            url.query_pairs_mut().append_pair("body", &body);
            url.to_string()
        }
    };
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

/// Looks for a newer lovelyDL. Returns its version, or `None` when this one is current.
#[tauri::command]
#[specta::specta]
pub async fn app_update_check(app: AppHandle, state: State<'_, AppState>) -> Res<Option<String>> {
    use tauri_plugin_updater::UpdaterExt;
    let Some(key) = crate::UPDATER_PUBKEY else { return Err(strings::NO_UPDATER.into()) };
    let endpoint = url::Url::parse(crate::UPDATER_ENDPOINT).map_err(|e| e.to_string())?;
    let updater = app
        .updater_builder()
        .pubkey(key)
        .endpoints(vec![endpoint])
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    let version = update.as_ref().map(|u| u.version.clone());
    if let Ok(mut pending) = state.update.lock() {
        *pending = update;
    }
    Ok(version)
}

/// Installs the update found by `app_update_check` and restarts into it.
#[tauri::command]
#[specta::specta]
pub async fn app_update_install(app: AppHandle, state: State<'_, AppState>) -> Res<()> {
    let update = state.update.lock().ok().and_then(|mut p| p.take()).ok_or("Check for updates first.")?;
    // Downloads stop and resume after the restart, like a normal quit.
    state.engine.shutdown().await;
    update.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    app.restart();
}

/// The current window's label, so one bundle can render either window.
#[tauri::command]
#[specta::specta]
pub fn window_label(window: WebviewWindow) -> String {
    window.label().to_string()
}
