# ADR-0047: Shared disciplines across the sibling Rust MCP repos — a cross-repo index

- **Status:** Accepted
- **Date:** 2026-07-15
- **Deciders:** Project owner
- **Follows:** [ADR-0045](0045-make-illegal-states-unrepresentable.md) and
  [ADR-0046](0046-thread-safety-of-the-public-types.md) (the two most recent
  discipline ADRs, both of which already cross-reference the siblings), and the
  ADRs this index cites throughout.

## Context

Three sibling Rust repositories in the same estate — **`ferric-fred`** (a FRED
client library + CLI + MCP server), **`rustrolabe`**, and **this crate,
`time_value`** — have independently converged on the same handful of development
disciplines: types as a design tool, testing the class rather than the instance,
a typed MCP output layer, closed-vs-open vocabularies, a decided-and-pinned
auto-trait profile, MCP surface hygiene, and a single Nix-native toolchain. As
that convergence happened, the repos' ADRs began cross-referencing each other ad
hoc (ADR-0045 and ADR-0046 here both cite `rustrolabe` and `ferric-fred` by
number).

The cross-references are useful but uncoordinated: the same lesson is argued in
two or three places, and there is no single record of *which* ADR is the
canonical statement of each shared lesson, of *how this repo conforms*, or of
*where this repo deliberately diverges* (so nobody "harmonises" an intentional
difference). The owner has decided to consolidate this into a **cross-repo ADR
index with canonical ownership**: each repo gets one self-contained ADR that
names the single canonical ADR per shared lesson, states how *that* repo
conforms, and records its deliberate divergences — referencing canonical ADRs by
number rather than duplicating their text.

## Decision

**This ADR is `time_value`'s entry in that cross-repo index. It is a map, not a
source of truth: it names the canonical ADR for each shared lesson (owned by
whichever sibling states it best), records how `time_value` conforms by citing
`time_value`'s own ADRs, and lists `time_value`'s deliberate divergences.** It
copies no ADR text from another repo; shared lessons are referenced by number.
The canonical assignments below are identical across all three siblings' index
ADRs; only the "how `time_value` conforms" and "`time_value`'s divergences"
columns are repo-specific.

`time_value` owns one canonical lesson (**L5**, the auto-trait decide-and-pin
discipline, [ADR-0046](0046-thread-safety-of-the-public-types.md)); the other
canonical owners are the siblings, cited by repo + number.

### Shared lessons → canonical owner, and how `time_value` conforms

| # | Shared lesson | Canonical ADR | How `time_value` conforms |
|---|---------------|---------------|---------------------------|
| **L1** | **Types as a first-class design tool** — encode invariants in types the compiler checks, *with* the anti-ceremony boundary: open / runtime-chosen sets stay values, not types; no ceremony that removes no real failure mode. | **`rustrolabe` ADR-0101** (conforming siblings: `ferric-fred` 0027) | [ADR-0045](0045-make-illegal-states-unrepresentable.md) rule 1 ("make illegal states unrepresentable", at the chokepoint), building on [ADR-0005](0005-domain-modelling-and-strong-typing.md) / [ADR-0033](0033-core-domain-model-two-axes-and-an-f64-engine.md). The boundary is lived, not just stated: periodicity is a compile-time tag, **currency is a runtime value** on `Money` ([ADR-0034](0034-money-and-currency.md)) precisely because it is a runtime-chosen set. |
| **L2** | **Test the class, not the instance; pin every stated assumption** — universals → property tests (proptest); finite enums → exhaustive iteration; type invariants → `compile_fail` doctests. | **`rustrolabe` ADR-0107** (`ferric-fred`: partial / gap) | [ADR-0045](0045-make-illegal-states-unrepresentable.md) rule 2, adopted verbatim, and restated in `CLAUDE.md`'s testing section. Realised as `tests/properties.rs` (proptest), `compile_fail` doctests locking the periodicity mismatch (which neither sibling has), and the `Currency::ALL` exhaustive-match tripwire. |
| **L3** | **Typed output layer** — MCP `outputSchema` derived from the real return type via `schemars`, with a conformance test validating real output against the declared schema. | **`rustrolabe` ADR-0102** (the most rigorous: a real JSON-Schema validator + a negative test; conforming: `ferric-fred` 0023) | [ADR-0039](0039-typed-output-layer-for-the-binaries.md) (typed output DTOs deriving `Serialize + JsonSchema`; `outputSchema` switched on per family once the family is typed) plus [ADR-0044](0044-schemars-support.md) (`schemars` support, `default-features = false`). |
| **L4** | **Closed vs. open vocabularies** — `#[non_exhaustive]` always; an `Other` / catch-all *only* where a value must survive a round-trip; closed sets are curated enums with exhaustive metadata matches. | **`ferric-fred` ADR-0005 / 0027** (conforming: `rustrolabe` 0046 / 0103 / 0105) | [ADR-0034](0034-money-and-currency.md): `Currency` is a closed `#[non_exhaustive]` ISO-4217 enum with an exhaustive metadata match and the `_every_variant_is_named` tripwire ([ADR-0045](0045-make-illegal-states-unrepresentable.md)); `XXX` is the currency-agnostic identity, and a mismatch is a runtime `CurrencyMismatch` — no open catch-all, because the set does not round-trip unknowns. |
| **L5** | **Auto-trait profile — decide it, then pin it with a compile-time test.** The profile is per-repo; opposite profiles are legitimate. | **`time_value` ADR-0046** — *this repo is canonical* | [ADR-0046](0046-thread-safety-of-the-public-types.md): `assert_send_sync_static` / `assert_send_sync` helpers in `tests/thread_safety.rs` over every public type, framed as a semver commitment (owned types `Send + Sync + 'static`; borrowing views `Send + Sync`). `rustrolabe` 0011 is the deliberate **inverse** (`Send + !Sync` handle) — same meta-rule, opposite profile (see Divergences). |
| **L6** | **MCP surface hygiene** — read-only + open-world annotations, one tool per operation, CLI/MCP parity, error classification by caller-fixability (`invalid_params` vs `internal_error`), reject unknown params / out-of-range at the boundary. | **`rustrolabe` ADR-0044 + 0045** | [ADR-0011](0011-mcp-server.md) (stateless server, one-to-one tool → operation, CLI parity, typed inputs → `inputSchema`), extended by [ADR-0028](0028-binary-surface-conventions.md) / [ADR-0029](0029-dated-cashflows-xnpv-xirr.md). **Local choice:** `time_value` runs **no agent-driven MCP audit** (unlike the siblings); surface hygiene here rests on CLI/MCP parity, typed schemas, and conformance tests. That is a deliberate optional-pattern difference, not a gap to fix. |
| **L7** | **Nix-native single toolchain** — `nix develop -c cargo …` as the one toolchain definition, identical locally and in CI, including the dedicated `.#msrv` shell. | **3-way identical** (all three ADR-0008); no single canonical needed | [ADR-0008](0008-nix-flake-dev-environment.md) plus the `.#msrv` shell of [ADR-0017](0017-per-crate-msrv-core-1.85.md); `CLAUDE.md`'s "Tooling" section is the command list. |

