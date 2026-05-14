//! Wire protocol for PHOSPHOR.
//!
//! Production transport is Protobuf-over-WebSocket binary frames. This crate
//! currently exposes a JSON debug encoder so the other crates can compile and
//! integration tests can run before the Protobuf schema lands (Milestone 1).
//!
//! See `proto/ph0sphor.proto` and `docs/protocol.md`.

#![forbid(unsafe_code)]

use ph0sphor_core::{Event, Snapshot, PROTOCOL_VERSION};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("encoding failed: {0}")]
    Encode(String),

    #[error("decoding failed: {0}")]
    Decode(String),

    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    Hello {
        protocol_version: u32,
        client_id: String,
    },
    AuthRequest {
        token: String,
    },
    AuthResponse {
        ok: bool,
        reason: Option<String>,
    },
    FullSnapshot(Snapshot),
    /// Placeholder for delta encoding; replaced by Protobuf in Milestone 1.
    DeltaUpdate(serde_json::Value),
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

/// Debug-only JSON encoder. Not the production transport.
pub fn encode_json(message: &Message) -> Result<Vec<u8>, ProtocolError> {
    serde_json::to_vec(message).map_err(|e| ProtocolError::Encode(e.to_string()))
}

/// Debug-only JSON decoder. Not the production transport.
pub fn decode_json(bytes: &[u8]) -> Result<Message, ProtocolError> {
    serde_json::from_slice(bytes).map_err(|e| ProtocolError::Decode(e.to_string()))
}

pub fn hello(client_id: impl Into<String>) -> Message {
    Message::Hello {
        protocol_version: PROTOCOL_VERSION,
        client_id: client_id.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_round_trip() {
        let m = hello("vaio-p");
        let bytes = encode_json(&m).unwrap();
        let back = decode_json(&bytes).unwrap();
        match back {
            Message::Hello {
                protocol_version,
                client_id,
            } => {
                assert_eq!(protocol_version, PROTOCOL_VERSION);
                assert_eq!(client_id, "vaio-p");
            }
            _ => panic!("expected Hello"),
        }
    }
}
