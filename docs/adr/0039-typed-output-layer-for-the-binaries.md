# ADR-0039: A typed output layer for the binaries — "types in, types out"

- **Status:** Accepted
- **Date:** 2026-07-14
- **Deciders:** Project owner
- **Follows:** [ADR-0005](0005-domain-modelling-and-strong-typing.md) (domain
  modelling & strong typing — the core's stance, now extended to the surfaces),
  [ADR-0028](0028-binary-surface-conventions.md) (binary surface conventions),
  [ADR-0037](0037-currency-in-the-binaries.md) (currency in the binaries)

## Context

[ADR-0005](0005-domain-modelling-and-strong-typing.md) is the crate's founding
stance: encode the domain in validated newtypes and put the invariant in a type
the compiler checks, so a periodicity mismatch or an economically-meaningless
value is unrepresentable rather than caught by a comment or a runtime check. The
**core** lives up to this completely — `Rate<P>`, `Period<P>`, `Money` + runtime
`Currency`, `TvmError`, periodicity as a zero-cost phantom tag.

The **binary surfaces do not carry that discipline all the way through.** They are
typed on the way *in* and untyped on the way *out*, and the input side has two
closed sets modelled as open strings.

**Input — typed, with a closed-set-as-string gap.** The MCP input structs derive
[`JsonSchema`](../../crates/time-value-mcp/src/params.rs), so every tool advertises
a precise `inputSchema` — good. But two closed sets are `String`:

- `currency: Option<String>` on the 16 amount-bearing structs, validated at
  runtime by `resolve_currency` → `Currency::from_code`
  ([`server.rs`](../../crates/time-value-mcp/src/server.rs)).
- `periodicity` / `from` / `to: String` on the `rate_*` tools, validated by the
  `dispatch_periodicity!` macro's fall-through arm.

An invalid value is refused by a hand-written error *inside the handler* rather
than by deserialization *before* it, and the schema does not publish the allowed
set. The two binaries also disagree: the CLI parses `--currency` straight to the
typed `Currency` at the boundary (a clap `value_parser`), while MCP keeps it a raw
string — the same concept typed on one surface and stringly on the other.

**Output — untyped hand-built JSON.** No tool declares an `outputSchema`, and no
output *types* exist to derive one from. Every result is assembled by hand:
`result()`, `result_money()`, and an inline `serde_json::json!` block in
`amortize` on the MCP side; a `Scalar` carrier printed as a plain number, a
hand-built `serde_json::Map`, or a TSV table on the CLI side. This is the exact
untyped substrate ADR-0005's philosophy exists to remove, one layer out from the
core, and its characteristic failure modes are already present:

- **Fields echoed by hand, easy to forget** — the `currency` field is
  conditionally `insert`ed on each monetary path (`if currency != Currency::Xxx {
  … }`), repeated at every result site rather than falling out of a type.
- **The same shape written twice, free to drift** — `amortize` encodes its row
  shape once as a JSON object and once as TSV columns in the CLI; the field set is
  duplicated and nothing keeps the two in step.
- **No published output contract** — a programmatic consumer can only learn a
  result's shape by calling a tool and inspecting a sample.

A sibling project of the same owner reached this conclusion independently: an
untyped output layer sitting next to a typed input layer is a recurring source of
consistency defects, and the fix is to type the output the same way the input is
typed — *types in, types out*. The forces here are identical, and now is the
moment: the next natural work (**#67** FX `convert`, **#68** continuous
compounding in the binaries, **#30** surface review) all *add* output shapes.
Landing them the current way means hand-writing more `json!` blocks and then
trying to freeze a schema over shapes that were just churned. Typing the output
layer first means the new operations are born on typed results.

## Decision

**Extend ADR-0005's "encode it in a type" discipline to both binary surfaces, on
both the input and the output side.** Concretely:

### 1. A typed output layer (both binaries)

Each operation's result becomes a Rust type in the binary crate that produced it,
built *from* the library types — never hand-assembled JSON.

- **MCP.** Each tool returns a result DTO in `time-value-mcp` that derives
  `Serialize` **and** `JsonSchema`, populated via `From`/a small builder from the
  library values, replacing the `result()` / `result_money()` / inline `json!`
  blocks. The derived `JsonSchema` is **declared as that tool's `outputSchema`**,
  and a **conformance test** asserts that real tool output validates against its
  declared schema, so the two cannot silently drift.
- **CLI.** Each command's result is a DTO in `time-value-cli` that backs both the
  `--json` rendering (via `Serialize`) and the human rendering (plain number /
  TSV, via its own `Display`/formatting), so a result's shape — the `amortize` row
  above all — is defined **once** and cannot diverge between the two output modes.
- **The library stays raw.** The DTOs live in the binary crates and are built from
  library types; the core carries no wire/serde contract, preserving its
  serde-independence ([ADR-0005](0005-domain-modelling-and-strong-typing.md) as
  amended by ADR-0019). This is a surface layer, not a coupling, and it does not
  add a dependency to the published core.
- **The `currency` echo becomes a typed field** on the monetary DTOs (an
  `Option<…>` that serializes to the ISO code when present and is omitted for
  `Xxx`), realizing [ADR-0037](0037-currency-in-the-binaries.md)'s "echoed, not
  rounded" rule once in a type instead of at every result site.

### 2. Closed-set inputs become boundary enums

- **Periodicity** (`daily … annual`) becomes a boundary enum in each binary crate
  — `#[derive(Deserialize, JsonSchema)]` for MCP, `#[derive(clap::ValueEnum)]` for
  the CLI — mapped to the core marker types where the `rate_*` operations need
  them. An unknown periodicity is then refused by deserialization / clap parsing,
  before a handler runs, and the schema/`--help` publish the six values.
- **Currency** stays validated against the closed set at the boundary as it is
  today (`Currency::from_code`), and MCP is brought into line with the CLI: the
  code is resolved to the typed `Currency` at the edge rather than threaded as a
  raw `String`. To make the valid set discoverable without a second copy of the
  ISO table, the MCP input field keeps a validated-**string** deserialize type
  (so the boundary error stays the friendly "unknown ISO 4217 currency code `X`")
  but carries a **custom `JsonSchema` that emits an `enum` constraint listing the
  codes, generated from the core's existing `Currency` code table**. The schema
  thus advertises every valid code, there is no hand-maintained mirror enum to
  drift, and the boundary error message is unchanged. It is **not** modelled as a
  duplicated ~180-variant Rust enum — see the alternatives.

### 3. Sequencing

The work lands **incrementally, one operation family at a time** (the ADR-0028
groupings: series, single-sum, annuity, rate, amortize), each slice bringing its
own DTOs, its MCP `outputSchema`, and its conformance test. An `outputSchema` for a
family is switched on only once that family is typed, so a schema is never frozen
against a shape still in flux — which also means **#67/#68 build on the typed
layer** rather than adding more hand-built JSON to migrate later.

## Consequences

- **The whole class of output defects closes structurally.** A forgotten or
  drifting field becomes a compile error or a conformance-test failure, not a
  silent wire defect; the `currency`-echo and `amortize` dual-encoding stop being
  possible rather than being fixed case by case.
- **A published output contract.** MCP consumers can generate typed clients and
  validate responses without reverse-engineering from samples — the missing half
  of the tool contract. This is a contract to maintain: every result-shape change
  is now a schema change (the point, but a cost), and the conformance test is what
  keeps the declared schema honest.
- **Invalid closed-set input is refused at the boundary,** uniformly across both
  binaries, and the allowed values are self-documenting in the schema and in
  `--help`.
- **The two surfaces converge.** Currency is resolved to the typed `Currency` at
  both edges; periodicity is an enum at both edges; result shapes are defined by
  types on both.
- **Cost.** A real refactor across the ~20 MCP tool sites and the CLI's run
  functions, plus the DTO types and their `From`/builder impls and the conformance
  tests. It is sequenced family-by-family so it never blocks other work and each
  step is reviewable. Some near-identical DTOs will exist in both binary crates;
  we accept the small duplication rather than introduce a shared surface crate
  (ADR-0002 keeps the crates independent) — revisited only if the duplication
  proves costly.
- **No change to the core or its MSRV.** Everything here is in the binary crates,
  which already depend on `serde`/`schemars` (MCP) and `clap` (CLI).

## Alternatives considered

- **Leave the output hand-built; add an `outputSchema` by hand** — rejected. A
  hand-written schema with no type behind it drifts from what the handler emits,
  which is the failure mode this ADR exists to remove; and there would be nothing
  to make the CLI's two render paths agree.
- **Type the output but leave the closed-set inputs as strings** — rejected. It
  fixes "types out" and leaves "types in" half-done, keeping the runtime-validated
  string edge and the CLI/MCP currency disagreement. The input enums are cheap and
  complete the symmetry the ADR is about.
- **Model `currency` as a full ~180-variant Rust enum on input** — rejected. To be
  a `schemars` enum it would have to be a *mirror* of the core `Currency` in the
  MCP crate (the core is `no_std` + zero-dep and derives no `serde`/`schemars`),
  giving two ISO code lists that must stay in lockstep — the exact drift this ADR
  removes elsewhere — and serde's default "unknown variant, expected one of …[180
  codes]" is a strictly worse boundary error than the current message. The chosen
  middle path (validated-string deserialize + a schema `enum` generated from the
  core table, in the decision above) gives the discoverability an enum would,
  without the mirror list or the degraded error.
- **Keep `currency` a validated string with no schema `enum` at all** — the
  lighter option, and the first draft's recommendation. Rejected in favour of the
  generated schema `enum`: the discoverability it gives an MCP consumer (an agent
  or codegen client seeing the valid codes rather than learning them by rejection)
  is worth the one real cost, a larger `inputSchema` payload — and `schemars`
  `$ref`s a single shared `$defs` entry, so the code set ships once, not per tool.
- **A single shared DTO/surface crate feeding both binaries** — rejected as
  premature. The shapes are close but the crates are deliberately independent
  (ADR-0002); a shared crate is a larger commitment than the duplication warrants
  today.
- **Do it in one sweep rather than family-by-family** — rejected. A staged
  migration keeps each step small and reviewable, lets the `outputSchema` follow
  each family as it is typed, and avoids freezing schemas against shapes #67/#68
  are about to add.
