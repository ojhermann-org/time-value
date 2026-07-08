# time_value-cli

The command-line interface for [`time_value`](../time_value) — type-safe
time-value-of-money calculations from your shell. Installs the `time-value`
binary. Its surface is designed in [ADR-0010](../../docs/adr/0010-cli-surface.md).

## Usage

`--rate` is a **per-period** rate; cashflows are positional (period 0 first,
outflows negative). Results print as a plain number, or as JSON with `--json`.

```sh
# Net present value / net future value / internal rate of return of a series
time-value npv --rate 0.01 -100 60 60      # 18.2237…
time-value nfv --rate 0.01 -100 60 60
time-value irr -100 60 60                  # 0.1307… per period

# Single-sum present / future value
time-value pv --rate 0.01 --periods 12 --future 1000    # 887.45
time-value fv --rate 0.01 --periods 12 --present 1000   # 1126.83

# Ordinary annuities
time-value annuity pv      --rate 0.01 --periods 12 --payment 100
time-value annuity fv      --rate 0.01 --periods 12 --payment 100
time-value annuity payment --rate 0.01 --periods 12 --present 1125.51

# JSON output for scripting
time-value --json npv --rate 0.01 -100 60 60    # {"npv":18.2237…}
```

## License

Dual-licensed under [Apache-2.0](../../LICENSE-APACHE) or [MIT](../../LICENSE-MIT),
at your option.
