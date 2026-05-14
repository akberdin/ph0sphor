//! WebSocket client task.
//!
//! Owns a single connection at a time:
//!
//! 1. Connect to the configured server URL.
//! 2. Send `Hello` + `AuthRequest`.
//! 3. Wait for `AuthResponse` (must be ok).
//! 4. Stream incoming `FullSnapshot` / `DeltaUpdate` / `Event` envelopes
//!    into the app channel as `AppEvent`s.
//! 5. On any error, emit `ConnectionStatus::Offline`, sleep with
//!    exponential backoff (1s → 30s), and reconnect.
//!
//! The task only exits when the channel receiver is dropped (i.e. the
//! UI has quit) or [`shutdown`](tokio::sync::Notify) fires.

use crate::event::{AppEvent, ConnectionStatus, LogLine};
use futures_util::{SinkExt, StreamExt};
use ph0sphor_core::APP_VERSION;
use ph0sphor_protocol::{decode, encode, envelope, AuthRequest, Hello, Payload};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, Notify};
use tokio_tungstenite::tungstenite::Message as WsMessage;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("websocket: {0}")]
    Ws(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("protocol: {0}")]
    Protocol(#[from] ph0sphor_protocol::ProtocolError),
    #[error("server closed during handshake")]
    EarlyClose,
    #[error("auth rejected by server: {0}")]
    AuthRejected(String),
    #[error("unexpected payload during handshake")]
    Unexpected,
}

/// Spawn the long-running WS client task.
///
/// `client_id` is sent in the `Hello`. `token` is sent in `AuthRequest`
/// and may be empty when the server runs with `require_token = false`.
pub fn spawn(
    server_url: String,
    client_id: String,
    token: String,
    tx: mpsc::Sender<AppEvent>,
    shutdown: Arc<Notify>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        run(server_url, client_id, token, tx, shutdown).await;
    })
}

async fn run(
    server_url: String,
    client_id: String,
    token: String,
    tx: mpsc::Sender<AppEvent>,
    shutdown: Arc<Notify>,
) {
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);

    loop {
        let _ = tx
            .send(AppEvent::Connection(ConnectionStatus::Connecting))
            .await;
        let session = run_session(&server_url, &client_id, &token, &tx);

        tokio::select! {
            res = session => match res {
                Ok(()) => {
                    let _ = tx.send(AppEvent::Connection(ConnectionStatus::Disconnected)).await;
                    let _ = tx.send(AppEvent::Log(LogLine::info("server closed connection"))).await;
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Connection(ConnectionStatus::Offline)).await;
                    let _ = tx.send(AppEvent::Log(LogLine::warn(format!("link error: {e}")))).await;
                }
            },
            _ = shutdown.notified() => return,
        }

        // Backoff with cancellation: a shutdown during sleep exits immediately.
        let sleep = tokio::time::sleep(backoff);
        tokio::select! {
            _ = sleep => {},
            _ = shutdown.notified() => return,
        }
        backoff = (backoff * 2).min(max_backoff);
    }
}

async fn run_session(
    url: &str,
    client_id: &str,
    token: &str,
    tx: &mpsc::Sender<AppEvent>,
) -> Result<(), ClientError> {
    let (mut ws, _) = tokio_tungstenite::connect_async(url).await?;
    let _ = tx
        .send(AppEvent::Connection(ConnectionStatus::Authenticating))
        .await;

    // Send Hello.
    let hello = envelope(Payload::Hello(Hello {
        client_id: client_id.to_string(),
        client_version: APP_VERSION.to_string(),
    }));
    ws.send(WsMessage::Binary(encode(&hello))).await?;

    // Send AuthRequest.
    let req = envelope(Payload::AuthRequest(AuthRequest {
        token: token.to_string(),
    }));
    ws.send(WsMessage::Binary(encode(&req))).await?;

    // Wait for AuthResponse.
    let resp = recv_envelope(&mut ws).await?;
    let Some(Payload::AuthResponse(r)) = resp.payload else {
        return Err(ClientError::Unexpected);
    };
    if !r.ok {
        return Err(ClientError::AuthRejected(r.reason));
    }

    let _ = tx
        .send(AppEvent::Connection(ConnectionStatus::Online))
        .await;

    // Stream payloads until the connection drops.
    while let Some(msg) = ws.next().await {
        match msg? {
            WsMessage::Binary(bytes) => {
                let env = decode(&bytes)?;
                match env.payload {
                    Some(Payload::FullSnapshot(snap)) => {
                        let domain: ph0sphor_core::Snapshot = (&snap).into();
                        if tx.send(AppEvent::Snapshot(domain)).await.is_err() {
                            return Ok(());
                        }
                    }
                    Some(Payload::Event(e)) => {
                        let _ = tx
                            .send(AppEvent::Log(LogLine::info(format!(
                                "server: {} — {}",
                                e.kind, e.message
                            ))))
                            .await;
                    }
                    Some(Payload::Pong(_)) | Some(Payload::DeltaUpdate(_)) => {
                        // Delta application lands in Milestone 4.
                    }
                    Some(Payload::Error(err)) => {
                        let _ = tx
                            .send(AppEvent::Log(LogLine::critical(format!(
                                "server error: {} ({})",
                                err.message, err.code
                            ))))
                            .await;
                    }
                    _ => {}
                }
            }
            WsMessage::Close(_) => return Ok(()),
            WsMessage::Ping(_) | WsMessage::Pong(_) => {}
            _ => {}
        }
    }
    Ok(())
}

async fn recv_envelope<S>(ws: &mut S) -> Result<ph0sphor_protocol::Envelope, ClientError>
where
    S: futures_util::Stream<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>>
        + Unpin,
{
    loop {
        match ws.next().await {
            None => return Err(ClientError::EarlyClose),
            Some(Err(e)) => return Err(e.into()),
            Some(Ok(WsMessage::Binary(bytes))) => return Ok(decode(&bytes)?),
            Some(Ok(WsMessage::Close(_))) => return Err(ClientError::EarlyClose),
            Some(Ok(WsMessage::Ping(_) | WsMessage::Pong(_))) => continue,
            Some(Ok(_)) => return Err(ClientError::Unexpected),
        }
    }
}

/// Demo data source. Replaces the WS client when `--demo` is used: emits
/// a single Snapshot then keeps the connection status "ONLINE" so the
/// UI exercises every code path that the live client would.
pub fn spawn_demo(tx: mpsc::Sender<AppEvent>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let _ = tx
            .send(AppEvent::Connection(ConnectionStatus::Online))
            .await;

        let snap = ph0sphor_protocol::fixtures::sample_domain_snapshot();
        let _ = tx.send(AppEvent::Snapshot(snap.clone())).await;

        let mut ticker = tokio::time::interval(Duration::from_secs(2));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut phase: f32 = 0.0;
        loop {
            ticker.tick().await;
            phase += 0.4;
            let mut s = snap.clone();
            s.cpu.usage_percent = (40.0 + 25.0 * phase.sin()).clamp(0.0, 100.0);
            if tx.send(AppEvent::Snapshot(s)).await.is_err() {
                return;
            }
        }
    })
}
