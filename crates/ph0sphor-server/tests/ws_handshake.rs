//! Milestone 2 acceptance test.
//!
//! Boots a real server bound to an ephemeral loopback port, connects with
//! a tokio-tungstenite client, runs the protobuf handshake, and asserts
//! that a FullSnapshot with live CPU/RAM/disk/network data lands on the
//! wire. This is the canonical "done" criterion for Milestone 2.

use futures_util::{SinkExt, StreamExt};
use ph0sphor_core::{CpuMetrics, MemoryMetrics};
use ph0sphor_protocol::{
    decode, encode, envelope, fixtures::FIXTURE_PROTOCOL_VERSION, AuthRequest, Hello, Payload,
};
use ph0sphor_server::{
    auth::AuthConfig,
    collectors::spawn_demo,
    config::{PerformanceSection, SecuritySection, ServerConfig},
    net::{serve, serve_with_perf},
    state::State,
};
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message as WsMessage;

#[tokio::test]
async fn server_streams_full_snapshot_to_authenticated_client() {
    let cfg = ServerConfig::demo();
    let state = State::new("test-host".into(), "test-os".into());
    let collectors = spawn_demo(state.clone());
    let auth = AuthConfig::from_security(&cfg.security);
    let mut handle = serve(&cfg.server.bind, state, auth)
        .await
        .expect("server binds");

    let url = format!("ws://{}/ws", handle.local_addr);
    let (mut ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("client connects");

    // ---- Hello ---------------------------------------------------------
    let hello = envelope(Payload::Hello(Hello {
        client_id: "test-client".into(),
        client_version: "0.0.1".into(),
    }));
    ws.send(WsMessage::Binary(encode(&hello))).await.unwrap();

    // ---- AuthRequest ---------------------------------------------------
    let auth_req = envelope(Payload::AuthRequest(AuthRequest {
        token: "anything".into(), // require_token = false in demo config
    }));
    ws.send(WsMessage::Binary(encode(&auth_req))).await.unwrap();

    // ---- AuthResponse --------------------------------------------------
    let resp_env = recv_envelope(&mut ws).await;
    assert_eq!(resp_env.protocol_version, FIXTURE_PROTOCOL_VERSION);
    let Some(Payload::AuthResponse(resp)) = resp_env.payload else {
        panic!("expected AuthResponse");
    };
    assert!(resp.ok, "auth rejected: {}", resp.reason);

    // ---- FullSnapshot --------------------------------------------------
    let snap_env = recv_envelope(&mut ws).await;
    let Some(Payload::FullSnapshot(snap)) = snap_env.payload else {
        panic!("expected FullSnapshot");
    };

    // Milestone 2 criterion: live CPU/RAM/DISK/NET in the snapshot.
    assert_eq!(snap.hostname, "test-host");
    let cpu = snap.cpu.as_ref().expect("cpu present");
    assert!(cpu.usage_percent >= 0.0 && cpu.usage_percent <= 100.0);
    let mem = snap.memory.as_ref().expect("memory present");
    assert!(mem.total_bytes > 0);
    assert!(!snap.disks.is_empty(), "demo collector seeded a disk");
    assert!(!snap.network.is_empty(), "demo collector seeded a network");

    // Clean shutdown — server and collectors should drain promptly.
    let _ = ws.close(None).await;
    handle.shutdown();
    collectors.shutdown();
    timeout(Duration::from_secs(5), handle.join())
        .await
        .expect("server shuts down cleanly");
    timeout(Duration::from_secs(5), collectors.join())
        .await
        .expect("collectors shut down cleanly");
}

#[tokio::test]
async fn server_rejects_invalid_token_when_required() {
    let mut cfg = ServerConfig::default();
    cfg.server.bind = "127.0.0.1:0".to_string();
    cfg.security = SecuritySection {
        require_token: true,
        tokens: vec!["the-only-good-token".into()],
        ..SecuritySection::default()
    };
    let state = State::new("test-host".into(), "test-os".into());
    let collectors = spawn_demo(state.clone());
    let auth = AuthConfig::from_security(&cfg.security);
    let handle = serve(&cfg.server.bind, state, auth).await.unwrap();

    let url = format!("ws://{}/ws", handle.local_addr);
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

    let hello = envelope(Payload::Hello(Hello {
        client_id: "test-client".into(),
        client_version: "0.0.1".into(),
    }));
    ws.send(WsMessage::Binary(encode(&hello))).await.unwrap();

    let bad_auth = envelope(Payload::AuthRequest(AuthRequest {
        token: "not-the-token".into(),
    }));
    ws.send(WsMessage::Binary(encode(&bad_auth))).await.unwrap();

    let resp_env = recv_envelope(&mut ws).await;
    let Some(Payload::AuthResponse(resp)) = resp_env.payload else {
        panic!("expected AuthResponse");
    };
    assert!(!resp.ok);
    assert!(resp.reason.contains("invalid"));

    let _ = ws.close(None).await;
    handle.shutdown_and_join().await;
    collectors.shutdown();
    collectors.join().await;
}

#[tokio::test]
async fn server_emits_delta_after_state_change() {
    // No collectors — drive state changes manually so the test is fully
    // deterministic. Token disabled and min_send_interval set short so
    // coalescing doesn't dominate the test runtime.
    let mut cfg = ServerConfig::default();
    cfg.server.bind = "127.0.0.1:0".to_string();
    cfg.security.require_token = false;
    let perf = PerformanceSection {
        min_send_interval_ms: 50,
        full_snapshot_interval_sec: 3600, // out of the way
        send_deltas_only: true,
        ..PerformanceSection::default()
    };

    let state = State::new("test-host".into(), "test-os".into());
    state.update_cpu(CpuMetrics {
        usage_percent: 10.0,
        temperature_c: None,
        core_count: Some(8),
    });
    state.update_memory(MemoryMetrics {
        used_bytes: 1_000_000_000,
        total_bytes: 8_000_000_000,
        ..MemoryMetrics::default()
    });

    let auth = AuthConfig::from_security(&cfg.security);
    let handle = serve_with_perf(&cfg.server.bind, state.clone(), auth, perf)
        .await
        .unwrap();

    let url = format!("ws://{}/ws", handle.local_addr);
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

    // Handshake.
    let hello = envelope(Payload::Hello(Hello {
        client_id: "test-client".into(),
        client_version: "0.0.1".into(),
    }));
    ws.send(WsMessage::Binary(encode(&hello))).await.unwrap();
    let req = envelope(Payload::AuthRequest(AuthRequest {
        token: String::new(),
    }));
    ws.send(WsMessage::Binary(encode(&req))).await.unwrap();
    let resp = recv_envelope(&mut ws).await;
    matches!(resp.payload, Some(Payload::AuthResponse(_)));

    // Initial FullSnapshot.
    let initial = recv_envelope(&mut ws).await;
    let Some(Payload::FullSnapshot(_)) = initial.payload else {
        panic!("expected initial FullSnapshot, got {:?}", initial.payload);
    };

    // Mutate state — this should cause the server to emit a DeltaUpdate
    // with cpu_usage_percent populated.
    state.update_cpu(CpuMetrics {
        usage_percent: 85.0,
        temperature_c: None,
        core_count: Some(8),
    });

    // The next envelope must be a DeltaUpdate carrying the changed CPU.
    let next = recv_envelope(&mut ws).await;
    let Some(Payload::DeltaUpdate(d)) = next.payload else {
        panic!("expected DeltaUpdate, got {:?}", next.payload);
    };
    assert_eq!(d.cpu_usage_percent, Some(85.0));
    assert!(
        d.memory_used_bytes.is_none(),
        "memory should not be re-sent"
    );

    let _ = ws.close(None).await;
    handle.shutdown_and_join().await;
}

async fn recv_envelope<S>(ws: &mut S) -> ph0sphor_protocol::Envelope
where
    S: futures_util::Stream<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>>
        + Unpin,
{
    loop {
        let msg = timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("recv timeout")
            .expect("ws stream closed")
            .expect("ws error");
        match msg {
            WsMessage::Binary(bytes) => return decode(&bytes).expect("decode envelope"),
            WsMessage::Ping(_) | WsMessage::Pong(_) => continue,
            other => panic!("unexpected ws message: {other:?}"),
        }
    }
}
