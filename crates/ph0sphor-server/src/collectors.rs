//! Telemetry collectors.
//!
//! Each collector is a long-running `tokio` task that wakes on its
//! configured interval, refreshes its `sysinfo` view of the host, and
//! writes a partial update into the shared [`State`]. Collectors must:
//!
//! - never run faster than their configured interval,
//! - never block the runtime (no synchronous sleeps),
//! - treat missing metrics as `None` rather than as a fatal error,
//! - cooperate with shutdown via a `Notify`.
//!
//! The demo collector exists for `--demo` and integration tests; it
//! produces plausible synthetic telemetry without touching the host.

use crate::config::CollectorsSection;
use crate::state::State;
use ph0sphor_core::{CpuMetrics, DiskMetrics, MemoryMetrics, NetworkMetrics};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System};
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;
use tracing::{info, warn};

/// Handle returned from [`spawn_real`] / [`spawn_demo`] so callers can
/// stop collectors during shutdown and wait for them to drain.
#[derive(Debug)]
pub struct Collectors {
    handles: Vec<JoinHandle<()>>,
    shutdown: Arc<Notify>,
}

impl Collectors {
    pub fn shutdown(&self) {
        // Wake all collector tasks; each shuts down on its next select.
        self.shutdown.notify_waiters();
    }

    pub async fn join(self) {
        for h in self.handles {
            let _ = h.await;
        }
    }
}

/// Spawn the four real collectors (CPU/memory/disk/network) per the
/// given configuration. Disabled collectors are silently skipped.
pub fn spawn_real(state: State, cfg: &CollectorsSection) -> Collectors {
    let shutdown = Arc::new(Notify::new());
    let mut handles = Vec::new();

    if cfg.cpu.enabled {
        handles.push(tokio::spawn(run_cpu(
            state.clone(),
            cfg.cpu.interval(),
            shutdown.clone(),
        )));
    }
    if cfg.memory.enabled {
        handles.push(tokio::spawn(run_memory(
            state.clone(),
            cfg.memory.interval(),
            shutdown.clone(),
        )));
    }
    if cfg.disk.enabled {
        handles.push(tokio::spawn(run_disk(
            state.clone(),
            cfg.disk.interval(),
            shutdown.clone(),
        )));
    }
    if cfg.network.enabled {
        handles.push(tokio::spawn(run_network(
            state.clone(),
            cfg.network.interval(),
            shutdown.clone(),
        )));
    }
    handles.push(tokio::spawn(run_uptime(state, shutdown.clone())));

    info!(count = handles.len(), "collectors spawned");
    Collectors { handles, shutdown }
}

/// Spawn the demo collector. Produces deterministic-ish synthetic data
/// so screenshots, integration tests and offline demos work without
/// touching the host.
pub fn spawn_demo(state: State) -> Collectors {
    let shutdown = Arc::new(Notify::new());
    let handle = tokio::spawn(run_demo(state, shutdown.clone()));
    Collectors {
        handles: vec![handle],
        shutdown,
    }
}

// ---------------------------------------------------------------------------
// CPU
// ---------------------------------------------------------------------------

