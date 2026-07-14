# ADR-0038: No scheduled release — continuous development

- **Status:** Accepted
- **Date:** 2026-07-14
- **Deciders:** Project owner
- **Supersedes:** [ADR-0022](0022-core-first-sequencing-before-the-first-release.md)
  (its release sequencing and "release all three at `1.0.0`" plan)
- **Amends:** [ADR-0012](0012-ci-and-release-automation.md) (its release *timing*;
  the release *machinery* it built stands)

## Context

ADR-0022 planned a specific release: complete and harden the core, bring the CLI
and MCP to a deliberate v1, then publish all three crates **together at `1.0.0`**,
tracked by a `1.0.0` milestone and a "Road to 1.0.0" epic, with the deferred work
in a "Post-1.0 backlog" milestone and the `release-plz` release PR held until the
sequence completed.

That framing has outlived its usefulness. The core-model rebuild (ADR-0033–0037)
and the earlier roadmap are done, and — as the owner put it — the project is now
developed **for its own sake**: we keep iterating, and if and when it feels like a
good spot, *then* we decide to cut a release. There is no target version, no
release-gating sequence, and no need to classify work as "release" or "not release"
or as "before" or "after" any version.

## Decision

**There is no scheduled release and no release-gating sequence. Develop
continuously; a release is an undated future decision by the owner.**

- **No release target or version goal.** Work is not organized toward `1.0.0` (or
  any version). Open issues are a flat backlog, prioritized by label
  (`tier-1`/`tier-3`), not sequenced toward a release. The `1.0.0` and
  "Post-1.0 backlog" milestones are dissolved, and the "Road to 1.0.0" epic is
  closed.
- **Quality is maintained per change, not saved for a release.** The standing bar
  is: green CI, an ADR for each real decision, and documentation kept current
  *with* each change (not as a pre-release cleanup). This is what makes continuous
  `main` development safe.
- **The release machinery stays in place but inert.** `release-plz` and the
  OIDC `publish.yml` (ADR-0012) remain wired and ready; they simply are not driven.
  Crates keep their in-tree versions; nothing is published. Old `0.1.0`–`0.8.0`
  remain the separate, immutable published history.
- **Cutting a release remains solely the owner's call, whenever they choose it.**
  Bumping versions, flipping any crate's `publish = false`, extending
  `release-plz`/`publish.yml`, tagging, or merging a release PR are out of scope for
  ordinary development and are never inferred from "the work looks done". When the
  owner decides to release, *that* is when release mechanics begin.

## Consequences

- Contributors (human or agent) stop reasoning about whether an item is
  release-relevant; they just do the next useful development work and keep the docs
  and ADRs honest as they go.
- The GitHub tracker reflects this: no release milestones, a flat label-organized
  backlog, and the release epic closed.
- ADR-0022's *release sequencing* is superseded; its incidental engineering
  observations (core-first hardening was worthwhile; the surface was free to change
  because nothing had shipped) remain true as history but no longer prescribe a
  path to a version.
- If a release is ever desired, this ADR is superseded by a new one that defines
  that release; until then, "when we're in a good spot" is the whole policy.

## Alternatives considered

- **Keep the `1.0.0` sequence (ADR-0022)** — imposes a release plan and a
  before/after-`1.0` taxonomy the owner no longer wants; it kept pulling attention
  toward publish mechanics that nothing is waiting on.
- **Set a looser release target (e.g. "release when the backlog is empty")** — still
  a gate, still frames every item as release-relevant. Rejected in favour of no gate
  at all.
- **Tear out the release machinery** — needless and destructive; leaving it inert
  costs nothing and makes an eventual release a bookkeeping step rather than a
  rebuild.
