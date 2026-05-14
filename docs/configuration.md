# Configuration

Both the server and the client are configured via TOML files. Defaults are
chosen to satisfy the performance budget (see `performance-budget.md`).

Reference configs live in:

- `examples/server.toml`
- `examples/client.toml`

## Server Configuration

Sections:

- `[server]` — bind address, name, protocol, debug toggles.
- `[security]` — pairing, token requirement, control commands.
- `[performance]` — tick interval, delta-only sending, snapshot interval,
  in-memory event cap.
- `[collectors.*]` — per-collector enablement and interval.

See README §18.1 for a complete example.

## Client Configuration

Sections:

- `[client]` — server URL, client name, theme, render FPS, low-power mode.
- `[ui]` — default screen, scanlines, ASCII fallback, compact mode.
- `[cache]` — last-snapshot persistence and cached event cap.
- `[keys]` — keybindings overrides.

See README §18.2 for a complete example.

## Validation

`ph0sphorctl validate-config <path>` will validate a config file against the
schema in `ph0sphor-core`. This subcommand is currently a stub and lands
with Milestone 5.

## Secrets

Secret-bearing fields (mail passwords, API keys, TLS private keys) should
**not** live in the main TOML. Recommended approaches:

- Reference an environment variable: `password_env = "PHOSPHOR_MAIL_PWD"`.
- Reference an OS credential storage entry (future).
- Reference a separate `*.secret` file with restrictive permissions
  (ignored by git via `.gitignore`).

The client config never carries mail credentials. See `security-model.md`.
