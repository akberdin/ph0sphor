//! Authentication, pairing and token persistence.
//!
//! Milestone 5 lifts auth from a static-allowlist stub into a real
//! pairing flow. Three moving parts:
//!
//! - [`TokenStore`] — persisted JSON file of `{ client_id, token, paired_at }`
//!   records. Tokens here are merged with the static `security.tokens`
//!   list when validating connections.
//! - [`PairingManager`] — in-memory map of outstanding pairing codes
//!   awaiting operator confirmation. Each entry carries a `oneshot`
//!   channel so the WS session blocked on `recv_token()` is woken the
//!   instant `ph0sphorctl pair confirm <code>` lands.
//! - [`AuthConfig`] — façade that the WS session uses. Wraps both of
//!   the above and applies the constant-time token compare used since
//!   Milestone 2.

use crate::config::SecuritySection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use thiserror::Error;
use tokio::sync::oneshot;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// Redaction helpers
// ---------------------------------------------------------------------------

/// Returns a redacted prefix of a token, suitable for logs and TUI.
///
/// `"a-very-secret-token"` becomes `"a-ve…"`. Empty tokens become
/// `"<empty>"`. Never put the raw token through any `tracing` macro;
/// always pass it through this function first.
pub fn redact_token(token: &str) -> String {
    if token.is_empty() {
        return "<empty>".to_string();
    }
    let prefix: String = token.chars().take(4).collect();
    format!("{prefix}…")
}

// ---------------------------------------------------------------------------
// Persisted token store
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub client_id: String,
    pub token: String,
    pub paired_at_unix_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenStoreFile {
    #[serde(default)]
    pub tokens: Vec<StoredToken>,
}

#[derive(Debug, Clone)]
pub struct TokenStore {
    inner: Arc<Mutex<TokenStoreInner>>,
}

#[derive(Debug)]
struct TokenStoreInner {
    path: Option<PathBuf>,
    tokens: Vec<StoredToken>,
}

impl TokenStore {
    /// Empty in-memory store; tokens are never persisted.
    pub fn in_memory() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TokenStoreInner {
                path: None,
                tokens: Vec::new(),
            })),
        }
    }

    /// Load (or create) a JSON token store at `path`. Missing files
    /// yield an empty store; malformed files surface as `AuthError`.
    pub fn load_or_create(path: impl AsRef<Path>) -> Result<Self, AuthError> {
        let p = path.as_ref().to_path_buf();
        let tokens = if p.exists() {
            let raw = std::fs::read_to_string(&p)?;
            let file: TokenStoreFile = serde_json::from_str(&raw)?;
            file.tokens
        } else {
            Vec::new()
        };
        info!(path = %p.display(), count = tokens.len(), "token store loaded");
        Ok(Self {
            inner: Arc::new(Mutex::new(TokenStoreInner {
                path: Some(p),
                tokens,
            })),
        })
    }

    pub fn issue(&self, client_id: &str) -> Result<StoredToken, AuthError> {
        let token = generate_token();
        let stored = StoredToken {
            client_id: client_id.to_string(),
            token,
            paired_at_unix_ms: now_unix_ms(),
        };
        let mut inner = self.inner.lock().expect("token store poisoned");
        inner.tokens.push(stored.clone());
        if let Some(path) = inner.path.clone() {
            persist(&path, &inner.tokens)?;
        }
        info!(
            client_id = %stored.client_id,
            token = %redact_token(&stored.token),
            "paired client",
        );
        Ok(stored)
    }

    pub fn contains(&self, token: &str) -> bool {
        let inner = self.inner.lock().expect("token store poisoned");
        inner
            .tokens
            .iter()
            .any(|t| constant_time_eq(t.token.as_bytes(), token.as_bytes()))
    }

    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .expect("token store poisoned")
            .tokens
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn persist(path: &Path, tokens: &[StoredToken]) -> Result<(), AuthError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let raw = serde_json::to_string_pretty(&TokenStoreFile {
        tokens: tokens.to_vec(),
    })?;
    // Best-effort: write to a tmp file and rename for atomicity.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, raw)?;
    std::fs::rename(&tmp, path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Pairing manager
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct PendingPairing {
    client_id: String,
    issued_at: Instant,
    sender: oneshot::Sender<StoredToken>,
}

#[derive(Debug, Clone)]
pub struct PairingManager {
    inner: Arc<Mutex<HashMap<String, PendingPairing>>>,
    ttl: Duration,
    store: TokenStore,
}

impl PairingManager {
    pub fn new(store: TokenStore, ttl: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            ttl,
            store,
        }
    }

    /// Register a pairing request and return the human-readable code
    /// plus a receiver that fires when the operator confirms it.
    pub fn request(&self, client_id: &str) -> (String, oneshot::Receiver<StoredToken>) {
        let code = generate_pairing_code();
        let (tx, rx) = oneshot::channel();
        let mut map = self.inner.lock().expect("pairing map poisoned");
        // Expire stale entries opportunistically.
        let ttl = self.ttl;
        map.retain(|_, v| v.issued_at.elapsed() < ttl);
        map.insert(
            code.clone(),
            PendingPairing {
                client_id: client_id.to_string(),
                issued_at: Instant::now(),
                sender: tx,
            },
        );
        info!(
            code = %code,
            client_id = %client_id,
            "pairing code issued (awaiting operator confirmation)",
        );
        (code, rx)
    }

    /// Confirm a pairing code. Returns the issued [`StoredToken`] on
    /// success and `None` if the code is unknown or expired. Issues
    /// the token through the store and notifies the waiting session.
    pub fn confirm(&self, code: &str) -> Option<StoredToken> {
        let pending = {
            let mut map = self.inner.lock().expect("pairing map poisoned");
            map.remove(code)
        }?;
        if pending.issued_at.elapsed() >= self.ttl {
            warn!(code = %code, "pairing code expired");
            return None;
        }
        let stored = match self.store.issue(&pending.client_id) {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "failed to persist issued token");
                return None;
            }
        };
        let _ = pending.sender.send(stored.clone());
        Some(stored)
    }
}

