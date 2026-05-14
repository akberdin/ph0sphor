use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuMetrics {
    pub usage_percent: f32,
    pub temperature_c: Option<f32>,
    pub core_count: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub swap_used_bytes: Option<u64>,
    pub swap_total_bytes: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub mount: String,
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub temperature_c: Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub interface: String,
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
    pub rx_total_bytes: Option<u64>,
    pub tx_total_bytes: Option<u64>,
}

/// A full telemetry snapshot. Wire encoding lives in `ph0sphor-protocol`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub timestamp_unix_ms: u64,
    pub hostname: String,
    pub os: String,
    pub uptime_secs: u64,
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub disks: Vec<DiskMetrics>,
    pub network: Vec<NetworkMetrics>,
}
