# ADR-0030: Shared day-count support crate

- **Status:** Accepted
- **Date:** 2026-07-13
- **Deciders:** Project owner

## Context

The core `time_value` library takes year-offsets, not a date type
([ADR-0029](0029-dated-cashflows-xnpv-xirr.md)): the dated-cashflow operations
(XNPV/XIRR) accept `f64` offsets so no calendar dependency reaches the `no_std`
core. The two binaries — the CLI ([ADR-0010](0010-cli-surface.md)) and the MCP
server ([ADR-0011](0011-mcp-server.md)) — accept real ISO `YYYY-MM-DD` dates and
convert them to year-offsets with a self-contained ACT/365 day-count (Howard
Hinnant's days-from-civil algorithm).

When XNPV/XIRR were surfaced on both binaries, that day-count was **copied into
each** — `days_from_civil`, `is_leap_year`, `days_in_month`, and an ISO date
parser, byte-for-byte identical in `time-value-cli` and `time-value-mcp`. Only
the error wrapping differed (an `anyhow` context in the CLI, an `invalid_params`
`ErrorData` in the MCP server). Duplicated arithmetic is a correctness hazard: a
fix or a leap-year edge case would have to be found and corrected twice.

## Decision

We will extract the day-count into a new internal workspace crate,
**`time-value-daycount`** (`crates/time-value-daycount`, `publish = false`),
depended on by both binaries via workspace path. It exposes:

- `iso_to_day(&str) -> Result<i64, ParseDateError>` — parse and validate an ISO
  `YYYY-MM-DD` date into a serial day number (days since the Unix epoch,
  proleptic Gregorian).
- `act365_year_fraction(reference, day) -> f64` — the ACT/365 year-fraction
  `(day − reference) / 365` between two serial day numbers.
- `ParseDateError` — a self-contained parse error (it owns the offending text);
  each binary maps it into its own error type via `Display`.

The calendar arithmetic and its unit tests live in this crate. It is
dependency-free, so no date/time crate reaches the binaries. Each binary keeps
its own *input* handling — the CLI's `DATE:AMOUNT` pair splitting, the MCP
server's `DatedFlow` structs — and its own error mapping; only the shared
day-count moves.

## Consequences

- The ACT/365 day-count is defined once; a fix or a new edge-case test lands in
  one place and both binaries get it.
- The crate carries focused unit tests (leap-year rule, month lengths, the epoch
  origin, malformed-input rejection) that the binaries could only reach
  indirectly through integration tests before.
- The workspace gains a fourth crate. It follows the existing conventions
  ([ADR-0002](0002-workspace-layout.md)): inherits `[workspace.package]` /
  `[workspace.lints]`, carries a `README.md`, and starts `publish = false` like
  the other non-core crates.
- Should a third consumer ever need the day-count, it depends on this crate
  rather than copying a third time.

## Alternatives considered

- **Leave the copies in place** — zero new crate, but locks in a two-place
  correctness hazard and gives the arithmetic no home for direct unit tests;
  rejected.
- **Put the day-count in the core `time_value` crate** — one fewer crate, but it
  would either add a date surface the core deliberately excludes (ADR-0029) or
  sit as dead weight behind a feature; rejected for keeping the core free of
  calendar concerns.
- **A third-party date crate (`time`, `chrono`)** — battle-tested, but pulls a
  dependency (and its transitive tree) into the binaries for a dozen lines of
  well-understood arithmetic; rejected as disproportionate, consistent with
  ADR-0029's self-contained-conversion choice.
