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
[ ] Implement WebSocket client.
[ ] Implement auth handshake.
[ ] Implement reconnect logic.
[ ] Implement HOME screen.
[ ] Implement SYS screen.
[ ] Implement LOG screen.
[ ] Implement theme support.
[ ] Implement screen switching.
[ ] Implement low-power render loop.
```

## Milestone 4 — Performance Pass

Goal: make the system efficient enough for continuous use.

```
[ ] Render only on state changes.
[ ] Send deltas instead of full snapshots where possible.
[ ] Add configurable collector intervals.
[ ] Add network usage logging.
[ ] Add server self-monitoring.
[ ] Add client self-monitoring.
[ ] Add low-power mode.
[ ] Add bounded queues.
```

## Milestone 5 — Security Pass

Goal: make the default system safe for LAN usage.

```
[ ] Implement pairing.
[ ] Implement client token storage.
[ ] Implement token validation.
[ ] Add secret redaction.
[ ] Confirm read-only default mode.
[ ] Document threat model.
[ ] Document mail privacy model.
[ ] Disable remote command execution by default.
```

## Milestone 6 — Useful Features

```
[ ] MAIL screen and unread count.
[ ] Mail privacy modes.
[ ] WEATHER screen.
[ ] TIME screen with local timer/stopwatch/alarm.
[ ] Richer event log.
```

## Milestone 7 — VAIO P Polish

```
[ ] VAIO P Linux setup guide.
[ ] Autostart instructions.
[ ] Tune layout for 1600x768.
[ ] Compact mode.
[ ] ASCII fallback.
[ ] Terminal font recommendations.
[ ] VAIO battery status.
[ ] Wi-Fi/IP status.
```

## Milestone 8 — Packaging and Releases

```
[ ] Release builds.
[ ] Linux server package.
[ ] Linux i686 client build.
[ ] Windows server build.
[ ] macOS server build.
[ ] Checksums.
[ ] Example configs.
[ ] Demo mode.
[ ] Screenshots.
[ ] Installation documentation.
```
