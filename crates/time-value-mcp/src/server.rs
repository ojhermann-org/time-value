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
    annuity, future_value, present_value, Cashflows, Money, Monthly, Period, Rate, TvmError,
};

use crate::params::{
    AnnuityPaymentInput, AnnuityValueInput, FutureValueInput, IrrInput, PresentValueInput,
    SeriesInput,
};

/// Told to clients on initialise.
const INSTRUCTIONS: &str = "\
Time-value-of-money calculations. Tools: `npv` and `nfv` (net present / future \
value of a cashflow series at a per-period rate); `irr` (internal rate of return \
of a series); `present_value` and `future_value` (a single sum over a number of \
periods); `annuity_present_value`, `annuity_future_value`, and `annuity_payment` \
(ordinary, end-of-period annuities). Rates are per period; cashflows are signed \
(outflow negative). Source: https://github.com/ojhermann-org/time-value";

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
        let value = series.net_present_value(rate(input.rate)?).value();
        Ok(result("npv", value))
    }

    #[tool(
        name = "nfv",
        description = "Net future value of a cashflow series compounded to its final period at a per-period rate."
    )]
    fn nfv(&self, Parameters(input): Parameters<SeriesInput>) -> Result<CallToolResult, ErrorData> {
        let flows = cashflows(&input.cashflows)?;
        let series = Cashflows::<Monthly>::new(&flows);
        let value = series.net_future_value(rate(input.rate)?).value();
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
        name = "present_value",
        description = "Present value of a single future amount, discounted at a per-period rate over a (possibly fractional) number of periods."
    )]
    fn present_value(
        &self,
        Parameters(input): Parameters<PresentValueInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = present_value(
            rate(input.rate)?,
            period(input.periods)?,
            money(input.future)?,
        )
        .value();
        Ok(result("present_value", value))
    }

    #[tool(
        name = "future_value",
        description = "Future value of a single present amount, compounded at a per-period rate over a (possibly fractional) number of periods."
    )]
    fn future_value(
        &self,
        Parameters(input): Parameters<FutureValueInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = future_value(
            rate(input.rate)?,
            period(input.periods)?,
            money(input.present)?,
        )
        .value();
        Ok(result("future_value", value))
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

fn period(value: f64) -> Result<Period, ErrorData> {
    Period::new(value).map_err(tvm)
}

fn money(value: f64) -> Result<Money, ErrorData> {
    Money::new(value).map_err(tvm)
}

fn cashflows(values: &[f64]) -> Result<Vec<Money>, ErrorData> {
    values.iter().copied().map(money).collect()
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
