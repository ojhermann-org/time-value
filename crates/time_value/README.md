# time_value

Type-safe time-value-of-money (TVM) calculations in Rust.

A deliberately type-heavy redesign of [`time_value`](https://crates.io/crates/time_value),
rebuilt from scratch for the `1.0` line. The goal: make TVM mistakes — applying
an annual rate to monthly cashflows, discounting with an economically
meaningless rate — into *compile errors*, while keeping the common path
ergonomic. `#![no_std]` and dependency-free by default.

> **Status:** `1.0.0` is under active design. The public API is being built up
> incrementally — this is the early scaffolding.

## Features

| Feature | Default | Effect |
|---------|:-------:|--------|
| `std`   |    no   | Use `std` for transcendental math and error impls. |
| `libm`  |    no   | `no_std` transcendental functions (`powf`/`ln`/`exp`) via [`libm`], for IRR and continuous compounding without `std`. |
| `serde` |    no   | Derive `Serialize`/`Deserialize` on the domain newtypes. |

[`libm`]: https://crates.io/crates/libm

This crate is the library at the core of the [`time-value`] workspace, which also
provides a CLI (`time-value`) and an MCP server (`time-value-mcp`). See the
workspace README for development setup.

[`time-value`]: https://github.com/ojhermann-org/time-value

## License

Licensed under either of [Apache License, Version 2.0](../../LICENSE-APACHE) or
[MIT license](../../LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
