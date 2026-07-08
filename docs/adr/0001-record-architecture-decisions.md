# ADR-0001: Record architecture decisions

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

`time_value` is a deliberately type-heavy redesign for the `1.0` line, and it is
growing from a single library crate into a workspace with a CLI and an MCP
server. Design choices — how periodicity is encoded in the type system, why the
core is `no_std`, why async is confined to one crate — are the interesting part
of the project, and they are easy to lose. Code shows *what*; it rarely shows
*why*, or which alternatives were weighed and rejected. The sibling repositories
(`ferric-fred`, `rustrolabe`) keep an architecture-decision log, and this project
adopts the same practice.

## Decision

We will keep **Architecture Decision Records** (ADRs), in the lightweight
[Nygard format](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions),
under `docs/adr/`.

- Each ADR is a numbered Markdown file, `NNNN-kebab-title.md`, starting at
  `0002` (`0000` is the template, `0001` is this record).
- New ADRs take the next free number and are copied from
  [`0000-adr-template.md`](0000-adr-template.md).
- An ADR is **immutable once Accepted**. We do not rewrite history; a decision
  that changes is captured in a *new* ADR that marks the old one **Superseded**
  (with a link in both directions).
- `docs/adr/README.md` is the index.
- ADRs are committed alongside the change they describe, so a design and its
  rationale land together.

## Consequences

- The rationale behind the type-system tricks, the `no_std` posture, and the
  tooling lives next to the code and survives contributor turnover.
- A small, standing discipline: non-trivial or hard-to-reverse choices get an
  ADR rather than living only in a commit message or a PR thread.
- The log is append-only, so reading it top-to-bottom tells the project's story
  in order.

## Alternatives considered

- **No formal record** — rely on commit messages and code comments. Cheapest,
  but the "why" and the rejected alternatives scatter and decay.
- **A single design document** — one file that is edited in place. Loses the
  chronology and the immutability that make individual decisions auditable.
- **A wiki / external tool** — drifts from the code, needs separate auth, and is
  not versioned with the change it describes.
