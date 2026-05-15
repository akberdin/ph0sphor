# Linux Packaging

The release workflow (`.github/workflows/release.yml`) ships two
Linux artifact lines. Current release: **v0.1.0**.

- **`ph0sphor-v0.1.0-linux-x86_64.tar.gz`** — `ph0sphor-server`,
  `ph0sphor-client` and `ph0sphorctl` built against
  `x86_64-unknown-linux-gnu`. Use for normal modern workstations.
- **`ph0sphor-v0.1.0-linux-i686.tar.gz`** — `ph0sphor-client` only,
  cross-built against `i686-unknown-linux-musl`. Static binary, no
  glibc dependency. This is the VAIO P target.

Both tarballs include `LICENSE`, `README.md`, `SECURITY.md`, every
example in `examples/`, and the two systemd units in `packaging/`.

## Download

```sh
REL=v0.1.0
wget https://github.com/akberdin/ph0sphor/releases/download/$REL/ph0sphor-$REL-linux-x86_64.tar.gz
wget https://github.com/akberdin/ph0sphor/releases/download/$REL/ph0sphor-$REL-linux-x86_64.tar.gz.sha256
sha256sum -c ph0sphor-$REL-linux-x86_64.tar.gz.sha256
tar xzf ph0sphor-$REL-linux-x86_64.tar.gz
cd ph0sphor-$REL-linux-x86_64
```

For the VAIO P client tarball, swap `linux-x86_64` for `linux-i686`.

## Installing the server

```sh
sudo install -Dm755 ph0sphor-server     /usr/local/bin/ph0sphor-server
sudo install -Dm755 ph0sphorctl         /usr/local/bin/ph0sphorctl
sudo install -Dm644 packaging/ph0sphor-server.service \
                                        /etc/systemd/system/ph0sphor-server.service
sudo install -Dm640 -o root -g root examples/server.toml \
                                        /etc/ph0sphor/server.toml
sudo mkdir -p /var/lib/ph0sphor && sudo chmod 0700 /var/lib/ph0sphor

sudo systemctl daemon-reload
sudo systemctl enable --now ph0sphor-server.service
```

## Installing the client on the VAIO P

```sh
sudo install -Dm755 ph0sphor-client \
                                        /usr/local/bin/ph0sphor-client
sudo install -Dm644 packaging/ph0sphor-client@.service \
                                        /etc/systemd/system/ph0sphor-client@.service
install -Dm640 examples/client.toml ~/.config/ph0sphor/client.toml

# Replace `vaio` with the unix user that should own the TTY-bound
# session.
sudo systemctl daemon-reload
sudo systemctl enable --now ph0sphor-client@vaio.service
```

On first start the client falls into pairing mode. Read the code from
its screen and confirm on the server host:

```sh
ph0sphorctl pair confirm ABCD-1234
```

See `docs/installation.md` for the full walkthrough and
`docs/vaio-p-client.md` for VAIO-specific notes.

## Checksums

Every release artifact is accompanied by a `.sha256` file generated
by `sha256sum`. To verify before installing:

```sh
sha256sum -c ph0sphor-v0.1.0-linux-x86_64.tar.gz.sha256
```

The same workflow uploads the checksum file directly to the GitHub
release alongside the tarball.
