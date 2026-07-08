# ADR-0018: Kebab-case binary crate names

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner
- **Amends:** [ADR-0002](0002-workspace-layout.md) (the binary crate names)

## Context

The workspace has three crates ([ADR-0002](0002-workspace-layout.md)). The core
library is published as **`time_value`** (snake_case), and that name is fixed
twice over: crates.io normalises `-`/`_` and locks the canonical form at first
publish, so the existing `0.1.0`–`0.8.0` history pins it; and independently, a
Rust library's crate/import name must be a valid identifier, so it is
`time_value` (with an underscore) whatever the package is called.

ADR-0002 named the two binary crates `time_value-cli` and `time_value-mcp`, to
match the core's package prefix. But those are a **mixed** snake+kebab form, and
every *other* user-facing identifier that is not the (forced-snake) library is
kebab-case: the GitHub repository (`time-value`, per the org ruleset), the two
installed binaries (`time-value`, `time-value-mcp`), and the MCP server's
advertised product name (`time-value-mcp`). The binary crates are unpublished
(`publish = false` — [ADR-0012](0012-ci-and-release-automation.md)), so their
package names carry no crates.io identity and are free to change.

## Decision

Name the binary crates in **kebab-case** — **`time-value-cli`** and
**`time-value-mcp`** (directories `crates/time-value-cli`,
`crates/time-value-mcp`) — matching the repository, the installed binary names,
and the MCP product name.

The core library package stays **`time_value`**: it is fixed (the crates.io
identity and the Rust import name), so it is the single, deliberate snake_case
name, not an inconsistency to chase. The rule across the surface is simply: the
*importable library* is `time_value`; every *product/binary/repo* identifier is
`time-value…`.

With the MCP package now named `time-value-mcp`, `env!("CARGO_PKG_NAME")` equals
the advertised server name, so the server reports its identity from the build
environment instead of a hardcoded string.

## Consequences

- One naming rule to state and remember; the mixed `time_value-cli` form is gone.
- `cargo install --path crates/time-value-cli` (and `…-mcp`); workspace member
  paths and doc references updated. No crates.io impact — the binaries are
  unpublished and the core name is unchanged.
- Existing ADR bodies still spell the old `time_value-cli` / `time_value-mcp`
  names as historical record; this ADR is the source of truth for the current
  names (ADRs are immutable — [ADR-0001](0001-record-architecture-decisions.md)).

## Alternatives considered

- **Keep `time_value-cli` / `time_value-mcp`** (match the core's package prefix,
  as the sibling repos do) — but their core name is itself kebab, so matching was
  free; here it propagates the *forced* underscore into names that don't need it
  and keeps a mixed form.
- **Rename the core to `time-value`** — not available: crates.io locks the
  published `time_value` name, and the Rust import stays `time_value` regardless,
  so this would fork the crate's identity for no code-level change.
