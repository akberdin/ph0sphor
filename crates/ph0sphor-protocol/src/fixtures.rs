//! Realistic protocol fixtures.
//!
//! Used by:
//! - protocol crate tests (round-trip encoding),
//! - the eventual demo data generator (`ph0sphorctl gen-demo`),
//! - server/client integration tests once those land.

use crate::wire;
use ph0sphor_core::{
    CpuMetrics, DiskMetrics, MailItem, MailPrivacy, MailSummary, MemoryMetrics, NetworkMetrics,
    Snapshot, WeatherInfo, PROTOCOL_VERSION,
};

/// A non-trivial telemetry snapshot meant to exercise every field that
/// might appear in a realistic FullSnapshot.
pub fn sample_domain_snapshot() -> Snapshot {
    Snapshot {
        timestamp_unix_ms: 1_715_700_000_000,
        hostname: "main-pc".to_string(),
        os: "Linux 6.18.5".to_string(),
        uptime_secs: 73_412,
        cpu: CpuMetrics {
            usage_percent: 61.4,
            temperature_c: Some(62.0),
            core_count: Some(16),
        },
        memory: MemoryMetrics {
            used_bytes: 17_179_869_184,  // 16 GiB used
            total_bytes: 33_285_996_544, // ~31 GiB total
            swap_used_bytes: Some(0),
            swap_total_bytes: Some(8_589_934_592),
        },
        disks: vec![
            DiskMetrics {
                mount: "/".to_string(),
                used_bytes: 540_000_000_000,
                total_bytes: 1_000_000_000_000,
                temperature_c: Some(41.0),
            },
            DiskMetrics {
                mount: "/data".to_string(),
                used_bytes: 1_400_000_000_000,
                total_bytes: 4_000_000_000_000,
                temperature_c: None,
            },
        ],
        network: vec![NetworkMetrics {
            interface: "eth0".to_string(),
            rx_bytes_per_sec: 12_345,
            tx_bytes_per_sec: 6_789,
            rx_total_bytes: Some(9_876_543_210),
            tx_total_bytes: Some(1_234_567_890),
        }],
        mail: Some(MailSummary {
            unread_count: 3,
            privacy: MailPrivacy::SenderSubject,
            recent: vec![MailItem {
                sender: "ops@example.com".into(),
                subject: "Backup completed".into(),
                preview: String::new(),
                timestamp_unix_ms: 1_715_699_500_000,
                account: "personal".into(),
            }],
            last_update_unix_ms: 1_715_700_000_000,
        }),
        weather: Some(WeatherInfo {
            temperature_c: 17.0,
            feels_like_c: Some(15.5),
            condition: "cloudy".into(),
            humidity_percent: Some(72.0),
            wind_kph: Some(11.0),
            short_forecast: "Cloudy with a chance of rain".into(),
            last_update_unix_ms: 1_715_700_000_000,
            location: "Saint Petersburg".into(),
        }),
    }
}

/// Build a `FullSnapshot` wire payload from the sample domain snapshot.
pub fn sample_full_snapshot() -> wire::FullSnapshot {
    (&sample_domain_snapshot()).into()
}

/// A small DeltaUpdate carrying only changed CPU usage + temperature.
pub fn sample_delta_update() -> wire::DeltaUpdate {
    wire::DeltaUpdate {
        timestamp_unix_ms: 1_715_700_001_000,
        cpu_usage_percent: Some(63.1),
        cpu_temperature_c: Some(62.5),
        ..Default::default()
    }
}

/// A realistic Event (new mail arrived).
pub fn sample_event_new_mail() -> wire::Event {
    let mut attributes = std::collections::HashMap::new();
    attributes.insert("account".to_string(), "personal".to_string());
    attributes.insert("count".to_string(), "3".to_string());
    wire::Event {
        timestamp_unix_ms: 1_715_700_002_000,
        severity: wire::Severity::Info as i32,
        kind: "new_mail".to_string(),
        message: "3 new messages on personal".to_string(),
        attributes,
    }
}

/// Hello envelope carrying the current protocol version.
pub fn sample_hello_envelope() -> wire::Envelope {
    crate::envelope(crate::Payload::Hello(wire::Hello {
        client_id: "vaio-p".to_string(),
        client_version: "0.0.1".to_string(),
    }))
}

/// Envelope wrapping the sample FullSnapshot, version-stamped.
pub fn sample_snapshot_envelope() -> wire::Envelope {
    crate::envelope(crate::Payload::FullSnapshot(sample_full_snapshot()))
}

/// Envelope wrapping a DeltaUpdate.
pub fn sample_delta_envelope() -> wire::Envelope {
    crate::envelope(crate::Payload::DeltaUpdate(sample_delta_update()))
}

/// Envelope wrapping a new-mail Event.
pub fn sample_event_envelope() -> wire::Envelope {
    crate::envelope(crate::Payload::Event(sample_event_new_mail()))
}

/// Convenience: the version the fixtures assume. Tests that decode an
/// envelope built by these helpers should expect this version.
pub const FIXTURE_PROTOCOL_VERSION: u32 = PROTOCOL_VERSION;