// ---------------------------------------------------------------------------
// AuthConfig — façade used by the WS session
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AuthConfig {
    inner: Arc<AuthInner>,
}

#[derive(Debug)]
struct AuthInner {
    require_token: bool,
    pairing_enabled: bool,
    static_tokens: Vec<String>,
    store: TokenStore,
    pairing: PairingManager,
}

impl AuthConfig {
    /// Build an [`AuthConfig`] from the [`SecuritySection`] config and
    /// an already-loaded [`TokenStore`].
    pub fn build(sec: &SecuritySection, store: TokenStore) -> Self {
        let ttl = Duration::from_secs(sec.pairing_ttl_secs.max(30));
        let pairing = PairingManager::new(store.clone(), ttl);
        Self {
            inner: Arc::new(AuthInner {
                require_token: sec.require_token,
                pairing_enabled: sec.pairing_enabled,
                static_tokens: sec.tokens.clone(),
                store,
                pairing,
            }),
        }
    }

    /// Convenience for tests and the M2-era API: build with an empty
    /// in-memory store. Static `security.tokens` still apply.
    pub fn from_security(sec: &SecuritySection) -> Self {
        Self::build(sec, TokenStore::in_memory())
    }

    pub fn require_token(&self) -> bool {
        self.inner.require_token
    }

    pub fn pairing_enabled(&self) -> bool {
        self.inner.pairing_enabled
    }

    pub fn pairing(&self) -> &PairingManager {
        &self.inner.pairing
    }

    pub fn store(&self) -> &TokenStore {
        &self.inner.store
    }

    /// Validate a presented client token under the current policy.
    ///
    /// - `require_token = false`: any token, including empty, accepted.
    /// - `require_token = true`: must match a static config entry
    ///   **or** a persisted, server-issued token from the store. Compared
    ///   in constant time against each candidate.
    pub fn validate(&self, presented: &str) -> bool {
        if !self.inner.require_token {
            return true;
        }
        let bytes = presented.as_bytes();
        if self
            .inner
            .static_tokens
            .iter()
            .any(|t| constant_time_eq(t.as_bytes(), bytes))
        {
            return true;
        }
        self.inner.store.contains(presented)
    }
}

// ---------------------------------------------------------------------------
// Crypto-ish helpers (no external dep)
// ---------------------------------------------------------------------------

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

/// Generates a human-friendly 8-char pairing code in the form
/// `ABCD-1234`. Uses the OS RNG via `/dev/urandom` on Unix and a hash
/// of the current time elsewhere as a last-ditch fallback. For
/// pairing-code entropy this is sufficient — the codes are short-lived
/// (5 min default), single-use, and the operator confirms each one out
/// of band.
fn generate_pairing_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // no O/0/I/1
    let mut bytes = [0u8; 8];
    fill_random(&mut bytes);
    let s: String = bytes
        .iter()
        .map(|b| ALPHABET[(*b as usize) % ALPHABET.len()] as char)
        .collect();
    format!("{}-{}", &s[..4], &s[4..])
}

