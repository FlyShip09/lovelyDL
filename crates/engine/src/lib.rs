//! lovelyDL's engine: everything except the window. No Tauri in here, so it can be tested
//! and driven from a command line.

pub mod engine;
pub mod model;
pub mod redact;
pub mod settings;
pub mod state;
pub mod store;
pub mod toolchain;
pub mod web;
pub mod ytdlp;

pub use engine::{Config, Dirs, Engine, EngineError, Sink};
