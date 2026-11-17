//! The system tray icon (off by default): a mini queue of what is downloading, and a few
//! commands. The Windows counterpart of the brief's menu bar extra.

use engine::model::{Change, UiCommand};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};
use tauri_specta::Event;

use crate::shell::{Update, show_main};
use crate::{AppState, strings};

const ID: &str = "lovelydl";

/// One line of the mini queue.
pub struct Entry {
    pub id: String,
    pub title: String,
    pub percent: Option<u32>,
    pub waiting: bool,
}

fn label(entry: &Entry) -> String {
    let title: String = entry.title.chars().take(48).collect();
    let title = if title.is_empty() { strings::UNTITLED.to_string() } else { title };
    match (entry.waiting, entry.percent) {
        (true, _) => format!("{title} — {}", strings::WAITING),
        (false, Some(p)) => format!("{title} — {p}%"),
        (false, None) => title,
    }
}

fn menu(app: &AppHandle, entries: &[Entry], paused: bool) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    if entries.is_empty() {
        menu.append(&MenuItem::with_id(app, "none", strings::NOTHING_DOWNLOADING, false, None::<&str>)?)?;
    }
    for entry in entries.iter().take(5) {
        menu.append(&MenuItem::with_id(app, format!("job:{}", entry.id), label(entry), true, None::<&str>)?)?;
    }
    if entries.len() > 5 {
        menu.append(&MenuItem::with_id(app, "more", strings::and_more(entries.len() - 5), false, None::<&str>)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "show", strings::SHOW_APP, true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "pause_all", strings::PAUSE_ALL, !entries.is_empty(), None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "resume_all", strings::RESUME_ALL, paused, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", strings::QUIT_APP, true, None::<&str>)?)?;
    Ok(menu)
}

fn on_menu(app: &AppHandle, id: &str) {
    let engine = app.state::<AppState>().engine.clone();
    match id {
        "show" => show_main(app),
        "pause_all" => engine.pause_all(),
        "resume_all" => engine.resume_all(),
        "quit" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if engine.active_count().await == 0 {
                    crate::quit(app).await;
                } else {
                    show_main(&app);
                    let _ = Update(vec![Change::Command(UiCommand::ConfirmQuit)]).emit(&app);
                }
            });
        }
        other => {
            if let Some(job) = other.strip_prefix("job:") {
                show_main(app);
                let _ = Update(vec![Change::ShowJob(job.to_string())]).emit(app);
            }
        }
    }
}

/// Shows, updates or removes the tray icon.
pub fn sync(app: &AppHandle, enabled: bool, entries: &[Entry], paused: bool) {
    if !enabled {
        let _ = app.remove_tray_by_id(ID);
        return;
    }
    let built = menu(app, entries, paused);
    let Ok(built) = built else { return };
    let tooltip = match entries.iter().filter(|e| !e.waiting).count() {
        0 => strings::APP.to_string(),
        1 => format!("{} — 1 download", strings::APP),
        n => format!("{} — {n} downloads", strings::APP),
    };
    if let Some(tray) = app.tray_by_id(ID) {
        let _ = tray.set_menu(Some(built));
        let _ = tray.set_tooltip(Some(tooltip));
        return;
    }
    let mut builder = TrayIconBuilder::with_id(ID)
        .tooltip(tooltip)
        .menu(&built)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    if let Err(error) = builder.build(app) {
        tracing::warn!(%error, "could not show the tray icon");
    }
}
