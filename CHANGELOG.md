# Changelog

All notable changes to PHOSPHOR are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project aspires to follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Project skeleton (Milestone 0): LICENSE, SECURITY.md, CONTRIBUTING.md,
  Rust workspace with `ph0sphor-core`, `ph0sphor-protocol`,
  `ph0sphor-server`, `ph0sphor-client`, `ph0sphorctl` crates.
- `docs/` stubs for design, protocol, performance budget, security model,
  VAIO P client setup, configuration, and roadmap.
- Example server and client TOML configs.
- Placeholder Protobuf schema in `proto/ph0sphor.proto`.
- GitHub Actions CI workflow (fmt, clippy, build, test).
- Issue templates for bug reports and feature requests.
- Protocol (Milestone 1): full Protobuf schema for `Envelope`, `Hello`,
  `AuthRequest`/`AuthResponse`, `PairingRequest`/`PairingChallenge`/
  `PairingConfirm`, `FullSnapshot`, `DeltaUpdate`, `Event`, `Ping`/`Pong`
  and `ErrorMessage`, generated via `prost` with a vendored `protoc`.
- `ph0sphor-protocol::encode`/`decode` (Protobuf, version-checked) and
  `encode_json`/`decode_json`/`encode_json_pretty` (debug mirror).
- Domain↔wire conversions (`From` impls) for `Snapshot`, `CpuMetrics`,
  `MemoryMetrics`, `DiskMetrics`, `NetworkMetrics`.
- Reusable protocol fixtures and a 9-test round-trip suite covering
  Hello, FullSnapshot (incl. through-domain), DeltaUpdate, Event, JSON
  parity with Protobuf, and version-mismatch handling.
- Server (Milestone 2): `ph0sphor-server` becomes a real binary built on
  `tokio` + `axum` 0.7 (ws) + `sysinfo`. Components:
  - TOML config loader (`ServerConfig` mirroring README §18.1), with
    `interval_ms`/`interval_sec` aliasing and `demo()` defaults.
  - In-memory `State` store using `std::sync::RwLock<Snapshot>` and a
    `tokio::sync::watch` notify channel so per-client tasks wake on
    change instead of polling.
  - CPU/memory/disk/network/uptime collectors as independent `tokio`
    tasks with `MissedTickBehavior::Skip` and cooperative shutdown via
    `Notify`. Missing metrics become `None`, not fatal errors.
  - Demo collector producing deterministic sinusoidal telemetry for
    `--demo`, integration tests and future screenshots.
  - Token auth stub (`AuthConfig::validate`) with constant-time compare
    against a config-supplied allowlist; `require_token = false` paths
    accept any token. Pairing lands in Milestone 5.
  - WebSocket endpoint at `/ws` with the Hello → AuthRequest →
    AuthResponse → FullSnapshot(initial) → FullSnapshot-on-change
    handshake, Ping/Pong handling and a 5 s safety snapshot tick.
    Graceful shutdown via `axum::serve(...).with_graceful_shutdown`.
  - CLI flags `--config`, `--demo`, `--version`, `--help` with structured
    `tracing-subscriber` logging.
- Integration test boots a real server on a loopback ephemeral port,
  connects via `tokio-tungstenite`, completes the handshake and asserts
  CPU/RAM/disk/network all present — the Milestone 2 "done" criterion.
- Client (Milestone 3): `ph0sphor-client` becomes a real TUI on
  `ratatui` 0.29 + `crossterm` 0.28, with a single-worker tokio runtime
  to keep the VAIO P budget honest.
  - `config.rs`: `ClientConfig` mirrors README §18.2; parses
    `examples/client.toml` verbatim (asserted by a unit test).
  - `theme.rs`: five palettes (`phosphor-green`, `amber-crt`,
    `ice-terminal`, `mono-lcd`, `high-contrast`) with `C`-key cycling.
  - `state.rs`: `AppState` owns current snapshot, connection status,
    current screen and a bounded event log. Single `apply(AppEvent)`
    entry point returns whether a redraw is warranted.
  - `event.rs`: one `AppEvent` enum (key/tick/snapshot/connection/log/
    quit) flowing through a single `mpsc::Receiver`. `LogLine` and
    `LogSeverity` model client- and server-originating notices.
  - `net.rs`: production WS client task with Hello → AuthRequest →
    AuthResponse → Snapshot stream, exponential backoff reconnect
    (1 s → 30 s), wire→domain `Snapshot` conversion at the boundary,
    and a `spawn_demo` source for `--demo`.
  - `ui.rs`: HOME (CPU/RAM/disk gauges + recent events), SYS (detailed
    CPU/memory/swap/disks/network), LOG (full scrollback), status bar
    with screen tabs and theme/mute/refresh hints. ASCII fallback ready.
  - `app.rs`: low-power render loop — draw on dirty events only, 1 Hz
    clock tick task drives the on-screen clock, input task uses
    `crossterm::event::EventStream`. Cooperative shutdown via `Notify`.
  - `main.rs`: CLI flags `--config`, `--server`, `--token`, `--demo`,
    `--version`, `--help`; raw-mode + alternate-screen lifecycle with a
    panic hook that always restores the terminal.
- Integration test in `crates/ph0sphor-client/tests/handshake.rs` boots
  a real `ph0sphor-server` on a loopback port, runs the production
  client WS task against it, and asserts the snapshot the TUI would
  render carries live CPU/RAM/disk/network — the Milestone 3 "done"
  criterion at the network layer.
