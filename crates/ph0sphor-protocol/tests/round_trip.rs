//! Round-trip tests for the PHOSPHOR wire protocol.
//!
//! These tests are the "done" criterion for Milestone 1: a realistic
//! telemetry snapshot must encode and decode cleanly through both the
//! Protobuf production path and the JSON debug path.

use ph0sphor_core::{Snapshot, PROTOCOL_VERSION};
use ph0sphor_protocol::{
    decode, decode_any_version, decode_json, encode, encode_json, encode_json_pretty, envelope,
    fixtures, Envelope, Payload, ProtocolError,
};

#[test]
fn hello_round_trip_protobuf() {
    let env = fixtures::sample_hello_envelope();
    let bytes = encode(&env);
    let back = decode(&bytes).expect("decode");

    assert_eq!(back.protocol_version, PROTOCOL_VERSION);
    match back.payload.expect("payload") {
        Payload::Hello(h) => {
            assert_eq!(h.client_id, "vaio-p");
            assert_eq!(h.client_version, "0.0.1");
        }
        other => panic!("expected Hello, got {other:?}"),
    }
}

#[test]
fn full_snapshot_round_trip_protobuf() {
    let original = fixtures::sample_snapshot_envelope();
    let bytes = encode(&original);

    // A realistic snapshot must fit comfortably in a typical WS frame.
    assert!(bytes.len() < 4096, "snapshot too large: {}", bytes.len());

    let decoded = decode(&bytes).expect("decode");
    assert_eq!(decoded, original);
}

#[test]
fn full_snapshot_round_trip_through_domain() {
    let domain = fixtures::sample_domain_snapshot();
    let wire = envelope(Payload::FullSnapshot((&domain).into()));
    let bytes = encode(&wire);

    let decoded = decode(&bytes).expect("decode");
    let Some(Payload::FullSnapshot(snap)) = decoded.payload else {
        panic!("expected FullSnapshot");
    };
    let back: Snapshot = (&snap).into();

    assert_eq!(back.hostname, domain.hostname);
    assert_eq!(back.cpu.usage_percent, domain.cpu.usage_percent);
    assert_eq!(back.disks.len(), domain.disks.len());
    assert_eq!(back.network[0].interface, domain.network[0].interface);
}

#[test]
fn delta_update_round_trip() {
    let env = fixtures::sample_delta_envelope();
    let bytes = encode(&env);
    let back = decode(&bytes).expect("decode");

    let Some(Payload::DeltaUpdate(d)) = back.payload else {
        panic!("expected DeltaUpdate");
    };
    assert_eq!(d.cpu_usage_percent, Some(63.1));
    assert!(d.memory_used_bytes.is_none());
    assert!(d.disks.is_empty());
}

#[test]
fn event_round_trip() {
    let env = fixtures::sample_event_envelope();
    let bytes = encode(&env);
    let back = decode(&bytes).expect("decode");

    let Some(Payload::Event(e)) = back.payload else {
        panic!("expected Event");
    };
    assert_eq!(e.kind, "new_mail");
    assert_eq!(e.attributes.get("count").map(String::as_str), Some("3"));
}

#[test]
fn json_debug_round_trip_matches_protobuf() {
    let env = fixtures::sample_snapshot_envelope();
    let json = encode_json(&env).expect("encode json");
    let back = decode_json(&json).expect("decode json");

    // Re-encoding to Protobuf must match the original byte-for-byte: the
    // JSON debug mirror must not lose information.
    assert_eq!(encode(&back), encode(&env));
}

#[test]
fn json_pretty_dumps_human_readable() {
    let env = fixtures::sample_event_envelope();
    let pretty = encode_json_pretty(&env).expect("encode pretty");
    assert!(pretty.contains("\"type\": \"event\""));
    assert!(pretty.contains("new_mail"));
}

#[test]
fn version_mismatch_is_reported() {
    let mut env = fixtures::sample_hello_envelope();
    env.protocol_version = PROTOCOL_VERSION + 99;
    let bytes = encode(&env);

    match decode(&bytes) {
        Err(ProtocolError::UnsupportedVersion { got, expected }) => {
            assert_eq!(got, PROTOCOL_VERSION + 99);
            assert_eq!(expected, PROTOCOL_VERSION);
        }
        other => panic!("expected UnsupportedVersion, got {other:?}"),
    }

    // decode_any_version must still parse it.
    let any = decode_any_version(&bytes).expect("decode any");
    assert_eq!(any.protocol_version, PROTOCOL_VERSION + 99);
}

#[test]
fn empty_envelope_round_trips_as_error_on_json() {
    let env = Envelope {
        protocol_version: PROTOCOL_VERSION,
        payload: None,
    };
    match encode_json(&env) {
        Err(ProtocolError::EmptyEnvelope) => {}
        other => panic!("expected EmptyEnvelope, got {other:?}"),
    }
}
