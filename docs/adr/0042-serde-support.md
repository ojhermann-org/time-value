# ADR-0042: `serde` support — an optional, validating wire format

- **Status:** Accepted
- **Date:** 2026-07-14
- **Deciders:** Project owner
- **Follows:** [ADR-0005](0005-domain-modelling-and-strong-typing.md) (strong
  typing / feature-gated serde), [ADR-0033](0033-core-domain-model-two-axes-and-an-f64-engine.md)
  / [ADR-0034](0034-money-and-currency.md) (the value types), [ADR-0037](0037-currency-in-the-binaries.md)
  (deferred the `Money`/`Currency` wire shape to this issue)
- **Amends:** [ADR-0019](0019-1.0-public-api-decisions.md) (which dropped the dead
  `serde` feature; this re-introduces a live one)

## Context

ADR-0005 planned feature-gated `serde` derives; ADR-0019 **dropped** them for 1.0
because the code wired nothing — the feature pulled the dependency and derived
nothing, dead weight. ADR-0019 recorded that a real re-introduction should give
the newtypes "a deliberate `#[serde(transparent)]` representation so [they]
serialise as [bare numbers]". Since then the domain model was rebuilt
(ADR-0033/0034): `Money` is no longer a bare newtype but `{ magnitude, currency }`,
and `Currency` is a runtime enum — so ADR-0037 explicitly **deferred** the
`Money`/`Currency` serialized shape to "issue #21 … on its own terms". This ADR
is that decision (issue #21).

The forces: keep the default core **dependency-free and `no_std`** (ADR-0009); make
the newtypes serialize as **bare numbers**, not structs carrying `PhantomData`
(ADR-0019's intent); and — the crux — the value types have **validating
constructors** (`Rate` finite and `> −1`, `Period` finite and `≥ 0`, `Money`
finite, `Currency` a known code, `FxRate` finite and `> 0`), which a naive
`#[serde(transparent)]` / plain `derive(Deserialize)` would **bypass**, letting the
wire construct invalid values.

## Decision

**Add an optional, off-by-default `serde` feature** deriving `Serialize` /
`Deserialize` for the public **owned value types**, with a format that validates
on the way in.

**Wire format:**

- **Bare numbers** — `Rate<P>`, `Period<P>`, `ContinuousRate` serialize as a plain
  `f64`; the periodicity tag is not on the wire.
- **`Money` → `{ "amount": <f64>, "currency": <code> }`**, the currency **always
  present** (`"XXX"` for the agnostic amount), so it round-trips losslessly. (This
  is deliberately *not* the binaries' presentation shape, which omits `currency`
  for `XXX` — that is display, this is data.)
- **`Currency` → its ISO 4217 code string** (`"USD"`, `"XXX"`), via `code()` /
  `from_code`.
- **`FxRate` → `{ from, to, rate }`**, **`DatedCashflow` → `{ offset_years, amount }`**,
  **`Installment` → `{ period, payment, interest, principal, balance }`**.

**Deserialization validates.** Every value is rebuilt through its fallible
constructor (`Rate::try_from`, `Money::new`, `Currency::from_code`, `FxRate::new`,
`DatedCashflow::new`), so an out-of-domain number or unknown code is a
deserialization error, never a silently-constructed invalid value. This is why the
newtypes get hand-written impls rather than `#[serde(transparent)]`.

**Scope — owned value types only.** The borrowing / lazy types (`Cashflows<'a, P>`,
`DatedCashflows<'a>`, `Schedule<P>`) are **excluded**: a borrowing type cannot
`Deserialize` (it does not own its data) and a lazy iterator has no natural owned
wire form — a consumer serializes the collected `Vec<Installment>` / `Vec<Money>`.
`TvmError` is **excluded**: an error is presented, not round-tripped, and deriving
serde on it would commit its variant shape as wire API for no consumer.

**`no_std` preserved.** The workspace `serde` dep is `default-features = false`
(+ `derive`); the impls avoid `alloc` (e.g. `Currency` deserializes from a
borrowed `&str`). The feature composes with the default `no_std` build; a
dedicated CI check (`--no-default-features --features serde`) guards that.

**Implementation shape.** The whole contract lives in one module
(`src/serde_impls.rs`), built only on the types' **public** API, so it needs no
private-field access and the format is reviewable in one place. Impls for the
`std`/`libm`-gated types (`Period`, `ContinuousRate`, `DatedCashflow`) carry the
same gate.

**Not surfaced in the binaries here.** Dropping the MCP `CurrencyCode` String
workaround additionally needs `schemars`/`JsonSchema` on `Currency` — a separate
front-run ADR-0037 flagged. This ADR is the core feature only; binary adoption is
an additive follow-up.

## Consequences

- Downstream consumers can serialize the core value types with a stable, validated
  format, without the core taking a non-optional dependency or leaving `no_std`.
- The invariant the type system enforces at construction is preserved across a
  serialization boundary — deserializing dirty data fails loudly.
- `Money`'s wire shape is now fixed (ADR-0037's deferral resolved); the binaries'
  presentation DTOs remain independent of it.
- A new feature configuration to keep green: `no_std + serde` (CI clippy) and the
  `serde` round-trip/validation tests (run under `--all-features`).
- Adopting serde in the binaries (retiring the `CurrencyCode` workaround) is left
  as a follow-up, gated on a separate `schemars`-on-`Currency` decision.

## Alternatives considered

- **`#[serde(transparent)]` / plain derive on the newtypes** — the obvious path,
  but it **skips validation** on deserialize, so the wire could mint a `Rate` of
  `−5` or a `NaN` `Period`. Rejected; hand-written impls route through
  `TryFrom<f64>`.
- **`Money` as a bare `f64`** (via its `f64` conversions) — silently drops the
  currency, which is now part of `Money`'s identity (ADR-0034). Rejected.
- **`Currency` by enum-variant name** (`"Usd"`) — serde's default, but the ISO
  code (`"USD"`) is the domain-standard, matches `from_code`, and aligns with the
  binaries' string convention. Rejected the variant name.
- **Omit `currency` for `XXX`** (mirroring the binary DTOs) — conflates
  presentation with data and makes the format lossy/asymmetric. Rejected; the core
  wire format always carries the currency.
- **Include `TvmError` / the borrowing types** — no consumer for the error's wire
  shape, and the borrowing/lazy types cannot round-trip. Rejected as scope.
- **Serialize-only on the aggregates** (`Cashflows`/`Schedule`) — asymmetric
  (write but not read); a consumer collects to a `Vec` and serializes that.
  Rejected.
