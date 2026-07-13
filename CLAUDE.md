# CLAUDE.md — time_value

## Purpose

`time_value` is a type-safe time-value-of-money (TVM) library for Rust,
published on crates.io as `time_value` (the GitHub repo is `time-value`,
kebab-cased per the org ruleset). It is a deliberately type-heavy redesign for
the `1.0` line — not a port of the old `0.x` series.

The repo is a **Cargo workspace** (see `docs/adr/0002-workspace-layout.md`) of
four crates — three primary, plus one internal support crate:

- `crates/time_value` — the `no_std` core library (the published crate).
- `crates/time-value-cli` — the `time-value` CLI binary.
- `crates/time-value-mcp` — the `time-value-mcp` MCP server binary.
- `crates/time-value-daycount` — internal, unpublished ACT/365 day-count shared
  by the two binaries (`docs/adr/0030-shared-day-count-support-crate.md`).

Dependencies point one way, toward the library; the binaries depend on
`time_value` (and on `time-value-daycount`) by workspace path. Async is contained
to `-mcp`; the core stays synchronous and `no_std`. Architecture decisions are
logged under `docs/adr/`.

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

- `flake.nix` is the single source of truth **for the toolchain**: a devShell
  (via `oxalica/rust-overlay`, toolchain pinned in `rust-toolchain.toml`) that
  provides `cargo`, `clippy`, `rustfmt`, `bacon`, `cargo-nextest`, `cargo-deny`,
  and `nixfmt`, plus `git-hooks.nix` pre-commit hooks installed on shell entry.
- Verification runs **through the flake** as `nix develop -c cargo …` (the same
  commands locally and in CI — see `docs/adr/0012-ci-and-release-automation.md`),
  so there is no second definition of the toolchain or the tools:

  ```sh
  nix develop -c cargo fmt --all -- --check
  nix develop -c cargo clippy --workspace --all-targets -- -D warnings              # no_std default
  nix develop -c cargo clippy --workspace --all-targets --all-features -- -D warnings
  nix develop -c cargo clippy -p time_value --no-default-features --features libm --all-targets -- -D warnings  # no_std + libm
  nix develop -c cargo nextest run --workspace --all-features
  nix develop -c cargo test --doc --workspace --all-features                        # doctests
  nix develop .#msrv -c cargo build -p time_value --all-features                    # core MSRV (1.85): build, not test, so dev-deps don't gate it
  nix develop -c cargo deny check
  ```

  The core `time_value` crate keeps a conservative **MSRV of 1.85** (declared per
  crate, below the workspace's 1.88 toolchain, which the MCP crate's deps force);
  the `.#msrv` devShell pins rustc 1.85 and CI verifies the core there so the
  promise can't silently regress (`docs/adr/0017-per-crate-msrv-core-1.85.md`).

  Both feature configurations are checked: default features build the `no_std`,
  zero-dep core (catching an accidental `std` dependency), and `--all-features`
  exercises the feature-gated operations and their tests. Doctests are run
  separately because `nextest` does not run them.

- `nix flake check` now only validates the pre-commit hook set (the crane build
  checks were retired when CI moved to `nix develop -c cargo …`).
- `bacon.toml` defines the bacon jobs; the default `clippy` job mirrors the CI
  clippy check across the workspace. `.zellij/layout.kdl` wires bacon into a
  project layout.

## Repo structure

```
flake.nix                 # devShell (toolchain + tools) + git-hooks
rust-toolchain.toml       # pinned toolchain (oxalica/rust-overlay reads this)
Cargo.toml                # [workspace]: members, shared package/deps/lints
deny.toml                 # cargo-deny: licenses + advisories + bans
release-plz.toml          # release-plz: versions/changelogs/tags/GH releases
crates/
  time_value/             # core library (no_std) — the published crate
  time-value-daycount/     # internal ACT/365 day-count (unpublished, ADR-0030)
  time-value-cli/          # binary `time-value`
  time-value-mcp/          # binary `time-value-mcp`
docs/adr/                 # architecture decision records
bacon.toml                # bacon jobs (default: clippy)
.zellij/layout.kdl        # project Zellij layout with a bacon pane
.helix/languages.toml     # Helix: rust (clippy check) + nix (nixfmt)
.github/workflows/
  ci.yml                  # CI: nix develop -c cargo fmt/clippy/nextest/deny
  release-plz.yml         # versions + changelogs + tags + GitHub releases
  publish.yml             # tag-triggered `cargo publish` (crates.io OIDC)
```

## CI / release

- CI runs `nix develop -c cargo fmt/clippy/nextest/deny` on pushes to `main` and
  on PRs (one `ci` job). The job id `ci` is the required status check enforced by
  the org ruleset — **do not rename it, set a custom `name:`, or remove it**.
- Release: `release-plz.yml` drives per-crate versions, changelogs, tags, and
  GitHub releases from Conventional Commits (`release-plz.toml`, `publish =
  false`); `publish.yml` then `cargo publish`es via crates.io OIDC trusted
  publishing (no token secret) on a version tag. **The first release ships all
  three crates together at `1.0.0`** — the core is completed and hardened *before*
  release (fallibility per ADR-0021, Tier-1 completeness), and the CLI/MCP launch
  with it, rather than the core publishing first with the binaries deferred
  (**ADR-0022**; tracked by the `1.0.0` milestone + the "Road to 1.0.0" epic #34,
  with deferred work in the "Post-1.0 backlog" milestone). Old versions
  `0.1.0`–`0.8.0` remain published+immutable.
- Release setup status: Actions read/write + "allow Actions to create PRs" are
  enabled (org-level, via the `github-settings` IaC repo); the crates.io Trusted
  Publisher for `time_value` is registered. Still to do before the binaries
  publish (issue #20): flip `-cli`/`-mcp` off `publish = false`, extend
  `release-plz`/`publish.yml` to version + publish them, and register their
  Trusted Publishers. The release-plz "chore: release v1.0.0" PR (#28) is **held**
  until the `1.0.0` sequence completes.

## Deletion & creation

Layered on the global floor (`~/.claude/CLAUDE.md`). What is sensitive *here*:

- **Ask before deleting** ADRs (`docs/adr/*` — an append-only decision log; supersede
  with a new ADR rather than deleting), `LICENSE-*`, `Cargo.lock`, or the pinned
  `rust-toolchain.toml`.
- **Never rename or delete** the `ci` job / status check (see above), and never
  rename the published `time_value` crate.
- **New crates** join the workspace under `crates/`, inherit `[workspace.package]`
  /`[workspace.dependencies]`/`[workspace.lints]` (`field.workspace = true`,
  `[lints] workspace = true`), and get a `README.md`; non-core crates start
  `publish = false`.
- **New ADRs** are the next free number under `docs/adr/`, copied from
  `0000-adr-template.md`.

## Conventions

- Never commit to `main`; branch + PR, squash-merge. Branch names must match
  `^(feat|fix|chore|docs|refactor)/.*` (repo ruleset). Commits are Conventional
  Commits (they feed `release-plz`).
- Run the `nix develop -c cargo …` checks (or let bacon / the pre-commit hooks
  run) before pushing.
