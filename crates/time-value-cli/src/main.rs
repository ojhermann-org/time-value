//! `time-value` — the command-line interface for the [`time_value`] library.
//!
//! A thin calculator over the library's operations (see
//! `docs/adr/0010-cli-surface.md`, extended by `docs/adr/0028-binary-surface-conventions.md`).
//! Cashflow-series operations live under the `series` group (`npv`, `nfv`, `irr`,
//! `mirr`, and the dated `xnpv` / `xirr`); single-sum `pv`/`fv` and the `annuity`
//! subcommands round it out. `--rate` is a per-period rate (annual for the dated
//! `series xnpv`/`xirr`); cashflows are positional. Results print as a plain
//! number, or as JSON with `--json`.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use time_value::{
    annuity, single_sum, Annual, Cashflows, DatedCashflow, DatedCashflows, Money, Monthly, Period,
    Rate,
};

/// Type-safe time-value-of-money calculations.
#[derive(Parser)]
#[command(name = "time-value", version, about)]
struct Cli {
    /// Print the result as a one-field JSON object instead of a plain number.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Cashflow-series operations: net present/future value, IRR, MIRR, and the
    /// dated XNPV/XIRR.
    Series {
        #[command(subcommand)]
        command: SeriesCommand,
    },
    /// Present value of a single future amount.
    Pv {
        /// Per-period discount rate.
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        /// Number of periods (may be fractional).
        #[arg(long, allow_hyphen_values = true)]
        periods: f64,
        /// The future amount to discount.
        #[arg(long, allow_hyphen_values = true)]
        future: f64,
    },
    /// Future value of a single present amount.
    Fv {
        /// Per-period rate.
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        /// Number of periods (may be fractional).
        #[arg(long, allow_hyphen_values = true)]
        periods: f64,
        /// The present amount to compound.
        #[arg(long, allow_hyphen_values = true)]
        present: f64,
    },
    /// Ordinary (end-of-period) annuity calculations.
    Annuity {
        #[command(subcommand)]
        command: AnnuityCommand,
    },
}

#[derive(Subcommand)]
enum SeriesCommand {
    /// Net present value of a cashflow series discounted at a per-period rate.
    Npv {
        /// Per-period discount rate (e.g. 0.01 for 1% per period).
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        /// Cashflows at periods 0, 1, 2, … (signed: outflow negative).
        #[arg(value_name = "CASHFLOW", allow_hyphen_values = true, num_args = 1.., required = true)]
        cashflows: Vec<f64>,
    },
    /// Net future value of a cashflow series compounded at a per-period rate.
    Nfv {
        /// Per-period rate.
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        /// Cashflows at periods 0, 1, 2, … (signed: outflow negative).
        #[arg(value_name = "CASHFLOW", allow_hyphen_values = true, num_args = 1.., required = true)]
        cashflows: Vec<f64>,
    },
    /// Internal rate of return (per period) of a cashflow series.
    Irr {
        /// Initial guess for the Newton–Raphson solve.
        #[arg(long, default_value_t = 0.1, allow_hyphen_values = true)]
        guess: f64,
        /// Cashflows at periods 0, 1, 2, … (signed: outflow negative).
        #[arg(value_name = "CASHFLOW", allow_hyphen_values = true, num_args = 1.., required = true)]
        cashflows: Vec<f64>,
    },
    /// Modified internal rate of return of a cashflow series.
    Mirr {
        /// Per-period finance rate for discounting the outflows.
        #[arg(long, allow_hyphen_values = true)]
        finance: f64,
        /// Per-period reinvestment rate for compounding the inflows.
        #[arg(long, allow_hyphen_values = true)]
        reinvest: f64,
        /// Cashflows at periods 0, 1, 2, … (signed: outflow negative).
        #[arg(value_name = "CASHFLOW", allow_hyphen_values = true, num_args = 1.., required = true)]
        cashflows: Vec<f64>,
    },
    /// Net present value of cashflows on irregular dates, discounted at an annual
    /// rate (XNPV). Flows are `DATE:AMOUNT` pairs, e.g. `2020-01-01:-1000`.
    Xnpv {
        /// Annual discount rate (e.g. 0.1 for 10% per year).
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        /// Dated cashflows as `YYYY-MM-DD:AMOUNT` (signed: outflow negative). The
        /// first date is the valuation reference.
        #[arg(value_name = "DATE:AMOUNT", num_args = 1.., required = true)]
        flows: Vec<String>,
    },
    /// Internal rate of return of cashflows on irregular dates (XIRR), as an
    /// annual rate. Flows are `DATE:AMOUNT` pairs.
    Xirr {
        /// Initial guess for the Newton–Raphson solve (annual).
        #[arg(long, default_value_t = 0.1, allow_hyphen_values = true)]
        guess: f64,
        /// Dated cashflows as `YYYY-MM-DD:AMOUNT` (signed: outflow negative). The
        /// first date is the valuation reference.
        #[arg(value_name = "DATE:AMOUNT", num_args = 1.., required = true)]
        flows: Vec<String>,
    },
}

