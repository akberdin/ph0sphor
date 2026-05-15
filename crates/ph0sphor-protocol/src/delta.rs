//! Delta encoding helpers.
//!
//! Computes a [`wire::DeltaUpdate`] from two snapshots and applies one
//! to a domain [`Snapshot`]. The shape mirrors the proto3 schema: every
//! scalar field carries its own `optional`, so an absent field means
//! "unchanged". Repeated fields are sent in full when any element
//! changes (per README §10.3: prefer fewer meaningful updates over
//! many tiny ones).
//!
//! Floating-point fields are compared with a small epsilon to suppress
//! the high-frequency noise CPU collectors normally produce.

use crate::wire;
use ph0sphor_core::Snapshot;

/// Smallest CPU-usage delta worth re-transmitting. CPU collectors
/// produce 0.1-percentage-point jitter even at steady load; sending
/// every tick would defeat delta compression.
pub const CPU_USAGE_EPSILON: f32 = 0.5;

/// Smallest CPU-temperature delta worth re-transmitting.
pub const CPU_TEMP_EPSILON: f32 = 0.5;

/// Build a [`wire::DeltaUpdate`] representing what changed from `prev`
/// to `cur`. The returned delta carries `cur.timestamp_unix_ms`.
pub fn compute_delta(prev: &wire::FullSnapshot, cur: &wire::FullSnapshot) -> wire::DeltaUpdate {
    let mut d = wire::DeltaUpdate {
        timestamp_unix_ms: cur.timestamp_unix_ms,
        ..Default::default()
    };

    match (prev.cpu.as_ref(), cur.cpu.as_ref()) {
        (Some(p), Some(c)) => {
            if (p.usage_percent - c.usage_percent).abs() >= CPU_USAGE_EPSILON {
                d.cpu_usage_percent = Some(c.usage_percent);
            }
            if !approx_eq_opt(p.temperature_c, c.temperature_c, CPU_TEMP_EPSILON) {
                d.cpu_temperature_c = c.temperature_c;
            }
        }
        (None, Some(c)) => {
            d.cpu_usage_percent = Some(c.usage_percent);
            d.cpu_temperature_c = c.temperature_c;
        }
        _ => {}
    }

    match (prev.memory.as_ref(), cur.memory.as_ref()) {
        (Some(p), Some(c)) => {
            if p.used_bytes != c.used_bytes {
                d.memory_used_bytes = Some(c.used_bytes);
            }
            if p.total_bytes != c.total_bytes {
                d.memory_total_bytes = Some(c.total_bytes);
            }
        }
        (None, Some(c)) => {
            d.memory_used_bytes = Some(c.used_bytes);
            d.memory_total_bytes = Some(c.total_bytes);
        }
        _ => {}
    }

    if prev.disks != cur.disks {
        d.disks = cur.disks.clone();
    }
    if prev.network != cur.network {
        d.network = cur.network.clone();
    }

    // Useful-features payloads (M6). Compared with structural equality;
    // any change resends the whole sub-message.
    if prev.mail != cur.mail {
        d.mail = cur.mail.clone();
    }
    if prev.weather != cur.weather {
        d.weather = cur.weather.clone();
    }

    d
}

/// True when the delta carries no field changes — only the timestamp.
/// Empty deltas should not be put on the wire.
pub fn is_empty(d: &wire::DeltaUpdate) -> bool {
    d.cpu_usage_percent.is_none()
        && d.cpu_temperature_c.is_none()
        && d.memory_used_bytes.is_none()
        && d.memory_total_bytes.is_none()
        && d.disks.is_empty()
        && d.network.is_empty()
        && d.mail.is_none()
        && d.weather.is_none()
}

/// Apply a delta to a domain [`Snapshot`]. Fields not mentioned by the
/// delta are left untouched.
pub fn apply_delta(snap: &mut Snapshot, delta: &wire::DeltaUpdate) {
    if delta.timestamp_unix_ms != 0 {
        snap.timestamp_unix_ms = delta.timestamp_unix_ms;
    }
    if let Some(v) = delta.cpu_usage_percent {
        snap.cpu.usage_percent = v;
    }
    if let Some(v) = delta.cpu_temperature_c {
        snap.cpu.temperature_c = Some(v);
    }
    if let Some(v) = delta.memory_used_bytes {
        snap.memory.used_bytes = v;
    }
    if let Some(v) = delta.memory_total_bytes {
        snap.memory.total_bytes = v;
    }
    if !delta.disks.is_empty() {
        snap.disks = delta.disks.iter().map(Into::into).collect();
    }
    if !delta.network.is_empty() {
        snap.network = delta.network.iter().map(Into::into).collect();
    }
    if let Some(m) = delta.mail.as_ref() {
        snap.mail = Some(m.into());
    }
    if let Some(w) = delta.weather.as_ref() {
        snap.weather = Some(w.into());
    }
}

