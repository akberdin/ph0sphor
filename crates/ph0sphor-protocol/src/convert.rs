//! Conversions between `ph0sphor-core` domain types and `wire` types.
//!
//! Keeping the two type families separate lets the wire format evolve
//! independently of the in-memory domain. Conversions are infallible in
//! both directions today; once the protocol grows fields that the domain
//! type cannot express, the reverse direction will become `TryFrom`.

use crate::wire;
use ph0sphor_core::{
    CpuMetrics, DiskMetrics, MailItem, MailPrivacy, MailSummary, MemoryMetrics, NetworkMetrics,
    Snapshot, WeatherInfo,
};

// ---------- domain -> wire ----------

impl From<&CpuMetrics> for wire::CpuMetrics {
    fn from(d: &CpuMetrics) -> Self {
        Self {
            usage_percent: d.usage_percent,
            temperature_c: d.temperature_c,
            core_count: d.core_count,
        }
    }
}

impl From<&MemoryMetrics> for wire::MemoryMetrics {
    fn from(d: &MemoryMetrics) -> Self {
        Self {
            used_bytes: d.used_bytes,
            total_bytes: d.total_bytes,
            swap_used_bytes: d.swap_used_bytes,
            swap_total_bytes: d.swap_total_bytes,
        }
    }
}

impl From<&DiskMetrics> for wire::DiskMetrics {
    fn from(d: &DiskMetrics) -> Self {
        Self {
            mount: d.mount.clone(),
            used_bytes: d.used_bytes,
            total_bytes: d.total_bytes,
            temperature_c: d.temperature_c,
        }
    }
}

impl From<&NetworkMetrics> for wire::NetworkMetrics {
    fn from(d: &NetworkMetrics) -> Self {
        Self {
            iface: d.interface.clone(),
            rx_bytes_per_sec: d.rx_bytes_per_sec,
            tx_bytes_per_sec: d.tx_bytes_per_sec,
            rx_total_bytes: d.rx_total_bytes,
            tx_total_bytes: d.tx_total_bytes,
        }
    }
}

impl From<&Snapshot> for wire::FullSnapshot {
    fn from(d: &Snapshot) -> Self {
        Self {
            timestamp_unix_ms: d.timestamp_unix_ms,
            hostname: d.hostname.clone(),
            os: d.os.clone(),
            uptime_secs: d.uptime_secs,
            cpu: Some((&d.cpu).into()),
            memory: Some((&d.memory).into()),
            disks: d.disks.iter().map(Into::into).collect(),
            network: d.network.iter().map(Into::into).collect(),
            mail: d.mail.as_ref().map(Into::into),
            weather: d.weather.as_ref().map(Into::into),
        }
    }
}

// ---------- Mail ----------

/// Encode a [`MailPrivacy`] as the proto3 enum integer. Used directly
/// where the wire type stores the raw `i32`.
pub fn mail_privacy_as_wire_i32(p: MailPrivacy) -> i32 {
    wire_mail_privacy_from(p) as i32
}

fn wire_mail_privacy_from(p: MailPrivacy) -> wire::MailPrivacy {
    match p {
        MailPrivacy::CountOnly => wire::MailPrivacy::CountOnly,
        MailPrivacy::SenderSubject => wire::MailPrivacy::SenderSubject,
        MailPrivacy::Preview => wire::MailPrivacy::Preview,
    }
}

impl From<wire::MailPrivacy> for MailPrivacy {
    fn from(p: wire::MailPrivacy) -> Self {
        match p {
            wire::MailPrivacy::Preview => MailPrivacy::Preview,
            wire::MailPrivacy::SenderSubject => MailPrivacy::SenderSubject,
            // Treat UNSPECIFIED as the most conservative option.
            _ => MailPrivacy::CountOnly,
        }
    }
}

impl From<&MailItem> for wire::MailItem {
    fn from(d: &MailItem) -> Self {
        Self {
            sender: d.sender.clone(),
            subject: d.subject.clone(),
            preview: d.preview.clone(),
            timestamp_unix_ms: d.timestamp_unix_ms,
            account: d.account.clone(),
        }
    }
}

