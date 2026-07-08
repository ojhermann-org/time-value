# time_value-mcp

A [Model Context Protocol](https://modelcontextprotocol.io) server exposing
[`time_value`](../time_value)'s time-value-of-money calculations as tools over
stdio. Installs the `time-value-mcp` binary. Its design is
[ADR-0011](../../docs/adr/0011-mcp-server.md); async is contained to this crate,
the core library stays synchronous.

## Tools

| Tool | Result |
|------|--------|
| `npv`, `nfv` | net present / future value of a cashflow series at a per-period rate |
| `irr` | internal rate of return of a series |
| `present_value`, `future_value` | a single sum over a number of periods |
| `annuity_present_value`, `annuity_future_value`, `annuity_payment` | ordinary (end-of-period) annuities |

Rates are per period; cashflows are signed (outflow negative). Each tool returns
a one-field structured JSON result keyed by the operation.

## Install

Not yet published to crates.io (see [ADR-0012](../../docs/adr/0012-ci-and-release-automation.md)).
From a checkout of the repository:

```sh
cargo install --path crates/time_value-mcp   # installs the `time-value-mcp` binary
```

## Running

```sh
time-value-mcp   # speaks MCP JSON-RPC over stdin/stdout
```

Point an MCP client (e.g. an assistant that speaks MCP) at the binary; it will
`initialize`, list the tools, and call them.

## License

Dual-licensed under [Apache-2.0](../../LICENSE-APACHE) or [MIT](../../LICENSE-MIT),
at your option.
