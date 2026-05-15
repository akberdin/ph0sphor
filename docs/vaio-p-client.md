# VAIO P Client Setup

The Sony VAIO VGN-P series is the canonical PHOSPHOR client. This document
collects the hands-on knowledge needed to make a VAIO P boot directly into
the phosphor-style TUI and operate as a dedicated terminal appliance
(README §29).

## Hardware Assumptions

```text
CPU:     Intel Atom Z520/Z530/Z540/Z550 (single core, hyperthreaded)
RAM:     1–2 GB
Display: 1600×768 native, 8" panel
GPU:     Intel GMA 500 / GMA 4500MHD — weak, often problematic
NIC:     Intel 802.11abgn wireless, sometimes broken Realtek wired
Battery: removable Li-ion, ~2 Wh standard / ~4 Wh extended
TPM:     not present
```

The CPU has no `aes-ni` and no `avx`. PHOSPHOR's release profile builds
with `opt-level = "z"` and `lto = "thin"` precisely so the resulting
binary stays small and cache-friendly on this machine.

## Operating System

Use a minimal Linux distribution. The original VAIO P CPU is **i686**;
ship a 32-bit `ph0sphor-client` build for those machines. Tested
distributions:

- **Alpine 3.19+ (i686)** — the smallest reasonable base. Pair with
  `agetty` autologin; works fine without X11.
- **Debian 12 (i386)** — official i386 install media still exists.
  Comes with systemd; pair with the unit at the end of this document.
- **NixOS 24.05 (i686 cross-build)** — for reproducible appliance images.

A typical install drops below 200 MB before adding PHOSPHOR.

### Console mode (no X11)

Recommended for an appliance experience. The kernel framebuffer plus
`fbterm` (or the bare `linux` console) gives a solid CRT-feel and skips
the GMA 500 driver landmines.

```sh
# Pin the console font; "ter-v18n" is dense yet readable on the VAIO P.
sudo setfont ter-v18n

# Drop into PHOSPHOR on TTY1 after autologin (see autostart below).
exec ph0sphor-client
```

### X11 / Wayland session

If a windowed session is preferred, run a single full-screen terminal
(`alacritty`, `foot` or `kitty`) and start PHOSPHOR inside it. Skip
window decorations entirely — there is no reason for the VAIO P to
chrome a single-purpose window.

## Terminal & Font Recommendations

PHOSPHOR is dense and benefits from a font that is both compact and
high-contrast. Recommended (in order of preference):

| Font          | Glyph size | Notes                                    |
| ------------- | ---------- | ---------------------------------------- |
| **Cozette**   | 6×13       | Pixel-perfect, readable on 1600×768.     |
| **Spleen**    | 8×16       | Solid box-drawing; great for `phosphor-green`. |
| **Terminus**  | 6×12+      | The classic. Multiple sizes packaged on every distro. |
| **Cascadia Mono** | 11px+ | Larger, but works if the user runs an X11 session. |
| **Fixed (built-in)** | 8×16 | Always available; ASCII fallback target. |

Set the `TERM` value to `xterm-256color` if not already; `crossterm`
auto-detects the rest. If your terminal lacks Unicode box-drawing,
enable `ui.ascii_fallback = true` in the client TOML — PHOSPHOR will
draw borders with `+ - |`, replace `↑/↓` with `up/dn` and switch the
gauges from block characters to ASCII bars.

## Layout for 1600×768

Two operating regimes are covered out of the box:

- **Normal** — at Cozette 6×13 the panel reaches roughly 264×52 cells.
  The default layout (`ui.compact_mode = true` is harmless here)
  shows everything comfortably.
- **Compact** — at Terminus 12×24 the panel drops to ~133×32 cells.
  Set `ui.compact_mode = true` so the header collapses to one line and
  leaves more rows for the body.

If you autostart in console mode at the smallest VGA console font
(8×8), you get ~200×96 cells — almost luxurious. Compact mode is
still cheaper to render.

