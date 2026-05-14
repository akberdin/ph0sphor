//! Client configuration. Mirrors README §18.2.

use crate::theme::Theme;
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
pub struct ClientConfig {
    pub client: ClientSection,
    pub ui: UiSection,
    pub cache: CacheSection,
    pub keys: KeysSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClientSection {
    pub server: String,
    pub client_name: String,
    /// Optional bearer token presented at handshake. Empty means
    /// "anonymous" — works only when the server is configured with
    /// `require_token = false`.
    pub token: String,
    pub theme: Theme,
    pub render_fps: u32,
    pub low_power_mode: bool,
}

impl Default for ClientSection {
    fn default() -> Self {
        Self {
            server: "ws://127.0.0.1:7077/ws".into(),
            client_name: "vaio-p".into(),
            token: String::new(),
            theme: Theme::PhosphorGreen,
            render_fps: 1,
            low_power_mode: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSection {
    pub default_screen: String,
    pub show_scanlines: bool,
    pub ascii_fallback: bool,
    pub compact_mode: bool,
}

impl Default for UiSection {
    fn default() -> Self {
        Self {
            default_screen: "home".into(),
            show_scanlines: false,
            ascii_fallback: true,
            compact_mode: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheSection {
    pub store_last_snapshot: bool,
    pub max_cached_events: usize,
}

impl Default for CacheSection {
    fn default() -> Self {
        Self {
            store_last_snapshot: true,
            max_cached_events: 100,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct KeysSection {
    pub next_screen: Option<String>,
    pub prev_screen: Option<String>,
    pub theme_cycle: Option<String>,
    pub mute: Option<String>,
    pub refresh: Option<String>,
    pub quit: Option<String>,
}

impl ClientConfig {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path_str = path.as_ref().display().to_string();
        let raw = std::fs::read_to_string(&path).map_err(|source| ConfigError::Read {
            path: path_str,
            source,
        })?;
        Ok(toml::from_str(&raw)?)
    }

    /// Demo profile: no server reachable, theme defaulted.
    pub fn demo() -> Self {
        let mut cfg = Self::default();
        cfg.client.server = "ws://127.0.0.1:0/ws".into();
        cfg.client.client_name = "vaio-p-demo".into();
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readme_example_parses() {
        let raw = include_str!("../../../examples/client.toml");
        let cfg: ClientConfig = toml::from_str(raw).expect("parse example config");
        assert_eq!(cfg.client.client_name, "vaio-p");
        assert_eq!(cfg.client.theme, Theme::PhosphorGreen);
        assert!(cfg.ui.compact_mode);
    }
}
