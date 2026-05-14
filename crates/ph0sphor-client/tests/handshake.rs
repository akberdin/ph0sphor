//! Milestone 3 acceptance test for the client's network layer.
//!
//! Boots a real `ph0sphor-server` on a loopback ephemeral port and runs
//! the production client WS task against it. Asserts that the handshake
//! completes and a `Snapshot` with live CPU/RAM/disk/network lands on
//! the app channel — the same guarantee the TUI relies on at runtime.

use ph0sphor_client::event::{AppEvent, ConnectionStatus};
use ph0sphor_client::net;
use ph0sphor_server::{
    auth::AuthConfig, collectors::spawn_demo, config::ServerConfig, net::serve, state::State,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Notify};
use tokio::time::timeout;

#[tokio::test]
async fn client_completes_handshake_and_receives_snapshot() {
    // ---- Server side: demo collectors, no token required. -------------
    let server_cfg = ServerConfig::demo();
    let state = State::new("test-host".into(), "test-os".into());
    let server_collectors = spawn_demo(state.clone());
    let auth = AuthConfig::from_security(&server_cfg.security);
    let server = serve(&server_cfg.server.bind, state, auth)
        .await
        .expect("server binds");
    let server_url = format!("ws://{}/ws", server.local_addr);

    // ---- Client side: run the real WS task into a channel. ------------
    let (tx, mut rx) = mpsc::channel::<AppEvent>(32);
    let shutdown = Arc::new(Notify::new());
    let _client = net::spawn(
        server_url,
        "test-vaio".into(),
        String::new(), // no token; server is in require_token = false
        tx,
        shutdown.clone(),
    );

    // We expect, in order: Connection(Connecting), Connection(Authenticating),
    // Connection(Online), Snapshot(_). Server events / extra Connection
    // messages may interleave; we tolerate that and just wait for the
    // two we care about.
    let mut saw_online = false;
    let mut snapshot = None;
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let Ok(Some(event)) = timeout(remaining, rx.recv()).await else {
            break;
        };
        match event {
            AppEvent::Connection(ConnectionStatus::Online) => saw_online = true,
            AppEvent::Snapshot(snap) => {
                snapshot = Some(snap);
                break;
            }
            _ => {}
        }
    }

    assert!(saw_online, "client never reported ONLINE");
    let snap = snapshot.expect("client never received a FullSnapshot");

    // Milestone 3 done-criterion: the snapshot the TUI would render has
    // live CPU/RAM/disk/network from the server.
    assert_eq!(snap.hostname, "test-host");
    assert!(snap.cpu.usage_percent >= 0.0 && snap.cpu.usage_percent <= 100.0);
    assert!(snap.memory.total_bytes > 0);
    assert!(!snap.disks.is_empty(), "demo server seeded disks");
    assert!(!snap.network.is_empty(), "demo server seeded network");

    // Shut everything down promptly.
    shutdown.notify_waiters();
    server.shutdown_and_join().await;
    server_collectors.shutdown();
    server_collectors.join().await;
}
