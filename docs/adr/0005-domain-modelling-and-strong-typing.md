# ADR-0005: Domain modelling & strong typing

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

The headline bug in time-value-of-money code is a **periodicity mismatch**:
applying a rate of one period (say, an annual rate) to cashflows of another (say,
monthly flows). It is silent — the arithmetic runs and returns a number that is
simply wrong. A second class of error is an **economically meaningless value**: a
rate ≤ −100%, a negative period count. These are the errors `time_value` exists
to catch. The design tension is that a maximally strict, dimensional-analysis
type system would catch them but bury the common path in ceremony.

## Decision

**Encode the domain in validated newtypes, and encode periodicity in the type.**

- `Rate`, `Money`, and `Period` are newtypes with **fallible constructors** that
  reject meaningless values at the boundary, returning `TvmError`
  ([ADR-0004](0004-error-handling.md)). Once constructed, a value is known-valid.
- **Periodicity is a zero-cost type parameter.** `Rate` and `Cashflows` are
  tagged with a periodicity marker, so applying a rate to cashflows of a
  different periodicity is a **compile error**, not a runtime surprise. The
  markers carry no data and cost nothing at runtime.
- **Ergonomics are preserved** with type aliases and inference so the common path
  stays a clean one-liner; the types work for you without being typed out in
  full.
- **`Money` is not currency-tagged in `1.0`** — it is a plain newtype. A
  feature-gated currency tag can be added later without breaking anyone; baking
  it in now could not be removed without a major bump.
- `serde` derives on these types are **feature-gated** (`serde` off by default),
  so the surfaces can (de)serialise them while the core stays dependency-free
  ([ADR-0009]).

## Consequences

- The class of bug the library targets — periodicity mismatch — cannot compile.
- Invalid values are unrepresentable past construction; downstream code need not
  re-validate.
- Marker types add some generic signatures; type aliases keep them out of
  everyday call sites.
- The currency question is deliberately deferred and remains a non-breaking
  future addition.

## Alternatives considered

- **Full dimensional analysis** (units on every quantity) — would catch these
  errors, but TVM stays entirely in "money"; the extra machinery adds ceremony
  without catching a *semantic* error the marker approach misses.
- **Plain `f64` everywhere** — the ergonomic baseline and exactly the status quo
  the redesign rejects: every mistake is silent.
- **Runtime periodicity checks** — catches the mismatch, but only when the code
  runs and only on the paths exercised; the whole point is to catch it at compile
  time.

[ADR-0009]: 0009-no_std-and-optional-libm.md
