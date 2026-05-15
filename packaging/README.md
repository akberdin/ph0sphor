# Packaging

Per-platform install notes live alongside the bundled assets:

- [`linux/`](linux/README.md) — server, ctl and i686 VAIO client, with
  hardened systemd units.
- [`windows/`](windows/README.md) — server + ctl, optional NSSM service
  recipe.

End-to-end install + pair walkthroughs live in
[`docs/installation.md`](../docs/installation.md). Release artifacts
themselves are produced by `.github/workflows/release.yml` on every
`v*.*.*` tag and uploaded straight to the GitHub release.
