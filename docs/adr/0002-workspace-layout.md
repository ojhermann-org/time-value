# ADR-0002: Workspace layout & crate boundaries

- **Status:** Accepted (binary-crate naming amended by [ADR-0018](0018-kebab-case-binary-crate-names.md) — the binary crates are `time-value-cli` / `time-value-mcp`; the layout and boundaries below stand)
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

`time_value` is a type-safe time-value-of-money library, and we want to ship it
three ways: as a library for Rust programs, as a command-line tool, and as an MCP
server for agents. The library is the product; the CLI and MCP server are thin
surfaces over it. Two properties matter. First, the core must stay `no_std` and
dependency-free ([ADR-0009](0009-no_std-and-optional-libm.md)) — the CLI's and
MCP server's heavier dependencies (arg parsing, an async runtime) must not leak
into it. Second, the surfaces must stay in lockstep with the library at the type
level, so a breaking change to the library cannot ship without updating them.

## Decision

We will use a **single Cargo workspace** with three member crates:

| Crate | Kind | Depends on |
|-------|------|-----------|
| `time_value` | library (`no_std`) | — (zero required deps) |
| `time_value-cli` | binary (`time-value`) | `time_value` |
| `time_value-mcp` | binary (`time-value-mcp`) | `time_value` |

- Dependencies point **one way**, toward the library; the library never depends
  on the binaries.
- The binaries depend on `time_value` **by workspace path**, so a breaking change
  cannot compile-pass its consumers without updating them. This compile-time
  coupling is our primary "stay in sync" guarantee.
- Shared metadata, dependency versions, and lints are declared once via
  `[workspace.package]`, `[workspace.dependencies]`, and `[workspace.lints]`, and
  inherited by members with `field.workspace = true` / `[lints] workspace = true`.
- There is **no shared `version`**: each crate carries its own, versioned
  independently ([ADR-0012](0012-ci-and-release-automation.md)).
- The CLI **crate** is `time_value-cli` (the name `time_value` is the library);
  its **binary** is `time-value`. The MCP crate is `time_value-mcp`, binary
  `time-value-mcp`.

## Consequences

- One `Cargo.lock`; one `cargo build` / `clippy` / `nextest` covers everything.
- The `no_std`, zero-dep boundary is a crate boundary: the CLI/MCP dependency
  trees physically cannot reach the library.
- Slightly more ceremony than a single crate with `[[bin]]` targets: three
  manifests, three READMEs, three changelogs.
- New crates join under `crates/`, inherit the workspace tables, and (if not the
  published library) start `publish = false`.

## Alternatives considered

- **One crate with `[[bin]]` targets** — fewer files, but the binary-only
  dependencies (`clap`, `rmcp`, `tokio`) become dependencies of the same crate
  that must stay `no_std`/zero-dep, defeating that boundary. Feature-gating them
  is possible but fragile — a stray default feature re-links them into the
  library.
- **Separate repositories** — maximal isolation, but loses the single lockfile
  and the compile-time path coupling that keeps the surfaces honest against the
  library's API.
- **A `-sys`/FFI crate** (as in `rustrolabe`) — unnecessary here; the core is
  pure Rust with no C to link.
