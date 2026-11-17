//! Windows: registry reads, power requests, toasts and the clipboard sequence number.

use std::sync::mpsc;

use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows_sys::Win32::System::Power::{ES_CONTINUOUS, ES_SYSTEM_REQUIRED, SetThreadExecutionState};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

fn reg_dword(root: HKEY, key: &str, value: &str) -> Option<u32> {
    let (key, value) = (wide(key), wide(value));
    let mut data: u32 = 0;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: the buffers outlive the call and `size` matches `data`.
    let status = unsafe {
        RegGetValueW(
            root,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&raw mut data).cast(),
            &mut size,
        )
    };
    (status == 0).then_some(data)
}

fn reg_string(root: HKEY, key: &str, value: &str) -> Option<String> {
    let (key, value) = (wide(key), wide(value));
    let mut buf = [0u16; 128];
    let mut size = size_of_val(&buf) as u32;
    // SAFETY: the buffers outlive the call and `size` is the buffer's size in bytes.
    let status = unsafe {
        RegGetValueW(
            root,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if status != 0 {
        return None;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

/// The Windows build number, such as 26300.
pub fn build_number() -> u32 {
    reg_string(HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "CurrentBuildNumber")
        .and_then(|b| b.trim().parse().ok())
        .unwrap_or(0)
}

/// Whether a real backdrop material can sit behind the window. Acrylic through the DWM
/// backdrop API needs build 22523; earlier versions only have a blur that lags while resizing.
pub fn material_supported() -> bool {
    build_number() >= 22523
}

/// The user's accent color as `#rrggbb`. The registry stores it as 0xAABBGGRR.
pub fn accent_color() -> Option<String> {
    let abgr = reg_dword(HKEY_CURRENT_USER, r"SOFTWARE\Microsoft\Windows\DWM", "AccentColor")?;
    let (r, g, b) = (abgr & 0xFF, (abgr >> 8) & 0xFF, (abgr >> 16) & 0xFF);
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}

/// Changes whenever anything is copied; reading it does not read the clipboard.
pub fn clipboard_sequence() -> u32 {
    // SAFETY: no arguments, no preconditions.
    unsafe { GetClipboardSequenceNumber() }
}

/// Keeps the computer from sleeping while `true` was the last value sent. An execution state
/// belongs to the thread that set it, so one thread owns it for the life of the app.
pub struct Awake(mpsc::Sender<bool>);

impl Default for Awake {
    fn default() -> Self {
        Self::new()
    }
}

impl Awake {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<bool>();
        let _ = std::thread::Builder::new().name("lovelydl-awake".into()).spawn(move || {
            for on in rx {
                let flags = if on { ES_CONTINUOUS | ES_SYSTEM_REQUIRED } else { ES_CONTINUOUS };
                // SAFETY: plain flag call on this thread.
                unsafe { SetThreadExecutionState(flags) };
            }
        });
        Self(tx)
    }

    pub fn set(&self, on: bool) {
        let _ = self.0.send(on);
    }
}

/// Turns off WebView2's browser shortcuts (F5 reload, Ctrl+P print, Ctrl+F find, zoom, F12).
/// They give the web view away; editing shortcuts such as Ctrl+C and Ctrl+V keep working.
pub fn quiet_browser_keys(window: &tauri::WebviewWindow) {
    let result = window.with_webview(|webview| {
        use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
        use windows::core::Interface;
        // SAFETY: COM calls on interfaces the webview owns, made on its own thread.
        unsafe {
            if let Ok(core) = webview.controller().CoreWebView2()
                && let Ok(settings) = core.Settings()
                && let Ok(settings) = settings.cast::<ICoreWebView2Settings3>()
            {
                let _ = settings.SetAreBrowserAcceleratorKeysEnabled(false);
            }
        }
    });
    if let Err(error) = result {
        tracing::debug!(%error, "could not reach the web view settings");
    }
}

/// Shows a toast. `on_click` runs on a system thread when the user clicks it.
pub fn toast(app_id: &str, title: &str, body: &str, on_click: impl Fn() + Send + 'static) {
    use tauri_winrt_notification::Toast;
    let shown = Toast::new(app_id)
        .title(title)
        .text1(body)
        .on_activated(move |_| {
            on_click();
            Ok(())
        })
        .show();
    if let Err(error) = shown {
        tracing::warn!(%error, "could not show a notification");
    }
}

/// Toasts need an app id that Windows knows. The installer registers lovelyDL's; a build run
/// from the source tree borrows PowerShell's, as Microsoft suggests for unpackaged tools.
pub fn toast_app_id(identifier: &str) -> String {
    let in_tree = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.ends_with(r"target\debug") || d.ends_with(r"target\release")))
        .unwrap_or(true);
    if in_tree { tauri_winrt_notification::Toast::POWERSHELL_APP_ID.to_string() } else { identifier.to_string() }
}
