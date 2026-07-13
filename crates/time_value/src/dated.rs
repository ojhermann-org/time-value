//! [`DatedCashflows`] — cashflows on irregular calendar dates, discounted by the
//! year-fraction from a reference (XNPV / XIRR).
//!
//! Unlike [`Cashflows`](crate::Cashflows), whose flows sit at evenly spaced
//! periods, these flows carry an explicit **year-offset** and are discounted by
//! `(1 + r)^t` for a fractional `t` — so this module is behind `std` / `libm`
//! (it needs [`powf`](crate::math::powf)). The rate is annual: offsets are years,
//! so a [`Rate<Annual>`] is required and a per-period rate is a compile error
//! (`docs/adr/0029-dated-cashflows-xnpv-xirr.md`).

use crate::math::powf;
use crate::root::{self, abs};
use crate::{Annual, Money, Rate, TvmError};

/// A single cashflow at an offset, in **years**, from a reference point.
///
/// The offset may be negative (a flow before the reference) or zero, but must be
/// finite. Amounts are signed (outflow negative, inflow positive). The reference
/// is supplied by the enclosing [`DatedCashflows`] — its first flow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DatedCashflow {
    offset_years: f64,
    amount: Money,
}

impl DatedCashflow {
    /// A cashflow of `amount` at `offset_years` from the reference.
    ///
    /// # Errors
    ///
    /// [`TvmError::NonFiniteOffset`] if `offset_years` is not finite.
    pub fn new(offset_years: f64, amount: Money) -> Result<Self, TvmError> {
        if !offset_years.is_finite() {
            return Err(TvmError::NonFiniteOffset);
        }
        Ok(Self {
            offset_years,
            amount,
        })
    }

    /// The offset, in years, from the reference.
    #[must_use]
    pub const fn offset_years(self) -> f64 {
        self.offset_years
    }

    /// The signed cashflow amount.
    #[must_use]
    pub const fn amount(self) -> Money {
        self.amount
    }
}

/// A series of cashflows on irregular dates, discounted by year-fraction.
///
/// `DatedCashflows` **borrows** its slice (allocation-free, like
/// [`Cashflows`](crate::Cashflows); ADR-0013). The **first** flow is the
/// valuation reference: every flow is discounted by `(1 + r)^(tᵢ − t₀)`, so the
/// first flow is undiscounted. Rebasing to the first entry (rather than the
/// earliest) matches Excel's XNPV/XIRR.
///
/// # Examples
///
/// ```
/// use time_value::{Annual, DatedCashflow, DatedCashflows, Money, Rate};
///
/// // Pay 100 now, receive 110 exactly one year later: a 10% annual return.
/// let flows = [
///     DatedCashflow::new(0.0, Money::new(-100.0)?)?,
///     DatedCashflow::new(1.0, Money::new(110.0)?)?,
/// ];
/// let dated = DatedCashflows::new(&flows);
///
/// let irr = dated.internal_rate_of_return()?;
/// assert!((irr.value() - 0.10).abs() < 1e-9);
/// # Ok::<(), time_value::TvmError>(())
/// ```
///
/// A per-period rate does not type-check — the discount is annual:
///
/// ```compile_fail
/// use time_value::{DatedCashflow, DatedCashflows, Money, Monthly, Rate};
///
/// let flows = [DatedCashflow::new(0.0, Money::new(-100.0).unwrap()).unwrap()];
/// let dated = DatedCashflows::new(&flows);
/// let monthly = Rate::<Monthly>::new(0.01).unwrap();
/// let _ = dated.net_present_value(monthly); // wrong periodicity — won't compile
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DatedCashflows<'a> {
    flows: &'a [DatedCashflow],
}

impl<'a> DatedCashflows<'a> {
    /// Wraps a slice of dated cashflows; the first flow is the valuation reference.
    #[must_use]
    pub const fn new(flows: &'a [DatedCashflow]) -> Self {
        Self { flows }
    }

