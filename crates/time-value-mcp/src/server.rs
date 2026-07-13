//! The MCP server: time-value-of-money operations exposed as tools.
//!
//! The server is stateless — every tool is a pure function of its arguments —
//! so it holds only its tool router. Tools build their result as structured
//! JSON; domain failures ([`TvmError`]) become MCP `invalid_params` errors.

// rmcp's `#[tool]` methods must take `&self`; the server is stateless, so they
// do not use it.
#![allow(clippy::unused_self)]

use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData, ServerHandler,
};
use time_value::{
    annuity, single_sum, Annual, Cashflows, DatedCashflow, DatedCashflows, Money, Monthly, Period,
    Rate, TvmError,
};

use crate::params::{
    AnnuityPaymentInput, AnnuityPeriodsInput, AnnuityRateInput, AnnuityValueInput, DatedFlow,
    DatedIrrInput, DatedSeriesInput, FutureValueInput, GrowingPerpetuityInput, IrrInput, MirrInput,
    PerpetuityInput, PresentValueInput, SeriesInput, SingleSumPeriodsInput, SingleSumRateInput,
};

/// Told to clients on initialise.
const INSTRUCTIONS: &str = "\
Time-value-of-money calculations, grouped by family. Series: `npv`, `nfv` (net \
present / future value at a per-period rate), `irr`, `mirr` (modified IRR, with \
finance and reinvestment rates), and `xnpv`/`xirr` (cashflows on irregular ISO \
dates, at an annual rate). Single sum: `single_sum_present_value`, \
`single_sum_future_value`, and the solves `single_sum_periods` (NPER) / \
`single_sum_rate` (RATE). Annuity: `annuity_present_value`, \
`annuity_future_value`, `annuity_payment`, the solves `annuity_periods` / \
`annuity_rate` (each from a present or future value), `annuity_perpetuity`, \
`annuity_growing_perpetuity`, and the annuity-due forms \
`annuity_due_present_value`, `annuity_due_future_value`, `annuity_due_payment`. \
Rates are per period (annual for `xnpv`/`xirr`); cashflows are signed (outflow \
negative). Source: https://github.com/ojhermann-org/time-value";