fn approx_eq_opt(a: Option<f32>, b: Option<f32>, eps: f32) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => (x - y).abs() < eps,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{sample_domain_snapshot, sample_full_snapshot};

    #[test]
    fn identical_snapshots_produce_empty_delta() {
        let s = sample_full_snapshot();
        let d = compute_delta(&s, &s);
        assert!(is_empty(&d), "delta should be empty: {d:?}");
    }

    #[test]
    fn cpu_usage_change_above_epsilon_is_captured() {
        let mut prev = sample_full_snapshot();
        let mut cur = prev.clone();
        // Mutate cpu usage by 5pp.
        cur.cpu.as_mut().unwrap().usage_percent = prev.cpu.as_ref().unwrap().usage_percent + 5.0;

        let d = compute_delta(&prev, &cur);
        assert_eq!(
            d.cpu_usage_percent,
            Some(prev.cpu.unwrap().usage_percent + 5.0)
        );
        assert!(d.cpu_temperature_c.is_none());
        prev = cur;
        let d2 = compute_delta(&prev, &prev);
        assert!(is_empty(&d2));
    }

    #[test]
    fn cpu_usage_change_below_epsilon_is_filtered() {
        let prev = sample_full_snapshot();
        let mut cur = prev.clone();
        cur.cpu.as_mut().unwrap().usage_percent = prev.cpu.as_ref().unwrap().usage_percent + 0.2; // < 0.5 epsilon

        let d = compute_delta(&prev, &cur);
        assert!(
            d.cpu_usage_percent.is_none(),
            "below-epsilon noise must be filtered"
        );
    }

    #[test]
    fn memory_change_is_captured_exactly() {
        let prev = sample_full_snapshot();
        let mut cur = prev.clone();
        cur.memory.as_mut().unwrap().used_bytes += 1_048_576;

        let d = compute_delta(&prev, &cur);
        assert_eq!(
            d.memory_used_bytes,
            Some(prev.memory.unwrap().used_bytes + 1_048_576)
        );
    }

    #[test]
    fn disk_change_resends_full_vector() {
        let prev = sample_full_snapshot();
        let mut cur = prev.clone();
        cur.disks[0].used_bytes += 1_000_000_000;

        let d = compute_delta(&prev, &cur);
        assert_eq!(d.disks.len(), cur.disks.len());
        assert_eq!(d.disks[0].used_bytes, cur.disks[0].used_bytes);
    }

    #[test]
    fn apply_delta_round_trips_into_domain() {
        let mut domain = sample_domain_snapshot();
        let prev_wire = (&domain).into();
        let mut cur_wire: wire::FullSnapshot = (&domain).into();
        cur_wire.cpu.as_mut().unwrap().usage_percent = domain.cpu.usage_percent + 7.5;
        cur_wire.memory.as_mut().unwrap().used_bytes += 500_000_000;
        cur_wire.timestamp_unix_ms += 1000;

        let delta = compute_delta(&prev_wire, &cur_wire);
        apply_delta(&mut domain, &delta);

        assert!((domain.cpu.usage_percent - (cur_wire.cpu.unwrap().usage_percent)).abs() < 1e-3);
        assert_eq!(
            domain.memory.used_bytes,
            cur_wire.memory.unwrap().used_bytes
        );
        assert_eq!(domain.timestamp_unix_ms, cur_wire.timestamp_unix_ms);
    }

    #[test]
    fn empty_delta_does_not_change_snapshot() {
        let mut domain = sample_domain_snapshot();
        let before = domain.clone();
        let empty = wire::DeltaUpdate {
            timestamp_unix_ms: 0,
            ..Default::default()
        };
        apply_delta(&mut domain, &empty);
        assert_eq!(domain.cpu.usage_percent, before.cpu.usage_percent);
        assert_eq!(domain.memory.used_bytes, before.memory.used_bytes);
        assert_eq!(domain.disks, before.disks);
    }
}