    /// The underlying dated cashflows.
    #[must_use]
    pub const fn as_slice(self) -> &'a [DatedCashflow] {
        self.flows
    }

    /// The number of cashflows in the series.
    #[must_use]
    pub const fn len(self) -> usize {
        self.flows.len()
    }

    /// Whether the series is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.flows.is_empty()
    }

    /// The net present value of the dated series discounted at an annual `rate`
    /// (XNPV): `Σᵢ CFᵢ / (1 + r)^(tᵢ − t₀)`, with `tᵢ` the offset in years and
    /// `t₀` the first flow's offset. An **empty** series has value `0`.
    ///
    /// # Errors
    ///
    /// [`TvmError::NonFiniteResult`] if the sum overflows to a non-finite value
    /// (ADR-0021).
    pub fn net_present_value(self, rate: Rate<Annual>) -> Result<Money, TvmError> {
        Money::from_operation(self.xnpv_at(rate.value()))
    }

    /// The internal rate of return of the dated series (XIRR): the annual
    /// [`Rate<Annual>`] at which its XNPV is zero, from a default guess of 10%.
    ///
    /// # Errors
    ///
    /// See [`internal_rate_of_return_from`](Self::internal_rate_of_return_from).
    pub fn internal_rate_of_return(self) -> Result<Rate<Annual>, TvmError> {
        self.internal_rate_of_return_from(0.1)
    }

    /// The XIRR, seeding the solver with `guess` (an annual rate).
    ///
    /// Like [`Cashflows::internal_rate_of_return_from`](crate::Cashflows::internal_rate_of_return_from),
    /// it tries **Newton–Raphson** from `guess` and falls back to a **bracketing
    /// bisection** over the valid rate domain (ADR-0020), so a root is found
    /// whenever the XNPV changes sign. The convergence tolerance scales with the
    /// cashflow magnitudes (ADR-0021).
    ///
    /// # Errors
    ///
    /// - [`TvmError::EmptyCashflows`] if the series is empty.
    /// - [`TvmError::IrrDidNotConverge`] if neither method finds a root — in
    ///   particular when the XNPV never changes sign over the valid rate domain
    ///   (e.g. cashflows that are all one sign).
    pub fn internal_rate_of_return_from(self, guess: f64) -> Result<Rate<Annual>, TvmError> {
        if self.flows.is_empty() {
            return Err(TvmError::EmptyCashflows);
        }
        // Newton from `guess`, then the robust bracketing fallback (ADR-0020), with
        // the magnitude-scaled tolerance (ADR-0021) — all shared with IRR via `root`.
        let tolerance = root::relative_tolerance(self.magnitude());
        match root::newton(|r| self.xnpv_and_derivative(r), guess, tolerance)
            .or_else(|| root::bracket_and_bisect(|r| self.xnpv_at(r), tolerance))
        {
            Some(rate) => Rate::new(rate),
            None => Err(TvmError::IrrDidNotConverge),
        }
    }

    /// The XNPV at a candidate annual `rate`: `Σᵢ CFᵢ (1 + r)^(−tᵢ)`, with `tᵢ`
    /// the offset in years rebased to the first flow. Empty series → `0`.
    fn xnpv_at(self, rate: f64) -> f64 {
        let Some(first) = self.flows.first() else {
            return 0.0;
        };
        let reference = first.offset_years;
        let base = 1.0 + rate;
        let mut npv = 0.0;
        for cf in self.flows {
            let years = cf.offset_years - reference;
            npv += cf.amount.value() * powf(base, -years);
        }
        npv
    }

    /// The XNPV and its derivative d(XNPV)/dr at a candidate annual `rate`.
    ///
    /// `XNPV(r) = Σᵢ CFᵢ (1+r)^(−tᵢ)`, `XNPV'(r) = Σᵢ −tᵢ CFᵢ (1+r)^(−tᵢ−1)`.
    fn xnpv_and_derivative(self, rate: f64) -> (f64, f64) {
        let Some(first) = self.flows.first() else {
            return (0.0, 0.0);
        };
        let reference = first.offset_years;
        let base = 1.0 + rate;
        let mut npv = 0.0;
        let mut derivative = 0.0;
        for cf in self.flows {
            let years = cf.offset_years - reference;
            let amount = cf.amount.value();
            let factor = powf(base, -years); // (1+r)^(−t)
            npv += amount * factor;
            derivative += -years * amount * factor / base; // (1+r)^(−t−1)
        }
        (npv, derivative)
    }

    /// `Σ|CFᵢ|` — an upper bound on `|XNPV|`, used to scale the solver tolerance
    /// (ADR-0021), mirroring [`Cashflows`](crate::Cashflows).
    fn magnitude(self) -> f64 {
        let mut scale = 0.0;
        for cf in self.flows {
            scale += abs(cf.amount.value());
        }
        scale
    }
}

#[cfg(test)]
mod tests {
    use crate::root::within;
    use crate::{Annual, DatedCashflow, DatedCashflows, Money, Rate, TvmError};

    /// `no_std`-safe approximate equality (no `f64::abs`).
    fn approx(a: f64, b: f64) -> bool {
        within(a - b, 1e-6)
    }

    fn flow(offset_years: f64, amount: f64) -> DatedCashflow {
        DatedCashflow::new(offset_years, Money::new(amount).unwrap()).unwrap()
    }

    fn annual(rate: f64) -> Rate<Annual> {
        Rate::<Annual>::new(rate).unwrap()
    }

    #[test]
    fn xnpv_over_one_year_is_the_annual_discount() {
        // -100 now, +110 in one year, discounted at 10% → exactly 0.
        let flows = [flow(0.0, -100.0), flow(1.0, 110.0)];
        let npv = DatedCashflows::new(&flows)
            .net_present_value(annual(0.10))
            .unwrap();
        assert!(approx(npv.value(), 0.0));
    }

    #[test]
    fn xirr_recovers_a_whole_year_rate() {
        let flows = [flow(0.0, -100.0), flow(1.0, 110.0)];
        let irr = DatedCashflows::new(&flows)
            .internal_rate_of_return()
            .unwrap();
        assert!(approx(irr.value(), 0.10));
    }

