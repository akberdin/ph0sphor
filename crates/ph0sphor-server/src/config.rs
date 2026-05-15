//! Server configuration loading.
//!
//! Mirrors the schema documented in README §18.1. Unknown keys are
//! ignored (per §26.5 compatibility rule). Missing sections fall back to
//! conservative defaults that satisfy the performance budget.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub server: ServerSection,
    pub security: SecuritySection,
    pub performance: PerformanceSection,
    pub collectors: CollectorsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerSection {
    pub bind: String,
    /// Loopback-only HTTP control endpoint. Used by `ph0sphorctl` to
    /// confirm pairing codes. The handler explicitly rejects non-loopback
    /// peers (see `control::pair_confirm`).
    pub control_bind: String,
    pub name: String,
    pub protocol: String,
    pub debug_json: bool,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:7077".to_string(),
            control_bind: "127.0.0.1:7078".to_string(),
            name: "phosphor".to_string(),
            protocol: "protobuf".to_string(),
            debug_json: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SecuritySection {
    pub pairing_enabled: bool,
    pub require_token: bool,
    /// Remote command execution. Always disabled in MVP per README §14.2.
    /// Even setting this to `true` does not enable arbitrary shell
    /// commands; the server has no `ClientCommandRequest` handler.
    pub allow_control_commands: bool,
    /// Static, manually-administered token allowlist. Useful for testing
    /// and headless deployments. Production deployments should rely on
    /// `token_store` instead.
    pub tokens: Vec<String>,
    /// Optional path to a JSON file holding server-issued tokens
    /// produced by pairing. Loaded on startup; appended to on each
    /// successful pairing. Tokens in this file are merged with `tokens`
    /// for validation.
    pub token_store: Option<String>,
    /// Pairing-code time-to-live in seconds. Defaults to 300 (5 min).
    #[serde(default = "default_pairing_ttl_secs")]
    pub pairing_ttl_secs: u64,
}

fn default_pairing_ttl_secs() -> u64 {
    300
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PerformanceSection {
    pub main_tick_ms: u64,
    pub send_deltas_only: bool,
    pub full_snapshot_interval_sec: u64,
    pub max_events_in_memory: usize,
    /// Minimum interval between successive payloads to a single
    /// client. State updates that arrive faster are coalesced so the
    /// client receives at most one payload per `min_send_interval_ms`
    /// (README §13.2: keep network usage low, avoid constant redraws).
    #[serde(default = "default_min_send_interval_ms")]
    pub min_send_interval_ms: u64,
}

fn default_min_send_interval_ms() -> u64 {
    500
}

impl Default for PerformanceSection {
    fn default() -> Self {
        Self {
            main_tick_ms: 1000,
            send_deltas_only: true,
            full_snapshot_interval_sec: 60,
            max_events_in_memory: 200,
            min_send_interval_ms: default_min_send_interval_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CollectorsSection {
    pub cpu: PeriodicCollector,
    pub memory: PeriodicCollector,
    pub network: PeriodicCollector,
    pub disk: PeriodicCollector,
}

impl Default for CollectorsSection {
    fn default() -> Self {
        Self {
            cpu: PeriodicCollector::ms(1000),
            memory: PeriodicCollector::ms(1000),
            network: PeriodicCollector::ms(1000),
            disk: PeriodicCollector::ms(15_000),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PeriodicCollector {
    pub enabled: bool,
    /// Interval in milliseconds. The README config uses `interval_ms` for
    /// fast collectors and `interval_sec` for slow ones; we normalize to
    /// milliseconds internally and accept both via aliases.
    #[serde(alias = "interval_ms")]
    pub interval_ms: u64,
    #[serde(default, alias = "interval_sec", skip_serializing)]
    interval_sec: Option<u64>,
}

impl Default for PeriodicCollector {
    fn default() -> Self {
        Self::ms(1000)
    }
}

impl PeriodicCollector {
    pub const fn ms(interval_ms: u64) -> Self {
        Self {
            enabled: true,
            interval_ms,
            interval_sec: None,
        }
    }

    /// Effective interval. Prefers `interval_sec` when set (matches the
    /// README example, which mixes the two for readability).
    pub fn interval(&self) -> std::time::Duration {
        let ms = self
            .interval_sec
            .map(|s| s.saturating_mul(1000))
            .unwrap_or(self.interval_ms);
        std::time::Duration::from_millis(ms.max(50))
    }
}

impl ServerConfig {
    /// Load from a TOML file at `path`.
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path_str = path.as_ref().display().to_string();
        let raw = std::fs::read_to_string(&path).map_err(|source| ConfigError::Read {
            path: path_str,
            source,
        })?;
        let cfg: Self = toml::from_str(&raw)?;
        Ok(cfg)
    }

    /// A loopback-bound config used by `--demo` and integration tests.
    pub fn demo() -> Self {
        let mut cfg = Self::default();
        cfg.server.bind = "127.0.0.1:0".to_string();
        cfg.server.name = "phosphor-demo".to_string();
        cfg.security.require_token = false;
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_round_trips_through_toml() {
        let cfg = ServerConfig::default();
        let s = toml::to_string(&cfg).unwrap();
        let back: ServerConfig = toml::from_str(&s).unwrap();
        assert_eq!(back.server.bind, cfg.server.bind);
        assert_eq!(
            back.performance.full_snapshot_interval_sec,
            cfg.performance.full_snapshot_interval_sec
        );
    }

    #[test]
    fn readme_example_parses() {
        let raw = include_str!("../../../examples/server.toml");
        let cfg: ServerConfig = toml::from_str(raw).expect("parse example config");
        assert_eq!(cfg.server.name, "main-pc");
        assert!(cfg.security.pairing_enabled);
        assert!(!cfg.security.allow_control_commands);
        // The example uses interval_sec for disk; ensure aliasing works.
        assert_eq!(cfg.collectors.disk.interval().as_secs(), 15);
    }
}
