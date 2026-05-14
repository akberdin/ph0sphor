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
