# ADR-0023: The `Money` arithmetic surface — `Neg` as an operator, `try_*` methods for the rest

- **Status:** Accepted
- **Date:** 2026-07-10
- **Deciders:** Project owner
- **Amends:** [ADR-0021](0021-fallible-operations-on-non-finite-results.md) (names the fallible scalar operations it anticipated)

## Context

`Money` is a validated newtype with no arithmetic of its own: a caller who wants
to flip a cashflow's sign, total two amounts, or turn a monthly payment into an
annual one has to unwrap to `f64`, compute, and re-validate with `Money::new`.
That is at odds with the crate's "type-heavy **and** friendly" principle
(`CLAUDE.md`), and it pushes callers through the very `f64` escape hatch the
newtype exists to close. [Issue #18](https://github.com/ojhermann-org/time-value/issues/18)
tracks closing the gap.

Two constraints bound the design.

1. **[ADR-0021](0021-fallible-operations-on-non-finite-results.md)'s rule:** an
   operation returns `Result<_, TvmError>` **iff** its result could be
   non-finite. That ADR already classified the arithmetic in passing — negation
   of a finite amount is finite, so it stays infallible, while a scalar
   `Money * f64` "*can* overflow, so per the rule it is a fallible method, not a
   `std::ops::Mul` impl". It did not name those methods.
2. **`std::ops` operators cannot fail.** `Add::add` returns `Self`, not
   `Result<Self, _>`. An `impl Add for Money` would therefore have to either
   panic on overflow or return a non-finite `Money` — and a silently non-finite
   `Money` is exactly the foot-gun ADR-0021 was written to remove. Issue #18's
   own text notes this for addition, though it assumed scalar multiplication was
   safe as an operator; it is not, and ADR-0021 is the later and governing call.

That leaves only the name of the fallible methods. Rust's standard library has
an established split: `checked_add` returns `Option`, and `try_from` / `try_into`
return `Result`. Issue #18 was filed with `checked_add` / `checked_sub` wording,
predating that observation.

## Decision

`Money` gains **one operator and four methods**:

```rust
impl Neg for Money { type Output = Money; }        // infallible

impl Money {
    pub fn try_add(self, rhs: Money)   -> Result<Money, TvmError>;
    pub fn try_sub(self, rhs: Money)   -> Result<Money, TvmError>;
    pub fn try_mul(self, factor: f64)  -> Result<Money, TvmError>;
    pub fn try_div(self, divisor: f64) -> Result<Money, TvmError>;
}
```

- **`Neg` is an operator.** The negation of a finite `f64` is finite, so by
  ADR-0021's rule it cannot fail, and `-money` is the natural spelling for
  flipping a cashflow's sign.
- **Addition, subtraction, and scaling are `try_*` methods.** Each can leave
  `f64` range, so each is fallible; each therefore cannot be an operator.
- **They are named `try_*`, not `checked_*`,** because they return `Result` and
  a Rust reader takes `checked_` to mean `Option`. This supersedes the wording
  in issue #18. It also matches the crate's existing fallible surface, which
  reads `?`-threaded throughout.
- **Every one routes through the existing `Money::from_operation`,** so a
  non-finite result surfaces as `TvmError::NonFiniteResult` through the same
  single code path as the TVM operations. No new error variant.

`try_mul` and `try_div` take a bare `f64` scalar rather than a `Money`: money
times money is not money (it has no monetary meaning), whereas money times a
dimensionless factor is. A non-finite `factor` needs no separate guard — it
propagates into a non-finite product, which `from_operation` already rejects.
Note that `try_div` by an *infinite* divisor yields zero and is therefore `Ok`;
only a zero or `NaN` divisor (or a genuine overflow) errors.

## Consequences

- The common ergonomic gestures — `-flow`, `a.try_add(b)?`, `payment.try_mul(12.0)?`
  — no longer require a round-trip through `f64` and `Money::new`, and the
  `NonFiniteAmount` / `NonFiniteResult` distinction stays meaningful: unwrapping
  to `f64` and back would have mislabelled an overflow as bad caller input.
- The surface is asymmetric on purpose: one operator, four methods. The
  asymmetry *is* the contract — if it takes an operator, it cannot fail.
- Additive and non-breaking. `Money`'s internals are untouched, and the internal
  Horner loops in `cashflows.rs` keep computing on raw `f64` (they validate once,
  at the end) rather than paying a `Result` per term.
- Any future `Money` arithmetic follows the same test: provably finite ⇒
  operator; otherwise ⇒ `try_*` method.

## Alternatives considered

- **`impl Add`/`Sub`/`Mul` returning a possibly non-finite `Money`** — restores
  the silent-`inf` foot-gun ADR-0021 removed, and would let a non-finite `Money`
  exist at all, breaking the type's central invariant.
- **`impl Add` that panics on overflow** — a panic in a `no_std` numeric library
  is a worse failure mode than an `Err` the caller can see in the signature, and
  it is invisible at the call site.
- **`checked_add` / `checked_sub` / `checked_mul`** (issue #18's wording) —
  familiar, but `checked_` means `Option`-returning to a Rust reader; reusing it
  for `Result` costs more in surprise than the familiarity buys.
- **`Add<Output = Result<Money, TvmError>>`** — legal (the output type is free),
  but `(a + b)?` reads as though `+` were infallible, and it composes badly:
  `a + b + c` does not typecheck.
- **Scalar `Money * Money`** — has no monetary meaning; the scalar is
  deliberately an `f64`.
- **`Sum` / `Default` / `TryFrom<f64>`** — deferred to
  [issue #26](https://github.com/ojhermann-org/time-value/issues/26); `Sum`
  cannot be fallible, so a total over an iterator needs its own bespoke method
  and its own decision.
