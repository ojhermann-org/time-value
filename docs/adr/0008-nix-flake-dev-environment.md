# ADR-0008: Nix flake dev environment

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

We want every contributor — and CI — to build and check the workspace with the
*same* toolchain and the *same* auxiliary tools (`cargo-nextest`, `cargo-deny`,
`nixfmt`, `bacon`), with no "works on my machine" drift and no second definition
of "the toolchain". The project is happy to require Nix.

## Decision

`flake.nix` provides a **devShell** that is the single source of truth for the
toolchain and tools:

- The Rust toolchain is built by `oxalica/rust-overlay` from
  `rust-toolchain.toml` ([ADR-0007](0007-rust-edition-and-msrv.md)), with
  `clippy`, `rustfmt`, and `rust-src`.
- The shell also provides `bacon`, `cargo-nextest`, `cargo-deny`, and `nixfmt`.
- `git-hooks.nix` installs fast pre-commit hooks (rustfmt, nixfmt, typos,
  trailing-whitespace, EOF, TOML/merge-conflict/private-key checks) on shell
  entry; kept as a local convenience.
- `direnv` (`.envrc`) enters the shell automatically; otherwise `nix develop`.

Verification runs **through the flake** — `nix develop -c cargo …` — both locally
and in CI ([ADR-0012](0012-ci-and-release-automation.md)), so CI executes the
exact tools the flake pins.

`nix flake check` validates only the pre-commit hook set. The heavy checks
(clippy/test/deny) are the `cargo` commands above, not crane derivations — CI is
the one place they run to completion, and re-encoding them as flake checks would
be a second source of truth for the same commands.

## Consequences

- The compiler, `nextest`, `deny`, and `nixfmt` versions are identical for every
  developer and for CI.
- Onboarding is `direnv allow` (or `nix develop`); nothing to install by hand.
- Contributors must have Nix. That is an accepted requirement for this project.

## Alternatives considered

- **`rustup` + system packages** — conventional and Nix-free, but re-declares the
  toolchain and tools that the flake already pins, inviting CI-vs-local drift.
- **Crane build/clippy/test checks in the flake** (the earlier setup) — runs the
  build as Nix derivations, but duplicates the `cargo` commands CI runs and adds
  a build path that must be maintained alongside them; retired in favour of one
  definition (`nix develop -c cargo …`).
