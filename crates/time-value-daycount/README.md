# time-value-daycount

Internal ACT/365 day-count for the [`time-value`](../..) binaries. **Not
published** — a private workspace support crate shared by the `time-value` CLI
and `time-value-mcp` server.

The core [`time_value`](../time_value) library takes year-offsets, not a date
type ([ADR-0029](../../docs/adr/0029-dated-cashflows-xnpv-xirr.md)). The binaries
accept real ISO `YYYY-MM-DD` dates and convert them to year-offsets for the
dated-cashflow operations (XNPV/XIRR). This crate is the single home for that
conversion so it is defined once rather than duplicated in each binary
([ADR-0030](../../docs/adr/0030-shared-day-count-support-crate.md)).

It is dependency-free: the calendar arithmetic is Howard Hinnant's
days-from-civil algorithm, so no date/time crate reaches the binaries.

## API

- `iso_to_day(&str) -> Result<i64, ParseDateError>` — parse and validate an ISO
  date into a serial day number (days since the Unix epoch, proleptic
  Gregorian).
- `act365_year_fraction(reference, day) -> f64` — the ACT/365 year-fraction
  `(day − reference) / 365` between two serial day numbers.
- `ParseDateError` — a self-contained parse error each binary maps into its own
  error type via `Display`.
