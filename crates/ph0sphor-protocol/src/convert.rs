//! Conversions between `ph0sphor-core` domain types and `wire` types.
//!
//! Keeping the two type families separate lets the wire format evolve
//! independently of the in-memory domain. Conversions are infallible in
//! both directions today; once the protocol grows fields that the domain
//! type cannot express, the reverse direction will become `TryFrom`.

use crate::wire;
use ph0sphor_core::{CpuMetrics, DiskMetrics, MemoryMetrics, NetworkMetrics, Snapshot};

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
        }
    }
}
