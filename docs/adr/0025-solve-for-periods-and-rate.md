# ADR-0025: Solve for periods (NPER) and rate (RATE)

- **Status:** Accepted
- **Date:** 2026-07-10
- **Deciders:** Project owner

## Context

The library solved the time-value relationships for *values* — present value,
future value, and the level payment — but not for the remaining two of the five
classic TVM variables: the number of periods `n` (NPER) and the per-period rate
`r` (RATE). A TVM library that cannot answer "how long?" or "at what rate?" has a
visible gap, so #31 completes the set for both the single sum
([ADR-0014](0014-transcendental-single-sum-operations.md)) and the annuity
([ADR-0015](0015-annuities.md)).

The forces: some of these solves have closed forms (needing `ln`, so `std`/`libm`
like the rest of the transcendental surface); one does not and must iterate. The
crate already owns a robust root finder — the Newton-with-bracketing-bisection
behind the internal rate of return
([ADR-0020](0020-robust-irr-newton-with-bisection-fallback.md)) — and the
fallibility contract ([ADR-0021](0021-fallible-operations-on-non-finite-results.md))
says degenerate inputs must surface an error, not a misleading finite value.

## Decision

Add solve-for operations that mirror the existing value functions' shape and
feature-gating.

### Single sum (both closed form)

- `single_sum::periods(rate, present, future)` → `n = ln(FV/PV) / ln(1+r)`.
- `single_sum::rate::<P>(periods, present, future)` → `r = (FV/PV)^(1/n) − 1`.

The scalar inputs carry no periodicity, so `rate` names it by turbofish.

### Annuity, present- and future-value forms

Solving for `n` is closed form (`ln`); solving for `r` is iterative. Both a
present-value and a future-value form are provided (owner's call — the fuller
surface), the PV forms unsuffixed (matching `annuity::payment`, which is also
PV-based and the default), the FV forms suffixed `_from_future`:

- `annuity::periods(rate, payment, present)` and
  `annuity::periods_from_future(rate, payment, future)` — closed form, with the
  `r → 0` limit (`n = PV/PMT`, `n = FV/PMT`).
- `annuity::rate::<P>(periods, payment, present)` and
  `annuity::rate_from_future::<P>(periods, payment, future)` — **iterative**.

### Reuse the IRR's robust root finder, without refactoring it

The annuity `rate` is the root of `PMT · factor(r, n) − target`, where `factor`
is the present- or future-value annuity factor. Both factors are **monotone** in
`r`, so the residual has a single root and the IRR's bracketing bisection finds
it directly — Newton is unnecessary here. To reuse it, the leaf root-finding
primitives (`bisect`, the geometric bracket-and-bisect scan, and the `no_std`
sign/abs helpers) move to a small **ungated `root` module** — ungated because
IRR itself is arithmetic-only and lives in the default `no_std` build. The IRR
code is refactored to call the shared bracket-and-bisect (its existing tests
cover the change); the new, gated annuity solves call the same function.

### Degenerate cases surface honest errors (ADR-0021)

- A non-finite solved value is `NonFiniteResult`; a finite-but-negative solved
  period count is `NegativePeriods` (via a new `pub(crate) Period::from_operation`,
  the mirror of `Money`/`Rate::from_operation`).
- Zero rate in a single-sum NPER (no growth), zero periods in a single-sum RATE
  (no elapsed time), and a non-amortising annuity (payment `≤` the period's
  interest, so the balance never retires) are all undefined and yield
  `NonFiniteResult`.
- A `rate` solve with no root over the valid domain (e.g. incompatible signs)
  yields a new, dedicated `TvmError::SolveDidNotConverge`, distinct from the
  IRR-specific `IrrDidNotConverge`.

## Consequences

- The five-variable TVM set is complete for both the single sum and the annuity.
- One reusable `root` module now backs both IRR and the annuity rate solves, so
  the robust fallback has a single implementation.
- `SolveDidNotConverge` and `Period::from_operation` are added (the enum variant
  is public; `#[non_exhaustive]` keeps it additive).
- Annuity NPER's present-value form is ill-conditioned once `(1+r)⁻ⁿ` underflows
  toward zero — the present value saturates at `PMT/r` and `n` is no longer
  recoverable from it. This is a cancellation limit at the *pricing* step, not a
  solver bug; the property test bounds its range accordingly and a comment records
  why (as the rate-conversion property does for its own boundary).

## Alternatives considered

- **PV-form annuity solves only** — tighter surface, but the owner chose the
  fuller PV+FV set so a savings (FV) goal is a first-class input, not just a loan.
- **Reuse `IrrDidNotConverge` for the annuity rate** — its name foregrounds IRR
  while the caller invoked `annuity::rate`; a dedicated variant is more honest.
- **A focused annuity-rate solver duplicating `bisect`** instead of extracting the
  shared `root` module — avoids touching IRR, but duplicates the robust logic the
  crate would rather keep in one place; the extraction is covered by IRR's tests.
- **Newton for the annuity rate** — unnecessary given the residual is monotone;
  bracketing bisection is simpler and cannot wander off.
- **Framing the annuity rate as the IRR of `[−PV, PMT, …]`** — conceptually the
  same root, but building the cashflow array needs allocation, which the
  `no_std`/allocation-free core forbids; solving the residual directly does not.
