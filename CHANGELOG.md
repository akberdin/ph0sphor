# Changelog

All notable changes to PHOSPHOR are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project aspires to follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Removed
- macOS dropped from the supported server platforms. The release
  workflow no longer builds `macos-x86_64` / `macos-arm64`
  artifacts, and `packaging/macos/` has been removed along with
  its references from the README, installation guide, packaging
  index, roadmap and contributor guide.

## [0.1.0] - 2026-05-15

First tagged MVP release. Cuts the line between scaffolding and a
working server/client/ctl trio that pairs, streams telemetry and
recovers from disconnects. Linux x86_64 and i686 (VAIO P), Windows
x86_64 and macOS x86_64/arm64 binaries are published as release
assets via `.github/workflows/release.yml`.

### Added
- `docs/vaio-p-client-vm-testing.md`: antiX 32-bit VirtualBox
  walkthrough for exercising the client without real VAIO P
  hardware (NAT/Bridged networking, demo mode, pairing, manual
  test checklist).
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
- Performance pass (Milestone 4):
  - `ph0sphor-protocol::delta` with `compute_delta`, `apply_delta` and
    `is_empty`. Uses a 0.5 pp epsilon on CPU usage / CPU temperature
    so steady-state collector jitter never reaches the wire.
  - `PartialEq` derives on `ph0sphor-core` metric types so wire-level
    repeated fields (disks, network) can be compared structurally.
  - Server session loop rewrites: per-client `last_sent_wire`,
    coalescing via `performance.min_send_interval_ms` (default 500 ms),
    periodic `FullSnapshot` every `full_snapshot_interval_sec`
    (default 60 s), and a `serve_with_perf` API to thread the config.
    Default `send_deltas_only = true` flips the wire to delta-mostly.
  - Server-side per-session self-monitoring: bytes sent, full
    snapshots, deltas and suppressed empty-deltas, logged at INFO when
    the session ends with avg bytes/sec.
  - Client `AppEvent::Delta`; `state::apply` calls
    `ph0sphor_protocol::delta::apply_delta`. Client tracks
    `SessionStats` (bytes received, full / delta counts) and emits a
    summary log line on disconnect.
  - Client honours `low_power_mode`: clock tick falls from 1 s to 2 s.
  - Bounded queues documented and verified: `watch` capacity 1 on the
    server, `mpsc::channel(64)` on the client app loop, awaited socket
    sends instead of internal buffering.
  - New tests: 7 protocol delta tests (incl. epsilon noise filter,
    empty-delta no-op, domain round-trip), 1 server integration test
    (`server_emits_delta_after_state_change`), 1 client unit test
    (`delta_event_patches_snapshot_in_place`).
  - `docs/performance-budget.md` gains "Coalescing and Delta
    Encoding", "Bounded Queues" and "Self-Monitoring" sections.
- Security pass (Milestone 5):
  - `PairingConfirm.token` (field 2) added to the wire schema so the
    server can deliver issued tokens over the same WebSocket the
    client already trusts.
  - `ph0sphor-server::auth` rewritten: `TokenStore` (in-memory or
    JSON-on-disk, atomic rename + 0600 perms on Unix), `PairingManager`
    (HashMap of pending codes with TTL, oneshot per pending session),
    `AuthConfig` façade that validates against the static allowlist
    **and** the persisted store in constant time.
  - 192-bit hex tokens generated from `/dev/urandom` with a
    time-based LCG fallback; 8-character `ABCD-1234` pairing codes use
    an alphabet without `0/O/1/I` to avoid operator typos.
  - `redact_token()` keeps the first four chars then `…`; every
    server-side log site that mentions a token now goes through it.
  - `ph0sphor-server::control`: loopback-only HTTP listener with one
    route, `POST /control/pair/confirm`. `is_loopback()` check on every
    request, regardless of bind address. Wired through `serve_control`
    and a `ControlHandle` symmetric with the WS server.
  - WS session forks after `Hello`: `AuthRequest` continues the M2 path,
    `PairingRequest` triggers code issue, then blocks on the operator
    confirmation, then sends `PairingConfirm` with the freshly issued
    token. Drops gracefully on client close mid-pairing.
  - `ph0sphorctl pair confirm <code> [--server URL]`: std-only HTTP
    client (`TcpStream` + raw HTTP/1.1) so no new dependency. Default
    target is `http://127.0.0.1:7078`.
  - Client: empty `client.token` triggers the pairing flow. The TUI
    shows a dedicated PAIRING banner with the code and the exact
    `ph0sphorctl` command. `AppEvent::TokenIssued` is intercepted by
    the app loop **before** `state.apply` so the raw token is written
    to `client.token_file` (mode 0600) without ever passing through
    the visible event log. `main.rs` reads the persisted token at
    startup so already-paired clients skip pairing on reconnect.
  - `examples/server.toml` documents `control_bind`, `token_store`,
    `pairing_ttl_secs`; `examples/client.toml` documents `token` and
    `token_file`.
  - `docs/security-model.md` expanded with the full pairing flow,
    code/token lifetimes, redaction guarantees and the read-only
    default reaffirmation. `SECURITY.md` continues to be the
    user-facing summary.
  - Read-only default and "no remote command execution" reaffirmed:
    `ClientCommandRequest` is still absent from the wire schema and
    nothing in `ph0sphor-server::net` dispatches on it — the
    `allow_control_commands = true` flag is a no-op until a future
    milestone wires an allowlisted set of commands.
  - 5 new auth unit tests (redact, pairing request/confirm,
    unknown-code, expired-code, store-persist), 1 new control unit
    test (loopback POST → token issued), 1 new client integration test
    `client_pairs_then_receives_token_and_snapshot` exercising the
    full server + WS + control + client flow.
