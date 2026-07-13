//! # `time_value`
//!
//! Type-safe time-value-of-money (TVM) calculations.
//!
//! This crate is a deliberately type-heavy redesign of `time_value`, rebuilt
//! from scratch for the `1.0` line. The design goal is to make TVM mistakes —
//! applying an annual rate to monthly cashflows, discounting with an
//! economically meaningless rate — *compile errors* rather than silent
//! arithmetic, while keeping the common path ergonomic.
//!
//! The crate is `#![no_std]` and dependency-free by default.
//!
//! ## Model
//!
//! - [`Money`] is a validated monetary amount — always finite, because every
//!   operation that could overflow returns a `Result` instead; cashflows are
//!   signed (outflow negative, inflow positive).
//! - [`Rate<P>`] is a per-period interest rate tagged with a [`Periodicity`]
//!   marker (`P` — e.g. [`Monthly`], [`Annual`]). The tag is zero-sized.
//! - [`Cashflows<P>`] is a periodicity-tagged series of cashflows at consecutive
//!   periods. Discounting a [`Cashflows<P>`] requires a [`Rate<P>`] of the *same*
//!   periodicity, so a mismatch is a compile error.
//!
//! ## Operations
//!
//! The discrete operations — [`net_present_value`], [`net_future_value`], and
//! [`internal_rate_of_return`] — need only elementary arithmetic and are
//! available in the default `no_std`, zero-dependency build, as is the
//! allocation-free [`amortization`] schedule iterator (from an explicit payment;
//! its term-based constructor needs a feature).
//!
//! Operations that require transcendental functions (`powf`, `ln`) live behind
//! the optional `std` / `libm` features (see
//! `docs/adr/0009-no_std-and-optional-libm.md`): the [`single_sum`] module
//! (present/future value and the solve-for `periods` / `rate` inverses, with the
//! [`Period`] type), the [`annuity`] module (ordinary, [annuity-due](annuity::due),
//! and [perpetuity](annuity::perpetuity) forms, plus the `periods` / `rate`
//! solves), the modified internal rate of return
//! ([`Cashflows::modified_internal_rate_of_return`]), the term-based
//! [`amortization`] constructor, effective rate conversions between
//! periodicities ([`Rate::convert`] / [`Rate::effective_annual`]), and
//! [`DatedCashflows`] (XNPV/XIRR over irregularly dated flows). Nominal-rate
//! conversion ([`Rate::from_nominal_annual`] / [`Rate::nominal_annual`]) is plain
//! arithmetic and needs no feature.
//!
//! ```
//! use time_value::{Cashflows, Money, Monthly, Rate};
//!
//! // A project: pay 100 now, receive 60 next month and 60 the month after.
//! let flows = [Money::new(-100.0)?, Money::new(60.0)?, Money::new(60.0)?];
//! let project = Cashflows::<Monthly>::new(&flows);
//!
//! let npv = project.net_present_value(Rate::<Monthly>::new(0.01)?)?;
//! assert!(npv.value() > 0.0); // worth doing at 1%/month
//!
//! let irr = project.internal_rate_of_return()?;
//! assert!((irr.value() - 0.1307).abs() < 1e-4); // ~13.07% per month
//! # Ok::<(), time_value::TvmError>(())
//! ```
//!
//! [`Cashflows<P>`]: Cashflows
//! [`Rate<P>`]: Rate
//! [`net_present_value`]: Cashflows::net_present_value
//! [`net_future_value`]: Cashflows::net_future_value
//! [`internal_rate_of_return`]: Cashflows::internal_rate_of_return

// `no_std` unless the `std` feature is enabled — the `std` feature turns this
// into an ordinary `std` crate so it can use `f64`'s transcendental methods.
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

pub mod amortization;
mod cashflows;
mod money;
mod periodicity;
mod rate;
mod root;

pub use cashflows::Cashflows;
pub use money::Money;
pub use periodicity::{Annual, Daily, Monthly, Periodicity, Quarterly, SemiAnnual, Weekly};
pub use rate::Rate;

