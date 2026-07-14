# ADR-0033: Core domain model — two axes, and an `f64` computation engine

- **Status:** Accepted
- **Date:** 2026-07-14
- **Deciders:** Project owner
- **Supersedes:** [ADR-0005](0005-domain-modelling-and-strong-typing.md) §"currency
  is not modelled" and its implicit "periodicity is the only tag" framing (restated
  and extended here)
- **Governs:** [ADR-0034](0034-money-and-currency.md) (money/currency),
  [ADR-0035](0035-periodicity-tagged-time.md) (time),
  [ADR-0036](0036-continuous-compounding-force-of-interest.md) (continuous rates)

## Context

The `1.0` core grew one type at a time — validated newtypes and periodicity tags
(ADR-0005), the discrete operations (ADR-0013), transcendental single-sum and
annuity operations (ADR-0014/0015), and so on. Before hardening for the first
release, we reviewed the *fundamental* model as a whole rather than type by type,
because — as the owner put it — nothing else the crate does matters if money and
its neighbouring quantities are not modelled soundly.

Two cross-cutting questions had never been decided head-on: **what numeric domain**
the engine computes in, and **how currency and time should be represented** so the
model is coherent across every operation. A false start — a feature-gated currency
*type* tag (issue #25) — was implemented and abandoned when it became clear that
forcing currency into the type system fights the domain (see Alternatives, and
ADR-0034). That failure is what clarified the principle below.

## Decision

### 1. The crate is a TVM *computation engine*, and it computes in `f64`

Time-value-of-money is built on *transcendental* operations — `(1 + r)ⁿ`, `ln`,
and root-finding for IRR. Their results are irrational and **cannot be represented
exactly** in decimal or fixed-point: the moment an IRR or a compounded value is
computed, the answer is an approximate real number regardless of the storage type.
An exact-decimal representation would therefore promise a precision the mathematics
does not have.

So the magnitude of every monetary quantity is `f64`, and the crate stays `no_std`
and zero-dependency. What we owe in exchange, and commit to:

- a **stated precision contract** — results are approximate reals (~15–16
  significant digits), not penny-exact ledger entries; and
- **currency-aware rounding at the boundary** — round to a currency's minor unit
  only for *presentation*, never during computation.

This crate is explicitly **not** an exact-decimal accounting/ledger system; that is
a different tool, and its TVM results would be approximate anyway.

### 2. Two axes, two mechanisms

The two properties that qualify a monetary quantity are modelled by *different*
mechanisms, chosen by whether the property is static or dynamic:

- **Periodicity is a static property of a model → a compile-time type tag.** It is
  known when the model is written (a schedule is monthly), so it is encoded in the
  type and checked by the compiler. Periodicity is the crate's **sole** compile-time
  tag, and it is applied **uniformly**: `Rate<P>`, `Period<P>` (ADR-0035), and
  `Cashflows<P>`. Applying a rate of one periodicity to a time base of another is a
  *compile error* everywhere, not just for cashflow series.
- **Currency is dynamic data → a runtime value.** It arrives at runtime (user input,
  a config, an exchange feed), an amount's currency can be chosen from data, and
  collections mix currencies. A phantom type cannot represent a currency picked at
  runtime, so currency is a runtime value carried inside `Money`, and a mismatch is a
  runtime error (ADR-0034).

This split is the organizing principle of the core. It also *simplifies* the
implementation: because currency is a value rather than a type parameter, the
containers and operations (`Cashflows`, `Schedule`, the single-sum/annuity
functions) stay non-generic in currency — they hold `Money` and check
one-currency-per-computation at their boundaries.

### 3. A deliberately light algebra

Reaffirming ADR-0005, the crate does **not** adopt full dimensional-analysis types.
`Money` is the only "dimensioned" quantity; a `Rate` is a dimensionless growth
ratio. The whole calculus is: `Money ± Money → Money` (same currency),
`Money × Rate → Money`, `Money ÷ Money → Rate`. That is expressed through the method
surface, not encoded as a type-level algebra.

## Consequences

- The model is now stated as a whole, and the sibling ADRs (0034–0036) each decide
  one quantity against this frame.
- This is a **breaking re-shape of the pre-release core**: `Money` gains a currency
  (ADR-0034) and `Period` gains a periodicity tag (ADR-0035), so construction sites,
  operation signatures, tests, doctests, and the CLI/MCP change. This is accepted as
  the cost of getting the foundation right before `1.0`, and it is done now precisely
  because no release yet constrains it.
- The abandoned currency *type-tag* work (issue #25) is closed in favour of the
  runtime model; the FX follow-up (issue #60) folds into ADR-0034 as core.
- Every future quantity is placed by asking which axis it belongs to: static → a
  periodicity-tagged type; dynamic → a runtime value on `Money`.

## Alternatives considered

- **Exact decimal / fixed-point magnitude** — correct for an accounting ledger,
  wrong for a TVM engine: the transcendental core makes results approximate anyway,
  so decimal would advertise an exactness the answers do not have, at the cost of
  `no_std`/zero-dep and speed.
- **Currency as a compile-time type tag** (the abandoned #25) — elegant in the
  small, but it cannot represent runtime-chosen or user-supplied currencies, breaks
  the CLI/MCP (which take a currency string at runtime), complicates FX,
  serialization, and mixed-currency collections, and its defaulted type parameter
  poisons constructor inference. Rejected in favour of a runtime value (ADR-0034).
- **Full dimensional-analysis types** — ceremony without catching the real
  (semantic, periodicity/currency) errors; rejected already in ADR-0005 and again
  here.
- **A numeric-type-generic `Money<N>`** — multiplies the ergonomic cost and buys
  nothing, since the transcendental operations require `f64` internally regardless.
