use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CpuMetrics {
    pub usage_percent: f32,
    pub temperature_c: Option<f32>,
    pub core_count: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub swap_used_bytes: Option<u64>,
    pub swap_total_bytes: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub mount: String,
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub temperature_c: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub interface: String,
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
    pub rx_total_bytes: Option<u64>,
    pub tx_total_bytes: Option<u64>,
}

/// What the server is permitted to put on the wire about mail.
/// Mirrors `MailPrivacy` in the wire schema and README §14.5.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MailPrivacy {
    #[default]
    CountOnly,
    SenderSubject,
    Preview,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MailItem {
    /// Empty under [`MailPrivacy::CountOnly`].
    pub sender: String,
    /// Empty under [`MailPrivacy::CountOnly`].
    pub subject: String,
    /// Non-empty only under [`MailPrivacy::Preview`].
    pub preview: String,
    pub timestamp_unix_ms: u64,
    pub account: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MailSummary {
    pub unread_count: u32,
    pub privacy: MailPrivacy,
    pub recent: Vec<MailItem>,
    pub last_update_unix_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WeatherInfo {
    pub temperature_c: f32,
    pub feels_like_c: Option<f32>,
    pub condition: String,
    pub humidity_percent: Option<f32>,
    pub wind_kph: Option<f32>,
    pub short_forecast: String,
    pub last_update_unix_ms: u64,
    pub location: String,
}

/// A full telemetry snapshot. Wire encoding lives in `ph0sphor-protocol`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub timestamp_unix_ms: u64,
    pub hostname: String,
    pub os: String,
    pub uptime_secs: u64,
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub disks: Vec<DiskMetrics>,
    pub network: Vec<NetworkMetrics>,
    pub mail: Option<MailSummary>,
    pub weather: Option<WeatherInfo>,
}
