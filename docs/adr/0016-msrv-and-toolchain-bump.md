# ADR-0016: Toolchain & MSRV bump to 1.88 for the MCP server

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner
- **Amends:** [ADR-0007](0007-rust-edition-and-msrv.md) (the version numbers)

## Context

[ADR-0007](0007-rust-edition-and-msrv.md) pinned the toolchain and MSRV to
**1.85**, kept in step. Building the MCP server ([ADR-0011](0011-mcp-server.md))
broke that: `rmcp` 2.1 pulls in `rmcp-macros`, which hard-requires
`darling ^0.23`, and `darling 0.23` requires **rustc 1.88**. `darling` cannot be
pinned lower without violating `rmcp-macros`'s own requirement, so 1.85 can no
longer build the workspace.

## Decision

Raise the pinned toolchain (`rust-toolchain.toml`) and the workspace MSRV
(`[workspace.package] rust-version`) to **1.88** — the lowest version the
dependency tree admits. ADR-0007's principle is kept: the toolchain and the
declared MSRV stay a single number, in step and CI-verified (CI compiles on
exactly this toolchain).

The floor is chosen as the **minimum** that builds, not the latest release, to
keep the library's MSRV as conservative as the workspace allows.

## Consequences

- The whole workspace — including the `no_std` core — now requires rustc ≥ 1.88.
  The core uses only long-stable features and would build on less, but the
  workspace has one pinned toolchain, and an unverified lower MSRV would be a
  claim CI does not test.
- Shipping the MCP server ties the MSRV to a fast-moving dependency tree
  (`rmcp`, `schemars`, `darling`); future updates may force further bumps. Each
  such bump is a deliberate, documented change (a new ADR or an amendment here),
  not incidental.

## Alternatives considered

- **Pin `darling` below 0.23** — impossible; `rmcp-macros 2.1` requires `^0.23`.
- **An older `rmcp` that builds on 1.85** — a different, older SDK API; a rewrite
  against an unknown surface to avoid a version bump. Rejected.
- **Split the versions — toolchain 1.88, core MSRV 1.85** — the core would then
  claim an MSRV nothing verifies unless CI runs a *second* 1.85 toolchain just to
  build the core. Disproportionate for the benefit; a single honest number is
  simpler.
- **Jump to the latest stable (1.96)** — more headroom against future bumps, but a
  needlessly high MSRV now; the conservative floor is friendlier and re-bumping
  is cheap when actually forced.