## Autostart

### systemd (recommended)

Drop the unit below into `/etc/systemd/system/ph0sphor-client@.service`,
then `systemctl enable --now ph0sphor-client@vaio.service`:

```ini
[Unit]
Description=PHOSPHOR client (%i)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=%i
ExecStart=/usr/local/bin/ph0sphor-client --config /etc/ph0sphor/client.toml
Restart=on-failure
RestartSec=2
StandardInput=tty-force
StandardOutput=tty
StandardError=journal
TTYPath=/dev/tty1
TTYReset=yes
TTYVHangup=yes

# Defense in depth — the client only ever needs to read /sys for the
# local battery and Wi-Fi/IP info plus its own config and token file.
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/var/lib/ph0sphor
PrivateTmp=yes
NoNewPrivileges=yes

[Install]
WantedBy=multi-user.target
```

### Bare TTY autologin (no systemd)

For Alpine / OpenRC users:

```sh
# /etc/inittab
tty1::respawn:/sbin/agetty -a vaio --noclear -n 38400 tty1 linux
```

Then in `/home/vaio/.profile`:

```sh
# Drop into PHOSPHOR exactly once per login on TTY1.
if [ "$(tty)" = "/dev/tty1" ] && [ -z "$PHOSPHOR_RUNNING" ]; then
    export PHOSPHOR_RUNNING=1
    exec ph0sphor-client --config /etc/ph0sphor/client.toml
fi
```

## Battery + Wi-Fi/IP

The header carries the VAIO's own battery percentage and the iface/IP
the client uses to reach the server. Both are read locally on the
client (from `/sys/class/power_supply/BAT*` and `/sys/class/net/`),
**not** from the workstation. They keep updating during a disconnect
so you can see at a glance whether the link is down because of you or
because of the server. README §13.2 calls this out as required for
"offline usability".

If your VAIO P reports its battery under `BAT1` instead of `BAT0`, no
configuration is needed — the client iterates every `BAT*` entry and
picks the first one that responds. Below 30 % the percentage turns
warning-yellow; below 15 % on discharge it turns critical-red.

## Theming

Two themes are mandatory in MVP and look great on the VAIO P:

- **`phosphor-green`** — default. Reads as a CRT terminal in a dark room.
- **`amber-crt`** — recommended on glossy screens; less eye fatigue at
  night.

Cycle them with `C` at runtime; persist the choice via `client.theme`
in the config TOML.

## Pairing a Fresh VAIO P

Per README §9.3 / `docs/security-model.md`:

```sh
# 1. On the workstation: start the server with pairing enabled
#    (security.pairing_enabled = true, security.token_store set).
# 2. On the VAIO: start the client with no token.
$ ph0sphor-client --config /etc/ph0sphor/client.toml
# The header switches to a PAIRING banner with a code like ABCD-1234.

# 3. On the workstation:
$ ph0sphorctl pair confirm ABCD-1234
# pairing confirmed

# The client immediately stores the issued token in
# ~/.config/ph0sphor/token (mode 0600) and connects.
```

## Operational Notes

- Don't run a window manager unless you need it. Even a tiling WM is
  wasted on a single full-screen terminal.
- Disable `kbrate` Caps-Lock-as-Ctrl if you don't use it; the VAIO P
  keyboard is small enough that misfires are easy.
- Use `tlp` or write a manual `/sys/class/power_supply/BAT0/...`
  charge-stop policy if you leave the VAIO P plugged in 24/7. This is
  a hardware-longevity concern, not a PHOSPHOR concern.
- Verify with `journalctl -u ph0sphor-client@vaio` that no stack trace
  is stored across restarts. The panic hook in `main.rs` always
  restores the terminal, so a panic looks like a normal exit from the
  user's perspective; you'd otherwise lose the VAIO to a stuck raw
  mode.