#[derive(Subcommand)]
enum AnnuityCommand {
    /// Present value of an annuity paying a fixed amount each period.
    Pv {
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        #[arg(long, allow_hyphen_values = true)]
        periods: f64,
        /// The payment made at the end of each period.
        #[arg(long, allow_hyphen_values = true)]
        payment: f64,
    },
    /// Future value of an annuity paying a fixed amount each period.
    Fv {
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        #[arg(long, allow_hyphen_values = true)]
        periods: f64,
        /// The payment made at the end of each period.
        #[arg(long, allow_hyphen_values = true)]
        payment: f64,
    },
    /// Level payment that amortises a present value over the periods.
    Payment {
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        #[arg(long, allow_hyphen_values = true)]
        periods: f64,
        /// The present value to amortise.
        #[arg(long, allow_hyphen_values = true)]
        present: f64,
    },
}

// The periodicity tag does not affect any result for the period-indexed series
// operations (ADR-0010); the CLI fixes it to one marker to satisfy the type
// parameter. The dated `xnpv`/`xirr` are intrinsically annual (ADR-0029).
type Per = Monthly;

fn rate(value: f64) -> Result<Rate<Per>> {
    Rate::new(value).context("invalid rate (must be finite and greater than -100%)")
}

fn annual_rate(value: f64) -> Result<Rate<Annual>> {
    Rate::new(value).context("invalid rate (must be finite and greater than -100%)")
}

fn period(value: f64) -> Result<Period> {
    Period::new(value).context("invalid period count (must be finite and non-negative)")
}

fn money(value: f64) -> Result<Money> {
    Money::new(value).context("invalid amount (must be finite)")
}

fn cashflows(values: &[f64]) -> Result<Vec<Money>> {
    values.iter().copied().map(money).collect()
}

// ---- Dated flows (XNPV/XIRR): ISO dates → ACT/365 year-offsets ----
//
// The core takes year-offsets, not a date type (ADR-0029); the CLI accepts real
// `YYYY-MM-DD` dates and converts them here with a self-contained ACT/365
// day-count, so no date dependency reaches the binary.

/// Days since the epoch (proleptic Gregorian) via Howard Hinnant's
/// days-from-civil algorithm. `month` is 1..=12, `day` valid for the month.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let year_of_era = y - era * 400; // [0, 399]
    let month_index = (month + 9) % 12; // Mar = 0 … Feb = 11
    let day_of_year = (153 * month_index + 2) / 5 + day - 1; // [0, 365]
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Parse an ISO `YYYY-MM-DD` date to a day number.
fn parse_date(text: &str) -> Result<i64> {
    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() != 3 {
        bail!("invalid date `{text}` (expected YYYY-MM-DD)");
    }
    let year: i64 = parts[0]
        .parse()
        .with_context(|| format!("invalid year in date `{text}`"))?;
    let month: i64 = parts[1]
        .parse()
        .with_context(|| format!("invalid month in date `{text}`"))?;
    let day: i64 = parts[2]
        .parse()
        .with_context(|| format!("invalid day in date `{text}`"))?;
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        bail!("invalid date `{text}` (month 1-12, day valid for the month)");
    }
    Ok(days_from_civil(year, month, day))
}

