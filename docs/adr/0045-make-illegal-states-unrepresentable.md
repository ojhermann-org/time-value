# ADR-0045: Make illegal states unrepresentable; test the class, not the instance

- **Status:** Accepted
- **Date:** 2026-07-15
- **Deciders:** Project owner
- **Follows:** [ADR-0005](0005-domain-modelling-and-strong-typing.md) (domain
  modelling / strong typing), [ADR-0033](0033-core-domain-model-two-axes-and-an-f64-engine.md)
  / [ADR-0034](0034-money-and-currency.md) (the two axes — periodicity as a
  compile-time tag, currency as a runtime value), [ADR-0039](0039-typed-output-layer-for-the-binaries.md)
  (types-in-types-out at the binary edge)

## Context

The crate's founding principle is "make TVM mistakes compile errors" (ADR-0005):
periodicity is a zero-sized type tag, so applying an annual rate to monthly flows
does not compile. That instinct — encode the invariant in a type the compiler
checks — has been applied ad hoc across the build-out (sealed `Periodicity`,
`#[non_exhaustive]` closed `Currency`, validated newtypes, validating serde), but
it was never written down as a *standing design test*, and the crate's testing
discipline was never stated either.

Two sibling repos in the same estate recently formalised exactly this and
cross-reference *this* crate's ADRs while doing so:

- **`rustrolabe`** — ADR-0101 ("types are a first-class design tool") and ADR-0107
  ("property tests for the library's stated invariants"), plus a `CLAUDE.md`
  testing section whose rule is *"test the class, not the instance; pin every
  stated assumption."* Its recurring blind spot was a good single-instance test
  that was never generalised to the invariant it exemplifies.
- **`ferric-fred`** — ADR-0027 ("types in, types out"), which adopts "make illegal
  states unrepresentable" as a standing test and names the *boundary*: genuinely
  open sets stay strings; a type that adds ceremony without removing a real
  failure mode is not an improvement.

An audit of this crate against that bar found it already well-aligned — property
tests (`tests/properties.rs`), `compile_fail` doctests locking the periodicity
mismatch (which neither sibling has), validating serde, schema-conformance tests —
with two loose ends: the discipline was undocumented, and one finite-set constant
(`Currency::ALL`) was hand-maintained with no guard tying it to the enum, so a new
variant could be added to the enum (and its compiler-checked `meta` table) yet
silently dropped from `ALL`, under-enumerating every schema and CLI surface built
from it.

## Decision

**Adopt two standing rules, documented here and summarised in `CLAUDE.md`.**

1. **Make illegal states unrepresentable.** When a decision could encode an
   invariant in a type the compiler (or serde, at the wire boundary) checks,
   prefer that over a comment, a convention, or a runtime check. Apply it at the
   *chokepoint* every path funnels through, not at each call site. This is a
   design test applied to new work, not a mandate to retrofit — and it has a
   boundary: **genuinely open or runtime-chosen sets stay values, not types.**
   Periodicity (static, known when the model is written) is a compile-time tag;
   currency (dynamic, chosen at runtime) is a runtime value on `Money`
   (ADR-0034). A newtype that adds ceremony without removing a real failure mode
   is not an improvement.

2. **Test the class, not the instance; pin every stated assumption.** When a
   rustdoc line or an ADR *asserts* a behaviour — "NPV decreases as the rate
   rises", "present value inverts future value", "`ALL` lists every currency",
   "the periodicities must match" — that assertion earns a test that fails the
   moment the code stops honouring it. Where the assumption is a *universal*,
   prefer a property test (proptest) over a point test; where the domain is a
   small *finite* enum, prefer exhaustive iteration over sampling; where the
   invariant lives in the type system, a `compile_fail` doctest is the test.

**Immediate application (this ADR's PR):** guard `Currency::ALL` against enum
drift. A `#[cfg(test)]` exhaustive-match tripwire (`_every_variant_is_named`)
forces every variant to be named, so adding one fails to compile until the author
is routed to `ALL`; a companion test pins `ALL`'s length and rejects duplicates.
This is a **tripwire, not a proof** — stable Rust cannot enumerate an enum's
variants without a hand-list or a proc-macro (`strum`), and the zero-dep core
(ADR-0009) forgoes the latter — but it converts *silent* drift into a compile
error with an instruction attached.

## Consequences

- The design instinct and the testing discipline are now citable, not folklore;
  future ADRs and reviews can point at ADR-0045 the way the siblings point at
  their own.
- New work carries an explicit question ("can the wrong state be made
  unrepresentable, at the chokepoint?") and a testing obligation ("does every
  assertion I wrote have a test that fails when it stops being true?").
- `Currency::ALL` can no longer silently under-enumerate: adding a variant is a
  compile error until the tripwire is updated, and the length/duplicate test
  guards the constant itself.
- The rule is a *test*, not a mandate: it does not require rewriting existing
  code, and it explicitly declines type ceremony that catches no real error.
- Follow-on obligation: when a future assertion in docs or an ADR is genuinely a
  universal, it should arrive with a property test rather than a single example.

## Alternatives considered

- **Leave it implicit.** The crate already behaves this way, so why write it down?
  Rejected: the siblings' experience is that the unstated version leaves gaps
  (a universal pinned by one example, a hand-list with no guard), and both cross-
  reference this crate — the discipline should be first-class here too.
- **`strum::EnumIter` (dev-dependency) for an airtight `ALL` completeness proof.**
  It would make drift *impossible* (iterate the enum, compare to `ALL`) rather
  than merely caught by a tripwire. Rejected for now to keep the core's dev-tree
  minimal and its zero-dep character intact; the exhaustive-match tripwire closes
  the realistic (hand-edit / regeneration) failure mode without a proc-macro. If
  more hand-maintained finite-set constants appear, revisit.
- **Auto-trait locks** (`fn _assert_send_sync::<Money>()` &c.) to pin that the
  public types stay `Send + Sync`. Rejected here: the crate does not currently
  *state* a thread-safety guarantee, and rule 2 is to pin *stated* assumptions —
  adding the assertion before the promise inverts it. Revisit if and when the
  guarantee is stated.
- **A full "types in, types out" retrofit** across the existing API. Rejected as
  scope: rule 1 is a design test for new decisions, not a refactor; the existing
  surface already reflects it (ADR-0033/0034/0039).
