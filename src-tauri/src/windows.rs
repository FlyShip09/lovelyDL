//! The two windows. Both are frameless: the UI draws its own title bar controls, the way the
//! macOS design it follows does.

use engine::model::SystemInfo;
use tauri::utils::config::Color;
use tauri::webview::ScrollBarStyle;
use tauri::window::{Effect, EffectsBuilder};
use tauri::{AppHandle, Manager, Theme, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::strings;

/// Facts the page needs before its first paint, so it never flashes the wrong colors.
/// `LOVELYDL_THEME=light` or `dark` forces a theme, for screenshots of both without
/// changing the Windows setting. Unset, the window follows Windows.
fn forced_theme() -> Option<Theme> {
    match std::env::var("LOVELYDL_THEME").as_deref() {
        Ok("light") => Some(Theme::Light),
        Ok("dark") => Some(Theme::Dark),
        _ => None,
    }
}

fn boot_script(system: &SystemInfo, view: &str) -> String {
    let boot = serde_json::json!({ "view": view, "system": system });
    format!("window.__LOVELYDL_BOOT__ = {boot};")
}

pub fn create_main(app: &AppHandle, system: &SystemInfo) -> tauri::Result<WebviewWindow> {
    let mut builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title(strings::APP)
        .inner_size(1000.0, 660.0)
        .min_inner_size(720.0, 460.0)
        .decorations(false)
        .shadow(true)
        // Shown by the page once it has painted.
        .visible(false)
        .transparent(true)
        .background_color(Color(0, 0, 0, 0))
        // Links dragged in from a browser arrive as HTML drag and drop, which the page handles.
        .disable_drag_drop_handler()
        .scroll_bar_style(ScrollBarStyle::FluentOverlay)
        .theme(forced_theme())
        .initialization_script(boot_script(system, "main"));
    if system.material {
        // The real system material, seen through the transparent sidebar.
        builder = builder.effects(EffectsBuilder::new().effect(Effect::Acrylic).build());
    }
    let window = builder.build()?;
    crate::platform::quiet_browser_keys(&window);
    Ok(window)
}

pub fn open_settings(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.unminimize();
        window.show()?;
        return window.set_focus();
    }
    let system = app.state::<crate::AppState>().system();
    let mut builder = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
        .title(strings::SETTINGS_TITLE)
        .inner_size(700.0, 560.0)
        .min_inner_size(620.0, 480.0)
        .decorations(false)
        .shadow(true)
        .visible(false)
        .maximizable(false)
        .minimizable(false)
        .disable_drag_drop_handler()
        .scroll_bar_style(ScrollBarStyle::FluentOverlay)
        .theme(forced_theme())
        .initialization_script(boot_script(&system, "settings"));
    if let Some(main) = app.get_webview_window("main") {
        builder = builder.parent(&main)?;
    }
    let window = builder.build()?;
    crate::platform::quiet_browser_keys(&window);
    Ok(())
}
