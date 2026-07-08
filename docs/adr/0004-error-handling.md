# ADR-0004: Error handling

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

The library's constructors and operations are **fallible in domain terms**: a
rate ≤ −100% is economically meaningless, a periodicity mismatch is rejected, an
IRR solve may not converge. Callers — including the CLI and MCP surfaces — need
to distinguish these cases programmatically and render them well. At the same
time the core is `no_std` and dependency-free ([ADR-0009]), which rules out the
usual `thiserror`/`anyhow` reach.

## Decision

**The library returns typed errors it owns.** A `#[non_exhaustive]` `TvmError`
enum, one variant per distinguishable failure, implementing `Display` and
`core::error::Error` (available on stable `no_std`). Fallible APIs return
`Result<T, TvmError>`. No error-handling dependency: no `thiserror` (its derive
is convenient but a dependency the zero-dep core will not take), no `anyhow` (a
boxed type, wrong for a library surface).

- `#[non_exhaustive]` lets us add variants without a breaking change; callers
  must include a wildcard arm.
- The binaries use **`anyhow`** in their `main`/handlers for ergonomic context
  and one-line propagation. `anyhow` is a *binary-only* dependency
  ([ADR-0002](0002-workspace-layout.md)); it never appears in the library.

## Consequences

- Callers can `match` on precise, documented failure modes; the CLI maps them to
  exit codes and the MCP server to structured tool errors.
- Adding a failure mode is a non-breaking `TvmError` variant, not a new error
  type.
- The library carries zero error-handling dependencies; the ergonomic sugar
  lives only where a dependency is already acceptable (the binaries).

## Alternatives considered

- **`thiserror` in the library** — removes a little boilerplate on the `Display`
  impl, but adds a proc-macro dependency the zero-dep core ([ADR-0009]) declines.
- **`anyhow`/boxed errors in the library** — erases the type information callers
  need to branch on; appropriate for an application, not a library.
- **`Box<dyn Error>`** — same loss of type information, plus an allocation the
  `no_std` core cannot assume.

[ADR-0009]: 0009-no_std-and-optional-libm.md
