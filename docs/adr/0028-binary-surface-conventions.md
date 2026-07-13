# ADR-0028: Binary surface conventions (CLI grammar & MCP tools)

- **Status:** Proposed
- **Date:** 2026-07-13
- **Deciders:** Project owner

## Context

The CLI ([ADR-0010](0010-cli-surface.md)) and MCP server
([ADR-0011](0011-mcp-server.md)) were designed at the eight-operation mark and
have since drifted behind the core: rate conversions (#17), annuity-due /
perpetuity / growing perpetuity (#19), solve-for-periods and solve-for-rate
(#31), MIRR (#32), and the amortization schedule (#33) all landed in the library
with no binary surface. More core features are still to come before the 1.0
freeze (XNPV/XIRR, continuous compounding, …).

Rather than retrofit each operation ad hoc, this ADR fixes the **conventions**
the binaries follow, so the surface grows deterministically and can be frozen
(ADR-0010/0011) in one deliberate pass at the end instead of reconciled
piecemeal. It extends — does not supersede — ADR-0010 and ADR-0011.

## Decision

### 1. The binaries track the core (coverage is a checklist item)

Every non-trivial core operation gets **both** a CLI command and an MCP tool.
"Surface the CLI + MCP" is a required step in every future core-feature PR, so
the binaries never silently drift behind the library again. Pure type-system
helpers (constructors, markers) are exempt.

### 2. CLI is grouped by *relationship family*

The operation space is a matrix — {value, periods, rate} solved for, over
{single-sum, annuity (ordinary / due), series}, plus rate conversions and the
amortization schedule. A flat verb namespace collides and forces overloaded
flags, so the CLI groups by the relationship the operation is about. Top-level
groups: `single-sum`, `annuity` (with a `due` sub-group), `series`, `rate`, and
the standalone `amortize`.

| Command | Library call | Arguments |
|---------|--------------|-----------|
| `single-sum pv`  | `single_sum::present_value` | `--rate --periods --future` |
| `single-sum fv`  | `single_sum::future_value`  | `--rate --periods --present` |
| `single-sum nper`| `single_sum::periods`       | `--rate --present --future` |
| `single-sum rate`| `single_sum::rate`          | `--periods --present --future` |
| `annuity pv`      | `annuity::present_value` | `--rate --periods --payment` |
| `annuity fv`      | `annuity::future_value`  | `--rate --periods --payment` |
| `annuity payment` | `annuity::payment`       | `--rate --periods --present` |
| `annuity nper`    | `annuity::periods` / `periods_from_future` | `--rate --payment (--present\|--future)` |
| `annuity rate`    | `annuity::rate` / `rate_from_future`       | `--periods --payment (--present\|--future)` |
| `annuity perpetuity`         | `annuity::perpetuity`         | `--rate --payment` |
| `annuity growing-perpetuity` | `annuity::growing_perpetuity` | `--rate --growth --payment` |
| `annuity due pv`      | `annuity::due::present_value` | `--rate --periods --payment` |
| `annuity due fv`      | `annuity::due::future_value`  | `--rate --periods --payment` |
| `annuity due payment` | `annuity::due::payment`       | `--rate --periods --present` |
| `series npv`  | `Cashflows::net_present_value`            | `--rate CASHFLOW…` |
| `series nfv`  | `Cashflows::net_future_value`            | `--rate CASHFLOW…` |
| `series irr`  | `Cashflows::internal_rate_of_return_from`| `[--guess] CASHFLOW…` |
| `series mirr` | `Cashflows::modified_internal_rate_of_return` | `--finance --reinvest CASHFLOW…` |
| `rate ear`          | `Rate::effective_annual`   | `--rate --periodicity` |
| `rate convert`      | `Rate::convert`            | `--rate --from --to` |
| `rate from-nominal` | `Rate::from_nominal_annual`| `--nominal --periodicity` |
| `rate nominal`      | `Rate::nominal_annual`     | `--rate --periodicity` |
| `amortize`    | `amortization::Schedule::for_term` / `with_payment` | `--rate --principal (--periods\|--payment)` |

This **reorganizes** the current flat `pv`/`fv`/`npv`/`nfv`/`irr` into
`single-sum …` / `series …` (the `annuity` group is unchanged). That is a
breaking change to the grammar, taken deliberately now — pre-1.0, unreleased —
so the grammar frozen at 1.0 is the consistent one.

Conventions within a family:

- **Solve-for variants collapse into one command** with a mutually-exclusive
  flag pair rather than two commands: `annuity nper --present … | --future …`
  (likewise `annuity rate`, and `amortize --periods … | --payment …`). `clap`
  arg-groups enforce exactly-one.
- **`rate` (the group) vs `… rate` (the leaf).** The top-level `rate` group is
  rate *conversion*; `single-sum rate` / `annuity rate` are Excel's RATE —
  *solving* for the per-period rate. Different paths, no collision; the shared
  word is accepted for familiarity over a coined synonym.

### 3. Periodicity appears only where it is semantic

Per ADR-0010, periodicity does not change any result except for rate
*conversions*, where it is intrinsic (annualising a per-period rate needs the
periods-per-year). So **only the `rate` group takes periodicity** — via
`--periodicity` (EAR/APR/nominal) or `--from`/`--to` (convert), naming a
[`Periodicity`] marker (`daily`, `weekly`, `monthly`, `quarterly`,
`semi-annual`, `annual`). Every other command stays periodicity-free, using the
fixed internal marker exactly as today. No global `--periodicity` label.

### 4. Output: single value, or a tabular array under the same `--json`

The existing shape stands: a scalar result prints as a plain number, or as a
one-field JSON object keyed by the operation under `--json`. **The scalar key is
the operation's MCP tool name (§5)** — bare for well-known acronyms (`npv`,
`nfv`, `irr`, `mirr`, `xnpv`, `xirr`), family-prefixed and spelled out otherwise
(`single_sum_present_value`, `annuity_present_value`, `rate_effective_annual`,
…). One operation, one identifier across both binaries: piping the CLI's `--json`
and reading the MCP tool's result yield the same key. **Tabular results**
(the amortization schedule, and any future multi-row op) extend it consistently:

- **Plain:** a header line, then one aligned row per period.
- **`--json`:** a JSON **array** of row objects (e.g. `[{"period":1,"payment":…,
  "interest":…,"principal":…,"balance":…}, …]`).

`--json` therefore always yields "the structured form of what plain text shows"
— an object for a scalar, an array for a table.

### 5. MCP mirrors the CLI, flattened to snake_case tool names

MCP has no sub-command nesting, so the CLI path flattens into the tool name.
Well-known acronyms stay bare (`npv`, `nfv`, `irr`, `mirr`); every other tool is
prefixed by its family to stay unambiguous: `single_sum_present_value`,
`single_sum_periods`, `annuity_present_value`, `annuity_due_present_value`,
`annuity_perpetuity`, `rate_effective_annual`, `rate_convert`, `amortize`, …
This **renames** the current `present_value`/`future_value` tools (now
`single_sum_*`); done pre-freeze, so the schema frozen at 1.0 is consistent.
Tabular tools return the `--json` array shape via `CallToolResult::structured`.
Inputs remain `schemars`-derived structs in `params.rs` (ADR-0011); the periodicity
markers deserialize from the lower-kebab names above.

## Consequences

- The surface grows by a fixed rule; ADR-0010/0011 freeze becomes mechanical.
- The CLI grammar and MCP tool names change once, now, before anyone depends on
  a released binary — the cost is paid while it is free.
- A future dated-cashflow family (XNPV/XIRR, #23) slots in as `series` variants
  under these same conventions; its date+amount input format is deferred to that
  feature's own design, constrained to fit here.

## Alternatives considered

- **Flat Excel-style verbs** (`pv fv npv nper rate mirr …` top-level) — familiar,
  but the solve-for matrix forces overloaded flags and a `rate` verb colliding
  with `--rate`; rejected for a grammar meant to be frozen.
- **Hybrid (keep flat, add groups for new families)** — zero churn now, but
  locks a permanently inconsistent surface into the 1.0 freeze; rejected.
- **Global optional `--periodicity` label** — uniform but adds a no-op flag to
  every command that cannot use it; rejected for noise.
- **JSON-only (or summary-only) schedules** — makes one operation behave unlike
  the rest; rejected in favour of the array extension.