/// The MCP server. Stateless: the operations are pure functions of their inputs.
#[derive(Clone)]
pub(crate) struct TimeValueServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl TimeValueServer {
    /// Build the server.
    pub(crate) fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "npv",
        description = "Net present value of a cashflow series discounted at a per-period rate: sum of CF_t / (1+r)^t."
    )]
    fn npv(&self, Parameters(input): Parameters<SeriesInput>) -> Result<CallToolResult, ErrorData> {
        let flows = cashflows(&input.cashflows)?;
        let series = Cashflows::<Monthly>::new(&flows);
        let value = series
            .net_present_value(rate(input.rate)?)
            .map_err(tvm)?
            .value();
        Ok(result("npv", value))
    }

    #[tool(
        name = "nfv",
        description = "Net future value of a cashflow series compounded to its final period at a per-period rate."
    )]
    fn nfv(&self, Parameters(input): Parameters<SeriesInput>) -> Result<CallToolResult, ErrorData> {
        let flows = cashflows(&input.cashflows)?;
        let series = Cashflows::<Monthly>::new(&flows);
        let value = series
            .net_future_value(rate(input.rate)?)
            .map_err(tvm)?
            .value();
        Ok(result("nfv", value))
    }

    #[tool(
        name = "irr",
        description = "Internal rate of return (per period) of a cashflow series: the rate at which its net present value is zero."
    )]
    fn irr(&self, Parameters(input): Parameters<IrrInput>) -> Result<CallToolResult, ErrorData> {
        let flows = cashflows(&input.cashflows)?;
        let series = Cashflows::<Monthly>::new(&flows);
        let irr = series
            .internal_rate_of_return_from(input.guess)
            .map_err(tvm)?;
        Ok(result("irr", irr.value()))
    }

    #[tool(
        name = "mirr",
        description = "Modified internal rate of return (per period): discounts outflows at a finance rate and compounds inflows at a reinvestment rate, then equates the two over the series' life."
    )]
    fn mirr(&self, Parameters(input): Parameters<MirrInput>) -> Result<CallToolResult, ErrorData> {
        let flows = cashflows(&input.cashflows)?;
        let series = Cashflows::<Monthly>::new(&flows);
        let mirr = series
            .modified_internal_rate_of_return(rate(input.finance)?, rate(input.reinvest)?)
            .map_err(tvm)?;
        Ok(result("mirr", mirr.value()))
    }

    #[tool(
        name = "xnpv",
        description = "Net present value of cashflows on irregular dates (XNPV), discounted at an annual rate by the year-fraction (ACT/365) from the first date."
    )]
    fn xnpv(
        &self,
        Parameters(input): Parameters<DatedSeriesInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let flows = dated_flows(&input.flows)?;
        let series = DatedCashflows::new(&flows);
        let value = series
            .net_present_value(annual_rate(input.rate)?)
            .map_err(tvm)?
            .value();
        Ok(result("xnpv", value))
    }

    #[tool(
        name = "xirr",
        description = "Internal rate of return of cashflows on irregular dates (XIRR), as an annual rate: the rate at which their XNPV (ACT/365 from the first date) is zero."
    )]
    fn xirr(
        &self,
        Parameters(input): Parameters<DatedIrrInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let flows = dated_flows(&input.flows)?;
        let series = DatedCashflows::new(&flows);
        let irr = series
            .internal_rate_of_return_from(input.guess)
            .map_err(tvm)?;
        Ok(result("xirr", irr.value()))
    }

    #[tool(
        name = "single_sum_present_value",
        description = "Present value of a single future amount, discounted at a per-period rate over a (possibly fractional) number of periods."
    )]
    fn single_sum_present_value(
        &self,
        Parameters(input): Parameters<PresentValueInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = single_sum::present_value(
            rate(input.rate)?,
            period(input.periods)?,
            money(input.future)?,
        )
        .map_err(tvm)?
        .value();
        Ok(result("single_sum_present_value", value))
    }

    #[tool(
        name = "single_sum_future_value",
        description = "Future value of a single present amount, compounded at a per-period rate over a (possibly fractional) number of periods."
    )]
    fn single_sum_future_value(
        &self,
        Parameters(input): Parameters<FutureValueInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = single_sum::future_value(
            rate(input.rate)?,
            period(input.periods)?,
            money(input.present)?,
        )
        .map_err(tvm)?
        .value();
        Ok(result("single_sum_future_value", value))
    }

    #[tool(
        name = "single_sum_periods",
        description = "Solve for the number of periods that grows a present amount to a future amount at a per-period rate (NPER)."
    )]
    fn single_sum_periods(
        &self,
        Parameters(input): Parameters<SingleSumPeriodsInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let periods = single_sum::periods(
            rate(input.rate)?,
            money(input.present)?,
            money(input.future)?,
        )
        .map_err(tvm)?;
        Ok(result("single_sum_periods", periods.value()))
    }

    #[tool(
        name = "single_sum_rate",
        description = "Solve for the per-period rate that grows a present amount to a future amount over a number of periods (RATE)."
    )]
    fn single_sum_rate(
        &self,
        Parameters(input): Parameters<SingleSumRateInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let solved = single_sum::rate::<Monthly>(
            period(input.periods)?,
            money(input.present)?,
            money(input.future)?,
        )
        .map_err(tvm)?;
        Ok(result("single_sum_rate", solved.value()))
    }

    #[tool(
        name = "annuity_present_value",
        description = "Present value of an ordinary annuity that pays a fixed amount at the end of each period."
    )]
    fn annuity_present_value(
        &self,
        Parameters(input): Parameters<AnnuityValueInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = annuity::present_value(
            rate(input.rate)?,
            period(input.periods)?,
            money(input.payment)?,
        )
        .map_err(tvm)?
        .value();
        Ok(result("annuity_present_value", value))
    }

    #[tool(
        name = "annuity_future_value",
        description = "Future value of an ordinary annuity that pays a fixed amount at the end of each period."
    )]
    fn annuity_future_value(
        &self,
        Parameters(input): Parameters<AnnuityValueInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = annuity::future_value(
            rate(input.rate)?,
            period(input.periods)?,
            money(input.payment)?,
        )
        .map_err(tvm)?
        .value();
        Ok(result("annuity_future_value", value))
    }

    #[tool(
        name = "annuity_payment",
        description = "The level end-of-period payment that amortises a present value over a number of periods at a per-period rate."
    )]
    fn annuity_payment(
        &self,
        Parameters(input): Parameters<AnnuityPaymentInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let payment = annuity::payment(
            rate(input.rate)?,
            period(input.periods)?,
            money(input.present)?,
        )
        .map_err(tvm)?;
        Ok(result("annuity_payment", payment.value()))
    }

    #[tool(
        name = "annuity_periods",
        description = "Solve for the number of level end-of-period payments, from a present value or a future value (provide exactly one)."
    )]
    fn annuity_periods(
        &self,
        Parameters(input): Parameters<AnnuityPeriodsInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let r = rate(input.rate)?;
        let pmt = money(input.payment)?;
        let periods = match anchor(input.present, input.future)? {
            Anchor::Present(p) => annuity::periods(r, pmt, money(p)?),
            Anchor::Future(f) => annuity::periods_from_future(r, pmt, money(f)?),
        }
        .map_err(tvm)?;
        Ok(result("annuity_periods", periods.value()))
    }

    #[tool(
        name = "annuity_rate",
        description = "Solve for the per-period rate of an annuity, from a present value or a future value (provide exactly one)."
    )]
    fn annuity_rate(
        &self,
        Parameters(input): Parameters<AnnuityRateInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let n = period(input.periods)?;
        let pmt = money(input.payment)?;
        let solved = match anchor(input.present, input.future)? {
            Anchor::Present(p) => annuity::rate::<Monthly>(n, pmt, money(p)?),
            Anchor::Future(f) => annuity::rate_from_future::<Monthly>(n, pmt, money(f)?),
        }
        .map_err(tvm)?;
        Ok(result("annuity_rate", solved.value()))
    }

    #[tool(
        name = "annuity_perpetuity",
        description = "Present value of a level perpetuity — a fixed end-of-period payment forever — at a per-period rate (which must exceed 0)."
    )]
    fn annuity_perpetuity(
        &self,
        Parameters(input): Parameters<PerpetuityInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = annuity::perpetuity(rate(input.rate)?, money(input.payment)?)
            .map_err(tvm)?
            .value();
        Ok(result("annuity_perpetuity", value))
    }

    #[tool(
        name = "annuity_growing_perpetuity",
        description = "Present value of a perpetuity whose payment grows each period, at a per-period rate that must exceed the growth rate."
    )]
    fn annuity_growing_perpetuity(
        &self,
        Parameters(input): Parameters<GrowingPerpetuityInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = annuity::growing_perpetuity(
            rate(input.rate)?,
            rate(input.growth)?,
            money(input.payment)?,
        )
        .map_err(tvm)?
        .value();
        Ok(result("annuity_growing_perpetuity", value))
    }

    #[tool(
        name = "annuity_due_present_value",
        description = "Present value of an annuity-due that pays a fixed amount at the start of each period."
    )]
    fn annuity_due_present_value(
        &self,
        Parameters(input): Parameters<AnnuityValueInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = annuity::due::present_value(
            rate(input.rate)?,
            period(input.periods)?,
            money(input.payment)?,
        )
        .map_err(tvm)?
        .value();
        Ok(result("annuity_due_present_value", value))
    }

    #[tool(
        name = "annuity_due_future_value",
        description = "Future value of an annuity-due that pays a fixed amount at the start of each period."
    )]
    fn annuity_due_future_value(
        &self,
        Parameters(input): Parameters<AnnuityValueInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = annuity::due::future_value(
            rate(input.rate)?,
            period(input.periods)?,
            money(input.payment)?,
        )
        .map_err(tvm)?
        .value();
        Ok(result("annuity_due_future_value", value))
    }

    #[tool(
        name = "annuity_due_payment",
        description = "The level start-of-period payment that amortises a present value over a number of periods at a per-period rate."
    )]
    fn annuity_due_payment(
        &self,
        Parameters(input): Parameters<AnnuityPaymentInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let payment = annuity::due::payment(
            rate(input.rate)?,
            period(input.periods)?,
            money(input.present)?,
        )
        .map_err(tvm)?;
        Ok(result("annuity_due_payment", payment.value()))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for TimeValueServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(INSTRUCTIONS);
        // `ServerInfo::new` fills `server_info` from the rmcp crate's build env;
        // identify ourselves with this crate's own name/version instead. The
        // package name (`time-value-mcp`) is the product/binary name.
        info.server_info.name = env!("CARGO_PKG_NAME").to_string();
        info.server_info.version = env!("CARGO_PKG_VERSION").to_string();
        info
    }
}

