# ADR-0014: Transcendental operations behind `std` / `libm` — single-sum value

- **Status:** Accepted (single-sum functions moved from the crate root into a `single_sum` module for 1.0 — [ADR-0019](0019-1.0-public-api-decisions.md))
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

[ADR-0013](0013-core-api-values-and-discrete-operations.md) shipped the discrete
operations (NPV, NFV, IRR) that need only arithmetic, and deferred everything
that needs `(1+r)ⁿ` for a **fractional** `n`, or `ln`/`exp`, to a feature-gated
follow-up. [ADR-0009](0009-no_std-and-optional-libm.md) already decided *where*
that math comes from — `std` when available, otherwise `libm` — but not how it is
wired, nor the first operations to use it. This ADR settles both, for the
single-sum present/future value.

## Decision

### A tiny internal `math` abstraction

A crate-private `math` module exposes `powf(base, exponent)`, compiled only when
`std` **or** `libm` is enabled, and dispatching to `f64::powf` under `std` or
`libm::pow` otherwise (`std` preferred when both are on). Callers of transcendental
math go through this one function rather than reaching for `std`/`libm` directly,
so the feature dispatch lives in exactly one place.

### `Period`: a fractional, validated period count

`Period` wraps `f64`, validated finite and non-negative (fallible constructor,
[`TvmError::NegativePeriods`]). It is **not** periodicity-tagged: the [`Rate`]
supplies the clock, so `n` is just "how many periods of that rate". `Period` is
gated to the same features as the operations that consume it — it would be an
inert type in the arithmetic-only default build.

### Single-sum present/future value

Two free functions, gated behind `std`/`libm`:

- `present_value(rate, periods, future) = future / (1 + r)ⁿ`
- `future_value(rate, periods, present) = present · (1 + r)ⁿ`

Both accept a fractional `Period` (hence `powf`). They return `Money`
infallibly: with `r > -1` and `n ≥ 0`, `(1+r)ⁿ` is positive and finite, so the
result is finite for realistic inputs (extreme overflow is documented, not
guarded).

### `no_std` hygiene extends into gated code

`f64::abs` and `f64::mul_add` are **not** in `core`, and under `libm`-only (no
`std`) they are still unavailable — so the gated code and its tests avoid them
too (approximate comparisons are written as `d < tol && d > -tol`). CI exercises
the gated paths by running tests and doctests with `--all-features`.

## Consequences

- The first transcendental operations exist, `no_std`-compatible via `libm`.
- All transcendental dispatch is funnelled through `math::powf`; adding `ln`/`exp`
  later (for rate conversions / continuous compounding) extends that one module.
- `Period` and the single-sum functions are absent from the default build — a
  `cargo doc` without features will not show them (acceptable; docs.rs builds
  with all features).

## Alternatives considered

- **Reach for `std`/`libm` at each call site** — scatters `#[cfg]` dispatch; the
  single `math::powf` seam is cheaper to maintain and to extend.
- **Integer-only single-sum in the arithmetic core** — would let PV/FV ship
  without a feature, but only for whole `n`; fractional periods are common enough
  that a split "integer core / fractional gated" API is worse than one gated API
  that handles both.
- **Make the functions fallible** — unnecessary for PV/FV (always finite for
  valid inputs); fallibility is reserved for genuinely degenerate operations
  (e.g. amortising an annuity over zero periods, in the annuities follow-up).

[`TvmError::NegativePeriods`]: ../../crates/time_value/src/lib.rs
[`Rate`]: ../../crates/time_value/src/rate.rs
