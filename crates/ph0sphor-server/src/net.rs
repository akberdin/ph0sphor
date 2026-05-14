//! WebSocket binary endpoint.
//!
//! Wire flow per README §9.1, simplified for Milestone 2:
//!
//! ```text
//! client                                       server
//!   |---- Envelope:Hello -------------------->  |
//!   |---- Envelope:AuthRequest --------------> |
//!   |<--- Envelope:AuthResponse --------------- |
//!   |<--- Envelope:FullSnapshot (initial) ----- |
//!   |<--- Envelope:FullSnapshot (on change) --- |
//!   |                ...                        |
//! ```
//!
//! DeltaUpdate/Event streaming and periodic safety snapshots land in
//! Milestone 4 (performance pass).

use crate::auth::AuthConfig;
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
use ph0sphor_protocol::{decode, encode, envelope, ErrorMessage, Payload};
use std::net::SocketAddr;
use std::time::Duration;
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
    /// Signal the server to stop accepting new connections and drain.
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
}

/// Bind the server on `bind_addr` and start serving the WebSocket
/// endpoint at `/ws`. Returns the resolved local address (useful when
/// the caller passed port 0) and a shutdown handle.
pub async fn serve(
    bind_addr: &str,
    state: State,
    auth: AuthConfig,
) -> std::io::Result<ServerHandle> {
    let listener = TcpListener::bind(bind_addr).await?;
    let local_addr = listener.local_addr()?;
    info!(%local_addr, "ph0sphor-server listening");

    let app_state = AppState { state, auth };
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
    if let Err(e) = run_session(&mut socket, &app).await {
        debug!(error = %e, "client session ended");
    }
    let _ = socket.send(Message::Close(None)).await;
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

async fn run_session(socket: &mut WebSocket, app: &AppState) -> Result<(), SessionError> {
    // 1. Hello.
    let hello_env = recv_envelope(socket).await?;
    let Some(Payload::Hello(hello)) = hello_env.payload else {
        return Err(SessionError::Unexpected);
    };
    debug!(client_id = %hello.client_id, "hello received");

    // 2. AuthRequest.
    let auth_env = recv_envelope(socket).await?;
    let Some(Payload::AuthRequest(req)) = auth_env.payload else {
        return Err(SessionError::Unexpected);
    };
    let ok = app.auth.validate(&req.token);

    // 3. AuthResponse.
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

    // 4. Initial FullSnapshot.
    send_snapshot(socket, &app.state).await?;

    // 5. Stream FullSnapshot on every state change, with a safety
    //    refresh every 5s in case the client missed the watch tick.
    let mut rx = app.state.subscribe();
    let mut safety = tokio::time::interval(Duration::from_secs(5));
    safety.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            biased;

            // Client-initiated traffic: pings, closes, or unexpected payloads.
            msg = socket.recv() => match msg {
                None => return Ok(()),
                Some(Err(e)) => return Err(e.into()),
                Some(Ok(Message::Close(_))) => return Ok(()),
                Some(Ok(Message::Ping(_)) | Ok(Message::Pong(_))) => continue,
                Some(Ok(Message::Binary(bytes))) => {
                    if let Ok(env) = decode(&bytes) {
                        match env.payload {
                            Some(Payload::Ping(p)) => {
                                let pong = envelope(Payload::Pong(ph0sphor_protocol::Pong { nonce: p.nonce }));
                                socket.send(Message::Binary(encode(&pong))).await?;
                            }
                            _ => debug!("ignoring unexpected client payload"),
                        }
                    }
                }
                Some(Ok(_)) => continue,
            },

            changed = rx.changed() => {
                if changed.is_err() { return Ok(()); }
                send_snapshot(socket, &app.state).await?;
            }

            _ = safety.tick() => {
                send_snapshot(socket, &app.state).await?;
            }
        }
    }
}

async fn recv_envelope(
    socket: &mut WebSocket,
) -> Result<ph0sphor_protocol::Envelope, SessionError> {
    loop {
        match socket.recv().await {
            None => return Err(SessionError::EarlyClose),
            Some(Err(e)) => return Err(e.into()),
            Some(Ok(Message::Binary(bytes))) => return Ok(decode(&bytes)?),
            Some(Ok(Message::Ping(_)) | Ok(Message::Pong(_))) => continue,
            Some(Ok(Message::Close(_))) => return Err(SessionError::EarlyClose),
            Some(Ok(_)) => {
                // Reject text/binary fragments etc. with a structured error.
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

async fn send_snapshot(socket: &mut WebSocket, state: &State) -> Result<(), SessionError> {
    let snap = state.snapshot();
    let env = envelope(Payload::FullSnapshot((&snap).into()));
    socket.send(Message::Binary(encode(&env))).await?;
    Ok(())
}
