# ADR-0032: Ergonomic convenience impls (`ZERO` / `Default` / `TryFrom` / `From`)

- **Status:** Accepted
- **Date:** 2026-07-13
- **Deciders:** Project owner

## Context

Issue #26 collected a batch of small, additive, non-breaking impls for the three
validated newtypes (`Money`, `Period`, `Rate<P>`): a `ZERO` constant, `Default`,
`TryFrom<f64>`, `From<Money> for f64`, and a `Money::is_finite()`. `Money::ZERO`
and `Period::ZERO` already existed; the rest did not. The batch was written under
[ADR-0019](0019-1.0-public-api-decisions.md)'s infallible-operations model, which
[ADR-0021](0021-fallible-operations-on-non-finite-results.md) has since
superseded — relevant to `is_finite()` below.

## Decision

Add, across `Money`, `Period`, and `Rate<P>`:

- **`ZERO`** — add `Rate::<P>::ZERO` (`= from_valid(0.0)`), completing the trio so
  every newtype has a `const` zero.
- **`Default`** — `default()` returns `ZERO` for all three. Zero is the additive
  identity for `Money`, "no time" for `Period`, and "no growth" for `Rate`; each
  is the natural neutral element.
- **`TryFrom<f64>`** — for all three, delegating to `new`, so a call site can use
  `f64::try_into()` / `TryInto` and get the constructor's exact validation and
  error (`NonFiniteAmount` / `NegativePeriods` / `RateOutOfRange`).

Add **only** `From<Money> for f64` (= `value()`):

- `Rate<P>` deliberately gets **no** `From<_> for f64` — the infallible extraction
  would silently drop its periodicity tag, which is the safety the type exists
  for; rates keep `value()` explicit.
- `Period` could take one harmlessly, but is left out to match #26's stated scope;
  it is a trivial additive follow-up if wanted.

**Drop `Money::is_finite()`** from the batch. Its purpose in #26 was ADR-0019's
"check `value().is_finite()` after an operation" story. ADR-0021 made operations
fallible, so a `Money` is now finite *by construction* — the method would be an
always-`true` accessor that misleadingly implies it could be `false`. Finiteness
is the type's invariant, not a runtime property to query.

## Consequences

- The common path gains ergonomic sugar (`Default`, `TryInto`, `Money`→`f64`)
  without touching the validated construction path — each new impl routes through
  the existing `new` / `value` / `ZERO`.
- All additions are non-breaking and additive, consistent with the trait-impl
  surface already present (`Neg`, `Debug`, `Display`).
- The `From`/`is_finite` omissions are recorded here so a later reader does not
  read the asymmetry as an oversight.

## Alternatives considered

- **`From<Rate<P>> for f64` and `From<Period> for f64` too, for symmetry** —
  rejected for `Rate` (drops the periodicity tag silently); deferred for `Period`
  (out of #26's scope, trivially addable).
- **Keep `Money::is_finite()` returning `true`** — a misleading accessor for a
  type whose invariant is finiteness; dropped instead.
- **`From<f64>` (infallible) instead of `TryFrom<f64>`** — impossible without
  discarding validation; the newtypes are fallibly constructed by design.
