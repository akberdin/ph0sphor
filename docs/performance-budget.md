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

## Measurement

The server publishes its own resource usage on the ABOUT screen and in the
LOG screen at INFO level. The client measures its own RSS and CPU usage in
debug builds and surfaces them only when explicitly enabled.

Regressions against these budgets are treated as bugs.