/// Parse `DATE:AMOUNT` pairs into dated cashflows, converting each ISO date to a
/// year-offset from the first flow (ACT/365).
fn dated_flows(pairs: &[String]) -> Result<Vec<DatedCashflow>> {
    let mut flows = Vec::with_capacity(pairs.len());
    let mut reference: Option<i64> = None;
    for pair in pairs {
        let (date, amount) = pair.split_once(':').with_context(|| {
            format!("invalid flow `{pair}` (expected DATE:AMOUNT, e.g. 2020-01-01:-1000)")
        })?;
        let day = parse_date(date)?;
        let reference = *reference.get_or_insert(day);
        let amount: f64 = amount
            .parse()
            .with_context(|| format!("invalid amount in flow `{pair}`"))?;
        // Day-count differences for real calendar dates are far below 2^53, so
        // this conversion is exact despite the lint's worst-case warning.
        #[allow(clippy::cast_precision_loss)]
        let offset_years = (day - reference) as f64 / 365.0;
        flows.push(DatedCashflow::new(offset_years, money(amount)?)?);
    }
    Ok(flows)
}

/// Dispatch the `series` subcommands to the library, returning the JSON label and
/// the result value.
fn run_series(command: SeriesCommand) -> Result<(&'static str, f64)> {
    Ok(match command {
        SeriesCommand::Npv {
            rate: r,
            cashflows: cf,
        } => {
            let flows = cashflows(&cf)?;
            let series = Cashflows::<Per>::new(&flows);
            ("npv", series.net_present_value(rate(r)?)?.value())
        }
        SeriesCommand::Nfv {
            rate: r,
            cashflows: cf,
        } => {
            let flows = cashflows(&cf)?;
            let series = Cashflows::<Per>::new(&flows);
            ("nfv", series.net_future_value(rate(r)?)?.value())
        }
        SeriesCommand::Irr {
            guess,
            cashflows: cf,
        } => {
            let flows = cashflows(&cf)?;
            let series = Cashflows::<Per>::new(&flows);
            let irr = series
                .internal_rate_of_return_from(guess)
                .context("no internal rate of return found")?;
            ("irr", irr.value())
        }
        SeriesCommand::Mirr {
            finance,
            reinvest,
            cashflows: cf,
        } => {
            let flows = cashflows(&cf)?;
            let series = Cashflows::<Per>::new(&flows);
            let mirr = series
                .modified_internal_rate_of_return(rate(finance)?, rate(reinvest)?)
                .context("modified internal rate of return is undefined")?;
            ("mirr", mirr.value())
        }
        SeriesCommand::Xnpv { rate: r, flows } => {
            let dated = dated_flows(&flows)?;
            let series = DatedCashflows::new(&dated);
            ("xnpv", series.net_present_value(annual_rate(r)?)?.value())
        }
        SeriesCommand::Xirr { guess, flows } => {
            let dated = dated_flows(&flows)?;
            let series = DatedCashflows::new(&dated);
            let irr = series
                .internal_rate_of_return_from(guess)
                .context("no internal rate of return found")?;
            ("xirr", irr.value())
        }
    })
}

fn run(cli: Cli) -> Result<()> {
    let (label, value) = match cli.command {
        Command::Series { command } => run_series(command)?,
        Command::Pv {
            rate: r,
            periods: n,
            future,
        } => (
            "pv",
            single_sum::present_value(rate(r)?, period(n)?, money(future)?)?.value(),
        ),
        Command::Fv {
            rate: r,
            periods: n,
            present,
        } => (
            "fv",
            single_sum::future_value(rate(r)?, period(n)?, money(present)?)?.value(),
        ),
        Command::Annuity { command } => match command {
            AnnuityCommand::Pv {
                rate: r,
                periods: n,
                payment,
            } => (
                "annuity_pv",
                annuity::present_value(rate(r)?, period(n)?, money(payment)?)?.value(),
            ),
            AnnuityCommand::Fv {
                rate: r,
                periods: n,
                payment,
            } => (
                "annuity_fv",
                annuity::future_value(rate(r)?, period(n)?, money(payment)?)?.value(),
            ),
            AnnuityCommand::Payment {
                rate: r,
                periods: n,
                present,
            } => {
                let pmt = annuity::payment(rate(r)?, period(n)?, money(present)?)
                    .context("annuity payment is undefined (e.g. zero periods)")?;
                ("annuity_payment", pmt.value())
            }
        },
    };

    if cli.json {
        let mut object = serde_json::Map::new();
        object.insert(label.to_owned(), serde_json::json!(value));
        println!("{}", serde_json::Value::Object(object));
    } else {
        println!("{value}");
    }
    Ok(())
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
