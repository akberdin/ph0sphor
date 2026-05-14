//! Single event type used to drive the app loop.
//!
//! Everything that wakes the main task — keystrokes, clock ticks,
//! incoming snapshots, connection state changes, server events — lands
//! here. The app loop selects on a single `mpsc::Receiver<AppEvent>` and
//! redraws when the resulting state change is visible.

use crossterm::event::KeyEvent;
use ph0sphor_core::Snapshot;
use ph0sphor_protocol::wire::DeltaUpdate;

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    /// Local clock tick — drives the on-screen clock without requiring
    /// a snapshot from the server. Frequency is throttled to 2 s when
    /// `low_power_mode` is enabled.
    Tick,
    /// A fresh telemetry snapshot from the server.
    Snapshot(Snapshot),
    /// A partial telemetry update from the server. Empty deltas are
    /// filtered server-side and should not appear here, but the client
    /// applies them tolerantly anyway.
    Delta(DeltaUpdate),
    /// Connection state changed.
    Connection(ConnectionStatus),
    /// Free-form log line — connection lifecycle, server events,
    /// client-side notices. Server `Event` payloads are folded into
    /// this until typed event routing lands in Milestone 6.
    Log(LogLine),
    /// Cooperative shutdown signal (Ctrl-C / Q).
    Quit,
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub severity: LogSeverity,
    pub text: String,
}

impl LogLine {
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            severity: LogSeverity::Info,
            text: text.into(),
        }
    }
    pub fn warn(text: impl Into<String>) -> Self {
        Self {
            severity: LogSeverity::Warn,
            text: text.into(),
        }
    }
    pub fn critical(text: impl Into<String>) -> Self {
        Self {
            severity: LogSeverity::Critical,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSeverity {
    Info,
    Warn,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Authenticating,
    Online,
    Offline,
}

impl ConnectionStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Disconnected => "DISCONN",
            Self::Connecting => "CONNECT",
            Self::Authenticating => "AUTH",
            Self::Online => "ONLINE",
            Self::Offline => "OFFLINE",
        }
    }

    pub fn is_stale(self) -> bool {
        matches!(self, Self::Disconnected | Self::Offline | Self::Connecting)
    }
}
