//! Local-only readings the client makes about its own machine
//! (Milestone 7).
//!
//! Specifically the VAIO P's own battery and the Wi-Fi/IP it is using
//! to reach the server. These values never come from the workstation
//! over the wire — the client reads them directly so they survive a
//! disconnect (per README §13.2 "Offline usability: required").
//!
//! Linux-only by design. Other platforms simply yield `None`/empty
//! strings; the UI shows them as `N/A` rather than crashing.

use std::net::{IpAddr, UdpSocket};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct LocalInfo {
    pub battery: Option<BatteryInfo>,
    pub ip: Option<IpAddr>,
    pub iface: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct BatteryInfo {
    pub charge_percent: u8,
    pub status: BatteryStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    Unknown,
}

impl BatteryStatus {
    pub fn short_label(self) -> &'static str {
        match self {
            Self::Charging => "CHG",
            Self::Discharging => "DSC",
            Self::Full => "FUL",
            Self::Unknown => "---",
        }
    }
}

impl LocalInfo {
    /// Refresh battery and IP info. Cheap enough to call once a second
    /// — both reads are tiny syscalls.
    pub fn refresh() -> Self {
        Self {
            battery: read_battery(),
            ip: detect_local_ip(),
            iface: detect_default_iface(),
        }
    }
}

// ---------------------------------------------------------------------------
// Battery — reads /sys/class/power_supply/{BAT0,BAT1,…}
// ---------------------------------------------------------------------------

fn read_battery() -> Option<BatteryInfo> {
    let root = Path::new("/sys/class/power_supply");
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("BAT") {
            continue;
        }
        let cap = read_trim(path.join("capacity")).and_then(|s| s.parse::<u8>().ok())?;
        let raw_status = read_trim(path.join("status")).unwrap_or_default();
        let status = match raw_status.as_str() {
            "Charging" => BatteryStatus::Charging,
            "Discharging" => BatteryStatus::Discharging,
            "Full" | "Not charging" => BatteryStatus::Full,
            _ => BatteryStatus::Unknown,
        };
        return Some(BatteryInfo {
            charge_percent: cap.min(100),
            status,
        });
    }
    None
}

fn read_trim(path: impl AsRef<Path>) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

// ---------------------------------------------------------------------------
// IP — connect a UDP socket to a non-routable address to discover the
// outbound interface address without sending any packets.
// ---------------------------------------------------------------------------

fn detect_local_ip() -> Option<IpAddr> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.set_read_timeout(Some(Duration::from_millis(200))).ok();
    // 198.18.0.0/15 is a benchmarking range that the OS routes via the
    // default gateway but never actually transmits when used over UDP
    // before any send.
    sock.connect("198.18.0.1:9").ok()?;
    let addr = sock.local_addr().ok()?.ip();
    if addr.is_unspecified() {
        None
    } else {
        Some(addr)
    }
}

// ---------------------------------------------------------------------------
// Default interface — first up, non-loopback entry under /sys/class/net.
// We don't try to detect SSID; that needs nl80211 or shelling out to
// `iwgetid`, both of which violate "no heavy dependencies" for a piece
// of cosmetic information.
// ---------------------------------------------------------------------------

fn detect_default_iface() -> Option<String> {
    let entries = std::fs::read_dir("/sys/class/net").ok()?;
    let mut candidates = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "lo" {
            continue;
        }
        if let Some(state) = read_trim(entry.path().join("operstate")) {
            if state == "up" {
                candidates.push(name);
            }
        }
    }
    // Prefer wireless-looking names; otherwise return the first up iface.
    candidates
        .iter()
        .find(|n| n.starts_with("wl") || n.starts_with("wlan"))
        .cloned()
        .or_else(|| candidates.into_iter().next())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_does_not_panic_on_any_host() {
        // Smoke test: battery / iface may legitimately be None (e.g. a
        // CI container), but the call must always return a value.
        let info = LocalInfo::refresh();
        if let Some(b) = info.battery {
            assert!(b.charge_percent <= 100);
        }
    }

    #[test]
    fn battery_status_labels_are_three_chars() {
        for s in [
            BatteryStatus::Charging,
            BatteryStatus::Discharging,
            BatteryStatus::Full,
            BatteryStatus::Unknown,
        ] {
            assert_eq!(s.short_label().len(), 3);
        }
    }
}
