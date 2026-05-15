# Windows Packaging

The release workflow ships `ph0sphor-<version>-windows-x86_64.zip`
containing `ph0sphor-server.exe`, `ph0sphorctl.exe`, `LICENSE`,
`README.md`, `SECURITY.md` and every example config. The client is
not built for Windows — its target is the Sony VAIO P running Linux.

## Installation

1. Extract the archive somewhere outside `Program Files` (the binaries
   do not need administrator rights to run).
2. Copy `examples/server.toml` next to `ph0sphor-server.exe` and edit
   it. At minimum set `[server].name` and either generate static
   tokens in `[security].tokens` or enable pairing.
3. Run `ph0sphor-server.exe --config server.toml`. The console window
   stays open while the server is alive; pair clients from another
   shell with `ph0sphorctl.exe pair confirm <code>`.

## Run as a Windows Service (optional)

For an always-on workstation, register the server with `sc.exe` or
a wrapper like NSSM:

```powershell
nssm install PHOSPHOR "C:\Tools\ph0sphor\ph0sphor-server.exe" "--config C:\Tools\ph0sphor\server.toml"
nssm set    PHOSPHOR AppExit Default Restart
Start-Service PHOSPHOR
```

The server binds to `127.0.0.1:7077` by default; open Windows
Firewall for the LAN range you want VAIO clients to reach from.

## Checksums

```powershell
Get-FileHash -Algorithm SHA256 ph0sphor-vX.Y.Z-windows-x86_64.zip
```

Compare against the value in the matching `.sha256` file.
