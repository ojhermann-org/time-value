//! `time-value` — the command-line interface for the [`time_value`] library.
//!
//! A thin calculator over the library's operations (see
//! `docs/adr/0010-cli-surface.md`, extended by `docs/adr/0028-binary-surface-conventions.md`).
//! Commands are grouped by relationship family: `series` (net present/future
//! value, IRR, MIRR, and the dated XNPV/XIRR), `single-sum` (present/future value
//! and the solve-for `nper`/`rate` inverses), `annuity` (ordinary, annuity-`due`,
//! and perpetuity forms, plus the `nper`/`rate` solves), `rate` (conversions
//! between periodicities and nominal/effective quotes — the only family that
//! takes a periodicity), and `amortize` (a schedule). `--rate` is a per-period
//! rate (annual for the dated `series xnpv`/`xirr`); cashflows are positional.
//! Most results print as a plain number, or as JSON with `--json`; `amortize`
//! prints a table, or a JSON array of row objects under `--json`.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use time_value::{
    amortization, annuity, single_sum, Annual, Cashflows, DatedCashflow, DatedCashflows, Money,
    Monthly, Period, Rate,
};
use time_value_daycount::{act365_year_fraction, iso_to_day};

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
    /// Single-sum operations: present/future value and the solve-for inverses.
    SingleSum {
        #[command(subcommand)]
        command: SingleSumCommand,
    },
    /// Annuity operations: ordinary, annuity-due, perpetuity, and the solves.
    Annuity {
        #[command(subcommand)]
        command: AnnuityCommand,
    },
    /// Rate conversions: effective-annual, between periodicities, and nominal.
    Rate {
        #[command(subcommand)]
        command: RateCommand,
    },
    /// Amortization schedule: one row per period until the balance is retired.
    Amortize {
        /// Per-period rate.
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        /// The principal to amortise.
        #[arg(long, allow_hyphen_values = true)]
        principal: f64,
        /// Amortise over this many periods (mutually exclusive with --payment).
        #[arg(long, allow_hyphen_values = true)]
        periods: Option<f64>,
        /// Amortise with this level payment (mutually exclusive with --periods).
        #[arg(long, allow_hyphen_values = true)]
        payment: Option<f64>,
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
enum SingleSumCommand {
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
    /// Solve for the number of periods that grows a present to a future amount.
    Nper {
        /// Per-period rate.
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        /// The present amount.
        #[arg(long, allow_hyphen_values = true)]
        present: f64,
        /// The future amount.
        #[arg(long, allow_hyphen_values = true)]
        future: f64,
    },
    /// Solve for the per-period rate that grows a present to a future amount.
    Rate {
        /// Number of periods (may be fractional).
        #[arg(long, allow_hyphen_values = true)]
        periods: f64,
        /// The present amount.
        #[arg(long, allow_hyphen_values = true)]
        present: f64,
        /// The future amount.
        #[arg(long, allow_hyphen_values = true)]
        future: f64,
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
    /// Solve for the number of level payments, from a present or future value.
    Nper {
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        /// The payment made at the end of each period.
        #[arg(long, allow_hyphen_values = true)]
        payment: f64,
        /// Solve from this present value (mutually exclusive with --future).
        #[arg(long, allow_hyphen_values = true)]
        present: Option<f64>,
        /// Solve from this future value (mutually exclusive with --present).
        #[arg(long, allow_hyphen_values = true)]
        future: Option<f64>,
    },
    /// Solve for the per-period rate of an annuity, from a present or future value.
    Rate {
        #[arg(long, allow_hyphen_values = true)]
        periods: f64,
        /// The payment made at the end of each period.
        #[arg(long, allow_hyphen_values = true)]
        payment: f64,
        /// Solve from this present value (mutually exclusive with --future).
        #[arg(long, allow_hyphen_values = true)]
        present: Option<f64>,
        /// Solve from this future value (mutually exclusive with --present).
        #[arg(long, allow_hyphen_values = true)]
        future: Option<f64>,
    },
    /// Present value of a level perpetuity (a payment forever).
    Perpetuity {
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        /// The payment made at the end of each period, forever.
        #[arg(long, allow_hyphen_values = true)]
        payment: f64,
    },
    /// Present value of a perpetuity whose payment grows each period.
    GrowingPerpetuity {
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        /// The per-period growth rate of the payment (must be below --rate).
        #[arg(long, allow_hyphen_values = true)]
        growth: f64,
        /// The first payment (at the end of period 1).
        #[arg(long, allow_hyphen_values = true)]
        payment: f64,
    },
    /// Annuity-due (start-of-period payment) calculations.
    Due {
        #[command(subcommand)]
        command: AnnuityDueCommand,
    },
}

#[derive(Subcommand)]
enum AnnuityDueCommand {
    /// Present value of an annuity-due paying a fixed amount each period.
    Pv {
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        #[arg(long, allow_hyphen_values = true)]
        periods: f64,
        /// The payment made at the start of each period.
        #[arg(long, allow_hyphen_values = true)]
        payment: f64,
    },
    /// Future value of an annuity-due paying a fixed amount each period.
    Fv {
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        #[arg(long, allow_hyphen_values = true)]
        periods: f64,
        /// The payment made at the start of each period.
        #[arg(long, allow_hyphen_values = true)]
        payment: f64,
    },
    /// Level start-of-period payment that amortises a present value.
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

#[derive(Subcommand)]
enum RateCommand {
    /// Effective annual rate (EAR) equivalent to a per-period rate.
    Ear {
        /// The per-period rate.
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        /// The periodicity of the rate (daily, weekly, monthly, quarterly,
        /// semi-annual, annual).
        #[arg(long)]
        periodicity: String,
    },
    /// Convert a per-period rate from one periodicity to another (same EAR).
    Convert {
        /// The per-period rate under `--from`.
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        /// The periodicity the rate is expressed in.
        #[arg(long)]
        from: String,
        /// The periodicity to express the rate in.
        #[arg(long)]
        to: String,
    },
    /// Per-period rate from a nominal annual rate (APR) at a periodicity.
    FromNominal {
        /// The nominal annual rate (APR).
        #[arg(long, allow_hyphen_values = true)]
        nominal: f64,
        /// The compounding periodicity.
        #[arg(long)]
        periodicity: String,
    },
    /// Nominal annual rate (APR) quoted from a per-period rate.
    Nominal {
        /// The per-period rate.
        #[arg(long, allow_hyphen_values = true)]
        rate: f64,
        /// The compounding periodicity.
        #[arg(long)]
        periodicity: String,
    },
}

// The periodicity tag does not affect any result for the period-indexed
// operations (ADR-0010); the CLI fixes it to one marker to satisfy the type
// parameter. The dated `series xnpv`/`xirr` are intrinsically annual (ADR-0029).
// The `rate` conversion group is the exception: periodicity is intrinsic there,
// so it dispatches the runtime name to a marker type (`dispatch_periodicity!`).
type Per = Monthly;

/// Run `$body` with the type alias `$ty` bound to the periodicity marker named by
/// `$name` at runtime; an unknown name is a usage error.
macro_rules! dispatch_periodicity {
    ($name:expr, $ty:ident => $body:expr) => {{
        match $name {
            "daily" => {
                type $ty = time_value::Daily;
                $body
            }
            "weekly" => {
                type $ty = time_value::Weekly;
                $body
            }
            "monthly" => {
                type $ty = time_value::Monthly;
                $body
            }
            "quarterly" => {
                type $ty = time_value::Quarterly;
                $body
            }
            "semi-annual" => {
                type $ty = time_value::SemiAnnual;
                $body
            }
            "annual" => {
                type $ty = time_value::Annual;
                $body
            }
            other => bail!(
                "unknown periodicity `{other}` \
                 (expected daily, weekly, monthly, quarterly, semi-annual, or annual)"
            ),
        }
    }};
}

fn rate(value: f64) -> Result<Rate<Per>> {
    Rate::new(value).context("invalid rate (must be finite and greater than -100%)")
}

fn annual_rate(value: f64) -> Result<Rate<Annual>> {
    Rate::new(value).context("invalid rate (must be finite and greater than -100%)")
}

fn period(value: f64) -> Result<Period<Per>> {
    Period::new(value).context("invalid period count (must be finite and non-negative)")
}

fn money(value: f64) -> Result<Money> {
    Money::agnostic(value).context("invalid amount (must be finite)")
}

fn cashflows(values: &[f64]) -> Result<Vec<Money>> {
    values.iter().copied().map(money).collect()
}

/// The value a solve-for operation is anchored to — exactly one of a present or a
/// future amount. The two `--present`/`--future` flags are mutually exclusive and
/// one is required.
enum Anchor {
    Present(f64),
    Future(f64),
}

fn anchor(present: Option<f64>, future: Option<f64>) -> Result<Anchor> {
    match (present, future) {
        (Some(p), None) => Ok(Anchor::Present(p)),
        (None, Some(f)) => Ok(Anchor::Future(f)),
        (None, None) => bail!("provide either --present or --future"),
        (Some(_), Some(_)) => bail!("--present and --future are mutually exclusive"),
    }
}

// ---- Dated flows (XNPV/XIRR): ISO dates → ACT/365 year-offsets ----
//
// The core takes year-offsets, not a date type (ADR-0029); the CLI accepts real
// `YYYY-MM-DD` dates and converts them with the shared `time-value-daycount`
// ACT/365 day-count (ADR-0030), so no date dependency reaches the binary.

/// Parse `DATE:AMOUNT` pairs into dated cashflows, converting each ISO date to a
/// year-offset from the first flow (ACT/365).
fn dated_flows(pairs: &[String]) -> Result<Vec<DatedCashflow>> {
    let mut flows = Vec::with_capacity(pairs.len());
    let mut reference: Option<i64> = None;
    for pair in pairs {
        let (date, amount) = pair.split_once(':').with_context(|| {
            format!("invalid flow `{pair}` (expected DATE:AMOUNT, e.g. 2020-01-01:-1000)")
        })?;
        let day = iso_to_day(date)?;
        let reference = *reference.get_or_insert(day);
        let amount: f64 = amount
            .parse()
            .with_context(|| format!("invalid amount in flow `{pair}`"))?;
        let offset_years = act365_year_fraction(reference, day);
        flows.push(DatedCashflow::new(offset_years, money(amount)?)?);
    }
    Ok(flows)
}

/// Dispatch the `series` subcommands, returning the JSON label and result value.
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

/// Dispatch the `single-sum` subcommands.
#[allow(clippy::needless_pass_by_value)]
fn run_single_sum(command: SingleSumCommand) -> Result<(&'static str, f64)> {
    Ok(match command {
        SingleSumCommand::Pv {
            rate: r,
            periods: n,
            future,
        } => (
            "single_sum_present_value",
            single_sum::present_value(rate(r)?, period(n)?, money(future)?)?.value(),
        ),
        SingleSumCommand::Fv {
            rate: r,
            periods: n,
            present,
        } => (
            "single_sum_future_value",
            single_sum::future_value(rate(r)?, period(n)?, money(present)?)?.value(),
        ),
        SingleSumCommand::Nper {
            rate: r,
            present,
            future,
        } => {
            let n = single_sum::periods(rate(r)?, money(present)?, money(future)?)
                .context("number of periods is undefined for these inputs")?;
            ("single_sum_periods", n.value())
        }
        SingleSumCommand::Rate {
            periods: n,
            present,
            future,
        } => {
            let r = single_sum::rate::<Per>(period(n)?, money(present)?, money(future)?)
                .context("no rate solves these inputs")?;
            ("single_sum_rate", r.value())
        }
    })
}

/// Dispatch the `annuity` subcommands (ordinary, solves, perpetuities, and due).
// By-value dispatch mirrors the other `run_*` helpers; the arms are all-`Copy`, so
// clippy would rather borrow — but owning the parsed command here is the clearer shape.
#[allow(clippy::needless_pass_by_value)]
fn run_annuity(command: AnnuityCommand) -> Result<(&'static str, f64)> {
    Ok(match command {
        AnnuityCommand::Pv {
            rate: r,
            periods: n,
            payment,
        } => (
            "annuity_present_value",
            annuity::present_value(rate(r)?, period(n)?, money(payment)?)?.value(),
        ),
        AnnuityCommand::Fv {
            rate: r,
            periods: n,
            payment,
        } => (
            "annuity_future_value",
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
        AnnuityCommand::Nper {
            rate: r,
            payment,
            present,
            future,
        } => {
            let n = match anchor(present, future)? {
                Anchor::Present(p) => annuity::periods(rate(r)?, money(payment)?, money(p)?),
                Anchor::Future(f) => {
                    annuity::periods_from_future(rate(r)?, money(payment)?, money(f)?)
                }
            }
            .context("number of periods is undefined for these inputs")?;
            ("annuity_periods", n.value())
        }
        AnnuityCommand::Rate {
            periods: n,
            payment,
            present,
            future,
        } => {
            let r = match anchor(present, future)? {
                Anchor::Present(p) => annuity::rate::<Per>(period(n)?, money(payment)?, money(p)?),
                Anchor::Future(f) => {
                    annuity::rate_from_future::<Per>(period(n)?, money(payment)?, money(f)?)
                }
            }
            .context("no rate solves these inputs")?;
            ("annuity_rate", r.value())
        }
        AnnuityCommand::Perpetuity { rate: r, payment } => {
            let pv = annuity::perpetuity(rate(r)?, money(payment)?)
                .context("perpetuity diverges (rate must exceed 0)")?;
            ("annuity_perpetuity", pv.value())
        }
        AnnuityCommand::GrowingPerpetuity {
            rate: r,
            growth,
            payment,
        } => {
            let pv = annuity::growing_perpetuity(rate(r)?, rate(growth)?, money(payment)?)
                .context("growing perpetuity diverges (rate must exceed growth)")?;
            ("annuity_growing_perpetuity", pv.value())
        }
        AnnuityCommand::Due { command } => run_annuity_due(command)?,
    })
}

/// Dispatch the `annuity due` subcommands.
#[allow(clippy::needless_pass_by_value)]
fn run_annuity_due(command: AnnuityDueCommand) -> Result<(&'static str, f64)> {
    Ok(match command {
        AnnuityDueCommand::Pv {
            rate: r,
            periods: n,
            payment,
        } => (
            "annuity_due_present_value",
            annuity::due::present_value(rate(r)?, period(n)?, money(payment)?)?.value(),
        ),
        AnnuityDueCommand::Fv {
            rate: r,
            periods: n,
            payment,
        } => (
            "annuity_due_future_value",
            annuity::due::future_value(rate(r)?, period(n)?, money(payment)?)?.value(),
        ),
        AnnuityDueCommand::Payment {
            rate: r,
            periods: n,
            present,
        } => {
            let pmt = annuity::due::payment(rate(r)?, period(n)?, money(present)?)
                .context("annuity-due payment is undefined (e.g. zero periods)")?;
            ("annuity_due_payment", pmt.value())
        }
    })
}

/// Dispatch the `rate` subcommands. Each names a periodicity at runtime, resolved
/// to a marker type via `dispatch_periodicity!`.
fn run_rate(command: RateCommand) -> Result<(&'static str, f64)> {
    Ok(match command {
        RateCommand::Ear {
            rate: r,
            periodicity,
        } => {
            let ear = dispatch_periodicity!(periodicity.as_str(), P => {
                Rate::<P>::new(r)?
                    .effective_annual()
                    .context("effective annual rate is undefined for this input")?
                    .value()
            });
            ("rate_effective_annual", ear)
        }
        RateCommand::Convert { rate: r, from, to } => {
            let converted = dispatch_periodicity!(from.as_str(), P => {
                let source = Rate::<P>::new(r)?;
                dispatch_periodicity!(to.as_str(), Q => {
                    source
                        .convert::<Q>()
                        .context("rate conversion is undefined for this input")?
                        .value()
                })
            });
            ("rate_convert", converted)
        }
        RateCommand::FromNominal {
            nominal,
            periodicity,
        } => {
            let periodic = dispatch_periodicity!(periodicity.as_str(), P => {
                Rate::<P>::from_nominal_annual(nominal)
                    .context("invalid nominal rate")?
                    .value()
            });
            ("rate_from_nominal", periodic)
        }
        RateCommand::Nominal {
            rate: r,
            periodicity,
        } => {
            let nominal = dispatch_periodicity!(periodicity.as_str(), P => {
                Rate::<P>::new(r)?
                    .nominal_annual()
                    .context("nominal annual rate is undefined for this input")?
            });
            ("rate_nominal", nominal)
        }
    })
}

/// The scalar (single-value) operations. Tabular `amortize` is handled separately.
fn run_scalar(command: Command) -> Result<(&'static str, f64)> {
    match command {
        Command::Series { command } => run_series(command),
        Command::SingleSum { command } => run_single_sum(command),
        Command::Annuity { command } => run_annuity(command),
        Command::Rate { command } => run_rate(command),
        Command::Amortize { .. } => unreachable!("amortize is handled by run_amortize"),
    }
}

/// Print an amortization schedule: aligned rows, or a JSON array of row objects
/// under `--json` (ADR-0028's tabular output convention).
fn run_amortize(
    json: bool,
    r: f64,
    principal: f64,
    periods: Option<f64>,
    payment: Option<f64>,
) -> Result<()> {
    let rate = rate(r)?;
    let principal = money(principal)?;
    let schedule = match (periods, payment) {
        (Some(n), None) => amortization::Schedule::<Per>::for_term(rate, period(n)?, principal),
        (None, Some(p)) => amortization::Schedule::<Per>::with_payment(rate, money(p)?, principal),
        (None, None) => bail!("provide either --periods or --payment"),
        (Some(_), Some(_)) => bail!("--periods and --payment are mutually exclusive"),
    }
    .context("amortization schedule is undefined for these inputs")?;

    let installments: Vec<_> = schedule.collect();
    if json {
        let rows: Vec<serde_json::Value> = installments
            .iter()
            .map(|i| {
                serde_json::json!({
                    "period": i.period,
                    "payment": i.payment.value(),
                    "interest": i.interest.value(),
                    "principal": i.principal.value(),
                    "balance": i.balance.value(),
                })
            })
            .collect();
        println!("{}", serde_json::Value::Array(rows));
    } else {
        println!("period\tpayment\tinterest\tprincipal\tbalance");
        for i in &installments {
            println!(
                "{}\t{}\t{}\t{}\t{}",
                i.period,
                i.payment.value(),
                i.interest.value(),
                i.principal.value(),
                i.balance.value(),
            );
        }
    }
    Ok(())
}

fn run(cli: Cli) -> Result<()> {
    let json = cli.json;
    match cli.command {
        Command::Amortize {
            rate: r,
            principal,
            periods,
            payment,
        } => return run_amortize(json, r, principal, periods, payment),
        command => {
            let (label, value) = run_scalar(command)?;
            if json {
                let mut object = serde_json::Map::new();
                object.insert(label.to_owned(), serde_json::json!(value));
                println!("{}", serde_json::Value::Object(object));
            } else {
                println!("{value}");
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        // Print only the outermost message, not anyhow's full `{:#}` chain: our
        // context strings already restate the library error in user terms, so the
        // chain just doubled the text (ADR-0028 / #30). An uncontexted `TvmError`
        // still surfaces its own Display here.
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