// Operations that need transcendental math (`powf`) are available only with the
// `std` or `libm` feature (see `docs/adr/0014-transcendental-single-sum-operations.md`).
#[cfg(any(feature = "std", feature = "libm"))]
pub mod annuity;
#[cfg(any(feature = "std", feature = "libm"))]
mod dated;
#[cfg(any(feature = "std", feature = "libm"))]
mod math;
#[cfg(any(feature = "std", feature = "libm"))]
mod period;
#[cfg(any(feature = "std", feature = "libm"))]
pub mod single_sum;

#[cfg(any(feature = "std", feature = "libm"))]
pub use dated::{DatedCashflow, DatedCashflows};
#[cfg(any(feature = "std", feature = "libm"))]
pub use period::Period;

use core::fmt;

/// Errors produced when constructing or operating on time-value types.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TvmError {
    /// A rate was not finite, or was less than or equal to `-1.0` (i.e. ≤ −100%),
    /// which is economically meaningless for discounting and compounding.
    RateOutOfRange,
    /// A monetary amount supplied to a constructor was not finite (`NaN` or an
    /// infinity). For a non-finite value *produced by an operation*, see
    /// [`NonFiniteResult`](Self::NonFiniteResult).
    NonFiniteAmount,
    /// An operation did not produce a finite amount — its `f64` arithmetic
    /// overflowed to an infinity or `NaN`, or the inputs were a mathematically
    /// undefined case (e.g. an [`annuity::payment`] over
    /// zero periods). Distinct from [`NonFiniteAmount`](Self::NonFiniteAmount),
    /// which is a non-finite value passed *in* (ADR-0021).
    NonFiniteResult,
    /// A period count was negative or not finite.
    NegativePeriods,
    /// A dated cashflow was given a non-finite year-offset (`NaN` or an infinity).
    /// The offset may be negative or zero, but must be finite
    /// ([`DatedCashflow`](crate::DatedCashflow); ADR-0029).
    NonFiniteOffset,
    /// An operation that requires at least one cashflow was given an empty
    /// series (e.g. [`Cashflows::internal_rate_of_return`]).
    EmptyCashflows,
    /// [`Cashflows::internal_rate_of_return`] did not converge to a root within
    /// its iteration budget, or the iteration left the valid rate domain.
    IrrDidNotConverge,
    /// A solve-for-rate operation did not converge to a root — no rate satisfies
    /// the relationship over the valid domain (e.g. [`annuity::rate`] when no rate
    /// prices the given payment stream at the target value). Distinct from
    /// [`IrrDidNotConverge`](Self::IrrDidNotConverge), which is specific to
    /// [`Cashflows::internal_rate_of_return`].
    SolveDidNotConverge,
    /// A perpetuity's present value diverges because its rate does not exceed its
    /// growth rate (`r <= g`; for a level perpetuity, `r <= 0`). The closed form
    /// `PMT / (r - g)` would return either an infinity (`r = g`) or a finite but
    /// economically meaningless value (`r < g`) for a series that does not
    /// converge, so [`annuity::perpetuity`] / [`annuity::growing_perpetuity`]
    /// reject it instead (ADR-0015).
    DivergentPerpetuity,
}

impl fmt::Display for TvmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::RateOutOfRange => {
                f.write_str("rate must be finite and greater than -1.0 (-100%)")
            }
            Self::NonFiniteAmount => f.write_str("monetary amount must be finite"),
            Self::NonFiniteResult => {
                f.write_str("operation did not produce a finite amount (overflow or undefined)")
            }
            Self::NegativePeriods => f.write_str("period count must be finite and non-negative"),
            Self::NonFiniteOffset => f.write_str("dated cashflow year-offset must be finite"),
            Self::EmptyCashflows => f.write_str("cashflow series is empty"),
            Self::IrrDidNotConverge => f.write_str("internal rate of return did not converge"),
            Self::SolveDidNotConverge => f.write_str("solve for rate did not converge"),
            Self::DivergentPerpetuity => {
                f.write_str("perpetuity present value diverges: rate does not exceed growth rate")
            }
        }
    }
}

impl core::error::Error for TvmError {}
