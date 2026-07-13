# time-value-cli

The command-line interface for [`time_value`](../time_value) — type-safe
time-value-of-money calculations from your shell. Installs the `time-value`
binary. Its surface is designed in [ADR-0010](../../docs/adr/0010-cli-surface.md).

## Install

Not yet published to crates.io (see [ADR-0012](../../docs/adr/0012-ci-and-release-automation.md)).
From a checkout of the repository:

```sh
cargo install --path crates/time-value-cli   # installs the `time-value` binary
```

## Usage

`--rate` is a **per-period** rate (an **annual** rate for the dated
`series xnpv`/`xirr`); cashflows are positional (period 0 first, outflows
negative). Results print as a plain number, or as JSON with `--json`.

```sh
# Cashflow series: net present/future value, IRR, MIRR
time-value series npv --rate 0.01 -100 60 60      # 18.2237…
time-value series nfv --rate 0.01 -100 60 60
time-value series irr -100 60 60                  # 0.1307… per period
time-value series mirr --finance 0.10 --reinvest 0.12 -1000 -500 800 900

# Dated (irregular) cashflows — XNPV/XIRR at an annual rate, DATE:AMOUNT pairs
time-value series xirr 2008-01-01:-10000 2008-03-01:2750 \
                       2008-10-30:4250 2009-02-15:3250 2009-04-01:2750   # 0.3734…
time-value series xnpv --rate 0.10 2020-01-01:-100 2021-01-01:110

# Single-sum present / future value
time-value pv --rate 0.01 --periods 12 --future 1000    # 887.45
time-value fv --rate 0.01 --periods 12 --present 1000   # 1126.83

# Ordinary annuities
time-value annuity pv      --rate 0.01 --periods 12 --payment 100
time-value annuity fv      --rate 0.01 --periods 12 --payment 100
time-value annuity payment --rate 0.01 --periods 12 --present 1125.51

# JSON output for scripting
time-value --json series npv --rate 0.01 -100 60 60    # {"npv":18.2237…}
```

## License

Dual-licensed under [Apache-2.0](../../LICENSE-APACHE) or [MIT](../../LICENSE-MIT),
at your option.
