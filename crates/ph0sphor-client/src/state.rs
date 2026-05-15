//! Local cached state powering the TUI.

use crate::config::ClientConfig;
use crate::event::{AppEvent, ConnectionStatus, LogSeverity};
use crate::theme::{next_theme, Theme};
use ph0sphor_core::Snapshot;
use std::collections::VecDeque;
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Sys,
    Mail,
    Time,
    Weather,
    Log,
}

impl Screen {
    pub fn all() -> [Screen; 6] {
        [
            Screen::Home,
            Screen::Sys,
            Screen::Mail,
            Screen::Time,
            Screen::Weather,
            Screen::Log,
        ]
    }
    pub fn label(self) -> &'static str {
        match self {
            Screen::Home => "HOME",
            Screen::Sys => "SYS",
            Screen::Mail => "MAIL",
            Screen::Time => "TIME",
            Screen::Weather => "WTHR",
            Screen::Log => "LOG",
        }
    }
    pub fn next(self) -> Screen {
        let order = Self::all();
        let idx = order.iter().position(|s| *s == self).unwrap_or(0);
        order[(idx + 1) % order.len()]
    }
    pub fn prev(self) -> Screen {
        let order = Self::all();
        let idx = order.iter().position(|s| *s == self).unwrap_or(0);
        order[(idx + order.len() - 1) % order.len()]
    }
    pub fn from_digit(d: char) -> Option<Screen> {
        match d {
            '1' => Some(Screen::Home),
            '2' => Some(Screen::Sys),
            '3' => Some(Screen::Mail),
            '4' => Some(Screen::Time),
            '5' => Some(Screen::Weather),
            '6' => Some(Screen::Log),
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

/// Local-only timer / stopwatch / alarms (README §15.8 — must keep
/// working when the server is disconnected).
#[derive(Debug)]
pub struct TimeState {
    /// Configured timer preset.
    pub timer_preset: Duration,
    /// Wall-clock instant when the timer was started, or `None` if
    /// stopped or never started.
    pub timer_started_at: Option<Instant>,
    /// Time already elapsed before the current run (lets pause/resume
    /// accumulate correctly).
    pub timer_elapsed_before_run: Duration,

    /// Stopwatch — same accumulator pattern as the timer.
    pub stopwatch_started_at: Option<Instant>,
    pub stopwatch_elapsed_before_run: Duration,

    /// Configured alarm targets in minutes-since-midnight (UTC).
    pub alarms: Vec<u32>,
    /// The minute-of-day index of the most recently fired alarm, so
    /// we don't re-fire it repeatedly inside the same minute.
    pub last_fired_minute_of_day: Option<u32>,
}

impl TimeState {
    pub fn new(timer_preset_secs: u64, alarms: Vec<u32>) -> Self {
        Self {
            timer_preset: Duration::from_secs(timer_preset_secs.max(1)),
            timer_started_at: None,
            timer_elapsed_before_run: Duration::ZERO,
            stopwatch_started_at: None,
            stopwatch_elapsed_before_run: Duration::ZERO,
            alarms,
            last_fired_minute_of_day: None,
        }
    }

    pub fn timer_running(&self) -> bool {
        self.timer_started_at.is_some()
    }

    pub fn stopwatch_running(&self) -> bool {
        self.stopwatch_started_at.is_some()
    }

    /// Total elapsed since the timer was first started, ignoring pauses.
    pub fn timer_elapsed(&self) -> Duration {
        self.timer_elapsed_before_run
            + self
                .timer_started_at
                .map(|s| s.elapsed())
                .unwrap_or_default()
    }

    /// Remaining time on the timer, saturating at zero.
    pub fn timer_remaining(&self) -> Duration {
        self.timer_preset.saturating_sub(self.timer_elapsed())
    }

    pub fn stopwatch_elapsed(&self) -> Duration {
        self.stopwatch_elapsed_before_run
            + self
                .stopwatch_started_at
                .map(|s| s.elapsed())
                .unwrap_or_default()
    }

    pub fn toggle_timer(&mut self) {
        match self.timer_started_at.take() {
            Some(start) => {
                self.timer_elapsed_before_run += start.elapsed();
            }
            None => {
                self.timer_started_at = Some(Instant::now());
            }
        }
    }

    pub fn reset_timer(&mut self) {
        self.timer_started_at = None;
        self.timer_elapsed_before_run = Duration::ZERO;
    }

    pub fn toggle_stopwatch(&mut self) {
        match self.stopwatch_started_at.take() {
            Some(start) => {
                self.stopwatch_elapsed_before_run += start.elapsed();
            }
            None => {
                self.stopwatch_started_at = Some(Instant::now());
            }
        }
    }

    pub fn reset_stopwatch(&mut self) {
        self.stopwatch_started_at = None;
        self.stopwatch_elapsed_before_run = Duration::ZERO;
    }

    pub fn adjust_timer_preset(&mut self, delta_secs: i64) {
        let cur = self.timer_preset.as_secs() as i64;
        let next = (cur + delta_secs).clamp(1, 24 * 3600);
        self.timer_preset = Duration::from_secs(next as u64);
    }
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
    /// Active pairing code waiting for operator confirmation. Cleared
    /// on `TokenIssued`. Displayed prominently in the header so the
    /// user can read it off the screen and run the matching
    /// `ph0sphorctl pair confirm` on the server.
    pub pairing_code: Option<String>,
    /// Local clock features (timer, stopwatch, alarms). Populated from
    /// `ClientConfig` and ticked locally — works fully offline.
    pub time: TimeState,
    /// Last seen mail unread count, used to detect "new mail" events
    /// for the rich event log.
    pub last_seen_unread_count: u32,
    /// Whether we have ever seen a mail snapshot (so the first one
    /// doesn't read as a flood of new messages).
    pub mail_seeded: bool,
}

impl AppState {
    pub fn new(config: ClientConfig) -> Self {
        let theme = config.client.theme;
        let events_cap = config.cache.max_cached_events.max(20);
        let screen = match config.ui.default_screen.as_str() {
            "sys" => Screen::Sys,
            "mail" => Screen::Mail,
            "time" => Screen::Time,
            "weather" => Screen::Weather,
            "log" => Screen::Log,
            _ => Screen::Home,
        };
        let timer_preset = config.time.timer_default_secs;
        let alarms = config
            .time
            .alarms
            .iter()
            .filter_map(|s| parse_hhmm_to_minute_of_day(s))
            .collect();
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
            pairing_code: None,
            time: TimeState::new(timer_preset, alarms),
            last_seen_unread_count: 0,
            mail_seeded: false,
        }
    }

    /// Detect any user-visible time-domain events that should land in
    /// the event log: timer completion, alarm fires.
    pub fn check_time_events(&mut self) {
        if self.time.timer_running() && self.time.timer_remaining() == Duration::ZERO {
            self.time.timer_started_at = None;
            self.time.timer_elapsed_before_run = self.time.timer_preset;
            if !self.muted {
                self.push_log(LogSeverity::Warn, "TIMER: completed".into());
            }
        }
        let mod_now = current_minute_of_day();
        if self.time.alarms.contains(&mod_now)
            && self.time.last_fired_minute_of_day != Some(mod_now)
        {
            self.time.last_fired_minute_of_day = Some(mod_now);
            if !self.muted {
                let h = mod_now / 60;
                let m = mod_now % 60;
                self.push_log(LogSeverity::Critical, format!("ALARM: {h:02}:{m:02} UTC"));
            }
        }
    }

    /// Apply an incoming event to the state. Returns `true` when the
    /// visible output may have changed and a redraw is warranted.
    pub fn apply(&mut self, event: AppEvent) -> bool {
        match event {
            AppEvent::Tick => {
                self.check_time_events();
                true
            }
            AppEvent::Snapshot(snap) => {
                self.snapshot = snap;
                self.detect_new_mail();
                true
            }
            AppEvent::Delta(delta) => {
                ph0sphor_protocol::delta::apply_delta(&mut self.snapshot, &delta);
                self.detect_new_mail();
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
            AppEvent::PairingChallenge(code) => {
                self.pairing_code = Some(code.clone());
                self.push_log(
                    LogSeverity::Warn,
                    format!("PAIRING CODE: {code} — confirm on server"),
                );
                true
            }
            AppEvent::TokenIssued(_) => {
                // The raw token is not put through the visible log: it
                // would leak the secret onto the screen. We mark the
                // pairing as done and let the app loop persist the
                // token off-screen.
                self.pairing_code = None;
                self.push_log(LogSeverity::Info, "client paired — token stored".into());
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

        // TIME-screen-scoped controls (timer / stopwatch / preset).
        if self.screen == Screen::Time {
            if let Some(handled) = self.handle_time_screen_key(key.code) {
                return handled;
            }
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

    /// Returns `Some(dirty)` if the key was consumed by the TIME screen,
    /// `None` to fall through to the global handler.
    fn handle_time_screen_key(&mut self, code: crossterm::event::KeyCode) -> Option<bool> {
        use crossterm::event::KeyCode;
        match code {
            // T toggles the timer; W toggles the stopwatch ("watch").
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.time.toggle_timer();
                Some(true)
            }
            KeyCode::Char('w') | KeyCode::Char('W') => {
                self.time.toggle_stopwatch();
                Some(true)
            }
            // R on the TIME screen resets both, instead of "refresh".
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.time.reset_timer();
                self.time.reset_stopwatch();
                Some(true)
            }
            // +/- adjusts the timer preset by 30 s.
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.time.adjust_timer_preset(30);
                Some(true)
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                self.time.adjust_timer_preset(-30);
                Some(true)
            }
            _ => None,
        }
    }

    fn detect_new_mail(&mut self) {
        let Some((unread_count, log_msg)) = self.snapshot.mail.as_ref().map(|mail| {
            let detail = mail
                .recent
                .first()
                .map(|m| {
                    if m.sender.is_empty() && m.subject.is_empty() {
                        String::new()
                    } else {
                        format!(": {} — {}", m.sender, m.subject)
                    }
                })
                .unwrap_or_default();
            (mail.unread_count, detail)
        }) else {
            return;
        };

        if !self.mail_seeded {
            self.last_seen_unread_count = unread_count;
            self.mail_seeded = true;
            return;
        }
        if unread_count > self.last_seen_unread_count && !self.muted {
            let new_count = unread_count - self.last_seen_unread_count;
            self.push_log(
                LogSeverity::Warn,
                format!("NEW MAIL ({new_count}){log_msg}"),
            );
        }
        self.last_seen_unread_count = unread_count;
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

/// Parse `"HH:MM"` (UTC) into minutes-since-midnight. Invalid strings
/// return `None`. We deliberately store alarms as minute-of-day rather
/// than absolute timestamps so alarm rules survive a clock change.
pub fn parse_hhmm_to_minute_of_day(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    let h: u32 = h.trim().parse().ok()?;
    let m: u32 = m.trim().parse().ok()?;
    if h >= 24 || m >= 60 {
        return None;
    }
    Some(h * 60 + m)
}

/// Current UTC minute of day. Local-time alarms are out of scope for
/// MVP — the README screen example shows a simple `HH:MM` clock and a
/// VAIO P parked next to a workstation typically has its system clock
/// at UTC under the hood.
pub fn current_minute_of_day() -> u32 {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    ((secs / 60) % (24 * 60)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use ph0sphor_core::CpuMetrics;

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
    fn delta_event_patches_snapshot_in_place() {
        use ph0sphor_protocol::wire::DeltaUpdate;
        let mut app = AppState::new(ClientConfig::default());
        // Seed with a snapshot so we have something to patch.
        let seed = Snapshot {
            cpu: CpuMetrics {
                usage_percent: 10.0,
                ..CpuMetrics::default()
            },
            ..Snapshot::default()
        };
        app.apply(AppEvent::Snapshot(seed));

        // Apply a delta that only changes cpu_usage_percent.
        let delta = DeltaUpdate {
            timestamp_unix_ms: 123,
            cpu_usage_percent: Some(72.5),
            ..DeltaUpdate::default()
        };
        assert!(app.apply(AppEvent::Delta(delta)));
        assert_eq!(app.snapshot.cpu.usage_percent, 72.5);
        // Other fields are untouched.
        assert_eq!(app.snapshot.cpu.temperature_c, None);
        assert_eq!(app.snapshot.timestamp_unix_ms, 123);
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
    fn screen_navigation_visits_all_six() {
        let mut s = Screen::Home;
        let mut seen = Vec::new();
        for _ in 0..6 {
            seen.push(s);
            s = s.next();
        }
        assert_eq!(s, Screen::Home, "cycle returns to start");
        assert_eq!(seen.len(), 6);
        // Number keys map to the same set.
        assert_eq!(Screen::from_digit('3'), Some(Screen::Mail));
        assert_eq!(Screen::from_digit('4'), Some(Screen::Time));
        assert_eq!(Screen::from_digit('5'), Some(Screen::Weather));
    }

    #[test]
    fn timer_toggle_and_reset() {
        let mut app = AppState::new(ClientConfig::default());
        assert!(!app.time.timer_running());
        app.time.toggle_timer();
        assert!(app.time.timer_running());
        app.time.toggle_timer();
        assert!(!app.time.timer_running());
        app.time.adjust_timer_preset(60);
        assert!(app.time.timer_preset.as_secs() >= 360);
        app.time.reset_timer();
        assert_eq!(app.time.timer_elapsed(), Duration::ZERO);
    }

    #[test]
    fn parse_hhmm_handles_valid_and_invalid() {
        assert_eq!(parse_hhmm_to_minute_of_day("00:00"), Some(0));
        assert_eq!(parse_hhmm_to_minute_of_day("12:30"), Some(750));
        assert_eq!(parse_hhmm_to_minute_of_day("23:59"), Some(23 * 60 + 59));
        assert_eq!(parse_hhmm_to_minute_of_day("24:00"), None);
        assert_eq!(parse_hhmm_to_minute_of_day("12:60"), None);
        assert_eq!(parse_hhmm_to_minute_of_day("garbage"), None);
    }

    #[test]
    fn new_mail_is_logged_only_after_seeded() {
        use ph0sphor_core::{MailItem, MailPrivacy, MailSummary};
        let mut app = AppState::new(ClientConfig::default());

        // First snapshot seeds the baseline; no log entry should fire.
        let snap1 = Snapshot {
            mail: Some(MailSummary {
                unread_count: 2,
                privacy: MailPrivacy::SenderSubject,
                recent: vec![],
                last_update_unix_ms: 0,
            }),
            ..Snapshot::default()
        };
        let before = app.events.len();
        app.apply(AppEvent::Snapshot(snap1));
        assert_eq!(app.events.len(), before, "first snapshot must not log");

        // Second snapshot with higher count must log.
        let snap2 = Snapshot {
            mail: Some(MailSummary {
                unread_count: 4,
                privacy: MailPrivacy::SenderSubject,
                recent: vec![MailItem {
                    sender: "a@b".into(),
                    subject: "hi".into(),
                    ..MailItem::default()
                }],
                last_update_unix_ms: 0,
            }),
            ..Snapshot::default()
        };
        app.apply(AppEvent::Snapshot(snap2));
        let head = app.events.front().expect("log entry");
        assert!(head.text.starts_with("NEW MAIL"));
        assert!(head.text.contains("a@b"));
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
