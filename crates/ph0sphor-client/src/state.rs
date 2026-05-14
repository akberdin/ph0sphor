//! Local cached state powering the TUI.

use crate::config::ClientConfig;
use crate::event::{AppEvent, ConnectionStatus, LogSeverity};
use crate::theme::{next_theme, Theme};
use ph0sphor_core::Snapshot;
use std::collections::VecDeque;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Sys,
    Log,
}

impl Screen {
    pub fn all() -> [Screen; 3] {
        [Screen::Home, Screen::Sys, Screen::Log]
    }
    pub fn label(self) -> &'static str {
        match self {
            Screen::Home => "HOME",
            Screen::Sys => "SYS",
            Screen::Log => "LOG",
        }
    }
    pub fn next(self) -> Screen {
        match self {
            Screen::Home => Screen::Sys,
            Screen::Sys => Screen::Log,
            Screen::Log => Screen::Home,
        }
    }
    pub fn prev(self) -> Screen {
        match self {
            Screen::Home => Screen::Log,
            Screen::Sys => Screen::Home,
            Screen::Log => Screen::Sys,
        }
    }
    pub fn from_digit(d: char) -> Option<Screen> {
        match d {
            '1' => Some(Screen::Home),
            '2' => Some(Screen::Sys),
            '3' => Some(Screen::Log),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp_unix_ms: u64,
    pub severity: LogSeverity,
    pub text: String,
}

#[derive(Debug)]
pub struct AppState {
    pub config: ClientConfig,
    pub theme: Theme,
    pub screen: Screen,
    pub snapshot: Snapshot,
    pub connection: ConnectionStatus,
    pub events: VecDeque<LogEntry>,
    pub events_cap: usize,
    pub muted: bool,
    pub quit: bool,
}

impl AppState {
    pub fn new(config: ClientConfig) -> Self {
        let theme = config.client.theme;
        let events_cap = config.cache.max_cached_events.max(20);
        let screen = match config.ui.default_screen.as_str() {
            "sys" => Screen::Sys,
            "log" => Screen::Log,
            _ => Screen::Home,
        };
        Self {
            config,
            theme,
            screen,
            snapshot: Snapshot::default(),
            connection: ConnectionStatus::Disconnected,
            events: VecDeque::new(),
            events_cap,
            muted: false,
            quit: false,
        }
    }

    /// Apply an incoming event to the state. Returns `true` when the
    /// visible output may have changed and a redraw is warranted.
    pub fn apply(&mut self, event: AppEvent) -> bool {
        match event {
            AppEvent::Tick => true, // clock element in title bar must tick
            AppEvent::Snapshot(snap) => {
                self.snapshot = snap;
                true
            }
            AppEvent::Connection(status) => {
                if self.connection != status {
                    self.push_log(LogSeverity::Info, format!("LINK: {}", status.label()));
                }
                self.connection = status;
                true
            }
            AppEvent::Log(line) => {
                self.push_log(line.severity, line.text);
                true
            }
            AppEvent::Quit => {
                self.quit = true;
                false
            }
            AppEvent::Key(key) => self.handle_key(key),
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};
        // Ignore non-press events (releases on terminals that support them).
        if key.kind != crossterm::event::KeyEventKind::Press {
            return false;
        }
        // Ctrl+C is a hard quit alongside `Q`.
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return false;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.quit = true;
                false
            }
            KeyCode::Char(c @ '1'..='9') => {
                if let Some(s) = Screen::from_digit(c) {
                    self.screen = s;
                    return true;
                }
                false
            }
            KeyCode::Tab => {
                self.screen = self.screen.next();
                true
            }
            KeyCode::BackTab => {
                self.screen = self.screen.prev();
                true
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.theme = next_theme(self.theme);
                true
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.muted = !self.muted;
                true
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // R = request full snapshot. The WS client task does not
                // yet act on this; the request is recorded in the log
                // until Milestone 4 wires the round-trip.
                self.push_log(LogSeverity::Info, "refresh requested".into());
                true
            }
            _ => false,
        }
    }

    pub fn push_log(&mut self, severity: LogSeverity, text: String) {
        let entry = LogEntry {
            timestamp_unix_ms: now_unix_ms(),
            severity,
            text,
        };
        self.events.push_front(entry);
        self.trim_events();
    }

    fn trim_events(&mut self) {
        while self.events.len() > self.events_cap {
            self.events.pop_back();
        }
    }
}

pub fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ph0sphor_core::CpuMetrics;

    #[test]
    fn screen_cycling_visits_all() {
        let mut s = Screen::Home;
        let mut seen = Vec::new();
        for _ in 0..3 {
            seen.push(s);
            s = s.next();
        }
        assert_eq!(s, Screen::Home);
        assert_eq!(seen.len(), 3);
    }

    #[test]
    fn snapshot_event_updates_state() {
        let mut app = AppState::new(ClientConfig::default());
        let snap = Snapshot {
            cpu: CpuMetrics {
                usage_percent: 42.5,
                ..CpuMetrics::default()
            },
            ..Snapshot::default()
        };
        assert!(app.apply(AppEvent::Snapshot(snap)));
        assert_eq!(app.snapshot.cpu.usage_percent, 42.5);
    }

    #[test]
    fn connection_change_writes_log_entry() {
        let mut app = AppState::new(ClientConfig::default());
        app.apply(AppEvent::Connection(ConnectionStatus::Online));
        assert_eq!(app.connection, ConnectionStatus::Online);
        assert!(app
            .events
            .front()
            .map(|e| e.text.contains("ONLINE"))
            .unwrap_or(false));
    }

    #[test]
    fn event_log_caps_at_max() {
        let mut cfg = ClientConfig::default();
        cfg.cache.max_cached_events = 20;
        let mut app = AppState::new(cfg);
        for i in 0..50 {
            app.push_log(LogSeverity::Info, format!("msg {i}"));
        }
        assert_eq!(app.events.len(), 20);
        // Newest entry is at the front.
        assert_eq!(app.events.front().unwrap().text, "msg 49");
    }
}
