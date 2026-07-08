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
