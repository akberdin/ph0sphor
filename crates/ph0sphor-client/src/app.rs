//! App loop: orchestrates network, input, clock and rendering.

use crate::config::ClientConfig;
use crate::event::AppEvent;
use crate::net;
use crate::state::AppState;
use crate::ui;
use crossterm::event::{Event as CtEvent, EventStream};
use futures_util::StreamExt;
use ratatui::backend::Backend;
use ratatui::Terminal;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, Notify};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Configuration knobs that control the app loop without going through
/// the on-disk TOML config.
#[derive(Debug, Default, Clone, Copy)]
pub struct RunOptions {
    pub demo: bool,
}

/// Top-level entry point.
///
/// Drives the app loop until the user quits or shuts down. Returns the
/// final exit code: `Ok(())` for clean shutdown.
pub async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    config: ClientConfig,
    options: RunOptions,
) -> Result<(), AppError> {
    let (tx, mut rx) = mpsc::channel::<AppEvent>(64);
    let shutdown = Arc::new(Notify::new());

    // Input task: forwards crossterm key events into the app channel.
    let input_handle = spawn_input(tx.clone(), shutdown.clone());

    // Clock tick: 1 Hz, drives the on-screen clock without depending on
    // server snapshots arriving.
    let clock_handle = spawn_clock(tx.clone(), shutdown.clone());

    // Data source: real WS client or demo source.
    let net_handle = if options.demo {
        net::spawn_demo(tx.clone())
    } else {
        net::spawn(
            config.client.server.clone(),
            config.client.client_name.clone(),
            config.client.token.clone(),
            tx.clone(),
            shutdown.clone(),
        )
    };

    let mut state = AppState::new(config);

    // Initial draw before any event arrives.
    terminal.draw(|f| ui::draw(f, &state))?;

    while let Some(event) = rx.recv().await {
        let dirty = state.apply(event);
        if state.quit {
            break;
        }
        if dirty {
            terminal.draw(|f| ui::draw(f, &state))?;
        }
    }

    // Signal background tasks to stop and wait for them to drain.
    shutdown.notify_waiters();
    drop(tx); // close the channel so input/clock tasks exit
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        let _ = input_handle.await;
        let _ = clock_handle.await;
        let _ = net_handle.await;
    })
    .await;

    Ok(())
}

fn spawn_input(tx: mpsc::Sender<AppEvent>, shutdown: Arc<Notify>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut events = EventStream::new();
        loop {
            tokio::select! {
                _ = shutdown.notified() => return,
                next = events.next() => match next {
                    None => return,
                    Some(Err(_)) => return,
                    Some(Ok(CtEvent::Key(k))) => {
                        if tx.send(AppEvent::Key(k)).await.is_err() {
                            return;
                        }
                    }
                    Some(Ok(CtEvent::Resize(_, _))) => {
                        // A resize requires a redraw; we synthesize a Tick.
                        if tx.send(AppEvent::Tick).await.is_err() {
                            return;
                        }
                    }
                    Some(Ok(_)) => {}
                },
            }
        }
    })
}

fn spawn_clock(tx: mpsc::Sender<AppEvent>, shutdown: Arc<Notify>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(1));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // First tick fires immediately; skip it so the initial render
        // (already done before this task started) isn't repeated.
        ticker.tick().await;
        loop {
            tokio::select! {
                _ = shutdown.notified() => return,
                _ = ticker.tick() => {
                    if tx.send(AppEvent::Tick).await.is_err() {
                        return;
                    }
                }
            }
        }
    })
}
