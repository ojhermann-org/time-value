# ADR-0017: Per-crate MSRV — the core keeps 1.85, verified separately

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner
- **Amends:** [ADR-0016](0016-msrv-and-toolchain-bump.md) (its single-MSRV stance), [ADR-0007](0007-rust-edition-and-msrv.md)

## Context

[ADR-0016](0016-msrv-and-toolchain-bump.md) raised the whole workspace to a
single MSRV of 1.88, because the MCP crate's `rmcp-macros → darling 0.23`
requires rustc 1.88. But that requirement is confined to `time_value-mcp`: the
core `time_value` crate depends only on `libm` and (optionally) `serde`, and in
fact **compiles and passes its tests on rustc 1.85** with the current lockfile
(verified: `libm 0.2`, `serde 1.0.228`, `syn 2.0` all build on 1.85). Forcing the
published, foundational `no_std` library to claim 1.88 turns downstream users on
1.85–1.87 away for no technical reason. A conservative MSRV is worth keeping — but
only if it is *verified*, since an untested MSRV silently rots.

## Decision

Keep a **per-crate MSRV**, and verify the core's:

- The core `time_value` crate declares `rust-version = "1.85"`. The workspace
  `[workspace.package] rust-version` stays `1.88`, inherited by the two binaries
  (`-cli`, `-mcp`) — tools may require a newer compiler than the library they
  wrap, which is standard practice.
- The **dev/build toolchain** (`rust-toolchain.toml`) stays **1.88** — it must
  build the whole workspace, including the MCP crate.
- **Verify the core MSRV in CI.** A `msrv` flake devShell pins a minimal rustc
  1.85; the `ci` job runs `nix develop .#msrv -c cargo build -p time_value
  --all-features` on it (a **build**, not a test — see the 2026-07-08 amendment
  below). It goes through the flake (so there is no second toolchain source —
  [ADR-0008](0008-nix-flake-dev-environment.md)) and is a step of the required
  `ci` job, so a regression **blocks the merge** rather than rotting.

## Consequences

- The core's 1.85 MSRV is a real, enforced promise; a commit that uses a newer
  feature in the core fails CI immediately.
- CI provisions two toolchains (the 1.88 workspace toolchain and the 1.85
  core-check); the 1.85 step compiles only the core, so it is cheap.
- **Shared-lockfile risk:** there is one `Cargo.lock`. If a future update bumps a
  *core* dependency (`serde`/`libm`) to a version whose own MSRV exceeds 1.85, the
  core-MSRV step fails. The fix is then a deliberate choice: pin that dependency
  down, or raise the core MSRV in a follow-up. `serde`/`libm` have low, slow-moving
  MSRVs, so this should be rare.

## Alternatives considered

- **Single MSRV of 1.88 everywhere** (ADR-0016 as first written) — simplest, but
  the core over-claims; it needs only 1.85.
- **Declare 1.85 without verifying it** — cheap, but the workspace compiles on
  1.88, so nothing catches a regression; an unverified MSRV is not a promise.
- **Move the MCP crate out of the workspace, or downgrade `rmcp`** — lets the main
  workspace sit on 1.85, but fragments the single-workspace/single-lockfile setup
  or pins to a stale SDK; more moving parts for the same result as verifying the
  core directly.

## Amendment (2026-07-08): the MSRV step *builds* the core, it does not *test* it

Adding property tests (`proptest`, a dev-dependency) surfaced a gap in the
original decision. `cargo test -p time_value` compiles the crate's
`[dev-dependencies]` and its `#[cfg(test)]` code, so the MSRV step was
transitively asserting that **the test harness** also builds on 1.85. But
dev-dependencies are not part of the published crate — they never reach a
downstream user — so a test framework that needs a newer compiler than 1.85 (as
`proptest` and its own dependency tree may) has no bearing on the core's MSRV
promise, yet would have failed the check.

The MSRV step therefore runs **`cargo build -p time_value --all-features`**
instead of `cargo test`. `build` compiles only the library target — the exact
code a downstream user compiles — and skips dev-dependencies and `#[cfg(test)]`,
so the check now asserts precisely the promise it is meant to (the *published*
core builds on 1.85) and nothing more. The core's unit tests still run, on the
1.88 workspace toolchain, in the main `Test` step; only the *1.85* verification
narrows from test to build. The shared-lockfile risk above is unchanged — it
concerns *core* runtime dependencies (`serde`/`libm`), which `build` still
compiles.
