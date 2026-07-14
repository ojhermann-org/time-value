# ADR-0044: `schemars` support — JsonSchema companion to the serde wire format

- **Status:** Accepted
- **Date:** 2026-07-14
- **Deciders:** Project owner
- **Follows:** [ADR-0042](0042-serde-support.md) (the serde wire format),
  [ADR-0043](0043-owned-cashflows.md) (the `alloc` feature),
  [ADR-0037](0037-currency-in-the-binaries.md) / [ADR-0039](0039-typed-output-layer-for-the-binaries.md)
  (the MCP `CurrencyCode` workaround this retires)

## Context

ADR-0042 gave the public value types a validated `serde` wire format, and noted
that a `schemars`/`JsonSchema` companion was a *separate* decision — ADR-0037 had
deliberately kept `JsonSchema` off core `Currency` to avoid front-running that.
The consequence: `time-value-mcp` carries a `CurrencyCode` string newtype
(`params.rs`) that hand-writes **both** a `Deserialize` (resolving via
`Currency::from_code`) **and** a `JsonSchema` (the ISO-code `enum` from
`Currency::ALL`). With serde done (#21), the deserialize half is redundant with
core `Currency`'s serde impl — but the newtype must remain for its `JsonSchema`,
because the **orphan rule** forbids `impl JsonSchema for Currency` anywhere but the
core.

Investigation (recorded on issue #82) found `schemars` 1.x is `no_std` + `alloc`
capable (`default-features = false`; its base deps — `serde_json`, `dyn-clone`,
`ref-cast`, `schemars_derive` — are all `alloc`-friendly; the heavy integrations
are optional), so a core feature composes with the `no_std` posture (ADR-0009).

## Decision

**Add an optional, off-by-default `schemars` feature** implementing `JsonSchema`
for the same public value types the `serde` feature covers — the JSON-Schema
companion, describing the **identical** shapes (ADR-0042).

- **Feature.** `schemars = ["dep:schemars", "alloc"]` — it **implies `alloc`**
  (schemars builds its schemas with `alloc`). The workspace dep is
  `default-features = false, features = ["derive"]` (drops schemars' own `std`) so
  the core stays `no_std`; `-mcp` re-adds `std` on its inheriting line.
- **Shapes track ADR-0042.** `Rate` / `Period` / `ContinuousRate` → an inlined
  `number`; `Currency` → an inlined `string` with the ISO-code `enum` from
  `Currency::ALL`; `Money` / `FxRate` / `DatedCashflow` / `Installment` → their
  field objects. The composites **reuse the very same private `*Wire` structs**
  (`src/wire.rs`) that back serde, so the serde and schemars descriptions cannot
  drift; the newtypes and `Currency` get hand-written impls (the derive would force
  `P: JsonSchema` on the phantom tag / can't map the enum to codes).
- **`-mcp` adopts it and retires `CurrencyCode`.** The tool inputs take core
  `Currency` directly: deserialization (via `from_code`, friendly "unknown ISO 4217
  code" error) and the `JsonSchema` code-`enum` now both come from the core. The
  `CurrencyCode` newtype and the call-time `resolve_currency` string-resolution are
  deleted; an unknown code is now rejected at **parameter deserialization** (a tool
  result with `isError`), still with the friendly message.

**`no_std` is preserved and verified** by a CI clippy check
(`--no-default-features --features schemars`); `--all-features` covers the rest.

## Consequences

- The core offers `JsonSchema` for its wire types as a first-class, opt-in
  capability, paired with the serde format it describes.
- `-mcp` drops the `CurrencyCode` newtype (~30 lines) and its call-time
  resolution; the currency input schema is now core-owned.
- Unknown currency codes fail one step earlier (at deserialize, not execution);
  the transport shape of that error changes (a `isError` tool result rather than a
  JSON-RPC `error`), but the friendly message is unchanged.
- New optional dependency surface behind the feature (~7-8 crates: `serde_json`,
  `dyn-clone`, `ref-cast`, `schemars_derive`, …), and a new CI configuration
  (`no_std + schemars`).
- The `*Wire` structs are now shared by both the serde and schemars impls
  (`src/wire.rs`), which is what keeps the two descriptions of one wire format in
  step.

## Alternatives considered

- **Keep the `CurrencyCode` workaround** — the cheaper side of the trade if the
  *only* goal were deleting ~30 binary lines. Rejected because the owner wants
  first-class JSON-Schema for the wire types; retiring the newtype is then a free
  side effect.
- **`schemars` on `Currency` only** (minimum to retire `CurrencyCode`) — leaves the
  other wire types without schemas, an odd gap next to the full serde coverage.
  Rejected; cover the same set as serde.
- **Duplicate wire structs in the schemars impl** — simpler module boundaries, but
  two hand-kept copies of the wire shape invite drift between the serde and
  schemars descriptions. Rejected; share `src/wire.rs`.
- **`serde_json` as a direct core dep** to build the `Currency` enum schema —
  unnecessary: schemars' `json_schema!` macro builds a `Schema` from a JSON literal
  (with interpolation) using its own bundled `serde_json`. Rejected the extra dep.
- **Adopt schemars in the CLI too** — the CLI has no schema surface (it uses clap
  `value_parser`), so there is nothing to adopt. Out of scope.
