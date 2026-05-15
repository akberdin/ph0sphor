//! Loopback-only HTTP control endpoint.
//!
//! Hosts a single route — `POST /control/pair/confirm` — used by
//! `ph0sphorctl pair confirm <code>` to confirm a pending pairing
//! request out of band. Any non-loopback peer is rejected with
//! `403 Forbidden`, regardless of bind address, as a defense in depth
//! against accidental exposure.

use crate::auth::{redact_token, AuthConfig};
use axum::extract::{ConnectInfo, State as AxumState};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tracing::{info, warn};

#[derive(Debug, Deserialize)]
pub struct ConfirmRequest {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct ConfirmResponse {
    pub ok: bool,
    pub client_id: Option<String>,
    pub message: String,
}

/// Live handle for the control listener.
#[derive(Debug)]
pub struct ControlHandle {
    pub local_addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    join: tokio::task::JoinHandle<()>,
}

impl ControlHandle {
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
struct ControlState {
    auth: AuthConfig,
}

/// Bind the control endpoint on `bind_addr`. Caller is responsible for
/// using a loopback address; the request handler additionally checks
/// the peer's IP on every request.
pub async fn serve_control(bind_addr: &str, auth: AuthConfig) -> std::io::Result<ControlHandle> {
    let listener = TcpListener::bind(bind_addr).await?;
    let local_addr = listener.local_addr()?;
    info!(%local_addr, "ph0sphor-server control endpoint listening");

    let app = Router::new()
        .route("/control/pair/confirm", post(pair_confirm))
        .with_state(ControlState { auth });

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let make = app.into_make_service_with_connect_info::<SocketAddr>();

    let join = tokio::spawn(async move {
        let server = axum::serve(listener, make).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });
        if let Err(e) = server.await {
            warn!(error = %e, "control endpoint stopped with error");
        }
    });

    Ok(ControlHandle {
        local_addr,
        shutdown_tx: Some(shutdown_tx),
        join,
    })
}

async fn pair_confirm(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<ControlState>,
    Json(req): Json<ConfirmRequest>,
) -> Result<Json<ConfirmResponse>, StatusCode> {
    if !addr.ip().is_loopback() {
        warn!(peer = %addr, "rejecting non-loopback control request");
        return Err(StatusCode::FORBIDDEN);
    }
    match state.auth.pairing().confirm(&req.code) {
        Some(token) => {
            info!(
                code = %req.code,
                client_id = %token.client_id,
                token = %redact_token(&token.token),
                "pairing confirmed via control endpoint",
            );
            Ok(Json(ConfirmResponse {
                ok: true,
                client_id: Some(token.client_id),
                message: "paired".into(),
            }))
        }
        None => Ok(Json(ConfirmResponse {
            ok: false,
            client_id: None,
            message: "unknown or expired code".into(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::TokenStore;
    use crate::config::SecuritySection;

    #[tokio::test]
    async fn loopback_post_confirms_pairing() {
        let store = TokenStore::in_memory();
        let auth = AuthConfig::build(
            &SecuritySection {
                require_token: true,
                pairing_enabled: true,
                ..SecuritySection::default()
            },
            store,
        );
        let (code, mut rx) = auth.pairing().request("vaio-p");
        let mut handle = serve_control("127.0.0.1:0", auth).await.unwrap();

        let url = format!("http://{}/control/pair/confirm", handle.local_addr);
        let body = format!("{{\"code\":\"{code}\"}}");

        // Use a tiny std-only HTTP client to avoid adding reqwest.
        let resp = http_post(&url, &body).await.unwrap();
        assert!(resp.contains("\"ok\":true"));

        // The waiting receiver gets the issued token.
        let token = rx
            .try_recv()
            .or_else(|_| {
                std::thread::sleep(std::time::Duration::from_millis(50));
                rx.try_recv()
            })
            .expect("token issued to session");
        assert_eq!(token.client_id, "vaio-p");
        assert!(!token.token.is_empty());

        handle.shutdown();
        handle.join().await;
    }

    async fn http_post(url: &str, body: &str) -> std::io::Result<String> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        // url = http://host:port/path
        let after = url.strip_prefix("http://").unwrap();
        let (hostport, path) = after.split_once('/').unwrap();
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
}
