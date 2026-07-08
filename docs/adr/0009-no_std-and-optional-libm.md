# ADR-0009: `no_std` core & optional `libm`

- **Status:** Accepted (the `serde` feature was dropped for 1.0, leaving `std`/`libm` — [ADR-0019](0019-1.0-public-api-decisions.md))
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

`time_value` is a foundational arithmetic library that should be embeddable
anywhere Rust runs — including embedded and WASM targets without an operating
system. That argues for `#![no_std]` and no mandatory dependencies. The
complication: some TVM operations need **transcendental functions** — `powf` for
compounding over a fractional number of periods, `ln`/`exp` for continuous
compounding and for solving. Those live in `std` (`f64::powf` et al. are not in
`core`).

## Decision

**The core is `#![no_std]` and depends on nothing by default.** Transcendental
math is reached through **optional, off-by-default features**:

| Feature | Effect |
|---------|--------|
| `std`   | Use `std`'s `f64` math (and any `std`-only conveniences). |
| `libm`  | Use [`libm`](https://crates.io/crates/libm) for `powf`/`ln`/`exp` in a `no_std` build. |
| `serde` | Derive `Serialize`/`Deserialize` on the domain newtypes ([ADR-0005]). |

- The default build (`default = []`) is `no_std`, zero-dependency, and provides
  everything that needs only elementary arithmetic.
- Operations requiring transcendental functions are available when **either**
  `std` **or** `libm` is enabled; `libm` is the `no_std` path, preferred over an
  unconditional dependency.
- The binaries enable the features they need (`std`, `serde`, and `libm` where a
  `no_std`-style build is wanted).

## Consequences

- The library links into `no_std` targets out of the box; the heavier operations
  are one feature flag away.
- No mandatory dependency: a plain `cargo add time_value` pulls in nothing.
- The API must be organised so the transcendental-dependent operations are
  clearly the ones gated behind `std`/`libm`.
- `serde` is likewise opt-in, keeping it out of the default tree
  ([ADR-0004](0004-error-handling.md) keeps error handling dependency-free too).

## Alternatives considered

- **Unconditional `std`** — simplest, but forfeits embedded/WASM-nostd use for a
  library whose whole appeal is being foundational.
- **Unconditional `libm` dependency** — makes the math always available but forces
  a dependency on every user, including those who only need the elementary ops or
  who already have `std`.
- **Hand-rolled `powf`/`ln`** — avoids the dependency but re-implements numerics
  that `libm`/`std` already do correctly; not worth the accuracy risk.

[ADR-0005]: 0005-domain-modelling-and-strong-typing.md
