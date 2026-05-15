//! Wire protocol for PHOSPHOR.
//!
//! Production transport is Protobuf binary frames over WebSocket. A JSON
//! debug encoder mirrors the same logical schema and is intended for tests,
//! development tooling and `--debug-json` runs of the server. JSON debug
//! mode must never be enabled in production.
//!
//! Layout:
//! - `wire` — prost-generated types from `proto/ph0sphor.proto`.
//! - `convert` — `From`/`TryFrom` impls between `ph0sphor-core` domain
//!   types and `wire` types.
//! - top-level functions — `encode`/`decode` (Protobuf) and
//!   `encode_json`/`decode_json` (debug).

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

use ph0sphor_core::PROTOCOL_VERSION;
use prost::Message as _;
use thiserror::Error;

pub mod convert;
pub mod delta;
pub mod fixtures;

pub mod wire {
    //! Generated wire types. Field numbers are stable; reordering is not.
    include!(concat!(env!("OUT_DIR"), "/ph0sphor.v1.rs"));
}

pub use wire::{
    envelope::Payload, AuthRequest, AuthResponse, CpuMetrics, DeltaUpdate, DiskMetrics, Envelope,
    ErrorMessage, Event, FullSnapshot, Hello, MemoryMetrics, NetworkMetrics, PairingChallenge,
    PairingConfirm, PairingRequest, Ping, Pong, Severity,
};

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("encoding failed: {0}")]
    Encode(String),

    #[error("decoding failed: {0}")]
    Decode(String),

    #[error("unsupported protocol version: got {got}, expected {expected}")]
    UnsupportedVersion { got: u32, expected: u32 },

    #[error("envelope has no payload")]
    EmptyEnvelope,
}

/// Build an `Envelope` stamped with the current protocol version.
pub fn envelope(payload: Payload) -> Envelope {
    Envelope {
        protocol_version: PROTOCOL_VERSION,
        payload: Some(payload),
    }
}

/// Encode an envelope to Protobuf binary bytes (production transport).
pub fn encode(env: &Envelope) -> Vec<u8> {
    let mut buf = Vec::with_capacity(env.encoded_len());
    // `Message::encode` writes into a `BufMut`; `Vec<u8>` satisfies that.
    env.encode(&mut buf)
        .expect("Vec<u8> never fails to receive bytes");
    buf
}

/// Decode an envelope from Protobuf binary bytes.
///
/// Verifies that `protocol_version` matches `ph0sphor_core::PROTOCOL_VERSION`.
/// Callers that want to handle older or newer clients explicitly should
/// decode with [`decode_any_version`] instead.
pub fn decode(bytes: &[u8]) -> Result<Envelope, ProtocolError> {
    let env = decode_any_version(bytes)?;
    if env.protocol_version != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion {
            got: env.protocol_version,
            expected: PROTOCOL_VERSION,
        });
    }
    Ok(env)
}

/// Decode an envelope without checking `protocol_version`.
pub fn decode_any_version(bytes: &[u8]) -> Result<Envelope, ProtocolError> {
    Envelope::decode(bytes).map_err(|e| ProtocolError::Decode(e.to_string()))
}

// ---------------------------------------------------------------------------
// Debug JSON mirror
// ---------------------------------------------------------------------------

/// Debug-only JSON encoder. Schema mirrors the Protobuf shape but is not the
/// production transport.
pub fn encode_json(env: &Envelope) -> Result<Vec<u8>, ProtocolError> {
    let dbg = debug_json::EnvelopeJson::from_envelope(env)?;
    serde_json::to_vec(&dbg).map_err(|e| ProtocolError::Encode(e.to_string()))
}

/// Pretty-printed JSON variant, suitable for `--debug-json` dumps.
pub fn encode_json_pretty(env: &Envelope) -> Result<String, ProtocolError> {
    let dbg = debug_json::EnvelopeJson::from_envelope(env)?;
    serde_json::to_string_pretty(&dbg).map_err(|e| ProtocolError::Encode(e.to_string()))
}

/// Debug-only JSON decoder.
pub fn decode_json(bytes: &[u8]) -> Result<Envelope, ProtocolError> {
    let dbg: debug_json::EnvelopeJson =
        serde_json::from_slice(bytes).map_err(|e| ProtocolError::Decode(e.to_string()))?;
    dbg.into_envelope()
}

