# VAIO P Client Setup

The Sony VAIO VGN-P series is the canonical PHOSPHOR client. This document
collects setup notes for running `ph0sphor-client` on that machine.

## Hardware Assumptions

```text
Very low CPU performance (Intel Atom era)
Limited RAM (1-2 GB typical)
Small physical screen
High screen resolution for its size (1600x768)
Weak or problematic GPU acceleration
Linux terminal environment
Keyboard-first operation
No mouse dependency
```

## Operating System

A minimal Linux distribution is recommended. A 32-bit (i686) build target
is necessary for the original VAIO P CPU. The release pipeline must ship a
Linux i686 client artifact.

Recommended terminal stack:

- A lightweight Wayland or X11 session, or a bare framebuffer + `fbterm` if
  the user prefers no graphical session.
- A monospace font with full box-drawing coverage (e.g. Cozette, Terminus,
  Spleen).
- `crossterm` handles input. No additional terminfo configuration should be
  needed beyond `xterm-256color`.

## Autostart

For an appliance-like experience, the client should launch automatically.
Two approaches:

1. **systemd user service** (recommended on systemd distros) — launches
   `ph0sphor-client` in a foreground terminal multiplexer-less session on
   boot or login.
2. **`.bash_profile`/`.zprofile` exec on TTY1** — drops the user straight
   into the client.

Sample systemd unit (to be expanded in Milestone 7):

```ini
[Unit]
Description=PHOSPHOR client
After=network-online.target

[Service]
ExecStart=/usr/local/bin/ph0sphor-client
Restart=on-failure

[Install]
WantedBy=default.target
```

## Layout

The 1600x768 panel suits a compact mode by default. Layout details land in
Milestone 7 alongside an ASCII fallback for terminals without good
Unicode coverage.

## Battery and Wi-Fi

Where available, the client reports VAIO battery state and Wi-Fi/IP status
on the ABOUT screen. These are local-only reads; the client must still not
poll heavy data sources by itself.