- Useful features (Milestone 6):
  - Wire schema gains `MailSummary` / `MailItem` / `WeatherInfo` and a
    `MailPrivacy` enum. Carried by both `FullSnapshot` and
    `DeltaUpdate` (optional message types). Domain types added to
    `ph0sphor-core` with `PartialEq`.
  - `compute_delta` and `apply_delta` now ferry mail/weather changes;
    `is_empty` recognises the new fields too.
  - Server: file-based ingest collectors `run_mail` and `run_weather`
    that read operator-managed JSON files (`collectors.mail.source`,
    `collectors.weather.source`). Mail privacy is enforced server-side
    with `count_only` / `sender_subject` / `preview` stripping fields
    before they reach the wire (README §14.5). Demo collector seeds
    plausible mail and weather data for `--demo` / screenshots.
  - Client: `Screen` enum extended to six (Home/Sys/Mail/Time/Weather/
    Log) with digit keys 1-6; status bar tabs all visible.
  - `TimeState` provides local timer (T toggle, R reset, +/- adjust),
    stopwatch (W toggle, R reset) and HH:MM UTC alarms loaded from
    `[time].alarms`. Timer completion logs `TIMER: completed`; alarms
    log `ALARM: HH:MM UTC` at Critical severity (unless muted).
  - Client-side new-mail detector compares each incoming snapshot
    against the last seen `unread_count`; first snapshot only seeds
    the baseline so the user doesn't see a phantom flood at startup.
  - New TUI screens `render_mail`, `render_time`, `render_weather`
    with privacy-aware mail rendering and a phosphor-style time card.
  - `examples/server.toml` and `examples/client.toml` updated;
    `docs/configuration.md` documents the JSON schemas the operator
    populates for mail and weather.
  - 4 new client unit tests (six-screen navigation, timer toggles,
    `parse_hhmm_to_minute_of_day`, new-mail detector seeding).
- VAIO P polish (Milestone 7):
  - `ph0sphor-client::local` reads `/sys/class/power_supply/BAT*`
    (`BAT0`/`BAT1`/...), the default outbound IP via a UDP-connect
    trick, and the first up non-loopback interface from
    `/sys/class/net/`. No new dependency. Refreshed on every Tick so
    the header keeps updating during a disconnect.
  - Header now carries `BAT XX% CHG/DSC/FUL` and `NET wlan0 192.168...`.
    Battery turns warning-yellow below 30 % and critical-red below
    15 % on discharge.
  - `ui.compact_mode` actually does something now: collapses the
    header to a single line and shrinks `Constraint::Length(3)` to
    `Length(2)` so a 24-row VAIO P TTY still gets a usable body.
  - `ui.ascii_fallback` switches every box-drawing border to
    `+ - |`, swaps `Gauge::use_unicode` to false, replaces `°C` with
    `C`, `↓/↑` with `dn/up`, and `::` in the title with `:`. Every
    bordered block now goes through the new `block_with_borders`
    helper to make this consistent.
  - `docs/vaio-p-client.md` rewritten as a real appliance setup guide:
    hardware/OS choices, Cozette/Spleen/Terminus font table,
    1600×768 layout regimes, hardened `systemd` unit
    (`ProtectSystem=strict`, `NoNewPrivileges=yes`,
    `TTYPath=/dev/tty1`), Alpine/OpenRC autologin recipe, pairing
    walkthrough, and operational notes.
  - 2 new client unit tests in `local` (smoke-test refresh on any
    host, battery status labels are three chars wide).
- Packaging and releases (Milestone 8):
  - `.github/workflows/release.yml`: a five-target matrix release
    workflow (linux-x86_64, linux-i686, windows-x86_64,
    macos-x86_64, macos-arm64) that fires on every `v*.*.*` tag.
    Linux i686 client (the VAIO P target) is cross-built via
    `cross` against `i686-unknown-linux-musl` so it's static.
    Server + ctl ride the other targets. Each archive ships
    `LICENSE`, `README.md`, `SECURITY.md`, every example config
    and (on Linux) the bundled systemd units; a `.sha256` companion
    file is generated alongside.
  - `packaging/linux/ph0sphor-server.service`: hardened systemd unit
    with `DynamicUser=yes`, `ProtectSystem=strict`,
    `NoNewPrivileges=yes`, `MemoryMax=128M` and the rest of the
    standard `Protect*` set. README §13.1 budget enforced from
    process supervision down.
  - `packaging/linux/ph0sphor-client@.service`: templated client
    unit that drops PHOSPHOR onto `/dev/tty1` for the named user
    and write-restricts to `~/.config/ph0sphor`.
  - `packaging/{linux,windows,macos}/README.md`: per-platform
    install recipes. Windows recipe references NSSM; macOS recipe
    documents the `xattr -d com.apple.quarantine` step and a
    launchd agent plist.
  - `ph0sphorctl gen-demo [--dir <path>]`: writes template
    `mail.json` and `weather.json` for `collectors.mail.source` /
    `collectors.weather.source` so an operator can wire up a real
    fetcher without bootstrapping from scratch.
  - `docs/screenshots/`: ASCII captures of HOME / SYS / MAIL /
    TIME / WEATHER / LOG. Plain text on purpose — diffs cleanly
    under git and renders in any monospace font.
  - `docs/installation.md`: end-to-end "download → install →
    pair → verify" walkthrough with troubleshooting tips for
    common failure modes.