mod debug_json {
    //! A serde mirror of `Envelope`. We don't use prost's own JSON support
    //! because it isn't part of `prost` core; instead, we map a small,
    //! explicit subset that is sufficient for fixtures and debug dumps.

    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub(super) struct EnvelopeJson {
        pub protocol_version: u32,
        #[serde(flatten)]
        pub payload: PayloadJson,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    pub(super) enum PayloadJson {
        Hello {
            client_id: String,
            client_version: String,
        },
        AuthRequest {
            token: String,
        },
        AuthResponse {
            ok: bool,
            reason: String,
        },
        PairingRequest {
            client_id: String,
        },
        PairingChallenge {
            code: String,
        },
        PairingConfirm {
            code: String,
            #[serde(default)]
            token: String,
        },
        FullSnapshot(FullSnapshot),
        DeltaUpdate(DeltaUpdate),
        Event(Event),
        Ping {
            nonce: u64,
        },
        Pong {
            nonce: u64,
        },
        Error {
            code: String,
            message: String,
        },
    }

    impl EnvelopeJson {
        pub(super) fn from_envelope(env: &Envelope) -> Result<Self, ProtocolError> {
            let payload = match env.payload.as_ref().ok_or(ProtocolError::EmptyEnvelope)? {
                Payload::Hello(m) => PayloadJson::Hello {
                    client_id: m.client_id.clone(),
                    client_version: m.client_version.clone(),
                },
                Payload::AuthRequest(m) => PayloadJson::AuthRequest {
                    token: m.token.clone(),
                },
                Payload::AuthResponse(m) => PayloadJson::AuthResponse {
                    ok: m.ok,
                    reason: m.reason.clone(),
                },
                Payload::PairingRequest(m) => PayloadJson::PairingRequest {
                    client_id: m.client_id.clone(),
                },
                Payload::PairingChallenge(m) => PayloadJson::PairingChallenge {
                    code: m.code.clone(),
                },
                Payload::PairingConfirm(m) => PayloadJson::PairingConfirm {
                    code: m.code.clone(),
                    token: m.token.clone(),
                },
                Payload::FullSnapshot(m) => PayloadJson::FullSnapshot(m.clone()),
                Payload::DeltaUpdate(m) => PayloadJson::DeltaUpdate(m.clone()),
                Payload::Event(m) => PayloadJson::Event(m.clone()),
                Payload::Ping(m) => PayloadJson::Ping { nonce: m.nonce },
                Payload::Pong(m) => PayloadJson::Pong { nonce: m.nonce },
                Payload::Error(m) => PayloadJson::Error {
                    code: m.code.clone(),
                    message: m.message.clone(),
                },
            };
            Ok(EnvelopeJson {
                protocol_version: env.protocol_version,
                payload,
            })
        }

        pub(super) fn into_envelope(self) -> Result<Envelope, ProtocolError> {
            let payload = match self.payload {
                PayloadJson::Hello {
                    client_id,
                    client_version,
                } => Payload::Hello(Hello {
                    client_id,
                    client_version,
                }),
                PayloadJson::AuthRequest { token } => Payload::AuthRequest(AuthRequest { token }),
                PayloadJson::AuthResponse { ok, reason } => {
                    Payload::AuthResponse(AuthResponse { ok, reason })
                }
                PayloadJson::PairingRequest { client_id } => {
                    Payload::PairingRequest(PairingRequest { client_id })
                }
                PayloadJson::PairingChallenge { code } => {
                    Payload::PairingChallenge(PairingChallenge { code })
                }
                PayloadJson::PairingConfirm { code, token } => {
                    Payload::PairingConfirm(PairingConfirm { code, token })
                }
                PayloadJson::FullSnapshot(m) => Payload::FullSnapshot(m),
                PayloadJson::DeltaUpdate(m) => Payload::DeltaUpdate(m),
                PayloadJson::Event(m) => Payload::Event(m),
                PayloadJson::Ping { nonce } => Payload::Ping(Ping { nonce }),
                PayloadJson::Pong { nonce } => Payload::Pong(Pong { nonce }),
                PayloadJson::Error { code, message } => {
                    Payload::Error(ErrorMessage { code, message })
                }
            };
            Ok(Envelope {
                protocol_version: self.protocol_version,
                payload: Some(payload),
            })
        }
    }
}
