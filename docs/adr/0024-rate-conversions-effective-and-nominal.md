# ADR-0024: Rate conversions — effective between periodicities, nominal as a quote

- **Status:** Accepted
- **Date:** 2026-07-10
- **Deciders:** Project owner

## Context

`Rate<P>` is a per-period rate tagged with its periodicity. The crate doc
promised "rate conversions to follow"
([issue #17](https://github.com/ojhermann-org/time-value/issues/17)): a caller
holding a `Rate<Monthly>` wants the equivalent `Rate<Annual>`, and a caller with
an APR quote wants a `Rate<Monthly>` to compute with.

There are two distinct notions of "annual rate", and conflating them is a classic
finance error:

- **Effective**: the rate that actually compounds to the same amount over a
  horizon. 1% per month is an *effective* annual `(1.01)^12 − 1 ≈ 12.68%`.
- **Nominal / APR**: a quoting convention — a headline number paired with a
  compounding frequency. "12% APR compounded monthly" means 1% per month; the 12%
  is `0.01 × 12`, a label, not a rate anything compounds at.

The crucial constraint is what the type system already guarantees. Every
existing operation (`net_present_value`, `single_sum`, `annuity`) treats a
`Rate<P>` as an **effective** per-period rate and compounds it directly. So a
`Rate<Annual>` *must* be an effective annual rate — anything else silently
computes wrong answers, which is exactly the class of bug this crate's types
exist to prevent. A nominal 12% is **not** a valid `Rate<Annual>`: constructing
`Rate::<Annual>::new(0.12)` from an APR and discounting with it would understate
the true cost.

The conversion also needs `(1 + r)^(m/k)`, i.e. `powf` — a transcendental that is
`std`/`libm`-only per [ADR-0009](0009-no_std-and-optional-libm.md).

## Decision

Model the *effective* conversion as the periodicity change, and *nominal* as a
quote-in / quote-out pair that never crosses into the type tag.

**Effective — behind `std` / `libm`** (needs `powf`):

```rust
impl<P: Periodicity> Rate<P> {
    pub fn convert<Q: Periodicity>(self) -> Result<Rate<Q>, TvmError>;  // (1+r)^(m/k) − 1
    pub fn effective_annual(self) -> Result<Rate<Annual>, TvmError>;    // = convert::<Annual>()
}
```

`convert` changes the periodicity **tag** and is the only operation that returns
a differently-tagged `Rate`. `m` and `k` are the periods-per-year of `P` and `Q`.
Round-tripping recovers the original rate up to floating-point rounding.

**Nominal — plain arithmetic, no feature** (scale by periods-per-year):

```rust
impl<P: Periodicity> Rate<P> {
    pub fn from_nominal_annual(nominal: f64) -> Result<Self, TvmError>; // nominal / m
    pub fn nominal_annual(self) -> Result<f64, TvmError>;              // per_period × m
}
```

`nominal_annual` returns a **plain `f64`, deliberately not a `Rate<Annual>`.** A
nominal quote is not an effective per-period rate, so tagging it `Annual` would
manufacture exactly the invalid rate described above. Returning a bare number
keeps the quote a quote. `from_nominal_annual` is the inverse: it divides the APR
by the compounding frequency to recover the genuine per-period rate, then
validates it through the normal `Rate` domain.

### Fallibility follows ADR-0021

Each operation is fallible iff its result can be non-finite
([ADR-0021](0021-fallible-operations-on-non-finite-results.md)):

- `convert` compounds, so it can overflow (a large rate to a coarser
  periodicity) → `Result`.
- `nominal_annual` multiplies by up to 365, which can overflow for an absurd
  rate → `Result<f64, TvmError>`. It returns `f64`, not `Money`/`Rate`, so it
  checks finiteness inline and yields `TvmError::NonFiniteResult`.
- `from_nominal_annual` only divides (never grows the magnitude), so it cannot
  overflow; it can still land `<= -1.0`, which its `Rate::new` call reports as
  `RateOutOfRange` — caller input out of domain, not an operation overflow.

### A `Rate::from_operation`, mirroring `Money::from_operation`

`convert`'s result is routed through a new internal
`Rate::from_operation(f64) -> Result<Self, TvmError>`, the analogue of
`Money::from_operation`. It maps a **non-finite** result (overflow) to
`NonFiniteResult`, and a **finite but out-of-domain** result to `RateOutOfRange`.
The second branch is not hypothetical: compounding a near-total-loss rate
(`1 + r` a hair above zero) to a coarser periodicity can underflow `1 + r` to
exactly `0.0`, making the converted rate exactly `−1.0` — finite, but below the
rate domain. The two variants keep "overflowed" and "meaningless" legible, the
same distinction ADR-0021 drew for `Money`.

## Consequences

- The type tag stays honest: a `Rate<Annual>` is always an effective annual rate,
  whatever route produced it, so every downstream operation keeps computing the
  right answer. The one operation that changes the tag, `convert`, preserves
  economic value by construction.
- The nominal/effective distinction is legible in the API surface itself:
  `effective_annual` returns a `Rate<Annual>`; `nominal_annual` returns an `f64`.
  A reader cannot accidentally treat an APR as a compounding rate.
- `convert` / `effective_annual` are feature-gated; the nominal pair is available
  in the default `no_std`, zero-dependency build (it is only arithmetic).
- The rule "provably finite ⇒ infallible" continues to hold across the growing
  surface: `from_nominal_annual` is the only new op that cannot overflow, and it
  is the only one whose failure is purely a domain check.
- CLI/MCP exposure is **out of scope**, deferred to the deliberate surface review
  in [issue #30](https://github.com/ojhermann-org/time-value/issues/30).

## Alternatives considered

- **A `to_nominal::<Q>()` returning a tagged `Rate<Q>`** — the tempting symmetry,
  and the trap: it fabricates a `Rate<Annual>` holding a nominal number that
  every operation would then miscompound. Rejected outright; it is the exact bug
  the crate exists to catch.
- **Offer effective and nominal as two conversion *modes* of one method**
  (issue #17's "effective as default with a nominal alternative") — implies both
  yield the same kind of result. They do not: one yields a rate, the other yields
  a quote. Splitting them by return type is clearer than a mode flag.
- **Make `nominal_annual` infallible (return `f64` directly)** — realistically it
  never overflows, but carving out an exception muddies ADR-0021's single rule;
  the `?` cost is trivial.
- **Take/return a dedicated `NominalRate` newtype** — heavier ceremony than the
  quote warrants; an APR is fundamentally just a labelled number, and a plain
  `f64` in and out keeps the common path a one-liner. A newtype remains a
  non-breaking future addition if a consumer needs one.