/// Generates a 32-character (192-bit) random hex token suitable for
/// long-lived client authentication.
fn generate_token() -> String {
    let mut bytes = [0u8; 24];
    fill_random(&mut bytes);
    hex_encode(&bytes)
}

fn fill_random(buf: &mut [u8]) {
    #[cfg(unix)]
    {
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            if f.read_exact(buf).is_ok() {
                return;
            }
        }
    }
    // Fallback: time-based LCG. Not cryptographically secure; we only
    // hit this path when /dev/urandom is unavailable, which should be
    // never in real deployments.
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xdead_beef);
    let mut state = seed;
    for b in buf.iter_mut() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        *b = (state >> 56) as u8;
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    fn sec(require_token: bool, tokens: Vec<&str>) -> SecuritySection {
        SecuritySection {
            require_token,
            tokens: tokens.into_iter().map(String::from).collect(),
            ..SecuritySection::default()
        }
    }

    #[test]
    fn redact_obscures_token() {
        assert_eq!(redact_token(""), "<empty>");
        let r = redact_token("super-secret-token");
        assert!(r.starts_with("supe"));
        assert!(!r.contains("secret"));
    }

    #[test]
    fn off_mode_accepts_anything() {
        let cfg = AuthConfig::from_security(&sec(false, vec![]));
        assert!(cfg.validate(""));
        assert!(cfg.validate("anything"));
    }

    #[test]
    fn on_mode_enforces_static_allowlist() {
        let cfg = AuthConfig::from_security(&sec(true, vec!["sekret"]));
        assert!(!cfg.validate(""));
        assert!(!cfg.validate("nope"));
        assert!(cfg.validate("sekret"));
    }

    #[test]
    fn pairing_request_then_confirm_yields_a_token_in_the_store() {
        let store = TokenStore::in_memory();
        let auth = AuthConfig::build(
            &SecuritySection {
                require_token: true,
                pairing_enabled: true,
                ..SecuritySection::default()
            },
            store.clone(),
        );
        let (code, mut rx) = auth.pairing().request("vaio-p");
        // Token not yet issued.
        assert!(store.is_empty());
        // Confirm.
        let issued = auth.pairing().confirm(&code).expect("issued");
        assert_eq!(issued.client_id, "vaio-p");
        // Token is now valid.
        assert!(auth.validate(&issued.token));
        // Waiting session receives it.
        let recv = rx.try_recv().expect("session received token");
        assert_eq!(recv.token, issued.token);
    }

    #[test]
    fn unknown_code_does_not_issue() {
        let store = TokenStore::in_memory();
        let auth = AuthConfig::build(
            &SecuritySection {
                require_token: true,
                pairing_enabled: true,
                ..SecuritySection::default()
            },
            store,
        );
        assert!(auth.pairing().confirm("BOGUS-9999").is_none());
    }

    #[test]
    fn expired_code_is_rejected() {
        let store = TokenStore::in_memory();
        let auth = AuthConfig::build(
            &SecuritySection {
                require_token: true,
                pairing_enabled: true,
                pairing_ttl_secs: 30,
                ..SecuritySection::default()
            },
            store,
        );
        let (code, _rx) = auth.pairing().request("vaio-p");
        // Hack: brute-force fast-forward by replacing issued_at via direct
        // mutex access in test scope.
        {
            let mut map = auth.pairing().inner.lock().unwrap();
            let entry = map.get_mut(&code).expect("entry present");
            entry.issued_at = Instant::now() - StdDuration::from_secs(3600);
        }
        assert!(auth.pairing().confirm(&code).is_none());
    }

    #[test]
    fn token_store_persists_across_reload() {
        let dir = std::env::temp_dir().join(format!("ph0sphor-test-{}", std::process::id()));
        let path = dir.join("tokens.json");
        let _ = std::fs::remove_file(&path);

        let store = TokenStore::load_or_create(&path).expect("load");
        let issued = store.issue("vaio-p").expect("issue");
        drop(store);

        let store2 = TokenStore::load_or_create(&path).expect("reload");
        assert!(store2.contains(&issued.token));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }

    #[test]
    fn generated_codes_are_well_formed() {
        let code = generate_pairing_code();
        assert_eq!(code.len(), 9); // 4 + '-' + 4
        assert!(code.chars().nth(4) == Some('-'));
    }
}
