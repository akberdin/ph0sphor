use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warn,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventKind {
    NewMail { count: u32 },
    ThresholdCrossed { metric: String, value: f64 },
    CollectorFailed { name: String, reason: String },
    CollectorRecovered { name: String },
    ClientReconnected,
    TimerCompleted { label: String },
    AlarmTriggered { label: String },
    Custom { tag: String, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub timestamp_unix_ms: u64,
    pub severity: Severity,
    pub kind: EventKind,
}
