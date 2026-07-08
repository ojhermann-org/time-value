# ADR-0012: CI and release automation

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

Two facts shape CI and release. First, the dev environment is a Nix flake
([ADR-0008](0008-nix-flake-dev-environment.md)) that already pins the toolchain,
`cargo-nextest`, and `cargo-deny`. Second, unlike the sibling repositories,
`time_value` is a **public** crate already on crates.io under a permissive license
([ADR-0006](0006-license.md)) — so publishing is a first-class goal, not a
deferred one.

## Decision

### CI runs *through the flake*

GitHub Actions installs Nix and runs every check as `nix develop -c cargo …`, so
CI executes the exact toolchain and tools the flake provides — no second,
drifting definition. One job runs, in order:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo nextest run --workspace`
- `cargo deny check`

It runs on every push to `main` and every pull request. The job id is **`ci`**,
which is the **required status check** enforced by the org ruleset — it must not
be renamed, given a custom `name:` (which would change the surfaced check name),
or removed.

### `release-plz`, versioning from Conventional Commits

Releases are driven by [`release-plz`](https://release-plz.dev): a `release-pr`
job maintains a "Release" PR that bumps versions and writes changelogs from
Conventional Commits; a `release` job tags and creates GitHub releases when that
PR merges. Versions are **per-crate and independent**
([ADR-0002](0002-workspace-layout.md)), `release-plz`'s default.

### crates.io publishing via OIDC trusted publishing

`time_value` is public, so we *do* publish — but without a long-lived token. A
tag-triggered workflow runs `cargo publish` authenticated by **crates.io OIDC
trusted publishing**, so there is no `CARGO_REGISTRY_TOKEN` secret to store or
rotate. **The core `time_value` crate publishes first**; `time_value-cli` and
`time_value-mcp` carry `publish = false` until their surfaces stabilise, then opt
in. Versions `0.1.0`–`0.8.0` remain published and immutable on crates.io; the
redesign continues the line at `1.0.0`.

### Dependency-audit policy

`deny.toml` allows the permissive licenses compatible with `MIT OR Apache-2.0`
([ADR-0006]) and denies yanked crates; duplicate versions are warned, not failed.

## Consequences

- Every push and PR is gated by the same checks a developer runs locally, from
  the same flake.
- A release is a single reviewed PR; the changelog is a byproduct of commit
  discipline.
- Publishing needs no stored secret — the OIDC exchange is scoped to the tag
  workflow.

## Alternatives considered

- **A `rustup` + apt CI runner** — conventional, but re-declares the toolchain the
  flake already pins, inviting drift ([ADR-0008]).
- **`publish = false` for now** (as `rustrolabe` does) — wrong here: the crate is
  already public and continuing its published line.
- **A crates.io API token secret** — works, but is a long-lived credential to
  guard and rotate; OIDC trusted publishing removes it entirely.
- **Hand-rolled tag/changelog scripting** — reimplements what `release-plz`
  derives from the commit convention already in use.
