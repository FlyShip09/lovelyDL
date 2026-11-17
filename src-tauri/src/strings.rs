//! The few user-facing strings the shell itself shows (notifications, window titles). Every
//! string inside the window lives in `ui/src/strings.ts`.

pub const APP: &str = "lovelyDL";
pub const SETTINGS_TITLE: &str = "lovelyDL Settings";
pub const DOWNLOADED: &str = "Downloaded";
pub const FAILED: &str = "Download failed";
pub const PLAYLIST_DONE: &str = "Playlist downloaded";
pub const UNTITLED: &str = "Getting details…";
pub const WAITING: &str = "waiting";
pub const NOTHING_DOWNLOADING: &str = "Nothing downloading";
pub const SHOW_APP: &str = "Show lovelyDL";
pub const PAUSE_ALL: &str = "Pause all";
pub const RESUME_ALL: &str = "Resume all";
pub const QUIT_APP: &str = "Quit lovelyDL";

pub fn and_more(n: usize) -> String {
    format!("and {n} more")
}

pub const NO_UPDATER: &str = "Updates are not set up for this build. Download new versions from GitHub.";

pub fn group_body(title: &str, completed: u32, failed: u32) -> String {
    let saved = if completed == 1 { "1 item saved".to_string() } else { format!("{completed} items saved") };
    match failed {
        0 => format!("{title}: {saved}"),
        1 => format!("{title}: {saved}, 1 failed"),
        n => format!("{title}: {saved}, {n} failed"),
    }
}
