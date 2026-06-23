# CLAUDE.md — time_value

## Purpose

`time_value` is a type-safe time-value-of-money (TVM) library for Rust,
published on crates.io as `time_value` (the GitHub repo is `time-value`,
kebab-cased per the org ruleset). It is a deliberately type-heavy redesign for
the `1.0` line — not a port of the old `0.x` series.

## Design principles

- **Make TVM mistakes compile errors.** The headline bug TVM cares about is
  applying a rate of one periodicity to cashflows of another (e.g. an annual
  rate on monthly flows). Encode periodicity in the type (`Rate`/`Cashflows`
  tagged with a zero-cost periodicity marker) so the compiler rejects the
  mismatch.
- **Type-heavy *and* friendly.** Validated newtypes (`Rate`, `Money`, `Period`)
  with fallible constructors, but type aliases and inference keep the common
  path one-liner-clean. Avoid full dimensional-analysis types — TVM stays in
  "money", so they add ceremony without catching the real (semantic) errors.
- **`#![no_std]` + zero dependencies.** Transcendental functions (`powf`, `ln`,
  `exp`) are `std`-only; when the API needs them, prefer an optional `libm`
  feature over an unconditional dependency.
- **Currency is *not* type-tagged in `1.0`** — `Money` is a plain newtype.
  Adding a feature-gated currency tag later is non-breaking; baking it in now
  would not be removable without a major bump.

## Tooling (Nix-native, no prek)

- `flake.nix` is the single source of truth: `crane` for build/clippy/test/doc/fmt
  checks, `git-hooks.nix` for fast pre-commit hooks, `oxalica/rust-overlay` for
  the toolchain pinned in `rust-toolchain.toml`.
- `nix flake check` runs everything CI runs. The dev shell (`nix develop`, or
  `direnv` via `.envrc`) provides the toolchain, `bacon`, `cargo-nextest`, and
  `nixfmt`, and installs the git hooks on entry.
- `bacon.toml` defines the bacon jobs; the default `clippy` job mirrors the
  `clippy` flake check. `.zellij/layout.kdl` wires bacon into a project layout.

## Repo structure

```
flake.nix              # devShell + crane checks + git-hooks (single source of truth)
rust-toolchain.toml    # pinned toolchain (oxalica/rust-overlay reads this)
Cargo.toml             # crate metadata + lints (dual MIT OR Apache-2.0)
src/lib.rs             # the crate (no_std)
bacon.toml             # bacon jobs (default: clippy)
.zellij/layout.kdl     # project Zellij layout with a bacon pane
.helix/languages.toml  # Helix: rust (clippy check) + nix (nixfmt)
.github/workflows/ci.yml  # CI: `nix flake check` + required `ci` gate job
```

## CI / release

- CI runs `nix flake check` on PRs. The `ci` gate job is the required status
  check enforced by the org ruleset — **do not rename or remove it**.
- Release is not yet wired. Plan: tag-triggered `cargo publish` via crates.io
  OIDC trusted publishing (no token secret), first publish at `1.0.0`. Old
  versions `0.1.0`–`0.8.0` remain published+immutable on crates.io.

## Conventions

- Never commit to `main`; branch + PR, squash-merge. Branch names must match
  `^(feat|fix|chore|docs|refactor)/.*` (repo ruleset).
- Run `nix flake check` (or let bacon/pre-commit hooks run) before pushing.
