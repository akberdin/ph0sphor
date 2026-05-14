//! WebSocket binary endpoint.
//!
//! Wire flow per README §9.1:
//!
//! ```text
//! client                                            server
//!   |---- Envelope:Hello -------------------------> |
//!   |---- Envelope:AuthRequest -------------------> |
//!   |<--- Envelope:AuthResponse ---------------------|
//!   |<--- Envelope:FullSnapshot (initial) -----------|
//!   |<--- Envelope:DeltaUpdate ............ x N -----|
//!   |<--- Envelope:FullSnapshot (safety) ............|
//! ```
//!
//! Milestone 4 wires up delta encoding, send-rate coalescing and
//! per-session byte counters for self-monitoring. The session keeps
//! `last_sent_wire` as its model of what the connected client has
//! already seen and computes deltas against it. A periodic full
//! snapshot (default 60 s, configurable) protects against drift.

use crate::auth::AuthConfig;
use crate::config::PerformanceSection;
use crate::state::State;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State as AxumState,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use ph0sphor_protocol::{decode, delta, encode, envelope, wire, ErrorMessage, Payload};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

/// Live server handle returned from [`serve`].
#[derive(Debug)]
pub struct ServerHandle {
    pub local_addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    join: tokio::task::JoinHandle<()>,
}

impl ServerHandle {
    pub fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
    pub async fn join(self) {
        let _ = self.join.await;
    }
    pub async fn shutdown_and_join(mut self) {
        self.shutdown();
        self.join().await;
    }
}

#[derive(Debug, Clone)]
struct AppState {
    state: State,
    auth: AuthConfig,
    perf: PerformanceSection,
}

pub async fn serve(
    bind_addr: &str,
    state: State,
    auth: AuthConfig,
) -> std::io::Result<ServerHandle> {
    serve_with_perf(bind_addr, state, auth, PerformanceSection::default()).await
}

/// Like [`serve`], but lets the caller pin the [`PerformanceSection`]
/// to drive coalescing intervals from a loaded config.
pub async fn serve_with_perf(
    bind_addr: &str,
    state: State,
    auth: AuthConfig,
    perf: PerformanceSection,
) -> std::io::Result<ServerHandle> {
    let listener = TcpListener::bind(bind_addr).await?;
    let local_addr = listener.local_addr()?;
    info!(%local_addr, "ph0sphor-server listening");

    let app_state = AppState { state, auth, perf };
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let join = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
            info!("ph0sphor-server shutdown signal received");
        });
        if let Err(e) = server.await {
            warn!(error = %e, "ph0sphor-server stopped with error");
        }
    });

    Ok(ServerHandle {
        local_addr,
        shutdown_tx: Some(shutdown_tx),
        join,
    })
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    AxumState(app): AxumState<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, app))
}

async fn handle_socket(mut socket: WebSocket, app: AppState) {
    let session_start = Instant::now();
    let stats = match run_session(&mut socket, &app).await {
        Ok(stats) => stats,
        Err(e) => {
            debug!(error = %e, "client session ended");
            SessionStats::default()
        }
    };
    let _ = socket.send(Message::Close(None)).await;

    // Self-monitoring summary line per session, per README §13.1 / §24.
    let secs = session_start.elapsed().as_secs().max(1);
    info!(
        bytes_sent = stats.bytes_sent,
        full_snapshots = stats.full_snapshots,
        deltas = stats.deltas,
        suppressed = stats.suppressed,
        secs,
        avg_bps = stats.bytes_sent / secs,
        "client session closed"
    );
}

#[derive(Debug, Default)]
struct SessionStats {
    bytes_sent: u64,
    full_snapshots: u64,
    deltas: u64,
    /// Number of state notifications that produced no payload because
    /// the resulting delta was empty (pure noise filter).
    suppressed: u64,
}

