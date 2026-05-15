# PHOSPHOR Protocol

Authoritative description of the wire protocol used between the server and
the client. Production transport: Protobuf-over-WebSocket binary frames.
Debug transport: JSON (development and tests only).

The current Protobuf schema lives in `proto/ph0sphor.proto`. Milestone 1
defines the full message family and round-trip tests live in
`crates/ph0sphor-protocol/tests/round_trip.rs`.

Code generation uses `prost-build` with a vendored `protoc` from the
`protoc-bin-vendored` crate, so no system `protoc` is required to build.

## Versioning

- Every message carries `protocol_version`.
- Breaking changes increment the major version.
- Backward-compatible additions preserve old field numbers.
- Unknown fields are ignored where it is safe to do so.

## Message Types

```text
Hello
AuthRequest
AuthResponse
PairingRequest
PairingChallenge
PairingConfirm
FullSnapshot
DeltaUpdate
Event
Ping
Pong
Error
ClientCommandRequest      (disabled in MVP)
ClientCommandResponse     (disabled in MVP)
```

## Lifecycle

```text
Client                                  Server
 |---- Hello (proto_v, client_id) -----> |
 |<--- AuthRequest (challenge) --------- |   (optional during pairing)
 |---- AuthResponse (token) -----------> |
 |<--- FullSnapshot -------------------- |
 |<--- DeltaUpdate ...  ---------------- |
 |<--- Event ...        ---------------- |
 |<--> Ping/Pong (keepalive) ----------- |
```

After reconnect, the client receives a fresh `FullSnapshot` before resuming
delta processing.

## Full Snapshot

`FullSnapshot` contains everything required to render all screens. It is
sent:

1. After successful authentication.
2. After reconnect.
3. On explicit client request (R key).
4. Periodically (default: every 60 seconds) as a safety mechanism against
   drift.

## Delta Update

`DeltaUpdate` carries only changed fields. The server may coalesce updates
and must not send high-frequency noise that does not change visible output.

## Events

`Event` represents a discrete occurrence (new mail, threshold crossed,
collector failed, timer completed, etc.). Events are routed through the
event bus and persisted in the in-memory event log on the server.

## Debug JSON

A JSON mirror of every message exists for development purposes only. The
JSON schema mirrors the Protobuf schema field-for-field. JSON debug mode
must never expose secrets and must not become the default production
transport.
