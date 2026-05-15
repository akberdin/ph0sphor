# PHOSPHOR / ph0sphor

A retro terminal telemetry system that turns a low-power pocket
computer — primarily the **Sony VAIO VGN-P** — into a dedicated
phosphor-style status display for a main workstation.

```text
LINK: ONLINE   HOST: main-pc   UP: 04:12:37   BAT: 73% DSC   NET: wlan0 192.168.1.42
+- CPU ----------------------+   +- MEMORY ------------------+
|  17%  [#####...............] |   |  3.4G / 16G  [###........] |
+----------------------------+   +---------------------------+
```

The **server** runs on the workstation, collects telemetry
(CPU / RAM / disk / network plus optional mail and weather), and
streams a compact binary feed. The **client** runs on the VAIO and
renders a keyboard-driven TUI with multiple switchable screens. The
client stays thin on purpose: no API parsing, no heavy state, no
GPU dependency.

```text
The server is smart.
The client is thin.
The protocol is compact.
The default security mode is read-only.
```

## Characteristics

| Aspect              | Value                                                                 |
| ------------------- | --------------------------------------------------------------------- |
| Client target       | Sony VAIO VGN-P (Intel Atom Z5xx, 1–2 GB RAM, 1600×768 8" panel)      |
| Client OS           | 32-bit Linux (Alpine i686, Debian i386, antiX 32-bit, NixOS i686)     |
| Server OS           | Linux x86_64, Windows x86_64                                          |
| Language            | Rust (workspace: `ph0sphor-server`, `-client`, `-protocol`, `-core`, `ph0sphorctl`) |
| TUI stack           | `ratatui` + `crossterm`, render on dirty events only, 1–2 FPS cap     |
| Wire protocol       | Protobuf over WebSocket; delta-mostly, periodic FullSnapshot          |
| Auth                | Operator-confirmed pairing → 192-bit token, store at 0600 on Unix     |
| Default ports       | `7077` data WS, `7078` loopback control                               |
| Client release size | ~775 KB (`linux-i686` static musl)                                    |
| Themes              | `phosphor-green` (default), `amber-crt`, plus three more              |

## Install (5-minute path)

Binaries for each tagged release live on the
[Releases page](https://github.com/akberdin/ph0sphor/releases).
The current release is **v0.1.0**.

### 1. Install the server

Linux x86_64:

```sh
wget https://github.com/akberdin/ph0sphor/releases/download/v0.1.0/ph0sphor-v0.1.0-linux-x86_64.tar.gz
wget https://github.com/akberdin/ph0sphor/releases/download/v0.1.0/ph0sphor-v0.1.0-linux-x86_64.tar.gz.sha256
sha256sum -c ph0sphor-v0.1.0-linux-x86_64.tar.gz.sha256
tar xzf ph0sphor-v0.1.0-linux-x86_64.tar.gz
cd ph0sphor-v0.1.0-linux-x86_64

sudo install -Dm755 ph0sphor-server /usr/local/bin/ph0sphor-server
sudo install -Dm755 ph0sphorctl     /usr/local/bin/ph0sphorctl
sudo install -Dm640 examples/server.toml /etc/ph0sphor/server.toml
```

For Windows or systemd-on-Linux, follow the matching section under
[`packaging/`](packaging/).

### 2. Install the client on the VAIO P

Use the 32-bit musl build:

```sh
wget https://github.com/akberdin/ph0sphor/releases/download/v0.1.0/ph0sphor-v0.1.0-linux-i686.tar.gz
wget https://github.com/akberdin/ph0sphor/releases/download/v0.1.0/ph0sphor-v0.1.0-linux-i686.tar.gz.sha256
sha256sum -c ph0sphor-v0.1.0-linux-i686.tar.gz.sha256
tar xzf ph0sphor-v0.1.0-linux-i686.tar.gz
cd ph0sphor-v0.1.0-linux-i686

sudo install -Dm755 ph0sphor-client /usr/local/bin/ph0sphor-client
install -Dm640 examples/client.toml ~/.config/ph0sphor/client.toml
```

Edit `~/.config/ph0sphor/client.toml` and set at least
`client.server = "main-pc:7077"`.

### 3. Pair

Start the server, then the client. On first launch the client shows
a `PAIRING` banner with a code like `ABCD-1234`. On the workstation:

```sh
ph0sphorctl pair confirm ABCD-1234
```

The server issues a 192-bit token, pushes it back to the waiting
client over the same WebSocket, and the client persists it at
`~/.config/ph0sphor/token` (0600). Future restarts skip pairing.

Sanity-check without a network at all:

```sh
ph0sphor-server --demo &
ph0sphor-client --demo
```

## Configure

Both binaries take `--config /path/to/config.toml`. Start from the
shipped examples in `examples/` and tune from there:

- `[server].bind` / `[server].control_bind` — listen sockets.
- `[security].pairing_enabled`, `[security].token_store` — pairing
  flow and where issued tokens are persisted.
- `[collectors].cpu.interval_ms`, `.memory.*`, `.disk.*`, `.network.*` —
  collector cadence. Defaults are conservative.
- `[performance].send_deltas_only` (default `true`),
  `min_send_interval_ms`, `full_snapshot_interval_sec` — wire pacing.
- `[client].server`, `.theme`, `.token_file`, `ui.compact_mode`,
  `ui.ascii_fallback` — client side.

Full reference: [`docs/configuration.md`](docs/configuration.md).

## Documentation

- [`docs/installation.md`](docs/installation.md) — end-to-end install
  walkthrough with troubleshooting.
- [`docs/vaio-p-client.md`](docs/vaio-p-client.md) — appliance recipe
  for a real VAIO P (fonts, autostart, console mode).
- [`docs/vaio-p-client-vm-testing.md`](docs/vaio-p-client-vm-testing.md)
  — testing the client in a VirtualBox antiX 32-bit VM.
- [`docs/configuration.md`](docs/configuration.md) — every TOML key.
- [`docs/protocol.md`](docs/protocol.md) — Protobuf schema and framing.
- [`docs/security-model.md`](docs/security-model.md) — threat model
  and pairing flow.
- [`docs/performance-budget.md`](docs/performance-budget.md) — CPU /
  memory / network budgets and how they are enforced.
- [`docs/design.md`](docs/design.md), [`docs/roadmap.md`](docs/roadmap.md)
  — design rationale and milestone tracker.
- [`CHANGELOG.md`](CHANGELOG.md) — what shipped in each tag.
- [`SECURITY.md`](SECURITY.md) — vulnerability disclosure.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — dev setup and rules.

## License

MIT — see [`LICENSE`](LICENSE).
