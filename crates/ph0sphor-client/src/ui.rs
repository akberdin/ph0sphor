//! Per-screen render functions.
//!
//! All rendering happens here. The app loop calls [`draw`] with the
//! current [`AppState`]; each helper below is responsible for a single
//! screen. Widgets are intentionally simple — no animation, no
//! per-frame allocation that the budget could not absorb at 1–2 FPS.

use crate::event::LogSeverity;
use crate::state::{AppState, LogEntry, Screen};
use crate::theme::ThemePalette;
use ph0sphor_core::Snapshot;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &AppState) {
    let palette = ThemePalette::for_theme(app.theme);
    let area = frame.area();

    // Apply a base background to the whole screen so the theme palette
    // is visible even outside the bordered blocks.
    let base = Block::default().style(Style::default().fg(palette.fg).bg(palette.bg));
    frame.render_widget(base, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),    // body
            Constraint::Length(1), // status bar
        ])
        .split(area);

    render_header(frame, chunks[0], app, &palette);
    match app.screen {
        Screen::Home => render_home(frame, chunks[1], app, &palette),
        Screen::Sys => render_sys(frame, chunks[1], app, &palette),
        Screen::Log => render_log(frame, chunks[1], app, &palette),
    }
    render_status_bar(frame, chunks[2], app, &palette);
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn render_header(frame: &mut Frame, area: Rect, app: &AppState, palette: &ThemePalette) {
    let title = format!("PHOSPHOR :: {}", app.screen.label());
    let (date, time) = format_clock_now();
    let link_style = link_style_for(app.connection, palette);
    let host = if app.snapshot.hostname.is_empty() {
        "(awaiting snapshot)".to_string()
    } else {
        app.snapshot.hostname.clone()
    };
    let stale = if app.connection.is_stale() {
        " *STALE*"
    } else {
        ""
    };

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                title,
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled(format!("{date}  {time}"), Style::default().fg(palette.fg)),
        ]),
        Line::from(vec![
            Span::styled("LINK: ", Style::default().fg(palette.dim)),
            Span::styled(app.connection.label(), link_style),
            Span::styled(stale, Style::default().fg(palette.warning)),
            Span::raw("   "),
            Span::styled("HOST: ", Style::default().fg(palette.dim)),
            Span::styled(host, Style::default().fg(palette.fg)),
            Span::raw("   "),
            Span::styled("UP: ", Style::default().fg(palette.dim)),
            Span::styled(
                format_uptime(app.snapshot.uptime_secs),
                Style::default().fg(palette.fg),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(palette.dim)),
    );
    frame.render_widget(header, area);
}

fn link_style_for(status: crate::event::ConnectionStatus, palette: &ThemePalette) -> Style {
    use crate::event::ConnectionStatus::*;
    let color = match status {
        Online => palette.accent,
        Authenticating | Connecting => palette.warning,
        Offline | Disconnected => palette.critical,
    };
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

// ---------------------------------------------------------------------------
// HOME
// ---------------------------------------------------------------------------

fn render_home(frame: &mut Frame, area: Rect, app: &AppState, palette: &ThemePalette) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // CPU gauge
            Constraint::Length(3), // RAM gauge
            Constraint::Length(3), // DISK gauge
            Constraint::Min(0),    // recent events
        ])
        .split(area);

    let snap = &app.snapshot;
    render_gauge(
        frame,
        chunks[0],
        "CPU",
        snap.cpu.usage_percent,
        format_cpu_label(snap),
        palette,
    );

    let mem_pct = percent(snap.memory.used_bytes, snap.memory.total_bytes);
    let mem_label = format!(
        "{} / {}",
        format_bytes(snap.memory.used_bytes),
        format_bytes(snap.memory.total_bytes)
    );
    render_gauge(frame, chunks[1], "RAM", mem_pct, mem_label, palette);

    let (disk_pct, disk_label) = disk_summary(snap);
    render_gauge(frame, chunks[2], "DSK", disk_pct, disk_label, palette);

    render_event_list(frame, chunks[3], app, palette, 6, "RECENT EVENTS");
}

