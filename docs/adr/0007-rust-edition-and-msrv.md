# ADR-0007: Rust edition & MSRV

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

A published library needs a stated edition and a minimum supported Rust version
(MSRV) so downstream users know what they are committing to. Two capabilities the
design relies on inform the floor: `core::error::Error` is stable on `no_std`
(needed by [ADR-0004](0004-error-handling.md)), and we want current-enough
language features without chasing the newest release.

## Decision

- **Edition 2021**, declared once in `[workspace.package]` and inherited by every
  crate.
- **MSRV is Rust 1.85**, declared as `rust-version = "1.85"` in
  `[workspace.package]` so `cargo` enforces it and the value is visible on
  crates.io.
- The **toolchain is pinned** in `rust-toolchain.toml` (`channel = "1.85.0"`),
  which `oxalica/rust-overlay` reads to build the exact toolchain in the dev shell
  and in CI ([ADR-0008](0008-nix-flake-dev-environment.md)). Pinned toolchain and
  declared MSRV are kept in step.
- Raising the MSRV is a deliberate, minor-version-worthy change with its own
  commit; it is not bumped incidentally.

## Consequences

- One place sets the edition and MSRV for the whole workspace.
- CI compiles on exactly the pinned toolchain, so "works on my machine" and "the
  MSRV builds" are the same statement.
- Using a language feature newer than 1.85 is a conscious MSRV bump, not an
  accident.

## Alternatives considered

- **Edition 2024** — newer, but pulls the MSRV forward for no feature this crate
  currently needs.
- **No declared MSRV / track `stable`** — less to maintain, but leaves downstream
  users guessing and lets an incidental commit silently raise the floor.
