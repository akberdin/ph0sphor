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

## Mail and weather sources

The mail and weather collectors are intentionally external-input
oriented (README §27.1: client must not poll providers, server must
not bind to one specific IMAP/HTTP API). Each reads a JSON file the
operator updates from any preferred fetcher (cron + curl, a personal
script, a sysadmin pipeline).

### `collectors.mail.source`

```json
{
  "unread_count": 3,
  "recent": [
    {
      "sender": "ops@example.com",
      "subject": "Backup completed",
      "preview": "Nightly run finished in 14m 02s.",
      "timestamp_unix_ms": 1715700000000,
      "account": "personal"
    }
  ]
}
```

The server applies `collectors.mail.privacy` before anything reaches
the wire:

- `count_only` strips `sender`, `subject` and `preview`.
- `sender_subject` strips `preview`.
- `preview` keeps everything (cap: 240 chars per item).

### `collectors.weather.source`

```json
{
  "temperature_c": 17.0,
  "feels_like_c": 15.5,
  "condition": "cloudy",
  "humidity_percent": 72,
  "wind_kph": 11,
  "short_forecast": "Cloudy with a chance of rain",
  "location": "Saint Petersburg"
}
```

All fields are optional except `temperature_c`. Missing files or
unparseable JSON are non-fatal: the collector emits an empty payload
and logs a debug-level note.

## Secrets

Secret-bearing fields (mail passwords, API keys, TLS private keys) should
**not** live in the main TOML. Recommended approaches:

- Reference an environment variable: `password_env = "PHOSPHOR_MAIL_PWD"`.
- Reference an OS credential storage entry (future).
- Reference a separate `*.secret` file with restrictive permissions
  (ignored by git via `.gitignore`).

The client config never carries mail credentials. See `security-model.md`.
