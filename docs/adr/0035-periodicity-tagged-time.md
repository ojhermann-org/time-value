# ADR-0035: Periodicity-tagged time (`Period<P>`)

- **Status:** Accepted
- **Date:** 2026-07-14
- **Deciders:** Project owner
- **Amends:** [ADR-0013](0013-core-api-values-and-discrete-operations.md),
  [ADR-0014](0014-transcendental-single-sum-operations.md),
  [ADR-0015](0015-annuities.md), [ADR-0025](0025-solve-for-periods-and-rate.md),
  [ADR-0027](0027-amortization-schedule.md) (all gain a periodicity on their time
  argument)
- **Follows:** [ADR-0033](0033-core-domain-model-two-axes-and-an-f64-engine.md)

## Context

The crate's headline safety property is that applying a rate of one periodicity to a
time base of another is a compile error. Reviewing the model as a whole (ADR-0033)
showed that this guarantee is applied **unevenly**, because *time* is modelled three
different ways:

- implicit integer indices in `Cashflows<P>` (`flows[t]` at period `t`),
- a **bare, untagged `Period` count** (`Period(f64)`) in the single-sum and annuity
  operations, and
- `f64` year-offsets in `DatedCashflows` (irregular calendar dates).

The middle one leaks the guarantee. `Period` carries no periodicity, so

```rust
single_sum::future_value(Rate::<Monthly>::new(0.01)?, Period::new(5.0)?, pv)?
//                                                     ^ meant "5 years"
```

silently computes **five months**. For cashflow *series* the mismatch is caught
(`Cashflows<P>` must meet `Rate<P>`); for the `Period`-based operations it is not.
The crate's core promise is only half-kept.

## Decision

**Tag time with periodicity: `Period<P: Periodicity>`.** A period count is now "how
many periods *of periodicity `P`*", so it shares the one compile-time axis with
`Rate<P>` and `Cashflows<P>`.

- Construction names the clock: `Period::<Monthly>::new(12.0)`.
- Every operation that pairs a rate with a duration requires the **same** `P` on
  both: `single_sum::future_value(rate: Rate<P>, periods: Period<P>, …)`,
  `annuity::*`, `Schedule::for_term`, and so on. A `Period<Annual>` with a
  `Rate<Monthly>` is a compile error, closing the gap uniformly.
- Solve-for-periods returns `Period<P>`, carrying the periodicity of the rate it was
  solved against.

**Calendar time stays separate and untagged by periodicity.** `DatedCashflows` keeps
`f64` year-offsets: a real, irregular date is a *different quantity* from a count of
regular periods (it is the XNPV/XIRR domain, discounting by year-fractions), and it
has no single periodicity to tag. This is a deliberate, documented exception, not an
inconsistency — periodic time and calendar time are genuinely different things.

## Consequences

- The single-sum / annuity / amortization signatures gain `P` on their time argument.
  Since they already take a `Rate<P>`, the periodicity is usually **inferred** from
  the accompanying rate, so most call sites read the same or need only name `P` where
  they already named it for the rate.
- The "monthly rate, annual duration" class of bug becomes a compile error
  everywhere, making the crate's central guarantee uniform across the whole
  operation surface rather than just the series operations.
- `Period::ZERO` and the other conveniences (ADR-0032) carry through as
  `Period::<P>::ZERO`, defaulting the periodicity where inference allows.
- Continuous compounding does **not** use `Period<P>` — its time is a continuous
  duration in years, handled by [ADR-0036](0036-continuous-compounding-force-of-interest.md).

## Alternatives considered

- **Leave `Period` untagged** — keeps a slightly lighter time type, but permanently
  half-keeps the crate's defining guarantee; the periodicity/duration mismatch is
  exactly the bug the type system is here to prevent.
- **One unified `Time` type spanning periodic counts and calendar dates** — conflates
  two genuinely different quantities (a dimensionless count of a fixed periodicity vs.
  an irregular real-world offset in years) and would force calendar dates into a
  periodicity they do not have.
- **Tag the duration on the operation rather than the value** (e.g. an untagged
  count plus a `PhantomData<P>` argument) — more ceremony at every call site than
  putting the tag on the value where it belongs.
