# ADR-0031: Split `NonFiniteResult` into `Overflow` and `Undefined`

- **Status:** Accepted
- **Date:** 2026-07-13
- **Deciders:** Project owner
- **Amends:** [ADR-0021](0021-fallible-operations-on-non-finite-results.md) (its
  "A dedicated error variant for a non-finite result" section)

## Context

[ADR-0021](0021-fallible-operations-on-non-finite-results.md) made the
value-returning operations fallible and added a single
`TvmError::NonFiniteResult` variant — "an operation did not produce a finite
amount." It deliberately covered **two different failure modes** under one name:

1. **Overflow** — a genuine result exists mathematically but is too large for
   `f64`, so the arithmetic became an infinity or `NaN` (e.g. compounding an
   enormous rate over a long horizon).
2. **Undefined** — the operation has *no* answer for the inputs at all; a
   degenerate case that happens to surface as a non-finite `f64` (e.g. an
   `annuity::payment` over zero periods, a solved period count whose logarithm
   has a non-positive argument, a `modified_internal_rate_of_return` on a series
   with no span or no outflows).

These are different things a caller wants to act on differently: overflow means
"your inputs are absurdly large"; undefined means "this question has no answer."
Reporting both as one variant forces the caller to guess which it hit, and it
made the variant's own name a poor fit — an *undefined* result is non-finite
too, so "non-finite result" did not distinguish the two cases it merged. The
crate is still pre-`1.0`, so `TvmError` can be reshaped now at no compatibility
cost (`TvmError` is also `#[non_exhaustive]`).

## Decision

**Replace `TvmError::NonFiniteResult` with two variants:**

- **`TvmError::Overflow`** — an operation's `f64` arithmetic overflowed the
  finite range: a real result too large to represent.
- **`TvmError::Undefined`** — an operation is mathematically undefined for the
  given inputs: a degenerate case with no answer, not an overflow of a real one.

The two are separated **at the source**, not guessed after the fact:

- The `Money`/`Period`/`Rate::from_operation` funnels (and `Rate::nominal_annual`)
  return **`Overflow`** — a non-finite value reaching them is, by construction, a
  real result that left the representable range, because every *nameable*
  degenerate precondition is guarded first.
- Each degenerate case gets an **explicit precondition guard** at its call site
  that returns **`Undefined`** *before* the arithmetic funnels through
  `from_operation`. This covers `annuity::payment` / `annuity::due::payment`
  (zero periods), `annuity::periods` / `periods_from_future` (non-positive
  logarithm argument or zero payment), `single_sum::periods` (zero rate or a
  non-positive/non-finite `future / present`), `single_sum::rate` (zero periods),
  `Cashflows::modified_internal_rate_of_return` (fewer than two cashflows, or no
  outflows), and `Schedule::with_payment` (a payment that never amortises the
  balance).
- The low-level `Money` arithmetic follows the same rule: `try_mul` returns
  `Undefined` for a non-finite factor and `try_div` returns `Undefined` for a
  zero or `NaN` divisor (division by zero has no defined value), reserving
  `Overflow` for a finite operand that pushes the result out of range.

Each guard pins the *exact* degenerate boundary (`periods == 0.0`,
`!(arg > 0.0)`, `present_outflows == 0.0`, …) so a genuine overflow is never
mislabelled `Undefined`, nor a degenerate case mislabelled `Overflow`.

## Consequences

- A caller can distinguish "inputs too large" from "no answer exists" by matching
  the variant, without inspecting the inputs.
- The CLI and MCP surfaces need no structural change: both already funnel every
  `TvmError` through their generic Display-based mapper, so only the error
  *message text* changes (`Overflow` → "operation overflowed the finite range",
  `Undefined` → "operation is undefined for these inputs").
- Every *future* operation follows the same rule: a known degenerate precondition
  is guarded and returns `Undefined`; only a genuine overflow reaches a
  `from_operation` funnel and returns `Overflow`.
- The historical ADRs (0015, 0021, 0023–0027, 0029) still name `NonFiniteResult`;
  per the append-only ADR convention they are left as written, and this ADR is
  the authority on the split.

## Alternatives considered

- **Keep `NonFiniteResult` for the overflow half and add only `Undefined`** — less
  churn, but leaves the retained name actively misleading (an undefined result is
  also non-finite), so the two variants it is meant to separate would not be
  named apart.
- **`Overflow` + `Indeterminate`** — `Indeterminate` leans on the `0/0`
  indeterminate-form framing, but several degeneracies here (`ln` of a negative,
  division by zero) are not indeterminate forms; `Undefined` is the honest
  umbrella.
- **Minimal structural split — reclassify only the existing explicit guards, and
  leave the degenerate cases that currently fall through `from_operation`
  reporting `Overflow`** — smaller diff, but `annuity::payment` over zero periods
  (ADR-0021's own exemplar of the undefined case) would still report `Overflow`,
  defeating the purpose of the split.
- **Leave a single variant and document both meanings** — the status quo the
  review found wanting; the caller still cannot tell the two apart.
