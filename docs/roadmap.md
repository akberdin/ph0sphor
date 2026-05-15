# Roadmap

The roadmap is mirrored from README §21 with status markers. Update this
file as milestones close.

Legend: `[x]` complete, `[~]` in progress, `[ ]` not started.

## Milestone 0 — Project Skeleton

Goal: clean public repository foundation.

```
[x] Create repository named ph0sphor.
[x] Add README.md.
[x] Add LICENSE.
[x] Add SECURITY.md.
[x] Add Rust workspace.
[x] Add crates directory.
[x] Add docs directory.
[x] Add example configs.
[x] Add GitHub Actions placeholder.
[x] Add initial issue templates.
```

Done when the repository clearly communicates what PHOSPHOR is and how it
will be built.

## Milestone 1 — Protocol First

Goal: define the data contract before building UI complexity.

```
[x] Define Protobuf schema.
[x] Define FullSnapshot message.
[x] Define DeltaUpdate message.
[x] Define Event message.
[x] Define Hello/Auth messages.
[x] Add protocol versioning.
[x] Add test fixtures.
[x] Add debug JSON dump.
```

Done when a test can encode and decode a realistic telemetry snapshot.
Met by `crates/ph0sphor-protocol/tests/round_trip.rs`.

## Milestone 2 — Minimal Server

Goal: a working server that exposes basic telemetry.

```
[x] Implement config loading.
[x] Implement CPU collector.
[x] Implement memory collector.
[x] Implement disk collector.
[x] Implement network collector.
[x] Implement state store.
[x] Implement WebSocket binary endpoint.
[x] Implement basic token auth stub.
[x] Implement demo data generator.
```

Done when a client or debug tool can receive live CPU/RAM/DISK/NET
snapshots. Met by
`crates/ph0sphor-server/tests/ws_handshake.rs::server_streams_full_snapshot_to_authenticated_client`.

## Milestone 3 — Minimal VAIO Client

Goal: display live telemetry on the VAIO P in terminal UI.

```
[x] Implement WebSocket client.
[x] Implement auth handshake.
[x] Implement reconnect logic.
[x] Implement HOME screen.
[x] Implement SYS screen.
[x] Implement LOG screen.
[x] Implement theme support.
[x] Implement screen switching.
[x] Implement low-power render loop.
```

Done when the VAIO P displays live workstation telemetry in a
phosphor-style TUI. Net layer covered by
`crates/ph0sphor-client/tests/handshake.rs`; UI exercised manually via
`ph0sphor-client --demo`.

## Milestone 4 — Performance Pass

Goal: make the system efficient enough for continuous use.

```
[x] Render only on state changes.
[x] Send deltas instead of full snapshots where possible.
[x] Add configurable collector intervals.
[x] Add network usage logging.
[x] Add server self-monitoring.
[x] Add client self-monitoring.
[x] Add low-power mode.
[x] Add bounded queues.
```

Done when normal operation uses minimal CPU, memory and network
bandwidth. Delta encoding + coalescing + per-session byte logging
implemented in `ph0sphor-protocol::delta`, `ph0sphor-server::net` and
`ph0sphor-client::net`. See `docs/performance-budget.md`.

## Milestone 5 — Security Pass

Goal: make the default system safe for LAN usage.

```
[x] Implement pairing.
[x] Implement client token storage.
[x] Implement token validation.
[x] Add secret redaction.
[x] Confirm read-only default mode.
[x] Document threat model.
[x] Document mail privacy model.
[x] Disable remote command execution by default.
```

Done when a new client can be paired securely and cannot execute
arbitrary server commands. Validated by
`crates/ph0sphor-client/tests/pairing.rs::client_pairs_then_receives_token_and_snapshot`,
which drives the full pairing path through a real WS link and the
loopback control endpoint.

## Milestone 6 — Useful Features

Goal: PHOSPHOR is useful even when the user is not actively debugging
the workstation.

```
[x] MAIL screen and unread count.
[x] Mail privacy modes.
[x] WEATHER screen.
[x] TIME screen with local timer/stopwatch/alarm.
[x] Richer event log.
```

Wire schema: `MailSummary` / `MailItem` / `WeatherInfo` plus a
`MailPrivacy` enum land in proto and are carried by both
`FullSnapshot` and `DeltaUpdate`. Server collectors ingest
operator-managed JSON files (`collectors.mail.source`,
`collectors.weather.source`) and apply the configured privacy mode
before anything reaches the wire. Client gains MAIL / TIME / WEATHER
screens, a local `TimeState` with timer/stopwatch/alarms, and a
client-side new-mail detector that pushes a Warn-level entry into the
event log on every count increase.

## Milestone 7 — VAIO P Polish

Goal: the VAIO P can boot directly into PHOSPHOR and operate like a
dedicated terminal appliance.

```
[x] VAIO P Linux setup guide.
[x] Autostart instructions.
[x] Tune layout for 1600x768.
[x] Compact mode.
[x] ASCII fallback.
[x] Terminal font recommendations.
[x] VAIO battery status.
[x] Wi-Fi/IP status.
```

`docs/vaio-p-client.md` is now the canonical setup guide and bundles a
hardened systemd unit + an Alpine/OpenRC autologin recipe. `compact_mode`
collapses the header to a single line; `ascii_fallback` swaps borders,
gauge fills and arrow glyphs for ASCII equivalents. The new
`ph0sphor-client::local` module reads `/sys/class/power_supply/BAT*` and
default-iface IP locally so the header keeps showing live battery + IP
even when the link is down.

## Milestone 8 — Packaging and Releases

Goal: a user can download a release, configure the server and run the
VAIO client.

```
[x] Release builds.
[x] Linux server package.
[x] Linux i686 client build.
[x] Windows server build.
[x] macOS server build.
[x] Checksums.
[x] Example configs.
[x] Demo mode.
[x] Screenshots.
[x] Installation documentation.
```

GitHub Actions release workflow
(`.github/workflows/release.yml`) builds a five-target matrix on every
`v*.*.*` tag and uploads the artifacts alongside `sha256sum`-style
companion files. Linux i686 (VAIO client) is cross-built via `cross`.
Bundled hardened systemd units in `packaging/linux/`, Windows/macOS
notes in their respective `packaging/<os>/README.md`. The new
`ph0sphorctl gen-demo` subcommand writes template `mail.json` /
`weather.json` for the operator-managed ingest paths.
ASCII screenshots of all six screens live in `docs/screenshots/`.
End-to-end install + pair walkthrough at `docs/installation.md`.
