//! Typed inputs for the MCP tools.
//!
//! Each struct derives [`JsonSchema`] (so the server advertises an input schema)
//! and [`Deserialize`] (so `rmcp` can parse the call arguments). Field doc
//! comments become the schema descriptions. Keeping the parsing here leaves the
//! library's typed core untouched (ADR-0011).

use schemars::JsonSchema;
use serde::Deserialize;

/// The compounding periodicity a `rate_*` tool operates at — the only place a
/// periodicity is a runtime input (ADR-0028 §3). A closed set, so it is a typed
/// enum rather than a free string (ADR-0039): an unknown value is refused by
/// deserialization at the boundary, and the schema advertises the six choices.
/// Serialized names are lower-kebab (`semi-annual`), matching the marker types
/// in the core.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Periodicity {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
}

/// A per-period rate and a cashflow series — inputs for `npv` and `nfv`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SeriesInput {
    /// Per-period rate (e.g. `0.01` for 1% per period).
    pub rate: f64,
    /// Cashflows at periods 0, 1, 2, … (signed: outflow negative, inflow
    /// positive). Period 0 is "now" and is not discounted.
    pub cashflows: Vec<f64>,
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
}

/// A cashflow series and an optional solver guess — input for `irr`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct IrrInput {
    /// Cashflows at periods 0, 1, 2, … (signed: outflow negative).
    pub cashflows: Vec<f64>,
    /// Initial guess for the Newton–Raphson solve (default `0.1`).
    #[serde(default = "default_guess")]
    pub guess: f64,
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
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
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
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
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
}

/// Dated cashflows and an optional solver guess — input for `xirr`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct DatedIrrInput {
    /// Dated cashflows; the first date is the valuation reference.
    pub flows: Vec<DatedFlow>,
    /// Initial guess for the Newton–Raphson solve, annual (default `0.1`).
    #[serde(default = "default_guess")]
    pub guess: f64,
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
}

/// Input for the `single_sum_present_value` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PresentValueInput {
    /// Per-period discount rate.
    pub rate: f64,
    /// Number of periods (may be fractional).
    pub periods: f64,
    /// The future amount to discount to today.
    pub future: f64,
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
}

/// Input for the `single_sum_future_value` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct FutureValueInput {
    /// Per-period rate.
    pub rate: f64,
    /// Number of periods (may be fractional).
    pub periods: f64,
    /// The present amount to compound forward.
    pub present: f64,
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
}

/// Input for the `single_sum_periods` tool (solve for the number of periods).
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SingleSumPeriodsInput {
    /// Per-period rate.
    pub rate: f64,
    /// The present amount.
    pub present: f64,
    /// The future amount.
    pub future: f64,
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
}

/// Input for the `single_sum_rate` tool (solve for the per-period rate).
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SingleSumRateInput {
    /// Number of periods (may be fractional).
    pub periods: f64,
    /// The present amount.
    pub present: f64,
    /// The future amount.
    pub future: f64,
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
}

/// Input for the `annuity_periods` tool. Provide exactly one of `present` or
/// `future` (the value the payment stream is anchored to).
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct AnnuityPeriodsInput {
    /// Per-period rate.
    pub rate: f64,
    /// The payment made at the end of each period.
    pub payment: f64,
    /// Solve from this present value (mutually exclusive with `future`).
    #[serde(default)]
    pub present: Option<f64>,
    /// Solve from this future value (mutually exclusive with `present`).
    #[serde(default)]
    pub future: Option<f64>,
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
}

/// Input for the `annuity_rate` tool. Provide exactly one of `present` or
/// `future`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct AnnuityRateInput {
    /// Number of periods (may be fractional).
    pub periods: f64,
    /// The payment made at the end of each period.
    pub payment: f64,
    /// Solve from this present value (mutually exclusive with `future`).
    #[serde(default)]
    pub present: Option<f64>,
    /// Solve from this future value (mutually exclusive with `present`).
    #[serde(default)]
    pub future: Option<f64>,
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
}

/// Input for the `annuity_perpetuity` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PerpetuityInput {
    /// Per-period rate (must exceed 0).
    pub rate: f64,
    /// The payment made at the end of each period, forever.
    pub payment: f64,
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
}

/// Input for the `annuity_growing_perpetuity` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GrowingPerpetuityInput {
    /// Per-period rate (must exceed the growth rate).
    pub rate: f64,
    /// The per-period growth rate of the payment.
    pub growth: f64,
    /// The first payment (at the end of period 1).
    pub payment: f64,
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
}

/// Input for the `rate_effective_annual` and `rate_nominal` tools.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct RateEffectiveAnnualInput {
    /// The per-period rate.
    pub rate: f64,
    /// The periodicity the rate is expressed in.
    pub periodicity: Periodicity,
}

/// Input for the `rate_convert` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct RateConvertInput {
    /// The per-period rate expressed under `from`.
    pub rate: f64,
    /// The periodicity the rate is expressed in.
    pub from: Periodicity,
    /// The periodicity to express the rate in.
    pub to: Periodicity,
}

/// Input for the `rate_from_nominal` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct RateFromNominalInput {
    /// The nominal annual rate (APR).
    pub nominal: f64,
    /// The compounding periodicity.
    pub periodicity: Periodicity,
}

/// Input for the `amortize` tool. Provide exactly one of `periods` (amortise over
/// a term) or `payment` (amortise with a level payment).
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct AmortizeInput {
    /// Per-period rate.
    pub rate: f64,
    /// The principal to amortise.
    pub principal: f64,
    /// Amortise over this many periods (mutually exclusive with `payment`).
    #[serde(default)]
    pub periods: Option<f64>,
    /// Amortise with this level payment (mutually exclusive with `periods`).
    #[serde(default)]
    pub payment: Option<f64>,
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
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
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
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
    /// ISO 4217 currency to denominate the amounts in (e.g. `USD`, `JPY`).
    /// Omit for currency-agnostic (`XXX`) amounts. An unknown code is rejected.
    #[serde(default)]
    pub currency: Option<String>,
}
