# macOS Packaging

The release workflow ships two macOS artifacts:

- **`ph0sphor-<version>-macos-x86_64.tar.gz`** — Intel Macs.
- **`ph0sphor-<version>-macos-arm64.tar.gz`** — Apple Silicon Macs.

Each tarball contains `ph0sphor-server`, `ph0sphorctl`, `LICENSE`,
`README.md`, `SECURITY.md` and every example config. The client is
not built for macOS — its target is the Sony VAIO P running Linux.

## Installation

```sh
sudo install -m755 ph0sphor-server /usr/local/bin/ph0sphor-server
sudo install -m755 ph0sphorctl     /usr/local/bin/ph0sphorctl
sudo install -d -m700 /etc/ph0sphor
sudo install -m640 examples/server.toml /etc/ph0sphor/server.toml
```

Edit `/etc/ph0sphor/server.toml`, then run interactively:

```sh
ph0sphor-server --config /etc/ph0sphor/server.toml
```

## Run as a launchd agent (optional)

A bare-minimum `~/Library/LaunchAgents/dev.ph0sphor.server.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>            <string>dev.ph0sphor.server</string>
  <key>ProgramArguments</key> <array>
    <string>/usr/local/bin/ph0sphor-server</string>
    <string>--config</string>
    <string>/etc/ph0sphor/server.toml</string>
  </array>
  <key>RunAtLoad</key>        <true/>
  <key>KeepAlive</key>        <true/>
  <key>StandardOutPath</key>  <string>/tmp/ph0sphor.out</string>
  <key>StandardErrorPath</key><string>/tmp/ph0sphor.err</string>
</dict>
</plist>
```

Then `launchctl load ~/Library/LaunchAgents/dev.ph0sphor.server.plist`.

## Codesigning / notarization

The binaries are not codesigned. macOS Gatekeeper will quarantine
downloaded ones — clear the attribute manually before first run:

```sh
xattr -d com.apple.quarantine ph0sphor-server ph0sphorctl
```

Proper notarization is out of scope for the initial release and would
require an Apple Developer ID.

## Checksums

```sh
shasum -a 256 -c ph0sphor-vX.Y.Z-macos-arm64.tar.gz.sha256
```
