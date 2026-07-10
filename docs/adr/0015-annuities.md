# ADR-0015: Annuities — convention, the `r → 0` limit, and a fallible payment

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

[ADR-0014](0014-transcendental-single-sum-operations.md) added the single-sum
present/future value behind `std`/`libm` and named annuities as the next
transcendental operation. Annuities carry three small modelling decisions that
deserve recording: *when* the payment falls, what to do at `r = 0` (where the
closed-form factors are `0/0`), and how to treat a payment that cannot be
defined.

## Decision

Annuity operations live in a public `annuity` module (`annuity::present_value`,
`annuity::future_value`, `annuity::payment`), gated to the same `std`/`libm`
features as the rest of the transcendental surface.

### Ordinary annuities (payments at period end)

The `1.0` line models the **ordinary** annuity — a payment at the *end* of each
period — which is the default in finance and the basis of loan amortisation.
Annuity-due (payment at period start) is a `(1 + r)` scaling away and can be
added later without breaking these signatures.

### The `r → 0` limit

Both factors — `(1 - (1+r)⁻ⁿ)/r` and `((1+r)ⁿ - 1)/r` — are `0/0` at `r = 0` and
ill-conditioned near it, but both have the finite limit `n`. Below a small rate
magnitude (`1e-9`) the functions use `n` directly; above it, the closed form.
This keeps a zero (or near-zero) rate a valid, correct input rather than a `NaN`
trap.

### `payment` is fallible; the PV/FV are not

`present_value` and `future_value` return `Money` infallibly — finite for every
valid input, including `r = 0` (via the limit). `payment` returns
`Result<Money, TvmError>`: amortising over `periods = 0` divides by a zero factor
and is genuinely undefined, so it surfaces `TvmError::NonFiniteAmount` rather than
returning a `Money` holding infinity. This matches ADR-0014's rule — fallibility
is reserved for genuinely degenerate operations.

## Consequences

- Loan/savings maths (PV of a stream, FV of contributions, level payment) is
  available `no_std`-via-`libm`, with `r = 0` handled correctly.
- The ordinary/`due` choice is explicit and non-breaking to extend.
- `payment`'s `Result` forces callers to handle the degenerate `n = 0` case.

## Alternatives considered

- **Annuity-due, or a payment-timing parameter, now** — more surface before there
  is a caller that needs it; deferred (non-breaking to add).
- **Special-case only exactly `r == 0`** — leaves a band of tiny non-zero rates
  where the closed form is numerically unstable; the small threshold is safer.
- **Infallible `payment` returning `Money(inf)` for `n = 0`** — silently violates
  the "`Money` is finite" invariant; a `Result` is honest.

## Amendment (2026-07-10): annuity-due and perpetuity

The original decision deferred annuity-due and perpetuity as non-breaking
additions (issue #19). They are now added, additively, keeping the ordinary
end-of-period functions as the top-level default.

### Annuity-due lives in an `annuity::due` submodule

Payments at the *start* of each period are a `(1 + r)` scaling of the ordinary
factors: `PV_due = PV · (1 + r)`, `FV_due = FV · (1 + r)`. The three functions
(`present_value`, `future_value`, `payment`) mirror the ordinary ones exactly —
same signatures, same `r → 0` limit (where `(1 + r) → 1`, so due and ordinary
coincide at `n`), same degenerate `payment`-over-`n = 0` (`NonFiniteResult`) — so
a **submodule with identical names** was chosen over a `_due` suffix or a
payment-timing parameter. The ordinary functions stay at the module top level as
the default; `annuity::due::…` names the variant explicitly.

### Perpetuity is present-value-only and rejects divergence

`perpetuity` (`PV = PMT / r`) and `growing_perpetuity` (`PV = PMT / (r − g)`,
`PMT` the first payment) return only a present value — a perpetual stream has no
finite future value. They are ordinary (end-of-period); a due perpetuity is again
a `(1 + r)` scaling and can be added later if wanted. `growth` is a `Rate<P>` of
the **same periodicity** as `rate`, so a periodicity mismatch is a compile error;
`perpetuity` is the `g = 0` special case and delegates to `growing_perpetuity`.

The sum converges only when `r > g` (for a level perpetuity, `r > 0`). When
`r ≤ g` the closed form is either an infinity (`r = g`) or a finite-but-meaningless
value (`r < g`) for a divergent series, so both constructors **reject** it with a
new, dedicated `TvmError::DivergentPerpetuity` rather than reusing
`NonFiniteResult`: the failure is a modelling condition on the rates, not an
`f64` overflow, and for `r < g` the raw quotient is finite, so a non-finite-result
error would misdescribe it. This keeps the "fallibility is reserved for genuinely
degenerate operations" rule while naming *why* it is degenerate.

### Alternatives considered (amendment)

- **`_due` suffix or a timing enum** instead of a submodule — the submodule keeps
  the mirrored functions name-for-name and reads as `annuity::due::present_value`;
  a timing parameter would thread a mostly-constant argument through every call.
- **Reusing `NonFiniteResult` for `r ≤ g`** — its doc already covers
  "mathematically undefined", but for `r < g` the quotient is finite, so a
  dedicated variant is more honest and lets callers distinguish the case.
- **`growth` as a bare `f64`** — loses the periodicity match that the rest of the
  crate enforces at the type level; a `Rate<P>` keeps the guarantee.