fn render_gauge(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    pct: f32,
    label: String,
    palette: &ThemePalette,
) {
    let ratio = (pct / 100.0).clamp(0.0, 1.0) as f64;
    let color = if pct >= 90.0 {
        palette.critical
    } else if pct >= 75.0 {
        palette.warning
    } else {
        palette.accent
    };
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.dim))
                .title(Span::styled(
                    format!(" {title} "),
                    Style::default().fg(palette.accent),
                )),
        )
        .gauge_style(Style::default().fg(color).bg(palette.bg))
        .ratio(ratio)
        .label(Span::styled(
            format!("{pct:>5.1}%  {label}"),
            Style::default().fg(palette.fg),
        ));
    frame.render_widget(gauge, area);
}

// ---------------------------------------------------------------------------
// SYS
// ---------------------------------------------------------------------------

fn render_sys(frame: &mut Frame, area: Rect, app: &AppState, palette: &ThemePalette) {
    let snap = &app.snapshot;
    let mut lines: Vec<Line> = Vec::new();

    let title_style = Style::default()
        .fg(palette.accent)
        .add_modifier(Modifier::BOLD);

    lines.push(Line::from(Span::styled("CPU", title_style)));
    lines.push(Line::from(vec![kv(
        "  usage",
        format!("{:.1}%", snap.cpu.usage_percent),
        palette,
    )]));
    lines.push(Line::from(vec![kv(
        "  temp",
        match snap.cpu.temperature_c {
            Some(t) => format!("{t:.1}°C"),
            None => "N/A".to_string(),
        },
        palette,
    )]));
    lines.push(Line::from(vec![kv(
        "  cores",
        match snap.cpu.core_count {
            Some(c) => c.to_string(),
            None => "N/A".to_string(),
        },
        palette,
    )]));
    lines.push(Line::raw(""));

    lines.push(Line::from(Span::styled("MEMORY", title_style)));
    lines.push(Line::from(vec![kv(
        "  used",
        format!(
            "{} / {} ({:.1}%)",
            format_bytes(snap.memory.used_bytes),
            format_bytes(snap.memory.total_bytes),
            percent(snap.memory.used_bytes, snap.memory.total_bytes),
        ),
        palette,
    )]));
    if let (Some(used), Some(total)) = (snap.memory.swap_used_bytes, snap.memory.swap_total_bytes) {
        if total > 0 {
            lines.push(Line::from(vec![kv(
                "  swap",
                format!(
                    "{} / {} ({:.1}%)",
                    format_bytes(used),
                    format_bytes(total),
                    percent(used, total)
                ),
                palette,
            )]));
        }
    }
    lines.push(Line::raw(""));

    lines.push(Line::from(Span::styled("DISKS", title_style)));
    if snap.disks.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no disks reported)",
            Style::default().fg(palette.dim),
        )));
    } else {
        for d in &snap.disks {
            lines.push(Line::from(vec![kv(
                &format!("  {}", d.mount),
                format!(
                    "{} / {} ({:.1}%)",
                    format_bytes(d.used_bytes),
                    format_bytes(d.total_bytes),
                    percent(d.used_bytes, d.total_bytes)
                ),
                palette,
            )]));
        }
    }
    lines.push(Line::raw(""));

    lines.push(Line::from(Span::styled("NETWORK", title_style)));
    if snap.network.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no interfaces reported)",
            Style::default().fg(palette.dim),
        )));
    } else {
        for n in &snap.network {
            lines.push(Line::from(vec![kv(
                &format!("  {}", n.interface),
                format!(
                    "↓ {}/s   ↑ {}/s",
                    format_bytes(n.rx_bytes_per_sec),
                    format_bytes(n.tx_bytes_per_sec)
                ),
                palette,
            )]));
        }
    }

    let body = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.dim))
            .title(Span::styled(
                " SYSTEM ",
                Style::default().fg(palette.accent),
            )),
    );
    frame.render_widget(body, area);
}

// ---------------------------------------------------------------------------
// LOG
// ---------------------------------------------------------------------------

fn render_log(frame: &mut Frame, area: Rect, app: &AppState, palette: &ThemePalette) {
    let cap = area.height.saturating_sub(2) as usize;
    render_event_list(frame, area, app, palette, cap, "EVENT LOG");
}

