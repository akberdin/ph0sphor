use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("metric unavailable: {0}")]
    MetricUnavailable(String),

    #[error("protocol version mismatch: client={client}, server={server}")]
    ProtocolMismatch { client: u32, server: u32 },
}