### `time_value`'s deliberate divergences

These are recorded as **intentionally local** — they must **not** be harmonised
toward the siblings, and (as noted) several must **not** leak the other way
either.

- **`no_std` + zero-dependency core — `time_value` only ([ADR-0009](0009-no_std-and-optional-libm.md)).**
  **This is the load-bearing warning of this ADR.** The zero-dep `no_std` core
  (embeddable, WASM-`no_std`) is `time_value`'s foundational constraint and
  **must not leak to `ferric-fred` or `rustrolabe`.** Several downstream
  `time_value` choices are *artifacts* of it and would be wrong advice if
  generalised: forgoing `strum` for the `Currency::ALL` tripwire (a hand-list +
  tripwire instead of `EnumIter`), hand-written `Wire` structs shared by `serde`
  and `schemars`, `schemars` with `default-features = false`, and declining a
  mirror currency enum. Read those as consequences of L1/L2/L4 *under a zero-dep
  constraint the siblings do not share* — not as the shared lesson itself.
- **Per-crate MSRV — `time_value` pins the core at 1.85 ([ADR-0017](0017-per-crate-msrv-core-1.85.md)),**
  verified in a dedicated `.#msrv` devShell as a **build** (not a test) so
  dev-deps don't gate it; the binaries stay at 1.88 ([ADR-0016](0016-msrv-and-toolchain-bump.md)).
  The siblings **decline** a pinned MSRV. This is `time_value`'s local posture,
  not a shared rule.
- **Auto-trait direction — `time_value`'s `Send + Sync + 'static` is the
  deliberate inverse of `rustrolabe`'s `Send + !Sync` handle.** Same L5
  meta-rule (decide, then pin), **opposite** profile. Recorded here so nobody
  "harmonises" the two: both are correct for their repo.
- **Release posture — no scheduled release; develop continuously
  ([ADR-0038](0038-no-scheduled-release-continuous-development.md)),** machinery
  wired but inert (the held `release-plz` PR #28). The siblings differ. Only the
  meta-rule is shared: *state your release model explicitly and hold the line.*
- **Sync model — `time_value` is synchronous ([ADR-0003](0003-synchronous-computation-model.md)),**
  like `rustrolabe`, unlike `ferric-fred`'s async. Async is confined to the
  `-mcp` crate. Local.

## Consequences

- There is now a single, citable answer to "who owns lesson *X*, and how does
  `time_value` conform?" — future ADRs and reviews point at ADR-0047 the way
  ADR-0045 / ADR-0046 already point at the siblings.
- The canonical assignments are duplicated (by design) as an identical map in
  each sibling's index ADR; only the conformance and divergence columns differ.
  If a canonical owner is reassigned, all three index ADRs are updated together
  (append-only: a new ADR supersedes, per [ADR-0001](0001-record-architecture-decisions.md)).
- The deliberate divergences are protected from well-meaning harmonisation — in
  particular, the `no_std` / zero-dep constraint and its artifacts are flagged as
  **not** to be generalised to the siblings, and the auto-trait direction is
  flagged as **not** to be aligned with `rustrolabe`.
- This ADR copies no other repo's text; it references by number. It therefore
  cannot drift from the canonical ADRs' wording — only their *numbers* are load-
  bearing here, and a renumber (which the repos' immutability rule forbids)
  would be the only way to break the map.

## Alternatives considered

- **A new shared repository holding the canonical ADRs.** Rejected: it would need
  its own auth, CI, and release story, and it would pull each lesson away from
  the code it governs — the opposite of ADRs living beside their code. The
  canonical ADR stays in whichever repo states it best; this index just points.
- **Duplicate the canonical ADRs' text into each repo.** Rejected: three copies
  of the same argument drift, and immutability makes fixing the drift painful.
  Reference by number instead.
- **No index — keep the ad-hoc cross-references.** Rejected: they answer "does
  repo A cite repo B?" but not "what is the *one* canonical statement of lesson
  X, and where does this repo deliberately differ?" — which is exactly what
  prevents an intentional divergence from being "fixed."
- **Record the divergences only in `CLAUDE.md`.** Rejected: `CLAUDE.md` is
  guidance that is edited in place; the divergences (especially the `no_std`
  leak-warning) are decisions that deserve the append-only, immutable ADR record.
