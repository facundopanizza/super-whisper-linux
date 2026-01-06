pub mod app;
pub mod audio;
pub mod clipboard;
pub mod config;
pub mod error;
pub mod ipc;
pub mod stt;
pub mod tray;

pub use app::{App, AppState};
pub use config::AppConfig;
pub use error::{AppError, Result};
