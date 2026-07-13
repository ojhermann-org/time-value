# ADR-0010: CLI surface

- **Status:** Accepted (amended by 0028, 0029)
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

`time_value-cli` (binary `time-value`) is a thin command-line surface over the
library. The library's headline feature — periodicity encoded in the type so a
mismatch is a compile error — is a *compile-time* guarantee; at a CLI boundary
the inputs arrive as runtime strings, and the periodicity tag does not change any
result (NPV/IRR/PV/FV/annuity arithmetic is identical for every periodicity). So
the CLI must decide how much of the type model to surface, and how to shape
commands, output, and errors.

## Decision

### Commands mirror the library operations

| Command | Library call | Key arguments |
|---------|--------------|---------------|
| `npv` | `Cashflows::net_present_value` | `--rate`, `CASHFLOW…` |
| `nfv` | `Cashflows::net_future_value` | `--rate`, `CASHFLOW…` |
| `irr` | `Cashflows::internal_rate_of_return_from` | `[--guess]`, `CASHFLOW…` |
| `pv` | `present_value` | `--rate`, `--periods`, `--future` |
| `fv` | `future_value` | `--rate`, `--periods`, `--present` |
| `annuity pv` | `annuity::present_value` | `--rate`, `--periods`, `--payment` |
| `annuity fv` | `annuity::future_value` | `--rate`, `--periods`, `--payment` |
| `annuity payment` | `annuity::payment` | `--rate`, `--periods`, `--present` |

Cashflows are **positional** (`CASHFLOW…`, period 0 first), with hyphen values
allowed so outflows are written naturally (`time-value npv --rate 0.01 -100 60
60`). Arg parsing is `clap` (derive).

### Rate is per-period; periodicity is implicit

`--rate` is a **per-period** rate. The CLI does not take a periodicity: it would
not affect any result, and the type-level safety it provides is a *library*
concern already discharged before the boundary. Internally the CLI uses a single
fixed periodicity marker to satisfy the type parameter. A `--periodicity` flag
(for labelling, or once rate *conversions* exist) is a non-breaking future add.

### Output: plain number, or `--json`

By default a command prints the single result value (full `f64` precision) to
stdout. A global `--json` flag instead prints a one-field JSON object keyed by
the operation (e.g. `{"npv":18.2237}`), built with `serde_json` — the CLI does
not require the library's `serde` feature.

### Errors and exit codes

`clap` handles usage errors (exit 2). Domain failures — an invalid rate, a
non-convergent IRR, a degenerate annuity payment — are `TvmError`s mapped through
`anyhow` with context and printed to stderr as `error: …`, exiting non-zero. The
library's typed errors are the source of truth; `anyhow` is the binary-only
ergonomic layer ([ADR-0004](0004-error-handling.md)).

## Consequences

- The CLI is a faithful, discoverable calculator over the library surface.
- Users are not asked for a periodicity that would not change the answer; the
  door is open to add one non-breakingly.
- JSON output is available for scripting without coupling the library to `serde`.
- Integration tests drive the compiled binary (`assert_cmd`) and assert on
  stdout/stderr/exit, matching how users invoke it.

## Alternatives considered

- **Require `--periodicity` and dispatch to the tagged types** — ceremony with no
  effect on results; rejected until a feature (rate conversion) makes it
  meaningful.
- **A single `calc`-style command with a mode flag** — less discoverable than
  named subcommands and worse `--help`.
- **Rounded/fixed-decimal output by default** — lossy; full precision is the
  honest default, and formatting is the caller's job (or a future `--precision`).
