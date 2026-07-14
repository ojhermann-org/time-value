# ADR-0040: FX convert in the binaries — a standalone `convert` surface

- **Status:** Accepted
- **Date:** 2026-07-14
- **Deciders:** Project owner
- **Follows:** [ADR-0034](0034-money-and-currency.md) (money and currency / FX),
  [ADR-0037](0037-currency-in-the-binaries.md) (currency in the binaries),
  [ADR-0028](0028-binary-surface-conventions.md) (binary surface conventions)

## Context

FX landed in the **core** in ADR-0034: [`FxRate`](../../crates/time_value/src/money.rs)
(a validated, directional `from → to` rate with `inverse`) and
[`Money::convert`](../../crates/time_value/src/money.rs). ADR-0037 deliberately
left it **core-only** — "a `convert` command/tool is a separate, additive
follow-up" — because the currency surface (`--currency` / the `currency` field)
was the step that mattered then. This ADR is that follow-up (issue #67): it fixes
how the CLI and MCP *expose* FX conversion.

The forces: the core API already exists, so this is pure surface with no core
change; a conversion names **two** currencies (source and target) that are
intrinsic to the operation, unlike the single per-invocation `--currency`; and
the result is denominated in the target currency, which the existing monetary
presentation (ADR-0037) already knows how to echo.

## Decision

**Expose FX as a single standalone `convert` operation on both binaries** — a
sibling of the standalone `amortize` (ADR-0028), not a member of a family.

- **CLI:** a top-level `convert` command taking `--from <CODE>`, `--to <CODE>`
  (each an ISO 4217 code parsed by [`Currency::from_code`], reusing the
  `--currency` value-parser from ADR-0037), `--rate <R>` (units of `to` per unit
  of `from`), and a positional `AMOUNT` denominated in `--from`. Example:
  `time-value convert --from USD --to EUR --rate 0.9 100` → `90 EUR`.
- **MCP:** a bare `convert` tool taking `{ amount, from, to, rate }`. `from`/`to`
  are the typed `CurrencyCode` (the schema-enumerated ISO-4217 string from
  ADR-0039), and are **required** — there is no agnostic-`XXX` default, because a
  conversion's currencies are intrinsic (unlike the optional `currency` field).

**The result reuses the monetary presentation unchanged (ADR-0037).** The
converted value is a `Money` tagged the target currency, so it flows through the
same `MoneyResult` DTO: a non-`XXX` target echoes its code (`90 EUR`, or a
`"currency"` JSON field); a `--to XXX` result is a bare number, exactly like any
other agnostic monetary result. No rounding is applied (ADR-0033/0037).

**The global `--currency` does not apply to `convert`.** `--from`/`--to` name the
currencies, so `convert` ignores the invocation-wide `--currency` — the way the
`rate` family ignores it (ADR-0037), because currency there is intrinsic, not a
blanket denomination.

**Naming: `convert`, bare on both surfaces.** A standalone op takes a bare name,
matching `amortize`; it does not collide with the existing `rate convert` /
`rate_convert` (a *periodicity* conversion of a rate), which stays family-prefixed
per ADR-0028 §5.

**Validation and errors are the core's.** `FxRate::new` rejects a non-finite or
non-positive rate (`InvalidExchangeRate`); the multiply can `Overflow`. Both
surface as the usual errors — a CLI `error:` line, an MCP `invalid_params`.

## Consequences

- Both binaries expose every current core operation; FX was the last core feature
  held back from the surface (ADR-0037), so "surface the CLI + MCP" (ADR-0028 §1)
  is caught up with the core.
- Purely additive: no existing command, tool, output shape, or default changes.
- Triangulation (via a base currency) and bid/ask spreads remain **out of scope** —
  they are rate-*sourcing* concerns for the caller, not core arithmetic
  (ADR-0034). A caller composes two `convert` calls, or supplies the cross rate.
- The `convert` result conforms to the ADR-0039 typed-output contract: it returns
  `Json<MoneyResult>` with an auto-declared `outputSchema`, covered by the
  output-schema conformance test.

## Alternatives considered

- **A `convert` under a currency/`fx` family** — there is only one FX operation,
  so a family prefix (`fx_convert`) buys nothing; a bare `convert` mirrors the
  bare `amortize`. Rejected as needless nesting.
- **Honour the global `--currency` as the source, `--to` as the target** — halves
  the flags but conflates an invocation-wide denomination with an operation whose
  two currencies are intrinsic; `--from`/`--to` read clearer and match the MCP
  tool. Rejected.
- **A per-currency amount / triangulation** — out of scope by ADR-0034; the core
  is one rate, one direction. Rejected here as it was there.
- **Round the converted amount to the target's minor unit** — advertises a
  precision the engine does not have; rounding stays an explicit future opt-in
  (ADR-0033/0037). Rejected.

[`Currency::from_code`]: ../../crates/time_value/src/currency.rs
