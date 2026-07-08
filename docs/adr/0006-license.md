# ADR-0006: License

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** Project owner

## Context

`time_value` is a general-purpose library intended for the widest possible reuse,
published on crates.io. The Rust ecosystem has a strong convention for such
crates, and the old `0.x` series is already public. We want a license that
imposes the fewest obstacles on adopters while retaining patent protection.

## Decision

The whole workspace is licensed **`MIT OR Apache-2.0`** (dual, at the user's
option) — the Rust ecosystem norm. Every crate declares `license = "MIT OR
Apache-2.0"` (via `[workspace.package]`), and the repository carries
`LICENSE-MIT` and `LICENSE-APACHE` at its root.

`deny.toml` ([ADR-0012](0012-ci-and-release-automation.md)) allows only
permissive licenses compatible with redistributing under either arm (MIT,
Apache-2.0, BSD-2/3-Clause, ISC, Unicode-3.0, Zlib, Unlicense), so no dependency
can introduce an incompatible obligation unnoticed.

## Consequences

- Adopters pick whichever arm suits them; Apache-2.0 provides an explicit patent
  grant, MIT provides maximum simplicity.
- Contributions are dual-licensed under the same terms (stated in the READMEs).
- The permissive-only `cargo deny` gate keeps the dependency tree license-clean.

## Alternatives considered

- **A single license (MIT *or* Apache-2.0 alone)** — simpler, but the dual form
  is what the ecosystem expects and gives adopters both the patent grant and the
  simple option.
- **A copyleft license (e.g. AGPL, as `rustrolabe` uses)** — appropriate for an
  application you do not want re-hosted closed-source, wrong for a foundational
  library meant to be embedded everywhere.
