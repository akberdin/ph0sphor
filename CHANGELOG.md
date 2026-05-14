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
