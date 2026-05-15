# PHOSPHOR Design

This document expands on the high-level architecture from README §8 and
records design decisions as the project evolves.

## Core Rule

```text
The server is smart.
The client is thin.
The network protocol is compact.
The default security mode is read-only.
```

Every architectural decision is checked against this rule. A change that
weakens any of the four lines must be rejected or redesigned.

## Components

- **`ph0sphor-core`** — shared domain types (metrics, events, themes,
  configuration shapes). No I/O.
- **`ph0sphor-protocol`** — wire schema and encode/decode. Protobuf in
  production, JSON in debug.
- **`ph0sphor-server`** — collectors, state store, event bus, WebSocket
  endpoint, auth and privacy filtering.
- **`ph0sphor-client`** — VAIO TUI. Connection, local cache, screen
  navigation, themes, local clock/timer/alarm/stopwatch.
- **`ph0sphorctl`** — administrative CLI for pairing confirmation, status
  inspection, config validation, demo data and protocol debugging.

## Preferred Implementation Pattern

```text
Collector -> Normalized State -> Delta/Event -> Protocol -> Client State -> TUI Widget
```

The client must not become a data-processing layer. If a transformation can
happen on the server, it must happen on the server.

## State Store

The server keeps a single in-memory state store. Collectors write into it;
the protocol layer reads from it. Deltas are computed by comparing the
current state with the last state sent to each connected client. The store
is not persisted to disk by default.

## Event Bus

Discrete events (new mail, threshold crossed, collector failed/recovered,
timer/alarm fired) flow through a bounded broadcast channel. Slow clients
do not block the bus; they receive an Event-Lost signal instead.

## Open Questions

These are intentionally unresolved at the project skeleton stage and will be
revisited as later milestones land:

- Exact Protobuf schema versioning policy.
- Whether to expose a Unix domain socket as a localhost-only transport in
  addition to WebSocket.
- Client cache format on disk (probably a single compact binary file).
