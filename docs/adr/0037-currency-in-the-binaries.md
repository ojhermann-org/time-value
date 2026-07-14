# ADR-0037: Currency in the binaries — an opt-in code that is echoed, not rounded

- **Status:** Accepted
- **Date:** 2026-07-14
- **Deciders:** Project owner
- **Follows:** [ADR-0034](0034-money-and-currency.md) (money and currency),
  [ADR-0028](0028-binary-surface-conventions.md) (binary surface conventions)

## Context

ADR-0034 put currency on `Money` as a runtime [`Currency`](../../crates/time_value/src/currency.rs)
and noted that "the binaries resolve a currency string against the closed set and
reject unknown codes". The core change landed currency-blind binaries (every amount
`Money::agnostic`, i.e. `XXX`), deferring the user-facing surface. This ADR fixes how
the CLI and MCP *take* a currency and *present* it, so the binaries expose the axis
the core now models.

The forces: keep today's plain-number behaviour for callers who do not ask for a
currency (backward compatibility); give the flag a *visible* effect (a
single-currency invocation never mismatches, so validation alone would be inert);
and honour the crate's approximate-real precision contract (ADR-0033) rather than
quietly rounding results.

## Decision

**One currency per invocation, opt-in, defaulting to `XXX`.** The core enforces one
currency per computation (ADR-0034), so a single currency for the whole invocation
is the natural unit — not a per-cashflow currency.

- **CLI:** a global `--currency <CODE>` flag (`default_value = "XXX"`), parsed by a
  clap `value_parser` calling [`Currency::from_code`]; an unknown code is a parse
  error. It denominates every amount in the invocation.
- **MCP:** an optional `currency: Option<String>` field on each amount-bearing tool's
  input struct (the rate-only tools have none), resolved by `from_code`; `None` is
  `XXX`. The core is left **serde-free** — the field is a `String` in the binary, so
  serde on `Currency`/`Money` (issue #21) stays an independent decision.

**Results echo the code at full precision; they are not rounded.** A monetary result
carries its currency, and a non-`XXX` code is shown alongside the (full-`f64`) value;
a rate or period result carries no currency. Presentation rounding
(`Money::round_to_currency`) is **not** applied — the headline number keeps its
precision, honouring ADR-0033. Rounding stays an explicit future opt-in.

- CLI plain: `18.2237 USD`; CLI JSON and MCP: an added `"currency"` field. `amortize`
  echoes the code once — a `# currency: USD` comment line (table) or a top-level
  `currency` field (JSON) — leaving the dense numeric rows bare.
- Omitting the currency (`XXX`) reproduces the pre-currency output **byte-for-byte**,
  so existing behaviour and tests are unchanged.

**FX is not surfaced in the binaries here.** `FxRate`/`Money::convert` stay core-only
for this step; a `convert` command/tool is a separate, additive follow-up.

## Consequences

- The binaries expose currency without a breaking change to their default output:
  the feature is entirely opt-in behind `--currency` / the `currency` field.
- Unknown codes fail fast with a clear message at both surfaces.
- The core gains no new dependency; issue #21 (serde) remains free to decide the
  serialized `Money` shape on its own terms.
- The `rate` family (no monetary amounts) ignores the currency, as it should.

## Alternatives considered

- **Validate-only (no echo)** — the flag would have almost no visible effect, since a
  single-currency invocation never mismatches. Rejected as inert.
- **Round results to the minor unit** — "money-like" output, but it bakes presentation
  rounding into every headline figure and advertises a precision the transcendental
  engine does not have (ADR-0033). Rejected; rounding stays opt-in.
- **Per-cashflow currency** — needless: the core is single-currency per computation,
  and mixed-currency series are a `CurrencyMismatch` by design.
- **`Deserialize`/`JsonSchema` on `Currency` now** — cleaner MCP structs, but it
  front-runs issue #21 and stamps a schemars shape onto the core enum. Deferred; a
  `String` + `from_code` is self-contained in the binary.

[`Currency::from_code`]: ../../crates/time_value/src/currency.rs
