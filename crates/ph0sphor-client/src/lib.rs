//! PHOSPHOR client library.
//!
//! Exposed so integration tests can drive the WebSocket client task
//! against a real `ph0sphor-server` without standing up the TUI.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod app;
pub mod config;
pub mod event;
pub mod net;
pub mod state;
pub mod theme;
pub mod ui;

pub use app::run as run_app;
pub use config::ClientConfig;
pub use event::{AppEvent, ConnectionStatus};
pub use state::{AppState, Screen};
pub use theme::{Theme, ThemePalette};
