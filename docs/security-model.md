# Security Model

This document is the long form of `SECURITY.md` and README §14. It is
normative.

## Trust Model

PHOSPHOR's default deployment assumes:

- A single user owns both the server (workstation) and the client (VAIO P).
- Server and client communicate over a local network.
- The user has physical control of both endpoints.

Even within this trusted environment, PHOSPHOR follows defensive defaults.

## Read-Only Default

The server exposes telemetry. By default, it does **not** accept commands
from the client. `ClientCommandRequest` is disabled in MVP.

If future versions enable control commands, those commands must:

1. Be explicitly enabled in server config.
2. Be allowlisted by stable ID.
3. Not accept arbitrary shell input.
4. Be logged.
5. Require explicit configuration for anything dangerous.

## Authentication and Pairing

Concepts:

```text
client_id          identifier supplied by the client
client_token       opaque secret issued by the server
server_id          identifier supplied by the server
protocol_version   integer version number
pairing_code       short, human-readable, one-time code
```

Pairing flow:

1. User starts the server with `security.pairing_enabled = true`.
2. User starts the client without a valid token (`client.token = ""` and
   no `client.token_file` on disk).
3. Client sends `Hello` then `PairingRequest`.
4. Server generates a short, single-use pairing code (8 random
   characters from a confusables-free alphabet, formatted `ABCD-1234`)
   and sends it back as `PairingChallenge`. The code is also logged at
   INFO on the server.
5. Client displays the code prominently in its TUI alongside a
   reminder of the confirmation command.
6. User runs `ph0sphorctl pair confirm <code>` on the server host. The
   CLI POSTs to the loopback-only control endpoint
   (`POST /control/pair/confirm`).
7. The control handler verifies the peer is loopback (`is_loopback()`),
   resolves the code in the in-memory pending map, calls
   `TokenStore::issue()` which generates a 192-bit random hex token,
   appends it to the JSON store (mode `0600` on Unix), and notifies
   the waiting WebSocket session via a `oneshot` channel.
8. Server sends `PairingConfirm { code, token }` back over the
   established WebSocket and proceeds directly into the snapshot stream.
9. Client emits `AppEvent::TokenIssued`, which is intercepted by the
   app loop **before** the visible event log is touched. The token is
   written to `client.token_file` (mode `0600` on Unix); the on-screen
   log only records "client paired — token stored". Raw tokens never
   appear in the TUI.
10. Future connections present the token via `AuthRequest` and bypass
    pairing entirely.

Code lifetime:

- Pairing codes are valid for `security.pairing_ttl_secs` (default
  300 s). Stale entries are evicted lazily on the next `request()`.
- Codes are single-use: confirming consumes the entry.

Token lifetime:

- Tokens are appended to `security.token_store` and persisted across
  restarts. Revocation is currently a "delete the line from the JSON
  and restart the server" operation; an online revocation command is
  out of scope for MVP.

## Secrets

Secrets live on the server. Examples:

- Mail account passwords.
- OAuth tokens.
- Third-party API keys.
- TLS private keys.

Rules:

- Secrets are never sent to the client.
- Secrets are never written to logs (redacted at logger level).
- Secrets are never echoed in debug JSON dumps.
- Client tokens specifically are passed through
  `ph0sphor_server::auth::redact_token` before any `tracing` macro on
  the server side. The function keeps the first four characters and
  replaces the rest with `…`, so a misrouted log entry leaks at most
  four characters.
- On the client side, the raw pairing-issued token is written to
  `client.token_file` directly from the WS task and is **never** put
  into `AppEvent::Log` or the on-screen LOG. The visible UI marks the
  pairing as done without naming the token.

## Mail Privacy

Three privacy modes, configured per mail collector:

```text
count_only       Show unread count only.
sender_subject   Show sender and subject (default).
preview          Show sender, subject and short preview.
```

Full email bodies are never transmitted by default.

## Transport

MVP transport is WebSocket binary frames over a LAN. TLS is a future
improvement and is required for any deployment that crosses an untrusted
network. The README explicitly does not require Internet access for basic
telemetry.

## Out of Scope

- Attacks requiring physical access to either endpoint.
- Attacks against transports the user has opted into without enabling TLS.
- Attacks via third-party plugins (no plugin system exists yet).
