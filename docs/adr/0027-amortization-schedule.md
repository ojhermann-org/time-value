# ADR-0027: Amortization schedule as a lazy iterator

- **Status:** Accepted
- **Date:** 2026-07-10
- **Deciders:** Project owner

## Context

The library can size a loan's level payment
([`annuity::payment`](0015-annuities.md)) but not show how each payment splits
into interest and principal, nor how the balance falls. #33 adds that per-period
breakdown. The constraint from the design principles is that the core stays
`no_std` and allocation-free, so the schedule must not build a `Vec`.

## Decision

Model the schedule as a lazy [`Iterator`], `amortization::Schedule<P>`, yielding
one `Installment { period, payment, interest, principal, balance }` per period. It
holds only scalars (opening balance, rate, level payment, period counter), so
iterating allocates nothing and a caller streams or collects as they choose.

### One iterator, two constructors

- `Schedule::with_payment(rate, payment, principal)` — takes the level payment as
  given and runs until the balance is retired. Arithmetic-only, so it lives in the
  default **`no_std`** build.
- `Schedule::for_term(rate, periods, principal)` — sizes the payment with
  `annuity::payment` (needing `powf`), so it is behind **`std` / `libm`**. It then
  delegates to `with_payment`.

Both feed the same `Iterator::next`, which each period computes
`interest = balance · rate`, `principal = payment − interest`, and reduces the
balance — until a final installment clears whatever remains. The final period is
detected when the payment covers the remaining balance plus its interest, with a
tiny relative slack (`1e-9`) so the floating-point residual of a *computed* level
payment lands the last installment exactly on period `n` rather than leaving a
vanishing balance for one more.

### Degenerate cases (ADR-0021)

- `with_payment` returns `TvmError::NonFiniteResult` if the payment does not
  exceed the first period's interest — the balance would never fall (the same
  "payment cannot amortise" condition `annuity::periods` rejects).
- A non-positive principal is simply an **empty** schedule (`next` returns `None`
  immediately) — there is nothing to repay, which is not an error.
- `for_term` propagates `annuity::payment`'s error (`NonFiniteResult` for zero
  periods).

## Consequences

- A loan's interest/principal/balance breakdown is available without allocation,
  and the `no_std` core keeps it (via `with_payment`); only the term-based
  convenience needs a feature.
- The result is a standard `Iterator`, so it composes with `take`, `skip`, `map`,
  `sum`, etc. `Schedule` derives `Clone` but **not** `Copy` — a `Copy` iterator
  silently forks on copy (`clippy::copy_iterator`); an explicit `.clone()` is
  required to fork a schedule deliberately.
- No new error variant is needed; the degenerate cases reuse existing ones.

## Alternatives considered

- **Return a `Vec<Installment>`** — simplest to consume, but allocates and needs
  `alloc`/`std`; rejected outright by the allocation-free core.
- **A fixed-count iterator using `periods as usize`** — the `usize`↔`f64` casts
  trip the pedantic lint set, and a balance-driven loop with a slack-based final
  period needs no cast and unifies both constructors.
- **Only `for_term` (gated)** — simpler, but forgoes the genuinely `no_std`
  `with_payment` path the owner wanted; both entry points share one `next`.
- **A dedicated "cannot amortise" error variant** — the existing
  `NonFiniteResult` already carries the "mathematically undefined" meaning here, as
  it does for `annuity::periods`; a new variant would not earn its keep.
