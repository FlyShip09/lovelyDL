//! Everything that depends on the operating system. Windows is the only target today; the
//! other module keeps the app compiling elsewhere so later ports start from working code.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(not(windows))]
mod other;
#[cfg(not(windows))]
pub use other::*;