// The periodicity tag does not affect any result (ADR-0010); the server fixes it
// to one marker to satisfy the type parameter.
fn rate(value: f64) -> Result<Rate<Monthly>, ErrorData> {
    Rate::new(value).map_err(tvm)
}

/// The dated `xnpv`/`xirr` discount is intrinsically annual (ADR-0029).
fn annual_rate(value: f64) -> Result<Rate<Annual>, ErrorData> {
    Rate::new(value).map_err(tvm)
}

fn period(value: f64) -> Result<Period, ErrorData> {
    Period::new(value).map_err(tvm)
}

fn money(value: f64) -> Result<Money, ErrorData> {
    Money::new(value).map_err(tvm)
}

fn cashflows(values: &[f64]) -> Result<Vec<Money>, ErrorData> {
    values.iter().copied().map(money).collect()
}

/// The value a solve-for operation is anchored to — exactly one of a present or a
/// future amount. `present` and `future` are mutually exclusive and one is required.
enum Anchor {
    Present(f64),
    Future(f64),
}

fn anchor(present: Option<f64>, future: Option<f64>) -> Result<Anchor, ErrorData> {
    match (present, future) {
        (Some(p), None) => Ok(Anchor::Present(p)),
        (None, Some(f)) => Ok(Anchor::Future(f)),
        (None, None) => Err(ErrorData::invalid_params(
            "provide either `present` or `future`".to_string(),
            None,
        )),
        (Some(_), Some(_)) => Err(ErrorData::invalid_params(
            "`present` and `future` are mutually exclusive".to_string(),
            None,
        )),
    }
}

