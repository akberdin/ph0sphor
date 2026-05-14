//! Token authentication stub.
//!
//! Milestone 2 scope: validate a client-supplied token against a static
//! allowlist loaded from config. Milestone 5 replaces this with a real
//! pairing flow that issues server-side tokens.

use crate::config::SecuritySection;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AuthConfig {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    require_token: bool,
    tokens: Vec<String>,
}

impl AuthConfig {
    pub fn from_security(sec: &SecuritySection) -> Self {
        Self {
            inner: Arc::new(Inner {
                require_token: sec.require_token,
                tokens: sec.tokens.clone(),
            }),
        }
    }

    /// Returns true when `presented` is accepted under the current policy.
    ///
    /// - `require_token = false`: any token, including empty, is accepted.
    /// - `require_token = true`: token must match an entry in the allowlist
    ///   in constant time (against each entry; allowlists are small).
    pub fn validate(&self, presented: &str) -> bool {
        if !self.inner.require_token {
            return true;
        }
        self.inner
            .tokens
            .iter()
            .any(|t| constant_time_eq(t.as_bytes(), presented.as_bytes()))
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_mode_accepts_anything() {
        let cfg = AuthConfig::from_security(&SecuritySection {
            require_token: false,
            tokens: vec![],
            ..SecuritySection::default()
        });
        assert!(cfg.validate(""));
        assert!(cfg.validate("anything"));
    }

    #[test]
    fn on_mode_enforces_allowlist() {
        let cfg = AuthConfig::from_security(&SecuritySection {
            require_token: true,
            tokens: vec!["sekret".into()],
            ..SecuritySection::default()
        });
        assert!(!cfg.validate(""));
        assert!(!cfg.validate("nope"));
        assert!(cfg.validate("sekret"));
    }
}
