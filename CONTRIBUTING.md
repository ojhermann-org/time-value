# Contributing to time-value

Thanks for your interest! `time-value` is a type-safe time-value-of-money
library for Rust, plus a CLI (`time-value`) and an MCP server
(`time-value-mcp`). Bug reports, questions, and pull requests are all welcome.

By contributing, you agree that your contributions are licensed under the
project's dual **MIT OR Apache-2.0** license (see
[ADR-0006](docs/adr/0006-license.md) and `LICENSE-MIT` / `LICENSE-APACHE`).

## Getting set up

A [Nix](https://nixos.org/) flake is the reproducible source of truth for the
toolchain ([ADR-0008](docs/adr/0008-nix-flake-dev-environment.md)). Entering the
dev shell provides the pinned Rust toolchain, `bacon`, `cargo-nextest`,
`cargo-deny`, and `nixfmt`, and installs the [git-hooks.nix](https://github.com/cachix/git-hooks.nix)
pre-commit hooks:

```sh
nix develop        # enter the dev shell
# or, with direnv: `direnv allow` once, then it loads automatically
```

A plain `rustup` toolchain can work too, but you are then responsible for
matching `rust-toolchain.toml` and supplying `cargo-nextest` / `cargo-deny`
yourself; the flake exists so you don't have to.

While you work, `bacon` runs the workspace `clippy` job continuously:

```sh
bacon                                  # or: zellij --layout .zellij/layout.kdl
```

## The gate

CI runs every check **through the flake** (`nix develop -c cargo …`), so a push
that passes locally passes CI — there is no second definition of the toolchain.
Inside the dev shell, the checks are:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings                                   # no_std default
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo clippy -p time_value --no-default-features --features libm --all-targets -- -D warnings  # no_std + libm
cargo nextest run --workspace --all-features
cargo test --doc --workspace --all-features                                             # doctests
cargo deny check                                                                        # licenses, advisories, bans
```

The core `time_value` library keeps a conservative **MSRV of 1.85**, below the
workspace's 1.88 toolchain (which the MCP crate's dependencies force). Verify it
on the pinned 1.85 toolchain — this is a CI gate too
([ADR-0017](docs/adr/0017-per-crate-msrv-core-1.85.md)):

```sh
nix develop .#msrv -c cargo test -p time_value --all-features
```

The required status check is the `ci` job; keep it green.

## Working on the code

The three crates form a one-way stack ([ADR-0002](docs/adr/0002-workspace-layout.md)):
the `time_value` library is the product; the CLI and MCP server are thin surfaces
over it. A new operation is a **vertical slice** — the library first, then (where
it makes sense) the CLI subcommand and the MCP tool:

- **`time_value` (library)** — the calculation, with unit tests and doctests.
  The core is `#![no_std]` and dependency-free by default: keep it that way, and
  remember that some `f64` methods (`abs`, `mul_add`, `powf`, …) are **not** in
  `core`. Transcendental math goes through the `std`/`libm`-gated `math` module
  ([ADR-0009](docs/adr/0009-no_std-and-optional-libm.md),
  [ADR-0014](docs/adr/0014-transcendental-single-sum-operations.md)).
- **`time-value-cli` / `time-value-mcp` (binaries)** — mirror the library op as a
  subcommand ([ADR-0010](docs/adr/0010-cli-surface.md)) or a tool
  ([ADR-0011](docs/adr/0011-mcp-server.md)), with `assert_cmd` integration tests
  that drive the compiled binary. Async stays contained to `-mcp`
  ([ADR-0003](docs/adr/0003-synchronous-computation-model.md)).

**Significant design choices get an ADR:** copy
[`docs/adr/0000-adr-template.md`](docs/adr/0000-adr-template.md), take the next
number, fill in Context → Decision → Consequences → Alternatives, and add a row
to [the index](docs/adr/README.md). An accepted ADR is immutable — supersede it
with a new one rather than editing it.

## Commit messages

Commits follow [Conventional Commits](https://www.conventionalcommits.org/) —
they are load-bearing: [`release-plz`](https://release-plz.dev/) derives each
crate's version bump and changelog from them
([ADR-0012](docs/adr/0012-ci-and-release-automation.md)).

Use `feat`, `fix`, `docs`, `refactor`, or `chore`; a `!` (e.g. `feat!:`) or a
`BREAKING CHANGE:` footer marks a breaking change. Keep each commit to one
logical layer where practical.

## Pull requests

1. Branch off `main`. Branch names must match `^(feat|fix|chore|docs|refactor)/.*`
   (a repository ruleset enforces this).
2. Make your change with tests; keep the gate green.
3. Open a PR against `main`, and squash-merge once the `ci` check passes.

`main` is protected — changes land through PRs with a linear history. Please be
respectful and constructive in issues and reviews: assume good faith and keep
discussion focused on the work.
