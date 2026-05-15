//! Shared domain types for PHOSPHOR.
//!
//! This crate intentionally has no I/O. It defines metrics, events, themes
//! and configuration shapes that both the server and the client agree on.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod error;
pub mod event;
pub mod metric;
pub mod theme;
pub mod version;

pub use error::CoreError;
pub use event::{Event, EventKind, Severity};
pub use metric::{
    CpuMetrics, DiskMetrics, MailItem, MailPrivacy, MailSummary, MemoryMetrics, NetworkMetrics,
    Snapshot, WeatherInfo,
};
pub use theme::Theme;
pub use version::{APP_VERSION, PROTOCOL_VERSION};
