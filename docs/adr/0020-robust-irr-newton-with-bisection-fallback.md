# ADR-0020: Robust IRR — Newton with a bisection fallback

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner
- **Amends:** [ADR-0013](0013-core-api-values-and-discrete-operations.md) (the IRR solver)

## Context

[ADR-0013](0013-core-api-values-and-discrete-operations.md) implemented
`internal_rate_of_return[_from]` as pure Newton–Raphson from an initial guess.
Newton is fast and, seeded near a root, precise — but it is *not robust*: on a
poor guess it can overshoot below −100% (leaving the valid domain), stall on a
near-flat derivative, or oscillate, and then the method reports
`IrrDidNotConverge` even for a series that plainly *has* an IRR. For a library
whose selling point is trustworthy TVM, "no answer" on a well-posed problem is a
real weakness, and one the pre-1.0 hardening pass should remove.

A root of the NPV is bracketed by any two rates at which the NPV has opposite
signs, and bisection on such a bracket always converges. That is the standard
robust complement to Newton.

## Decision

`internal_rate_of_return_from(guess)` now tries **Newton–Raphson from `guess`
first**, and falls back to a **bracketing bisection** when Newton fails:

- **Newton first** — unchanged in spirit: fast, and it converges to the root
  *nearest the guess*, so the `_from` variant still lets a caller steer toward a
  chosen root when a non-conventional series has several. On any failure (budget
  exhausted, flat derivative, or an iterate that leaves `r > −1` / goes
  non-finite) it now returns "no result" rather than erroring, handing off to:
- **Bracket then bisect** — scan `1 + r` geometrically from just above `0`
  (`r → −1⁺`) upward by a fixed ratio, looking for the first sign change in the
  NPV between consecutive samples, then bisect that bracket to tolerance. If no
  sign change is found across the scan, the NPV never crosses zero and the series
  has no real IRR → `IrrDidNotConverge`.

Both stages use only elementary arithmetic (integer powers of the discount factor
accumulated in one pass), so **IRR stays in the default `no_std`,
dependency-free build** — the fallback adds no `powf`, no `std`, no `libm`.

The fallback returns the **lowest** bracketed root. For a conventional series
(one sign change in the cashflows) there is exactly one IRR and this is it; for a
non-conventional series with several, callers wanting a specific root use
`internal_rate_of_return_from` with a guess near it (Newton then lands on that
one).

## Consequences

- Well-posed problems that pure Newton abandoned now converge; `IrrDidNotConverge`
  becomes an honest "no real IRR exists" (or a genuinely pathological series)
  rather than "the solver gave up."
- The error surface is unchanged (`EmptyCashflows`, `IrrDidNotConverge`), so this
  is not a breaking change to the API — only more cases now succeed.
- Slightly more code and a bounded amount of extra work in the fallback path
  (~a few hundred NPV evaluations, each O(n)); the Newton fast path is unchanged
  for the common case.
- The scan's resolution (geometric ratio) is fine enough not to step over a lone
  root but could, in principle, miss a pair of very close roots of a
  non-conventional series; the guided `_from` variant remains the tool for that.

## Alternatives considered

- **Keep pure Newton** — simplest, but leaves the robustness hole the hardening
  pass set out to close.
- **Replace Newton with pure bisection** — always converges given a bracket, but
  loses Newton's speed and its guess-steering (which root it finds), and needs a
  bracketing scan regardless. Newton-first keeps both virtues.
- **Brent's method** — faster asymptotic convergence than bisection, but more
  code and edge cases for a solver that already meets its tolerance in a handful
  of bisections; not worth the complexity here.
