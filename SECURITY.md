# Security Policy

PHOSPHOR aims to be safe by default. This document describes how to report
vulnerabilities and what the security model guarantees.

## Reporting a Vulnerability

Please do **not** open a public GitHub issue for security problems.

Instead, contact the maintainers privately (preferred channel: a GitHub
security advisory on this repository, or direct email to the project owner
listed in the repository profile). Provide:

- A description of the issue.
- Steps to reproduce, including configuration and version.
- Any impact assessment you can offer.

Reports are acknowledged on a best-effort basis. We try to respond within a
reasonable window. Public disclosure should be coordinated with the
maintainers.

## Threat Model (Summary)

PHOSPHOR is designed primarily for use on a local network (LAN), with a single
trusted user who owns both the server (main workstation) and the client (a
Sony VAIO P or equivalent low-power device).

Default assumptions:

- The server runs on a trusted machine.
- The client runs on a trusted device controlled by the same user.
- The link between server and client is local.

Despite this, PHOSPHOR follows these baseline rules:

1. **Read-only by default.** The server exposes telemetry. It does not accept
   arbitrary commands from the client in MVP.
2. **No remote shell execution.** Arbitrary remote shell execution is
   forbidden in MVP and must be explicitly enabled and allowlisted in any
   future iteration.
3. **Client tokens.** Clients are paired with a server-issued token. Tokens
   are never hardcoded in source.
4. **No secrets on the client.** Mail credentials, OAuth tokens, API keys
   and TLS private keys must live on the server. The VAIO client never sees
   them.
5. **Mail privacy.** Mail metadata may be transmitted to the client only
   according to the configured privacy mode (`count_only`, `sender_subject`,
   `preview`). Full email bodies are not transmitted by default.
6. **Redacted logs.** Secrets are redacted in logs and debug output.

See `docs/security-model.md` for the full security model and pairing flow.

## Supported Versions

PHOSPHOR is pre-MVP. Security fixes are applied to the latest commit on
`main`. There is no released version line yet.

## Out of Scope

- Attacks requiring physical access to the server or client device.
- Attacks against transports the user has explicitly opted into (for
  example, exposing the server to the public Internet without TLS).
- Issues caused by third-party plugins that may be added in a future
  plugin system.
