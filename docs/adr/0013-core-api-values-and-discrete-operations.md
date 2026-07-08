# ADR-0013: Core API — values, cashflows & discrete operations

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

[ADR-0005](0005-domain-modelling-and-strong-typing.md) settled the *philosophy*
(validated newtypes, periodicity as a zero-sized tag) and
[ADR-0009](0009-no_std-and-optional-libm.md) settled the *dependency posture*
(`no_std`, zero-dep default; transcendental math behind `std`/`libm`). This ADR
records the **first concrete API surface** that realises them: the value types,
the cashflow container, and the operations that ship in the default build.

The shaping constraint is ADR-0009's default build: with neither `std` nor `libm`,
`core` offers `f64` arithmetic (`+ - * /`) and bit-level predicates
(`is_finite`, `is_nan`) but **no** `powf`/`ln`/`exp` — nor even `abs`/`mul_add`.
So the question is: how much genuinely useful TVM can we express with arithmetic
alone?

## Decision

### Value types

- `Money` — a newtype over `f64`, validated finite (rejects `NaN`/±∞); signed
  (outflow negative, inflow positive). Currency is not tagged ([ADR-0005]).
- `Rate<P>` — a per-period rate tagged with a `Periodicity` marker `P`, validated
  finite and `> -1.0`.
- `Periodicity` — a **sealed** trait implemented by zero-sized markers (`Annual`,
  `SemiAnnual`, `Quarterly`, `Monthly`, `Weekly`, `Daily`), each carrying
  `PERIODS_PER_YEAR` and a `NAME`. Sealed so the set is fixed by the crate.

### Cashflows: a *borrowed*, tagged series

`Cashflows<'a, P>` wraps `&'a [Money]` (period `t` = index `t`, period 0 = now),
tagged with periodicity `P`. It **borrows** rather than owning a `Vec`, so the
core needs no allocator — honouring the `no_std`, zero-dep default. An owned,
`alloc`-backed constructor can be added later behind an `alloc` feature without
breaking this one.

### Only arithmetic in the default build

The discrete operations are available with **no features**, because each reduces
to a single pass of multiply/add over the series:

- `net_present_value(rate)` — `Σ CFₜ (1+r)⁻ᵗ`, accumulating a running discount
  factor.
- `net_future_value(rate)` — `Σ CFₜ (1+r)ⁿ⁻¹⁻ᵗ`, by Horner's method.
- `internal_rate_of_return[_from]` — Newton–Raphson; each step evaluates NPV and
  its derivative (both polynomials in the discount factor) in one pass. Fallible:
  empty series, non-convergence, or leaving the valid rate domain return
  `TvmError`.

Helpers that would pull in `std` are avoided: `|x| < tol` is written out (no
`abs`), and Horner uses `a*b + c` (no `mul_add`, which needs FMA/libm).

### Deferred to a `std`/`libm` follow-up

Operations that intrinsically need `(1+r)ⁿ` for a **fractional** `n`, or
`ln`/`exp` — single-sum present/future value over fractional periods, annuity
payment/PV/FV, and rate conversions — require `powf`/`ln`/`exp` and so land in a
later, feature-gated PR. That is also where the `Period` newtype (a
possibly-fractional period count) is introduced, since the arithmetic-only
operations index the series by position and do not need it.

## Consequences

- A plain `cargo add time_value` yields useful analysis — NPV, NFV, IRR — with
  zero dependencies and `no_std` support.
- The periodicity mismatch that motivates the crate is a compile error, proven by
  a `compile_fail` doctest.
- `Cashflows` borrowing a slice is ergonomic for the common "I already have the
  numbers" case and keeps the core allocator-free; owning is a future opt-in.
- The value types have deliberately small surfaces now; getters/among others will
  grow as the operations that need them land.

## Alternatives considered

- **Owned `Cashflows(Vec<Money>)`** — friendlier to build incrementally, but
  needs `alloc`, breaking the zero-dep default; deferred to an opt-in feature.
- **Put NPV/IRR behind `libm` too** — unnecessary; they need no transcendental
  function, and gating them would make the default build nearly empty.
- **Return `Result` from `net_present_value`** — inputs are validated finite and
  `1+r > 0`, so the result is finite; an infallible `Money` keeps the common path
  clean ([ADR-0005]'s "type-heavy *and* friendly").
