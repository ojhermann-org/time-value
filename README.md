# time-value

Type-safe time-value-of-money (TVM) calculations in Rust — a core library, a
command-line interface, and a Model Context Protocol server.

The headline goal: make TVM mistakes — applying an annual rate to monthly
cashflows, discounting with an economically meaningless rate — into *compile
errors*, while keeping the common path ergonomic.

## Workspace

| Crate | Kind | Description |
|-------|------|-------------|
| [`time_value`](crates/time_value) | library (`no_std`) | The TVM calculations. Published on crates.io. |
| [`time_value-cli`](crates/time_value-cli) | binary `time-value` | Command-line interface over the library. |
| [`time_value-mcp`](crates/time_value-mcp) | binary `time-value-mcp` | MCP server exposing the calculations as tools. |

Dependencies point one way, toward the library; the binaries depend on
`time_value` by workspace path (see [ADR-0002](docs/adr/0002-workspace-layout.md)).
Architecture decisions are recorded under [`docs/adr/`](docs/adr).

## Development

This repo is Nix-native. `direnv` activates the dev shell automatically via
`.envrc`; otherwise enter it manually:

```sh
nix develop
```

The dev shell provides the pinned Rust toolchain (see `rust-toolchain.toml`),
[`bacon`](https://dystroy.org/bacon/), `cargo-nextest`, `cargo-deny`, and
`nixfmt`, and installs the [git-hooks.nix](https://github.com/cachix/git-hooks.nix)
managed pre-commit hooks on entry.

Run the checks exactly as CI does — every check goes through the flake, so there
is no second source of truth for the toolchain:

```sh
nix develop -c cargo fmt --all -- --check
nix develop -c cargo clippy --workspace --all-targets --all-features -- -D warnings
nix develop -c cargo nextest run --workspace
nix develop -c cargo deny check
```

`bacon` runs the workspace `clippy` job by default (see `bacon.toml`); a project
Zellij layout wires it into a dedicated pane:

```sh
zellij --layout .zellij/layout.kdl
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this workspace by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
