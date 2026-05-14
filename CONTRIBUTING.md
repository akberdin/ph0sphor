# Contributing to PHOSPHOR

Thanks for your interest in PHOSPHOR. This document explains how to set up a
development environment and what rules contributions must follow.

## Ground Rules

Before opening a pull request, read the README — especially section 2
("Core Design Decision") and section 27 ("Development Rules for Language
Models and Contributors"). The short version:

```text
The server is smart.
The client is thin.
The network protocol is compact.
The default security mode is read-only.
```

Any change that violates these rules will be rejected or redesigned.

## Development Setup

Requirements:

- Rust (stable). The version pinned in `rust-toolchain.toml` is the source of
  truth.
- A Linux, macOS or Windows host. For the client, a low-power Linux machine
  (ideally a Sony VAIO P) is the canonical target.

Build everything:

```bash
cargo build --workspace
```

Run tests:

```bash
cargo test --workspace
```

Lint:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Project Layout

See `README.md` section 19 for the canonical layout. Briefly:

- `crates/ph0sphor-core` — shared domain types.
- `crates/ph0sphor-protocol` — wire schema and encode/decode logic.
- `crates/ph0sphor-server` — server binary and collectors.
- `crates/ph0sphor-client` — VAIO client binary and TUI.
- `crates/ph0sphorctl` — administrative CLI.
- `proto/` — Protobuf schemas.
- `docs/` — design, protocol, security, configuration docs.
- `examples/` — sample configs and demo data.

## Commit and PR Style

- Keep commits focused. One logical change per commit when possible.
- Subject line ≤ 70 characters, imperative mood
  (e.g. `protocol: add FullSnapshot encoder`).
- The body explains *why* the change is necessary, not just *what* changed.
- Reference issues with `Refs #N` or `Fixes #N` where applicable.

Pull requests must:

1. Pass `cargo fmt`, `cargo clippy`, and `cargo test`.
2. Not introduce new heavy dependencies without justification.
3. Not move heavy work onto the VAIO client.
4. Not relax the default read-only security mode.
5. Update relevant docs in `docs/` when behavior or protocol changes.

## When Adding a Feature

Answer the seven questions from README §27.2 before writing code. If any
answer is "no", redesign.

## License

By contributing, you agree that your contributions will be licensed under
the project's MIT license (see `LICENSE`).
