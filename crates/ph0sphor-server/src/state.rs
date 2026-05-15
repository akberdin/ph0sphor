//! In-memory state store.
//!
//! Single source of truth for the current telemetry snapshot. Collectors
//! write partial updates; per-client tasks read full snapshots and stream
//! them out. A `tokio::sync::watch` channel notifies subscribers on change.

use ph0sphor_core::{
    CpuMetrics, DiskMetrics, MailSummary, MemoryMetrics, NetworkMetrics, Snapshot, WeatherInfo,
};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use tokio::sync::watch;

#[derive(Debug, Clone)]
pub struct State(Arc<Inner>);

#[derive(Debug)]
struct Inner {
    snapshot: RwLock<Snapshot>,
    notify_tx: watch::Sender<u64>,
}

impl State {
    pub fn new(hostname: String, os: String) -> Self {
        let snapshot = Snapshot {
            timestamp_unix_ms: now_unix_ms(),
            hostname,
            os,
            ..Snapshot::default()
        };
        let (notify_tx, _) = watch::channel(0);
        Self(Arc::new(Inner {
            snapshot: RwLock::new(snapshot),
            notify_tx,
        }))
    }

    /// Subscribe for change notifications. The yielded value is an
    /// opaque tick; receivers should re-read [`State::snapshot`] when it
    /// changes.
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.0.notify_tx.subscribe()
    }

    /// Cheap clone of the current snapshot.
    pub fn snapshot(&self) -> Snapshot {
        self.0.snapshot.read().expect("state poisoned").clone()
    }

    pub fn update_cpu(&self, cpu: CpuMetrics) {
        self.update(|s| s.cpu = cpu);
    }

    pub fn update_memory(&self, mem: MemoryMetrics) {
        self.update(|s| s.memory = mem);
    }

    pub fn update_disks(&self, disks: Vec<DiskMetrics>) {
        self.update(|s| s.disks = disks);
    }

    pub fn update_network(&self, net: Vec<NetworkMetrics>) {
        self.update(|s| s.network = net);
    }

    pub fn update_uptime(&self, uptime_secs: u64) {
        self.update(|s| s.uptime_secs = uptime_secs);
    }

    pub fn update_mail(&self, mail: MailSummary) {
        self.update(|s| s.mail = Some(mail));
    }

    pub fn update_weather(&self, weather: WeatherInfo) {
        self.update(|s| s.weather = Some(weather));
    }

    fn update<F: FnOnce(&mut Snapshot)>(&self, f: F) {
        {
            let mut snap = self.0.snapshot.write().expect("state poisoned");
            snap.timestamp_unix_ms = now_unix_ms();
            f(&mut snap);
        }
        self.0.notify_tx.send_modify(|t| *t = t.wrapping_add(1));
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updates_bump_notify_tick() {
        let s = State::new("h".into(), "linux".into());
        let mut rx = s.subscribe();
        let initial = *rx.borrow_and_update();
        s.update_cpu(CpuMetrics {
            usage_percent: 42.0,
            temperature_c: None,
            core_count: Some(4),
        });
        let after = *rx.borrow();
        assert_ne!(initial, after);
        assert_eq!(s.snapshot().cpu.usage_percent, 42.0);
    }
}