async fn run_cpu(state: State, interval: Duration, shutdown: Arc<Notify>) {
    let mut sys =
        System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));
    // First refresh primes the counters; the second one (a tick later)
    // gives the actual usage delta.
    sys.refresh_cpu_usage();
    let mut ticker = tokio::time::interval(interval.max(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            _ = ticker.tick() => {
                sys.refresh_cpu_usage();
                let usage = sys.global_cpu_usage();
                state.update_cpu(CpuMetrics {
                    usage_percent: usage,
                    temperature_c: None, // Component temperatures land post-MVP.
                    core_count: Some(sys.cpus().len() as u32),
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Memory
// ---------------------------------------------------------------------------

async fn run_memory(state: State, interval: Duration, shutdown: Arc<Notify>) {
    let mut sys =
        System::new_with_specifics(RefreshKind::new().with_memory(MemoryRefreshKind::everything()));
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            _ = ticker.tick() => {
                sys.refresh_memory();
                state.update_memory(MemoryMetrics {
                    used_bytes: sys.used_memory(),
                    total_bytes: sys.total_memory(),
                    swap_used_bytes: Some(sys.used_swap()),
                    swap_total_bytes: Some(sys.total_swap()),
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Disk
// ---------------------------------------------------------------------------

async fn run_disk(state: State, interval: Duration, shutdown: Arc<Notify>) {
    let mut disks = Disks::new_with_refreshed_list();
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            _ = ticker.tick() => {
                disks.refresh();
                let snap: Vec<DiskMetrics> = disks
                    .iter()
                    .map(|d| {
                        let total = d.total_space();
                        let avail = d.available_space();
                        DiskMetrics {
                            mount: d.mount_point().display().to_string(),
                            used_bytes: total.saturating_sub(avail),
                            total_bytes: total,
                            temperature_c: None,
                        }
                    })
                    .collect();
                if snap.is_empty() {
                    warn!("disk collector: no disks reported by sysinfo");
                }
                state.update_disks(snap);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------

async fn run_network(state: State, interval: Duration, shutdown: Arc<Notify>) {
    let mut networks = Networks::new_with_refreshed_list();
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut last = std::time::Instant::now();

    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            _ = ticker.tick() => {
                networks.refresh();
                let now = std::time::Instant::now();
                let elapsed = now.duration_since(last).as_secs_f64().max(0.001);
                last = now;

                let snap: Vec<NetworkMetrics> = networks
                    .iter()
                    .filter(|(_, data)| {
                        // Skip loopback-only interfaces with zero counters.
                        data.total_received() > 0 || data.total_transmitted() > 0
                    })
                    .map(|(iface, data)| NetworkMetrics {
                        interface: iface.clone(),
                        rx_bytes_per_sec: ((data.received() as f64) / elapsed) as u64,
                        tx_bytes_per_sec: ((data.transmitted() as f64) / elapsed) as u64,
                        rx_total_bytes: Some(data.total_received()),
                        tx_total_bytes: Some(data.total_transmitted()),
                    })
                    .collect();
                state.update_network(snap);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Uptime (cheap, runs once a second alongside other collectors)
// ---------------------------------------------------------------------------

async fn run_uptime(state: State, shutdown: Arc<Notify>) {
    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            _ = ticker.tick() => {
                state.update_uptime(System::uptime());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Demo collector
// ---------------------------------------------------------------------------

async fn run_demo(state: State, shutdown: Arc<Notify>) {
    let mut ticker = tokio::time::interval(Duration::from_millis(500));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut tick: u64 = 0;

    // Seed the state with a single update before the first tick so any
    // client that connects immediately gets a non-empty snapshot.
    seed_demo_state(&state);

    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            _ = ticker.tick() => {
                tick = tick.wrapping_add(1);
                let phase = (tick as f32) * 0.2;
                let cpu = 40.0 + 25.0 * phase.sin();
                state.update_cpu(CpuMetrics {
                    usage_percent: cpu.clamp(0.0, 100.0),
                    temperature_c: Some(55.0 + 10.0 * phase.cos()),
                    core_count: Some(8),
                });
                state.update_memory(MemoryMetrics {
                    used_bytes: 8_000_000_000 + (tick % 20) * 50_000_000,
                    total_bytes: 16_000_000_000,
                    swap_used_bytes: Some(0),
                    swap_total_bytes: Some(8_000_000_000),
                });
                state.update_network(vec![NetworkMetrics {
                    interface: "demo0".into(),
                    rx_bytes_per_sec: 1_000 + (tick % 50) * 100,
                    tx_bytes_per_sec: 500 + (tick % 30) * 50,
                    rx_total_bytes: Some(tick * 100_000),
                    tx_total_bytes: Some(tick * 40_000),
                }]);
                state.update_uptime(60 + tick);
            }
        }
    }
}

fn seed_demo_state(state: &State) {
    state.update_cpu(CpuMetrics {
        usage_percent: 42.0,
        temperature_c: Some(55.0),
        core_count: Some(8),
    });
    state.update_memory(MemoryMetrics {
        used_bytes: 8_000_000_000,
        total_bytes: 16_000_000_000,
        swap_used_bytes: Some(0),
        swap_total_bytes: Some(8_000_000_000),
    });
    state.update_disks(vec![DiskMetrics {
        mount: "/".into(),
        used_bytes: 250_000_000_000,
        total_bytes: 500_000_000_000,
        temperature_c: None,
    }]);
    state.update_network(vec![NetworkMetrics {
        interface: "demo0".into(),
        rx_bytes_per_sec: 1_000,
        tx_bytes_per_sec: 500,
        rx_total_bytes: Some(0),
        tx_total_bytes: Some(0),
    }]);
    state.update_uptime(60);
}