// ---- Dated flows (XNPV/XIRR): ISO dates → ACT/365 year-offsets ----
//
// The core takes year-offsets, not a date type (ADR-0029); the server accepts ISO
// `YYYY-MM-DD` dates and converts them with a self-contained ACT/365 day-count, so
// no date dependency reaches the binary.

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

/// Parse an ISO `YYYY-MM-DD` date to a day number, mapping any malformed input to
/// an MCP `invalid_params` error.
fn parse_date(text: &str) -> Result<i64, ErrorData> {
    let invalid =
        || ErrorData::invalid_params(format!("invalid date `{text}` (expected YYYY-MM-DD)"), None);
    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() != 3 {
        return Err(invalid());
    }
    let year: i64 = parts[0].parse().map_err(|_| invalid())?;
    let month: i64 = parts[1].parse().map_err(|_| invalid())?;
    let day: i64 = parts[2].parse().map_err(|_| invalid())?;
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return Err(invalid());
    }
    Ok(days_from_civil(year, month, day))
}

/// Convert dated inputs to core [`DatedCashflow`]s, rebasing offsets to the first
/// flow (ACT/365).
fn dated_flows(flows: &[DatedFlow]) -> Result<Vec<DatedCashflow>, ErrorData> {
    let mut out = Vec::with_capacity(flows.len());
    let mut reference: Option<i64> = None;
    for flow in flows {
        let day = parse_date(&flow.date)?;
        let reference = *reference.get_or_insert(day);
        // Day-count differences for real calendar dates are far below 2^53, so
        // this conversion is exact despite the lint's worst-case warning.
        #[allow(clippy::cast_precision_loss)]
        let offset_years = (day - reference) as f64 / 365.0;
        out.push(DatedCashflow::new(offset_years, money(flow.amount)?).map_err(tvm)?);
    }
    Ok(out)
}

/// A single-field structured tool result, keyed by the operation.
fn result(label: &str, value: f64) -> CallToolResult {
    let mut object = serde_json::Map::new();
    object.insert(label.to_owned(), serde_json::json!(value));
    CallToolResult::structured(serde_json::Value::Object(object))
}

/// Map a library error to an MCP `invalid_params` error — every `TvmError` here
/// is caused by the caller's arguments (an out-of-range rate, a non-convergent
/// IRR, a degenerate annuity). Takes the error by value so it can be used
/// directly as a `Result::map_err` argument.
#[allow(clippy::needless_pass_by_value)]
fn tvm(error: TvmError) -> ErrorData {
    ErrorData::invalid_params(error.to_string(), None)
}