fn render_event_list(
    frame: &mut Frame,
    area: Rect,
    app: &AppState,
    palette: &ThemePalette,
    cap: usize,
    title: &str,
) {
    let items: Vec<ListItem> = app
        .events
        .iter()
        .take(cap.max(1))
        .map(|e| format_event(e, palette))
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.dim))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(palette.accent),
        ));

    if items.is_empty() {
        let p = Paragraph::new(Span::styled(
            "(no events yet)",
            Style::default().fg(palette.dim),
        ))
        .block(block);
        frame.render_widget(p, area);
    } else {
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
    }
}

fn format_event<'a>(e: &LogEntry, palette: &ThemePalette) -> ListItem<'a> {
    let color = match e.severity {
        LogSeverity::Info => palette.fg,
        LogSeverity::Warn => palette.warning,
        LogSeverity::Critical => palette.critical,
    };
    let time = format_hms(e.timestamp_unix_ms);
    ListItem::new(Line::from(vec![
        Span::styled(format!("{time}  "), Style::default().fg(palette.dim)),
        Span::styled(e.text.clone(), Style::default().fg(color)),
    ]))
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

fn render_status_bar(frame: &mut Frame, area: Rect, app: &AppState, palette: &ThemePalette) {
    let mut spans = Vec::new();
    for (i, s) in Screen::all().iter().enumerate() {
        let label = format!("[{}]{} ", i + 1, s.label());
        let style = if *s == app.screen {
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.dim)
        };
        spans.push(Span::styled(label, style));
    }
    let suffix = format!(
        "  [Tab]next  [C]theme:{}  [R]refresh  [Q]quit{}",
        app.theme.as_str(),
        if app.muted { "  [MUTED]" } else { "" }
    );
    spans.push(Span::styled(suffix, Style::default().fg(palette.dim)));
    let p = Paragraph::new(Line::from(spans));
    frame.render_widget(p, area);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn kv<'a>(label: &str, value: String, palette: &ThemePalette) -> Span<'a> {
    Span::styled(
        format!("{label:<12} {value}"),
        Style::default().fg(palette.fg),
    )
}

fn format_cpu_label(snap: &Snapshot) -> String {
    let temp = match snap.cpu.temperature_c {
        Some(t) => format!("  {t:.0}°C"),
        None => String::new(),
    };
    let cores = match snap.cpu.core_count {
        Some(c) => format!("  ({c} cores)"),
        None => String::new(),
    };
    format!("{temp}{cores}")
}

fn disk_summary(snap: &Snapshot) -> (f32, String) {
    let used: u64 = snap.disks.iter().map(|d| d.used_bytes).sum();
    let total: u64 = snap.disks.iter().map(|d| d.total_bytes).sum();
    let pct = percent(used, total);
    let label = if total == 0 {
        "no disks".into()
    } else {
        format!(
            "{} / {}  ({} mounts)",
            format_bytes(used),
            format_bytes(total),
            snap.disks.len()
        )
    };
    (pct, label)
}

fn percent(used: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (used as f64 / total as f64 * 100.0) as f32
    }
}

fn format_bytes(b: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T", "P"];
    let mut value = b as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", b, UNITS[0])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn format_uptime(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

fn format_clock_now() -> (String, String) {
    // Local-time clock from `std`. We use UTC for portability and label
    // accordingly in the header; rich local-time formatting lands with
    // the TIME screen in Milestone 6.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let days = secs / 86400;
    // 1970-01-01 -> day 0. Convert to civil date with Howard Hinnant's
    // algorithm so we don't need a date crate at the client.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let yyyy = y + if m <= 2 { 1 } else { 0 };
    let h = (secs / 3600) % 24;
    let mi = (secs / 60) % 60;
    let s = secs % 60;
    (
        format!("{yyyy:04}-{m:02}-{d:02}"),
        format!("{h:02}:{mi:02}:{s:02}Z"),
    )
}

fn format_hms(unix_ms: u64) -> String {
    let secs = unix_ms / 1000;
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_formatting() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.0 K");
        assert_eq!(format_bytes(1_048_576), "1.0 M");
    }

    #[test]
    fn percent_handles_zero() {
        assert_eq!(percent(0, 0), 0.0);
        assert_eq!(percent(50, 100), 50.0);
    }

    #[test]
    fn uptime_formats_hms() {
        assert_eq!(format_uptime(0), "00:00:00");
        assert_eq!(format_uptime(3661), "01:01:01");
    }
}
