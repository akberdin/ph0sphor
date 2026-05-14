# Performance Budget

Performance is a feature. These budgets are normative for MVP and any
later release. See README §13 for the canonical statement.

## Server

```text
Idle CPU usage:           ~0%
Normal CPU usage:         < 1-2% on a modern workstation
Memory usage:             < 50-100 MB where practical
Network usage:            normally < few KB/s
Disk writes:              minimal
Default telemetry rate:   1 Hz for primary metrics
```

Forbidden patterns:

- Busy loops.
- Unbounded queues.
- Excessive logging.
- Excessive disk writes.
- High-frequency polling of slow sensors.
- Repeated expensive process scans.
- Sending unchanged full snapshots every tick.

## Client (Sony VAIO P)

```text
Render rate:              1-2 FPS by default
Memory usage:             < 30-50 MB where practical
CPU usage:                as low as possible
Network usage:            normally < 1-5 KB/s
Offline usability:        required
```

Forbidden patterns:

- Heavy animations.
- Constant redraws.
- Complex layout recalculation every tick.
- Large history storage.
- Blocking network calls on the UI path.
- Parsing large JSON payloads in production mode.

## Coalescing and Delta Encoding

The server reacts to state changes through a `tokio::sync::watch` channel
(bounded by 1, oldest-value-wins) but does **not** put one envelope on
the wire per change. Instead each per-client session keeps a model of
what the connected client has already seen and:

- coalesces incoming notifications into one payload every
  `performance.min_send_interval_ms` (default 500 ms);
- emits a `DeltaUpdate` (only fields that changed) when
  `performance.send_deltas_only = true` (the default), and a
  `FullSnapshot` otherwise;
- emits a `FullSnapshot` every `performance.full_snapshot_interval_sec`
  (default 60 s) as a drift-safety mechanism, and once at handshake;
- suppresses deltas that resolve to zero changed fields (post-epsilon).

CPU usage and CPU temperature use a 0.5 pp epsilon
(`ph0sphor_protocol::delta::CPU_USAGE_EPSILON` /
`CPU_TEMP_EPSILON`) so sub-percent jitter from the collector never
reaches the client.

## Bounded Queues

Every queue between tasks has a finite upper bound:

- **Server `State` notify**: `tokio::sync::watch` — capacity 1, the
  oldest pending value is replaced on a new update. Slow clients cannot
  hold back the collectors.
- **Server WebSocket sends**: every send is `.await`ed, so the
  underlying socket's flow control is the queue. No unbounded buffering.
- **Client app channel** (`app::run`): `tokio::sync::mpsc::channel(64)`
  — small enough that a backpressure stall is observable; key/tick/log
  tasks never burst high enough to hit it under the performance budget.

## Self-Monitoring

Both sides log a structured summary at the end of every WebSocket
session. The server logs at INFO with `bytes_sent`, `full_snapshots`,
`deltas`, `suppressed` and average bytes-per-second. The client pushes
the same numbers into its on-screen LOG and the standard
`AppEvent::Log` stream so they are visible in the TUI when the link
drops.

## Measurement

The server publishes its own resource usage on the ABOUT screen and in the
LOG screen at INFO level. The client measures its own RSS and CPU usage in
debug builds and surfaces them only when explicitly enabled.

Regressions against these budgets are treated as bugs.