    #[test]
    fn xirr_recovers_a_fractional_year_rate() {
        // (1 + r)^0.5 = 1.05  ⇒  1 + r = 1.1025  ⇒  r = 0.1025.
        let flows = [flow(0.0, -100.0), flow(0.5, 105.0)];
        let irr = DatedCashflows::new(&flows)
            .internal_rate_of_return()
            .unwrap();
        assert!(approx(irr.value(), 0.1025));
        // …and discounting at that rate zeroes the XNPV.
        let npv = DatedCashflows::new(&flows).net_present_value(irr).unwrap();
        assert!(approx(npv.value(), 0.0));
    }

    #[test]
    fn xirr_matches_the_excel_reference() {
        // Microsoft's XIRR example (values on ACT/365 year-offsets from the first
        // date 2008-01-01): dates 2008-03-01, 2008-10-30, 2009-02-15, 2009-04-01
        // are 60, 303, 411, 456 days out. Excel returns 0.373362535.
        let flows = [
            flow(0.0, -10_000.0),
            flow(60.0 / 365.0, 2_750.0),
            flow(303.0 / 365.0, 4_250.0),
            flow(411.0 / 365.0, 3_250.0),
            flow(456.0 / 365.0, 2_750.0),
        ];
        let irr = DatedCashflows::new(&flows)
            .internal_rate_of_return()
            .unwrap();
        assert!(within(irr.value() - 0.373_362_535, 1e-5));
        // The located rate zeroes the XNPV.
        let npv = DatedCashflows::new(&flows).net_present_value(irr).unwrap();
        assert!(within(npv.value(), 1e-3));
    }

    #[test]
    fn xirr_is_invariant_to_shifting_the_reference() {
        // Rebasing to the first flow means a uniform shift of every offset leaves
        // the rate unchanged.
        let base = [flow(0.0, -100.0), flow(0.5, 40.0), flow(1.25, 80.0)];
        let shifted = [flow(10.0, -100.0), flow(10.5, 40.0), flow(11.25, 80.0)];
        let a = DatedCashflows::new(&base)
            .internal_rate_of_return()
            .unwrap();
        let b = DatedCashflows::new(&shifted)
            .internal_rate_of_return()
            .unwrap();
        assert!(approx(a.value(), b.value()));
    }

    #[test]
    fn xirr_falls_back_to_bisection_from_a_bad_guess() {
        let flows = [flow(0.0, -100.0), flow(0.5, 105.0)];
        let irr = DatedCashflows::new(&flows)
            .internal_rate_of_return_from(1e6)
            .unwrap();
        assert!(approx(irr.value(), 0.1025));
    }

    #[test]
    fn empty_xnpv_is_zero_and_xirr_errors() {
        let empty: [DatedCashflow; 0] = [];
        let series = DatedCashflows::new(&empty);
        assert_eq!(series.net_present_value(annual(0.05)).unwrap(), Money::ZERO);
        assert_eq!(
            series.internal_rate_of_return(),
            Err(TvmError::EmptyCashflows)
        );
    }

    #[test]
    fn all_inflows_have_no_xirr() {
        let flows = [flow(0.0, 100.0), flow(0.5, 60.0), flow(1.0, 60.0)];
        assert_eq!(
            DatedCashflows::new(&flows).internal_rate_of_return(),
            Err(TvmError::IrrDidNotConverge)
        );
    }

    #[test]
    fn a_flow_before_the_reference_compounds_forward() {
        // First flow is the reference at t=0; an earlier-dated flow gets a negative
        // rebased offset, so it is compounded (not discounted). -100 at reference,
        // and a +? one half-year *earlier*: with the second flow at offset -0.5.
        let flows = [flow(0.0, -100.0), flow(-0.5, 105.0)];
        // XNPV(r) = -100 + 105·(1+r)^{0.5}; zero at (1+r)^0.5 = 100/105.
        let irr = DatedCashflows::new(&flows)
            .internal_rate_of_return()
            .unwrap();
        let base = 100.0 / 105.0;
        let expected = base * base - 1.0; // (1+r) = (100/105)^2
        assert!(approx(irr.value(), expected));
    }

    #[test]
    fn non_finite_offset_is_rejected() {
        assert_eq!(
            DatedCashflow::new(f64::INFINITY, Money::new(1.0).unwrap()),
            Err(TvmError::NonFiniteOffset)
        );
        assert_eq!(
            DatedCashflow::new(f64::NAN, Money::new(1.0).unwrap()),
            Err(TvmError::NonFiniteOffset)
        );
    }

    #[test]
    fn accessors_round_trip() {
        let cf = flow(1.5, -42.0);
        assert!(approx(cf.offset_years(), 1.5));
        assert_eq!(cf.amount(), Money::new(-42.0).unwrap());

        let flows = [cf];
        let series = DatedCashflows::new(&flows);
        assert_eq!(series.len(), 1);
        assert!(!series.is_empty());
        assert_eq!(series.as_slice(), &flows);
    }
}
