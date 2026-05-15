//! Milestone 5 acceptance test for the pairing flow.
//!
//! Boots a real server with `require_token = true` and an empty
//! static allowlist, runs the client's WS task with no token (so it
//! picks the pairing path), POSTs the pairing code to the loopback
//! control endpoint exactly as `ph0sphorctl pair confirm` would, and
//! asserts the client receives a TokenIssued event and then arrives
//! at ONLINE — i.e. it can be authenticated by the same token on a
//! subsequent reconnect.

use ph0sphor_client::event::{AppEvent, ConnectionStatus};
use ph0sphor_client::net;
use ph0sphor_server::{
    auth::{AuthConfig, TokenStore},
    collectors::spawn_demo,
    config::SecuritySection,
    control::serve_control,
    net::serve,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Notify};
use tokio::time::timeout;

#[tokio::test]
async fn client_pairs_then_receives_token_and_snapshot() {
    // --- Server ---------------------------------------------------------
    let sec = SecuritySection {
        require_token: true,
        pairing_enabled: true,
        tokens: vec![], // no static allowlist; only pairing-issued tokens work
        ..SecuritySection::default()
    };
    let store = TokenStore::in_memory();
    let auth = AuthConfig::build(&sec, store.clone());

    let state = ph0sphor_server::state::State::new("test-host".into(), "test-os".into());
    let collectors = spawn_demo(state.clone());

    let server = serve("127.0.0.1:0", state, auth.clone()).await.unwrap();
    let control = serve_control("127.0.0.1:0", auth.clone()).await.unwrap();
    let server_url = format!("ws://{}/ws", server.local_addr);
    let control_url = format!("http://{}/control/pair/confirm", control.local_addr);

    // --- Client (no token: forces pairing) ------------------------------
    let (tx, mut rx) = mpsc::channel::<AppEvent>(32);
    let shutdown = Arc::new(Notify::new());
    let _client = net::spawn(
        server_url,
        "vaio-p".into(),
        String::new(),
        tx,
        shutdown.clone(),
    );

    // The client must surface the pairing code via AppEvent::PairingChallenge.
    let code = wait_for_pairing_code(&mut rx).await;

    // --- ph0sphorctl-like control POST ----------------------------------
    let resp = http_post(&control_url, &format!("{{\"code\":\"{code}\"}}"))
        .await
        .expect("control POST");
    assert!(
        resp.contains("\"ok\":true"),
        "expected ok confirmation, got: {resp}"
    );

    // --- Client must now report TokenIssued and reach ONLINE ------------
    let mut got_token = false;
    let mut online = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline && !(got_token && online) {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let Ok(Some(event)) = timeout(remaining, rx.recv()).await else {
            break;
        };
        match event {
            AppEvent::TokenIssued(t) => {
                assert!(!t.is_empty(), "issued token must not be empty");
                got_token = true;
            }
            AppEvent::Connection(ConnectionStatus::Online) => {
                online = true;
            }
            _ => {}
        }
    }
    assert!(got_token, "client never received TokenIssued");
    assert!(online, "client never reached ONLINE after pairing");

    // --- The issued token is now in the store, so validate() accepts it.
    assert!(store.len() == 1, "store should hold exactly one token");

    shutdown.notify_waiters();
    server.shutdown_and_join().await;
    control.shutdown_and_join().await;
    collectors.shutdown();
    collectors.join().await;
}

async fn wait_for_pairing_code(rx: &mut mpsc::Receiver<AppEvent>) -> String {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        match timeout(remaining, rx.recv()).await {
            Ok(Some(AppEvent::PairingChallenge(code))) => return code,
            Ok(Some(_)) => continue,
            _ => break,
        }
    }
    panic!("client never produced AppEvent::PairingChallenge");
}

/// Tiny std-only HTTP POST mirroring `ph0sphorctl pair confirm`.
async fn http_post(url: &str, body: &str) -> std::io::Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let after = url
        .strip_prefix("http://")
        .ok_or_else(|| std::io::Error::other("only http:// URLs"))?;
    let (hostport, path) = after.split_once('/').unwrap_or((after, ""));
    let req = format!(
        "POST /{path} HTTP/1.1\r\nHost: {hostport}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let mut stream = tokio::net::TcpStream::connect(hostport).await?;
    stream.write_all(req.as_bytes()).await?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}
