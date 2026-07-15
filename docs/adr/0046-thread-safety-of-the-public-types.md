# ADR-0046: The public value types are thread-safe (`Send + Sync`), locked by a test

- **Status:** Accepted
- **Date:** 2026-07-15
- **Deciders:** Project owner
- **Follows:** [ADR-0045](0045-make-illegal-states-unrepresentable.md) (pin every
  *stated* assumption — this states one, then pins it), [ADR-0009](0009-no_std-and-optional-libm.md)
  (`no_std` + zero-dep — the lock adds neither), [ADR-0043](0043-owned-cashflows.md)
  (the owned vs. borrowed split this contract distinguishes)

> **Canonical for the sibling Rust MCP repos** (`ferric-fred`, `rustrolabe`) for
> the auto-trait decide-and-pin discipline — see [ADR-0047](0047-shared-disciplines-across-the-sibling-rust-mcp-repos.md).

## Context

`time_value` is synchronous and holds no shared mutable state, so "thread safety"
here is **not** about locking or data races in the crate's own code — there are
none. It is entirely a question of the **auto-trait profile of the public types**:
are they `Send` (movable across threads), `Sync` (shareable by `&`), and `'static`
(free of borrowed lifetimes)?

Today every public type is trivially so — they are plain `f64`/enum/`Vec`/
`PhantomData` data with **no interior mutability** (even the lazy [`Schedule`]
iterator carries only a `u32` cursor mutated through `&mut self`). Downstream code
already relies on this: moving a [`Money`] into `thread::spawn`, sharing an
`Arc<Schedule>` between tasks, holding a [`Cashflows`] across an `.await` in a
`Send` future (the `-mcp` server's world), or valuing in parallel with `rayon`.

The problem is that **auto traits leak from fields and cannot be declared.** A
`Send`/`Sync`/`'static` guarantee is never written in a signature, so a routine,
semver-*patch* refactor — adding an `Rc` cache, a `Cell`, a `&dyn Trait`, a raw
pointer to a public type — can silently strip `Sync` (or `Send`, or `'static`)
with no signature change and no warning, breaking every downstream `spawn`/`Arc`/
`Send`-future use. That is exactly the "a stated assumption pinned by nothing"
failure mode ADR-0045 targets: the assumption here is stated only implicitly, by
the types happening to be plain data.

## Decision

**State the thread-safety profile as a maintained part of the public API, and lock
it with a compile-time test.**

The profile is two-tier, because the borrowing views cannot be `'static`:

- **Owned types are `Send + Sync + 'static`:** [`Money`], [`Currency`], [`FxRate`],
  [`Rate<P>`], [`TvmError`], the [`amortization`] types ([`Installment`],
  [`Schedule<P>`]), the periodicity markers, and — behind their feature gates —
  [`Period<P>`], [`ContinuousRate`], [`DatedCashflow`] (`std`/`libm`) and
  [`OwnedCashflows<P>`] (`alloc`).
- **Borrowing views are `Send + Sync`** (not `'static`, by nature): [`Cashflows<'a, P>`]
  and [`DatedCashflows<'a>`]. Their `Sync` rides on `Money: Sync`, which holds.

**The lock** is `tests/thread_safety.rs`: two generic helpers
(`fn assert_send_sync_static<T: Send + Sync + 'static>() {}` and
`assert_send_sync<T: Send + Sync>() {}`) invoked on every public type, with the
periodicity tag witnessed by one representative marker (it is `PhantomData`, so the
family behaves identically) and assertions feature-gated to mirror each type's own
gate. It is zero-cost, zero-dependency, and `no_std`-clean (an integration test
links `std`, but asserts nothing that requires it of the crate). It passes today;
it fails to compile the moment a field regresses the profile.

## Consequences

- The profile is now a **semver commitment**: the owned types will stay
  `Send + Sync + 'static` and the views `Send + Sync`. Removing that is a breaking
  change. In practice this costs almost nothing for a pure-math value crate — there
  is no realistic future in which [`Money`] wants a `Cell` — the one theoretical
  casualty being a non-thread-safe internal optimization (e.g. `Rc`-based
  memoization in [`Schedule`]), which we forgo.
- Async and multithreaded consumers are explicitly supported: the profile is
  precisely the one that lets values cross `spawn`/`.await`/`Arc` boundaries. The
  guarantee *enables* async use — it does not constrain it.
- A new lock to keep green (`tests/thread_safety.rs`), run under `--all-features`
  like the other integration tests. It doubles as documentation of the profile.
- The obligation is small and local: a new public type adds one line to the test,
  in the tier that matches its lifetime story.

## Alternatives considered

- **Lock without stating it** — add the test as a silent regression guard but make
  no docs/semver promise. Rejected: it inverts ADR-0045 (pin a *stated* assumption)
  — a guard with no promise is a test defending a contract we refuse to admit we
  have, and it leaves downstream users guessing whether they may rely on it.
- **Do nothing** — the types are obviously `Send + Sync`, so why bother? Rejected:
  "obvious" is exactly what a silent auto-trait regression exploits, and the
  reliance is real (the `-mcp` server holds these across `.await`). Cheap insurance
  against an invisible break.
- **`unsafe impl Send/Sync` or a `PhantomData` marker to *force* the profile** (the
  inverse of rustrolabe's `!Sync` handle). Rejected as unnecessary and dishonest:
  the types earn the traits naturally; forcing them would paper over a real future
  regression instead of surfacing it.
- **Assert only the "risky" types** (those with heap/lazy state). Rejected: a reader
  should not have to reverse-engineer which types were judged safe; asserting all of
  them is one line each and reads as a complete contract.
