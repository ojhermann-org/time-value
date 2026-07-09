# ADR-0021: Operations are fallible when their result can be non-finite

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner
- **Supersedes:** [ADR-0019](0019-1.0-public-api-decisions.md) §2 (its "operations stay infallible" decision)
- **Amends:** [ADR-0013](0013-core-api-values-and-discrete-operations.md), [ADR-0014](0014-transcendental-single-sum-operations.md), [ADR-0015](0015-annuities.md) (operation signatures), [ADR-0020](0020-robust-irr-newton-with-bisection-fallback.md) (IRR convergence tolerance)

## Context

[ADR-0019](0019-1.0-public-api-decisions.md) §2 decided that the value-returning
operations stay **infallible** (return `Money`, not `Result<Money>`): `Money`
guarantees finiteness only on *construction*, and on extreme inputs an operation
can overflow to `±∞` (or, with mixed large magnitudes, `NaN`). Its reasoning was
that making every operation fallible is "a large ergonomic cost for an overflow
that needs absurd inputs, and one that could not be walked back post-`1.0`."

Two things changed that calculus:

1. **`1.0` has not shipped.** No release of the new line exists and no
   compatibility promise is outstanding, so the change *can* be walked back — now
   is exactly the free moment ADR-0019 assumed had passed.
2. **The infallible contract pushes a silent foot-gun onto every consumer.** A
   review of the CLI and MCP surfaces found that an overflow is not loud — it is
   *invisible*. `serde_json` cannot represent a non-finite `f64`, so it emits
   `null`: the CLI printed `inf` (exit 0) or `{"fv":null}`, and the MCP server
   returned `{"future_value":null}` with `isError:false` — a wrong answer
   presented to an assistant as a *successful* tool call. Every downstream caller
   would have to remember to call `value().is_finite()`; nothing makes them.

The project's stated aim is that the core is reliable and clear throughout, and
that TVM mistakes are hard to make by accident. An operation that can silently
return a non-answer is at odds with both.

## Decision

**A value-returning operation returns `Result<_, TvmError>` if and only if its
result could be non-finite. Operations whose result is provably always finite
stay infallible.** This is a single rule with no per-operation judgement calls.

Applying it:

- **Become fallible** (previously returned `Money`):
  - `Cashflows::net_present_value` → `Result<Money, TvmError>`
  - `Cashflows::net_future_value` → `Result<Money, TvmError>`
  - `single_sum::present_value`, `single_sum::future_value` → `Result<Money, TvmError>`
  - `annuity::present_value`, `annuity::future_value` → `Result<Money, TvmError>`
- **Already fallible, unchanged:** `Cashflows::internal_rate_of_return[_from]`,
  `annuity::payment`.
- **Stay infallible** (result provably finite): the `value()` accessors,
  `Rate::periods_per_year`, the `ZERO` / `Default` constructors, and sign
  negation of a `Money` (negating a finite value is finite). A future scalar
  `Money * f64` *can* overflow, so per the rule it is a fallible method, not a
  `std::ops::Mul` impl (see [issue #18](https://github.com/ojhermann-org/time-value/issues/18)).

Each fallible operation computes its `f64` result and passes it through
`Money::new`, so the finiteness check is the constructor's existing validation
rather than a second code path.

### A dedicated error variant for a non-finite result

Add `TvmError::NonFiniteResult` — "an operation did not produce a finite amount."
It is distinct from `NonFiniteAmount`, which keeps its precise meaning: *a
non-finite value was passed to a constructor*. `NonFiniteResult` covers both true
overflow and the mathematically undefined degenerate cases that manifest as a
non-finite result — in particular `annuity::payment` over **zero periods**
(amortising over nothing), which previously surfaced as the ill-fitting
`NonFiniteAmount`. `TvmError` is `#[non_exhaustive]`, so the new variant is a
non-breaking addition.

### Empty-series convention

Fixed and documented, so it is a decision rather than an accident:
`net_present_value` and `net_future_value` of an **empty** series return
`Ok(Money::ZERO)` — there is nothing to discount or compound, and zero is the
additive identity of a sum. `internal_rate_of_return` of an empty series stays
`Err(TvmError::EmptyCashflows)` — zero has no rate that makes it zero.

### Accompanying numerical hardening

So that the new errors fire only on genuinely unrepresentable results and not on
ordinary inputs:

- **`net_present_value` is evaluated by Horner's method** (as `net_future_value`
  already is), rather than accumulating a running `discountᵗ` factor. This avoids
  the factor itself overflowing for a rate near `−100%` or a long series, so NPV
  overflows only when the true result is genuinely out of `f64` range.
- **The IRR solver's convergence tolerance is scaled by the cashflow
  magnitude** (amending [ADR-0020](0020-robust-irr-newton-with-bisection-fallback.md)):
  the previous absolute `1e-9` on the NPV was unreachable for large-magnitude
  series, spuriously failing with `IrrDidNotConverge`. A magnitude-relative
  tolerance converges across scales.

## Consequences

- Overflow can no longer masquerade as an answer: a caller gets an `Err`, not a
  silent `inf`/`null`. The CLI and MCP thread the `Result` and surface it as an
  error (fixing the review finding above) in the same change.
- The common path gains a `?` on the value operations. This matches the crate's
  existing style — constructors (`Money::new`, `Rate::new`, `Period::new`) are
  already fallible — so a typical call site is `?`-threaded throughout rather than
  mixing infallible and fallible calls.
- Every *future* operation (rate conversions, continuous compounding, XNPV/XIRR)
  follows the same rule, so the surface stays uniform as it grows.
- ADR-0019's other three decisions (drop `serde`, `single_sum` module, `f64` IRR
  seed) are unaffected; only its §2 is superseded.
- The `try_*`-variants escape hatch that ADR-0019 kept open is no longer needed —
  the operations themselves are the checked form.

## Alternatives considered

- **Keep operations infallible + add `_checked` variants** (ADR-0019's escape
  hatch) — roughly doubles the operation surface, and leaves the default a
  foot-gun; a caller who does not know to reach for `_checked` still gets a silent
  non-answer.
- **Draw the fallibility line at *realistic risk*** — make only the
  `powf`-based operations (`single_sum`, `annuity`) fallible and keep the
  arithmetic ones (`npv`/`nfv`) infallible, since after Horner-stabilization their
  overflow needs absurd inputs. Rejected: "infallible unless overflow is
  *possible*" is a cleaner, more teachable contract than "infallible unless
  overflow is *likely*", which requires a judgement call at every new operation.
- **Keep infallible + document + `is_finite()`** (ADR-0019 as written) — the
  smallest change, but it is precisely the contract the review found wanting; the
  onus stays on every consumer to remember a check nothing enforces.
- **A single "the amount is not finite" variant covering both construction and
  results** — conflates a caller passing bad input with an operation overflowing
  on good input; two variants keep the cause legible.
