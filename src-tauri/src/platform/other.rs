//! Systems without a port yet: no material, no accent, no toasts, no power requests.

pub fn material_supported() -> bool {
    false
}

pub fn accent_color() -> Option<String> {
    None
}

pub fn clipboard_sequence() -> u32 {
    0
}

#[derive(Default)]
pub struct Awake;

impl Awake {
    pub fn new() -> Self {
        Self
    }

    pub fn set(&self, _on: bool) {}
}

pub fn quiet_browser_keys(_window: &tauri::WebviewWindow) {}

pub fn toast(_app_id: &str, _title: &str, _body: &str, _on_click: impl Fn() + Send + 'static) {}

pub fn toast_app_id(identifier: &str) -> String {
    identifier.to_string()
}
