//! Typed inputs for the MCP tools.
//!
//! Each struct derives [`JsonSchema`] (so the server advertises an input schema)
//! and [`Deserialize`] (so `rmcp` can parse the call arguments). Field doc
//! comments become the schema descriptions. Keeping the parsing here leaves the
//! library's typed core untouched (ADR-0011).

use schemars::JsonSchema;
use serde::Deserialize;

/// A per-period rate and a cashflow series — inputs for `npv` and `nfv`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SeriesInput {
    /// Per-period rate (e.g. `0.01` for 1% per period).
    pub rate: f64,
    /// Cashflows at periods 0, 1, 2, … (signed: outflow negative, inflow
    /// positive). Period 0 is "now" and is not discounted.
    pub cashflows: Vec<f64>,
}

/// A cashflow series and an optional solver guess — input for `irr`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct IrrInput {
    /// Cashflows at periods 0, 1, 2, … (signed: outflow negative).
    pub cashflows: Vec<f64>,
    /// Initial guess for the Newton–Raphson solve (default `0.1`).
    #[serde(default = "default_guess")]
    pub guess: f64,
}

fn default_guess() -> f64 {
    0.1
}

/// A finance rate, a reinvestment rate, and a cashflow series — input for `mirr`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct MirrInput {
    /// Per-period finance rate: discounts the outflows to the present.
    pub finance: f64,
    /// Per-period reinvestment rate: compounds the inflows to the final period.
    pub reinvest: f64,
    /// Cashflows at periods 0, 1, 2, … (signed: outflow negative).
    pub cashflows: Vec<f64>,
}

/// A single dated cashflow — an ISO date and a signed amount.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct DatedFlow {
    /// The cashflow date, ISO `YYYY-MM-DD`.
    pub date: String,
    /// The signed cashflow amount (outflow negative, inflow positive).
    pub amount: f64,
}

/// An annual rate and dated cashflows — input for `xnpv`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct DatedSeriesInput {
    /// Annual discount rate (e.g. `0.1` for 10% per year).
    pub rate: f64,
    /// Dated cashflows; the first date is the valuation reference.
    pub flows: Vec<DatedFlow>,
}

/// Dated cashflows and an optional solver guess — input for `xirr`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct DatedIrrInput {
    /// Dated cashflows; the first date is the valuation reference.
    pub flows: Vec<DatedFlow>,
    /// Initial guess for the Newton–Raphson solve, annual (default `0.1`).
    #[serde(default = "default_guess")]
    pub guess: f64,
}

/// Input for the single-sum `present_value` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PresentValueInput {
    /// Per-period discount rate.
    pub rate: f64,
    /// Number of periods (may be fractional).
    pub periods: f64,
    /// The future amount to discount to today.
    pub future: f64,
}

/// Input for the single-sum `future_value` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct FutureValueInput {
    /// Per-period rate.
    pub rate: f64,
    /// Number of periods (may be fractional).
    pub periods: f64,
    /// The present amount to compound forward.
    pub present: f64,
}

/// Input for the `annuity_present_value` and `annuity_future_value` tools.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct AnnuityValueInput {
    /// Per-period rate.
    pub rate: f64,
    /// Number of periods (may be fractional).
    pub periods: f64,
    /// The payment made at the end of each period.
    pub payment: f64,
}

/// Input for the `annuity_payment` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct AnnuityPaymentInput {
    /// Per-period rate.
    pub rate: f64,
    /// Number of periods (may be fractional).
    pub periods: f64,
    /// The present value to amortise into level payments.
    pub present: f64,
}
