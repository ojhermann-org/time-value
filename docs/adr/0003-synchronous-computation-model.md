# ADR-0003: Synchronous computation model

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

Time-value-of-money is pure arithmetic over in-memory values: discount a
cashflow series, solve for a rate, compute an annuity payment. There is no I/O,
no network, and no waiting — the work is CPU-bound and finishes in microseconds.
The MCP server ([ADR-0011]) is the one component with an outside edge: it speaks
the Model Context Protocol over stdio, and its SDK (`rmcp`) is async.

## Decision

The **core library and the CLI are fully synchronous.** No `async`, no runtime,
no futures in `time_value` or `time_value-cli`.

**Async is contained to `time_value-mcp`.** Only that crate pulls in `tokio` and
drives `rmcp`'s async transport; it calls the synchronous library directly inside
its async tool handlers. The async boundary is therefore a crate boundary
([ADR-0002](0002-workspace-layout.md)), not a convention.

## Consequences

- The library stays `no_std`-friendly ([ADR-0009](0009-no_std-and-optional-libm.md))
  and trivially callable from any context — no coloured functions, no runtime to
  thread through.
- `tokio` and the async machinery appear in exactly one crate's dependency tree.
- If a future feature genuinely needs concurrency in the library, it gets its own
  ADR; async is not adopted pre-emptively.

## Alternatives considered

- **Async throughout** — pointless for CPU-bound arithmetic; it would colour the
  entire API and drag a runtime into a library that has nothing to await.
- **A `blocking`-style async facade in the library** — added surface for no
  benefit; the MCP crate can call the sync API directly.

[ADR-0011]: 0011-mcp-server.md