#[derive(Debug, thiserror::Error)]
enum SessionError {
    #[error("websocket: {0}")]
    Ws(#[from] axum::Error),
    #[error("protocol: {0}")]
    Protocol(#[from] ph0sphor_protocol::ProtocolError),
    #[error("unexpected message type")]
    Unexpected,
    #[error("client closed connection during handshake")]
    EarlyClose,
    #[error("auth failed")]
    AuthFailed,
}

async fn run_session(socket: &mut WebSocket, app: &AppState) -> Result<SessionStats, SessionError> {
    // ---- Handshake ----------------------------------------------------
    let hello_env = recv_envelope(socket).await?;
    let Some(Payload::Hello(hello)) = hello_env.payload else {
        return Err(SessionError::Unexpected);
    };
    debug!(client_id = %hello.client_id, "hello received");

    let auth_env = recv_envelope(socket).await?;
    let Some(Payload::AuthRequest(req)) = auth_env.payload else {
        return Err(SessionError::Unexpected);
    };
    let ok = app.auth.validate(&req.token);

    let resp = envelope(Payload::AuthResponse(ph0sphor_protocol::AuthResponse {
        ok,
        reason: if ok {
            String::new()
        } else {
            "invalid token".to_string()
        },
    }));
    socket.send(Message::Binary(encode(&resp))).await?;
    if !ok {
        warn!(client_id = %hello.client_id, "client auth rejected");
        return Err(SessionError::AuthFailed);
    }
    info!(client_id = %hello.client_id, "client authenticated");

    // ---- Streaming ----------------------------------------------------
    let mut stats = SessionStats::default();
    let perf = &app.perf;
    let min_send = Duration::from_millis(perf.min_send_interval_ms.max(50));
    let full_interval = Duration::from_secs(perf.full_snapshot_interval_sec.max(5));
    let send_deltas = perf.send_deltas_only;

    let initial = app.state.snapshot();
    let mut last_sent_wire: wire::FullSnapshot = (&initial).into();
    let env = envelope(Payload::FullSnapshot(last_sent_wire.clone()));
    let bytes = encode(&env);
    stats.bytes_sent += bytes.len() as u64;
    stats.full_snapshots += 1;
    socket.send(Message::Binary(bytes)).await?;
    let mut last_send_at = Instant::now();
    let mut last_full_at = last_send_at;

    let mut rx = app.state.subscribe();
    let mut safety = tokio::time::interval(full_interval);
    safety.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    safety.tick().await; // discard the immediate first tick

    let mut pending = false;
    loop {
        tokio::select! {
            biased;

            msg = socket.recv() => match msg {
                None => return Ok(stats),
                Some(Err(e)) => return Err(e.into()),
                Some(Ok(Message::Close(_))) => return Ok(stats),
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
                Some(Ok(Message::Binary(bytes))) => {
                    if let Ok(env) = decode(&bytes) {
                        match env.payload {
                            Some(Payload::Ping(p)) => {
                                let pong = envelope(Payload::Pong(ph0sphor_protocol::Pong { nonce: p.nonce }));
                                let buf = encode(&pong);
                                stats.bytes_sent += buf.len() as u64;
                                socket.send(Message::Binary(buf)).await?;
                            }
                            _ => debug!("ignoring unexpected client payload"),
                        }
                    }
                }
                Some(Ok(_)) => continue,
            },

            changed = rx.changed() => {
                if changed.is_err() { return Ok(stats); }
                pending = true;
            }

            _ = tokio::time::sleep_until((last_send_at + min_send).into()), if pending => {
                pending = false;
                let cur = app.state.snapshot();
                let cur_wire: wire::FullSnapshot = (&cur).into();

                let needs_full = !send_deltas
                    || last_full_at.elapsed() >= full_interval;

                if needs_full {
                    send_full(socket, &cur_wire, &mut stats).await?;
                    last_sent_wire = cur_wire;
                    last_full_at = Instant::now();
                    last_send_at = last_full_at;
                } else {
                    let d = delta::compute_delta(&last_sent_wire, &cur_wire);
                    if delta::is_empty(&d) {
                        stats.suppressed += 1;
                    } else {
                        let env = envelope(Payload::DeltaUpdate(d));
                        let buf = encode(&env);
                        stats.bytes_sent += buf.len() as u64;
                        stats.deltas += 1;
                        socket.send(Message::Binary(buf)).await?;
                        last_sent_wire = cur_wire;
                        last_send_at = Instant::now();
                    }
                }
            }

            _ = safety.tick() => {
                let cur = app.state.snapshot();
                let cur_wire: wire::FullSnapshot = (&cur).into();
                send_full(socket, &cur_wire, &mut stats).await?;
                last_sent_wire = cur_wire;
                last_full_at = Instant::now();
                last_send_at = last_full_at;
                pending = false;
            }
        }
    }
}

async fn send_full(
    socket: &mut WebSocket,
    cur_wire: &wire::FullSnapshot,
    stats: &mut SessionStats,
) -> Result<(), SessionError> {
    let env = envelope(Payload::FullSnapshot(cur_wire.clone()));
    let buf = encode(&env);
    stats.bytes_sent += buf.len() as u64;
    stats.full_snapshots += 1;
    socket.send(Message::Binary(buf)).await?;
    Ok(())
}

async fn recv_envelope(
    socket: &mut WebSocket,
) -> Result<ph0sphor_protocol::Envelope, SessionError> {
    loop {
        match socket.recv().await {
            None => return Err(SessionError::EarlyClose),
            Some(Err(e)) => return Err(e.into()),
            Some(Ok(Message::Binary(bytes))) => return Ok(decode(&bytes)?),
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            Some(Ok(Message::Close(_))) => return Err(SessionError::EarlyClose),
            Some(Ok(_)) => {
                let err = envelope(Payload::Error(ErrorMessage {
                    code: "expected_binary".into(),
                    message: "binary protobuf frames only".into(),
                }));
                socket.send(Message::Binary(encode(&err))).await?;
                return Err(SessionError::Unexpected);
            }
        }
    }
}
