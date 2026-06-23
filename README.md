# time_value

Type-safe time-value-of-money (TVM) calculations in Rust.

A deliberately type-heavy redesign of [`time_value`](https://crates.io/crates/time_value),
rebuilt from scratch for the `1.0` line. The goal: make TVM mistakes — applying
an annual rate to monthly cashflows, discounting with an economically
meaningless rate — into *compile errors*, while keeping the common path
ergonomic. `#![no_std]` and dependency-free.

> **Status:** `1.0.0` is under active design. The public API is being built up
> incrementally — this is the early scaffolding.

## Development

This repo is Nix-native. `direnv` activates the dev shell automatically via
`.envrc`; otherwise enter it manually:

```sh
nix develop
```

The dev shell provides the pinned Rust toolchain (see `rust-toolchain.toml`),
[`bacon`](https://dystroy.org/bacon/), `cargo-nextest`, and `nixfmt`, and
installs the [git-hooks.nix](https://github.com/cachix/git-hooks.nix) managed
pre-commit hooks on entry.

Run the full check suite exactly as CI does:

```sh
nix flake check
```

This runs `crane` build/clippy/test/doc/fmt plus the pre-commit hooks. There is
no `prek` — all checks are native Nix flake checks.

### bacon

`bacon` runs the `clippy` job by default (see `bacon.toml`), matching the
`clippy` flake check. A project Zellij layout wires it into a dedicated pane:

```sh
zellij --layout .zellij/layout.kdl
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