impl From<&wire::MailItem> for MailItem {
    fn from(w: &wire::MailItem) -> Self {
        Self {
            sender: w.sender.clone(),
            subject: w.subject.clone(),
            preview: w.preview.clone(),
            timestamp_unix_ms: w.timestamp_unix_ms,
            account: w.account.clone(),
        }
    }
}

impl From<&MailSummary> for wire::MailSummary {
    fn from(d: &MailSummary) -> Self {
        Self {
            unread_count: d.unread_count,
            privacy: mail_privacy_as_wire_i32(d.privacy),
            recent: d.recent.iter().map(Into::into).collect(),
            last_update_unix_ms: d.last_update_unix_ms,
        }
    }
}

impl From<&wire::MailSummary> for MailSummary {
    fn from(w: &wire::MailSummary) -> Self {
        Self {
            unread_count: w.unread_count,
            privacy: wire::MailPrivacy::try_from(w.privacy)
                .unwrap_or(wire::MailPrivacy::Unspecified)
                .into(),
            recent: w.recent.iter().map(Into::into).collect(),
            last_update_unix_ms: w.last_update_unix_ms,
        }
    }
}

// ---------- Weather ----------

impl From<&WeatherInfo> for wire::WeatherInfo {
    fn from(d: &WeatherInfo) -> Self {
        Self {
            temperature_c: d.temperature_c,
            feels_like_c: d.feels_like_c,
            condition: d.condition.clone(),
            humidity_percent: d.humidity_percent,
            wind_kph: d.wind_kph,
            short_forecast: d.short_forecast.clone(),
            last_update_unix_ms: d.last_update_unix_ms,
            location: d.location.clone(),
        }
    }
}

impl From<&wire::WeatherInfo> for WeatherInfo {
    fn from(w: &wire::WeatherInfo) -> Self {
        Self {
            temperature_c: w.temperature_c,
            feels_like_c: w.feels_like_c,
            condition: w.condition.clone(),
            humidity_percent: w.humidity_percent,
            wind_kph: w.wind_kph,
            short_forecast: w.short_forecast.clone(),
            last_update_unix_ms: w.last_update_unix_ms,
            location: w.location.clone(),
        }
    }
}

// ---------- wire -> domain ----------

impl From<&wire::CpuMetrics> for CpuMetrics {
    fn from(w: &wire::CpuMetrics) -> Self {
        Self {
            usage_percent: w.usage_percent,
            temperature_c: w.temperature_c,
            core_count: w.core_count,
        }
    }
}

impl From<&wire::MemoryMetrics> for MemoryMetrics {
    fn from(w: &wire::MemoryMetrics) -> Self {
        Self {
            used_bytes: w.used_bytes,
            total_bytes: w.total_bytes,
            swap_used_bytes: w.swap_used_bytes,
            swap_total_bytes: w.swap_total_bytes,
        }
    }
}

impl From<&wire::DiskMetrics> for DiskMetrics {
    fn from(w: &wire::DiskMetrics) -> Self {
        Self {
            mount: w.mount.clone(),
            used_bytes: w.used_bytes,
            total_bytes: w.total_bytes,
            temperature_c: w.temperature_c,
        }
    }
}

impl From<&wire::NetworkMetrics> for NetworkMetrics {
    fn from(w: &wire::NetworkMetrics) -> Self {
        Self {
            interface: w.iface.clone(),
            rx_bytes_per_sec: w.rx_bytes_per_sec,
            tx_bytes_per_sec: w.tx_bytes_per_sec,
            rx_total_bytes: w.rx_total_bytes,
            tx_total_bytes: w.tx_total_bytes,
        }
    }
}

impl From<&wire::FullSnapshot> for Snapshot {
    fn from(w: &wire::FullSnapshot) -> Self {
        Self {
            timestamp_unix_ms: w.timestamp_unix_ms,
            hostname: w.hostname.clone(),
            os: w.os.clone(),
            uptime_secs: w.uptime_secs,
            cpu: w.cpu.as_ref().map(Into::into).unwrap_or_default(),
            memory: w.memory.as_ref().map(Into::into).unwrap_or_default(),
            disks: w.disks.iter().map(Into::into).collect(),
            network: w.network.iter().map(Into::into).collect(),
            mail: w.mail.as_ref().map(Into::into),
            weather: w.weather.as_ref().map(Into::into),
        }
    }
}
