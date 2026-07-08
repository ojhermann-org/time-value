//! `time-value` — the command-line interface for the [`time_value`] library.
//!
//! A thin calculator over the library's operations (see
//! `docs/adr/0010-cli-surface.md`): `npv`, `nfv`, `irr`, single-sum `pv`/`fv`,
//! and the `annuity` subcommands. `--rate` is a per-period rate; cashflows are
//! positional. Results print as a plain number, or as JSON with `--json`.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use time_value::{annuity, single_sum, Cashflows, Money, Monthly, Period, Rate};

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

// The periodicity tag does not affect any result (ADR-0010); the CLI fixes it to
// one marker to satisfy the type parameter.
type Per = Monthly;

fn rate(value: f64) -> Result<Rate<Per>> {
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

fn run(cli: Cli) -> Result<()> {
    let (label, value) = match cli.command {
        Command::Npv {
            rate: r,
            cashflows: cf,
        } => {
            let flows = cashflows(&cf)?;
            let series = Cashflows::<Per>::new(&flows);
            ("npv", series.net_present_value(rate(r)?).value())
        }
        Command::Nfv {
            rate: r,
            cashflows: cf,
        } => {
            let flows = cashflows(&cf)?;
            let series = Cashflows::<Per>::new(&flows);
            ("nfv", series.net_future_value(rate(r)?).value())
        }
        Command::Irr {
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
        Command::Pv {
            rate: r,
            periods: n,
            future,
        } => (
            "pv",
            single_sum::present_value(rate(r)?, period(n)?, money(future)?).value(),
        ),
        Command::Fv {
            rate: r,
            periods: n,
            present,
        } => (
            "fv",
            single_sum::future_value(rate(r)?, period(n)?, money(present)?).value(),
        ),
        Command::Annuity { command } => match command {
            AnnuityCommand::Pv {
                rate: r,
                periods: n,
                payment,
            } => (
                "annuity_pv",
                annuity::present_value(rate(r)?, period(n)?, money(payment)?).value(),
            ),
            AnnuityCommand::Fv {
                rate: r,
                periods: n,
                payment,
            } => (
                "annuity_fv",
                annuity::future_value(rate(r)?, period(n)?, money(payment)?).value(),
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
