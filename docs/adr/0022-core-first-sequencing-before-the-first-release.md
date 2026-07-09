# ADR-0022: Complete the core before the first release (core-first sequencing)

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner
- **Amends:** [ADR-0012](0012-ci-and-release-automation.md) (its release sequencing), [ADR-0019](0019-1.0-public-api-decisions.md) (its "publishing 1.0.0 fixes the API" premise)
- **Related:** [ADR-0021](0021-fallible-operations-on-non-finite-results.md)

## Context

[ADR-0012](0012-ci-and-release-automation.md) planned to **publish the core
`time_value` first** and keep `time-value-cli` / `time-value-mcp` at
`publish = false` "until their surfaces stabilise", with the additive roadmap
(rate conversions, Money arithmetic, annuity-due, …) filed as post-`1.0` work.
[ADR-0019](0019-1.0-public-api-decisions.md) reviewed the surface "before the
first release" on the premise that publishing `1.0.0` would then freeze it.

Two things reframed that:

1. **Nothing has shipped.** The `1.0` line has no release and no compatibility
   promise outstanding (the old `0.1.0`–`0.8.0` are a separate, immutable
   history). So the surface is *not yet* frozen — it can still change freely.
2. **A pre-release evaluation of the core** found (a) a robustness hole worth
   fixing in the API itself, not papering over at the boundary — the operations
   returned non-finite results silently ([ADR-0021](0021-fallible-operations-on-non-finite-results.md));
   and (b) that the most conspicuous domain gaps (rate conversions between
   periodicities above all, plus Money arithmetic and annuity-due/perpetuity) are
   exactly the API-*shaping* additions best done while the surface is still free
   to change, not bolted on after a `1.0` freeze.

Releasing a deliberately-incomplete core and immediately needing `1.1`/`1.2` for
its headline features — while its published operations carried a silent
overflow foot-gun — is the outcome this ADR avoids.

## Decision

**Complete and harden the core, and bring the CLI and MCP to a deliberate v1,
*before* the first release; then publish all three crates together at `1.0.0`.**

Concretely, the pre-release scope is:

- **Core hardening** — the fallibility contract and numerical robustness of
  [ADR-0021](0021-fallible-operations-on-non-finite-results.md).
- **Tier-1 completeness, folded in now** — rate conversions between periodicities
  ([#17](https://github.com/ojhermann-org/time-value/issues/17)), Money arithmetic
  ([#18](https://github.com/ojhermann-org/time-value/issues/18)), and annuity-due
  + perpetuity ([#19](https://github.com/ojhermann-org/time-value/issues/19)).
- **A deliberate CLI + MCP v1** — a surface review and freeze
  ([#30](https://github.com/ojhermann-org/time-value/issues/30)), then flip the
  binaries to publishable
  ([#20](https://github.com/ojhermann-org/time-value/issues/20)).

The three crates take **`1.0.0` together** for a coherent v1 launch; per-crate
independent versioning ([ADR-0002](0002-workspace-layout.md)) resumes afterward.

Everything else on the old roadmap — serde, continuous compounding, XNPV/XIRR,
owned `Cashflows`, currency tag, small conveniences, and the newly-surfaced
NPER/RATE, MIRR, and amortization schedule — is **deferred to after `1.0.0`**,
to be reassessed once the Tier-1 core lands (it may be pulled forward there).

### Tracking

Work is tracked in GitHub under the **`1.0.0`** milestone, sequenced by the
**"Road to 1.0.0"** epic ([#34](https://github.com/ojhermann-org/time-value/issues/34)).
The old **"1.x roadmap"** milestone is renamed **"Post-1.0 backlog"** and holds
the deferred items. The release-plz "chore: release v1.0.0" PR (#28) is **held**
until the sequence completes.

## Consequences

- The first published `1.0.0` is a complete, hardened TVM core with matching v1
  binaries — not a lean core that immediately needs minor releases for its
  headline features.
- The release *infrastructure* is already in place (release-plz + OIDC
  `publish.yml`, landed by #12), so "release" is a bookkeeping step once the work
  is done; it still needs crates.io Trusted Publishers registered for
  `time-value-cli` and `time-value-mcp` (the core's is already registered).
- ADR-0019's surface review stands, *except* where later ADRs deliberately revise
  it (e.g. ADR-0021 supersedes its §2); "frozen at `1.0`" now means frozen at the
  `1.0.0` this sequence produces, not at the moment ADR-0019 was written.
- The Tier-1 additions are done under the fallibility rule of ADR-0021, so the
  growing surface stays uniform.

## Alternatives considered

- **Ship the core `1.0.0` now, add Tier-1 as fast-follow `1.x`** (the original
  ADR-0012 plan) — releases an incomplete core with the overflow foot-gun, and
  spends the first minor releases on features we already know we want; the only
  gain is an earlier tag, which nothing is waiting on.
- **Publish the binaries much later, well after the core** — leaves the CLI/MCP in
  limbo and forgoes a coherent "here is `time_value` 1.0, on the shell and in your
  assistant" launch.
- **Freeze the surface as-is per ADR-0019 and release** — locks in a silent
  overflow contract and a periodicity-typed library that cannot convert
  periodicities; both are cheap to fix now and expensive after a `1.0` promise.
