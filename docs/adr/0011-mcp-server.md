# ADR-0011: MCP server

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

`time_value-mcp` (binary `time-value-mcp`) exposes the library to Model Context
Protocol clients — an assistant can call the time-value-of-money operations as
tools. MCP is a JSON-RPC protocol; the reference Rust SDK, `rmcp`, is async and
speaks it over a transport (here, stdio). The library itself is synchronous and
`no_std` ([ADR-0003](0003-synchronous-computation-model.md)), so the async
surface must be confined to this crate.

## Decision

Build the server with the **`rmcp` SDK over stdio**, mirroring the `-cli` surface
([ADR-0010](0010-cli-surface.md)) as tools.

### Stateless server, tools mirror the library

The operations are pure functions of their inputs, so the server holds no state
beyond its tool router. Eight tools map one-to-one to the library:

`npv`, `nfv`, `irr`, `present_value`, `future_value`, `annuity_present_value`,
`annuity_future_value`, `annuity_payment`.

Tools are declared with `rmcp`'s `#[tool_router]` / `#[tool]` / `#[tool_handler]`
macros. Each returns a **structured** JSON result keyed by the operation
(`CallToolResult::structured`), matching the CLI's `--json` shape.

### Typed inputs → advertised schemas

Each tool's arguments are a `schemars`-derived struct in `params.rs` (`#[derive(Deserialize, JsonSchema)]`),
with field doc comments as the schema descriptions. `tools/list` therefore
advertises a JSON-Schema input for every tool; parsing stays in the binary, and
the library's typed core is untouched.

### Errors and async containment

Domain failures (`TvmError` — an out-of-range rate, a non-convergent IRR, a
degenerate annuity) map to MCP `invalid_params` errors carrying the library's
message; they are caused by the caller's arguments. Async lives **only** here:
`tokio` drives `rmcp`'s stdio transport, and the tool bodies call the synchronous
library directly. The server advertises its identity (`time-value-mcp`) and a
short instruction string on initialise. Like the CLI, it builds JSON with
`serde_json` and does not require the library's `serde` feature.

## Consequences

- An MCP client can discover and call every library operation, with schemas.
- The async runtime and the MCP dependency tree appear in exactly one crate.
- The tool surface tracks the CLI surface, so the two stay in parity.
- `rmcp`'s dependency tree sets a compiler floor — see
  [ADR-0016](0016-msrv-and-toolchain-bump.md).

## Alternatives considered

- **Hand-roll JSON-RPC / MCP** — reimplements the protocol `rmcp` already
  provides; rejected.
- **Return text rather than structured results** — structured output lets clients
  consume the number directly and carries the schema; text would be a
  regression.
- **Share code with the CLI via a common crate** — the surfaces are small and the
  input shapes differ (positional vs named JSON); premature to factor out.
