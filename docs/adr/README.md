# Architecture Decision Records

This directory records the significant design decisions behind `time_value`, in
the lightweight [Nygard format](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions).
The practice itself is [ADR-0001](0001-record-architecture-decisions.md).

## How to add one

1. Copy [`0000-adr-template.md`](0000-adr-template.md) to the next free number,
   `NNNN-kebab-title.md`.
2. Fill in Context → Decision → Consequences → Alternatives considered.
3. Commit it **with the change it describes**.
4. Add a row to the index below.

An Accepted ADR is immutable. To change a decision, write a new ADR that marks
the old one **Superseded** (link both ways) — don't rewrite the old one.

## Index

| # | Title | Status |
|---|-------|--------|
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Accepted |
| [0002](0002-workspace-layout.md) | Workspace layout & crate boundaries | Accepted |
| [0003](0003-synchronous-computation-model.md) | Synchronous computation model | Accepted |
| [0004](0004-error-handling.md) | Error handling | Accepted |
| [0005](0005-domain-modelling-and-strong-typing.md) | Domain modelling & strong typing | Accepted |
| [0006](0006-license.md) | License | Accepted |
| [0007](0007-rust-edition-and-msrv.md) | Rust edition & MSRV | Accepted |
| [0008](0008-nix-flake-dev-environment.md) | Nix flake dev environment | Accepted |
| [0009](0009-no_std-and-optional-libm.md) | `no_std` core & optional `libm` | Accepted |
| [0012](0012-ci-and-release-automation.md) | CI and release automation | Accepted |
| [0013](0013-core-api-values-and-discrete-operations.md) | Core API — values, cashflows & discrete operations | Accepted |
| [0014](0014-transcendental-single-sum-operations.md) | Transcendental operations behind `std`/`libm` — single-sum value | Accepted |
| [0015](0015-annuities.md) | Annuities — convention, the `r → 0` limit, and a fallible payment | Accepted |

### Planned

These decisions are designed alongside the surfaces they describe; the numbers
are reserved and referenced from the code:

| # | Title |
|---|-------|
| 0010 | CLI surface |
| 0011 | MCP server |
