//! [`Cashflows`] — a periodicity-tagged cashflow series and the discrete
//! operations over it.

use core::marker::PhantomData;

#[cfg(any(feature = "std", feature = "libm"))]
use crate::math::powf;
use crate::root::{self, abs};
use crate::{Money, Periodicity, Rate, TvmError};

/// A periodicity-tagged series of cashflows at consecutive periods `0, 1, 2, …`.
///
/// `flows[t]` is the cashflow at period `t`; period `0` is "now" and is not
/// discounted. Cashflows are signed (outflow negative, inflow positive).
///
/// `Cashflows` **borrows** its underlying slice rather than owning a `Vec`, so
/// it stays `no_std` and allocation-free (see
/// `docs/adr/0013-core-api-values-and-discrete-operations.md`). The periodicity
/// tag `P` ties it to a matching [`Rate<P>`] in every discounting operation.
///
/// # Examples
///
/// ```
/// use time_value::{Cashflows, Money, Monthly, Rate};
///
/// let flows = [Money::new(-100.0)?, Money::new(60.0)?, Money::new(60.0)?];
/// let series = Cashflows::<Monthly>::new(&flows);
/// let rate = Rate::<Monthly>::new(0.01)?;
///
/// let npv = series.net_present_value(rate)?;
/// assert!((npv.value() - 18.2237).abs() < 1e-3);
/// # Ok::<(), time_value::TvmError>(())
/// ```
///
/// A rate of the wrong periodicity does not type-check:
///
/// ```compile_fail
/// use time_value::{Annual, Cashflows, Money, Monthly, Rate};
///
/// let flows = [Money::new(-100.0).unwrap(), Money::new(60.0).unwrap()];
/// let series = Cashflows::<Monthly>::new(&flows);
/// let annual = Rate::<Annual>::new(0.05).unwrap();
/// let _ = series.net_present_value(annual); // mismatched periodicity — won't compile
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cashflows<'a, P: Periodicity> {
    flows: &'a [Money],
    marker: PhantomData<P>,
}

impl<'a, P: Periodicity> Cashflows<'a, P> {
    /// Wraps a slice of cashflows; `flows[t]` occurs at period `t`.
    #[must_use]
    pub const fn new(flows: &'a [Money]) -> Self {
        Self {
            flows,
            marker: PhantomData,
        }
    }

    /// The underlying cashflows.
    #[must_use]
    pub const fn as_slice(self) -> &'a [Money] {
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

    /// The net present value of the series discounted at `rate`.
    ///
    /// `NPV = Σₜ CFₜ / (1 + r)ᵗ`, evaluated with only elementary arithmetic (no
    /// transcendental functions), so it is available in the default `no_std`,
    /// dependency-free build. An **empty** series has value `0` (nothing to
    /// discount).
    ///
    /// Evaluated by Horner's method from the last cashflow — `CF₀ + d(CF₁ + d(CF₂ + …))`
    /// with `d = 1/(1+r)` — which keeps every partial bounded by the cashflow
    /// magnitudes.
    ///
    /// # Errors
    ///
    /// [`TvmError::Overflow`] if the sum overflows to a non-finite value,
    /// which needs cashflows near `f64::MAX` or a rate a hair above `−100%`
    /// (ADR-0021).
    pub fn net_present_value(self, rate: Rate<P>) -> Result<Money, TvmError> {
        let discount = 1.0 / (1.0 + rate.value());
        let mut acc = 0.0;
        for cf in self.flows.iter().rev() {
            acc = acc * discount + cf.value();
        }
        Money::from_operation(acc)
    }

    /// The net future value of the series at its final period, compounded at
    /// `rate`.
    ///
    /// `NFV = Σₜ CFₜ (1 + r)ⁿ⁻¹⁻ᵗ` for a series of `n` cashflows, evaluated by
    /// Horner's method — again arithmetic-only. An **empty** series has value `0`.
    ///
    /// # Errors
    ///
    /// [`TvmError::Overflow`] if the compounded sum overflows to a
    /// non-finite value (ADR-0021).
    pub fn net_future_value(self, rate: Rate<P>) -> Result<Money, TvmError> {
        let growth = 1.0 + rate.value();
        let mut acc = 0.0;
        for cf in self.flows {
            acc = acc * growth + cf.value();
        }
        Money::from_operation(acc)
    }

    /// The internal rate of return: the [`Rate<P>`] at which the series' net
    /// present value is zero, from a default initial guess of 10% per period.
    ///
    /// # Errors
    ///
    /// See [`internal_rate_of_return_from`](Self::internal_rate_of_return_from).
    pub fn internal_rate_of_return(self) -> Result<Rate<P>, TvmError> {
        self.internal_rate_of_return_from(0.1)
    }

    /// The internal rate of return, seeding the solver with `guess` (a per-period
    /// rate).
    ///
    /// It first tries **Newton–Raphson** from `guess` — fast, and it converges to
    /// the root nearest the guess, which lets a caller steer toward the intended
    /// one when a non-conventional series has several. If Newton wanders off (a
    /// poor guess, a flat derivative, or an iterate that leaves the valid domain),
    /// it falls back to a **bracketing search**: it scans the rate domain for a
    /// sign change in the NPV and bisects it, so a root is found whenever one
    /// exists. The fallback returns the lowest bracketed root. Both methods are
    /// arithmetic-only (integer powers of the discount factor), so IRR stays in
    /// the default `no_std` build.
    ///
    /// # Errors
    ///
    /// - [`TvmError::EmptyCashflows`] if the series is empty.
    /// - [`TvmError::IrrDidNotConverge`] if neither method finds a root — in
    ///   particular when the NPV never changes sign over the valid rate domain,
    ///   so the series has no real IRR (e.g. cashflows that are all one sign).
    pub fn internal_rate_of_return_from(self, guess: f64) -> Result<Rate<P>, TvmError> {
        if self.flows.is_empty() {
            return Err(TvmError::EmptyCashflows);
        }
        // Scale the convergence tolerance by `Σ|CFₜ|` (an upper bound on `|NPV|`):
        // an absolute tolerance would be unreachable for a series measured in
        // millions (ADR-0021). Try Newton from `guess`, then the robust bracketing
        // fallback (ADR-0020) — both shared with XIRR via the `root` module.
        let tolerance = root::relative_tolerance(self.magnitude());
        match root::newton(|r| self.npv_and_derivative(r), guess, tolerance)
            .or_else(|| root::bracket_and_bisect(|r| self.npv_at(r), tolerance))
        {
            Some(rate) => Rate::new(rate),
            None => Err(TvmError::IrrDidNotConverge),
        }
    }

    /// `Σ|CFₜ|` — an upper bound on `|NPV|`, used to scale the solver tolerance.
    fn magnitude(self) -> f64 {
        let mut scale = 0.0;
        for cf in self.flows {
            scale += abs(cf.value());
        }
        scale
    }

    /// The NPV at a candidate per-period `rate` (no derivative), accumulated in
    /// one pass: `NPV(r) = Σₜ CFₜ (1+r)⁻ᵗ`.
    fn npv_at(self, rate: f64) -> f64 {
        let discount = 1.0 / (1.0 + rate);
        let mut factor = 1.0; // discountᵗ
        let mut npv = 0.0;
        for cf in self.flows {
            npv += cf.value() * factor;
            factor *= discount;
        }
        npv
    }

    /// NPV and its derivative d(NPV)/dr at a candidate per-period `rate`.
    ///
    /// `NPV(r)  = Σₜ CFₜ (1+r)⁻ᵗ`, `NPV'(r) = Σₜ −t·CFₜ (1+r)⁻ᵗ⁻¹`. Both are
    /// accumulated in one pass over the series.
    fn npv_and_derivative(self, rate: f64) -> (f64, f64) {
        let discount = 1.0 / (1.0 + rate);
        let mut factor = 1.0; // discountᵗ
        let mut npv = 0.0;
        let mut derivative = 0.0;
        let mut t = 0.0;
        for cf in self.flows {
            let amount = cf.value();
            npv += amount * factor;
            derivative += -t * amount * factor * discount;
            factor *= discount;
            t += 1.0;
        }
        (npv, derivative)
    }
}

/// The modified internal rate of return — a transcendental cashflow operation, so
/// behind the `std` / `libm` features (ADR-0026), unlike the arithmetic-only
/// NPV/NFV/IRR above.
#[cfg(any(feature = "std", feature = "libm"))]
impl<P: Periodicity> Cashflows<'_, P> {
    /// The **modified** internal rate of return: the per-period rate at which the
    /// present value of the series' outflows grows to the future value of its
    /// inflows over the series' life.
    ///
    /// Unlike [`internal_rate_of_return`](Self::internal_rate_of_return), MIRR is
    /// unique. It discounts the **outflows** (negative cashflows) back to period
    /// `0` at `finance_rate`, compounds the **inflows** (positive cashflows)
    /// forward to the final period at `reinvestment_rate`, and returns the single
    /// rate equating the two: `MIRR = (TVᵢₙ / −PVₒᵤₜ)^(1/N) − 1` for a series
    /// spanning `N` periods (the index of its last cashflow). This resolves the
    /// multiple-root ambiguity a non-conventional series gives plain IRR
    /// (`docs/adr/0020-robust-irr-newton-with-bisection-fallback.md`), at the cost
    /// of two explicit rate assumptions instead of none.
    ///
    /// The two accumulations are arithmetic-only, but the terminal `N`-th root
    /// needs `powf`, which is why this operation is feature-gated (ADR-0026). All
    /// three rates share the periodicity `P`.
    ///
    /// # Examples
    ///
    /// ```
    /// use time_value::{Cashflows, Money, Monthly, Rate};
    ///
    /// // Pay 1000 now and 500 next month, then receive 800 and 900.
    /// let flows = [
    ///     Money::new(-1000.0)?,
    ///     Money::new(-500.0)?,
    ///     Money::new(800.0)?,
    ///     Money::new(900.0)?,
    /// ];
    /// let project = Cashflows::<Monthly>::new(&flows);
    ///
    /// let mirr = project.modified_internal_rate_of_return(
    ///     Rate::<Monthly>::new(0.10)?, // finance rate for the outflows
    ///     Rate::<Monthly>::new(0.12)?, // reinvestment rate for the inflows
    /// )?;
    /// assert!((mirr.value() - 0.072819).abs() < 1e-5);
    /// # Ok::<(), time_value::TvmError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// - [`TvmError::EmptyCashflows`] if the series is empty.
    /// - [`TvmError::Undefined`] if the series has fewer than two cashflows (so
    ///   `N = 0`: no span to annualise over), or has no outflows to discount (a
    ///   zero present value to grow from) — both are degenerate with no answer.
    /// - [`TvmError::Overflow`] if the terminal value overflows on extreme
    ///   magnitudes.
    /// - [`TvmError::RateOutOfRange`] if the series has no inflows — the terminal
    ///   value is zero, so the implied rate is `−100%`.
    pub fn modified_internal_rate_of_return(
        self,
        finance_rate: Rate<P>,
        reinvestment_rate: Rate<P>,
    ) -> Result<Rate<P>, TvmError> {
        if self.flows.is_empty() {
            return Err(TvmError::EmptyCashflows);
        }
        if self.flows.len() < 2 {
            // A single cashflow spans no periods, so there is nothing to annualise.
            return Err(TvmError::Undefined);
        }

        let finance_discount = 1.0 / (1.0 + finance_rate.value());
        let reinvest_growth = 1.0 + reinvestment_rate.value();
        let reinvest_discount = 1.0 / reinvest_growth;

        // Present value of the outflows at period 0 (finance rate), and the inflows
        // discounted to period 0 at the reinvestment rate — compounded up to the
        // final period below. Factors run alongside a float period counter, so no
        // `usize as f64` cast is needed.
        let mut present_outflows = 0.0; // ≤ 0
        let mut discounted_inflows = 0.0; // Σ inflow · reinvest_growth⁻ᵗ
        let mut finance_factor = 1.0; // finance_discount ᵗ
        let mut reinvest_factor = 1.0; // reinvest_discount ᵗ
        let mut periods = 0.0;
        for cf in self.flows {
            let amount = cf.value();
            if amount < 0.0 {
                present_outflows += amount * finance_factor;
            } else if amount > 0.0 {
                discounted_inflows += amount * reinvest_factor;
            }
            finance_factor *= finance_discount;
            reinvest_factor *= reinvest_discount;
            periods += 1.0;
        }
        let n = periods - 1.0; // index of the last cashflow, ≥ 1 here

        if present_outflows == 0.0 {
            // No outflows: there is no present value to grow from, so the
            // annualised return is undefined rather than merely too large.
            return Err(TvmError::Undefined);
        }

        // Compound the inflows forward to period n, then take the n-th root of the
        // growth from the outflows' present value to the inflows' terminal value.
        let terminal_inflows = discounted_inflows * powf(reinvest_growth, n);
        let growth = terminal_inflows / -present_outflows;
        Rate::from_operation(powf(growth, 1.0 / n) - 1.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::root::within;
    use crate::{Cashflows, Money, Monthly, Rate, TvmError};

    /// `no_std`-safe approximate equality for the tests (no `f64::abs`).
    fn approx(a: f64, b: f64) -> bool {
        within(a - b, 1e-6)
    }

    fn money(values: &[f64]) -> [Money; 3] {
        assert_eq!(values.len(), 3);
        [
            Money::new(values[0]).unwrap(),
            Money::new(values[1]).unwrap(),
            Money::new(values[2]).unwrap(),
        ]
    }

    #[test]
    fn npv_matches_manual_sum() {
        let flows = money(&[-100.0, 60.0, 60.0]);
        let series = Cashflows::<Monthly>::new(&flows);
        let rate = Rate::<Monthly>::new(0.01).unwrap();
        let expected = -100.0 + 60.0 / 1.01 + 60.0 / (1.01 * 1.01);
        assert!(approx(
            series.net_present_value(rate).unwrap().value(),
            expected
        ));
    }

    #[test]
    fn npv_at_zero_rate_is_the_plain_sum() {
        let flows = money(&[-100.0, 60.0, 60.0]);
        let series = Cashflows::<Monthly>::new(&flows);
        let rate = Rate::<Monthly>::new(0.0).unwrap();
        assert!(approx(
            series.net_present_value(rate).unwrap().value(),
            20.0
        ));
    }

    #[test]
    fn nfv_is_npv_compounded_to_the_final_period() {
        let flows = money(&[-100.0, 60.0, 60.0]);
        let series = Cashflows::<Monthly>::new(&flows);
        let rate = Rate::<Monthly>::new(0.01).unwrap();
        let present = series.net_present_value(rate).unwrap().value();
        let future = series.net_future_value(rate).unwrap().value();
        // NFV = NPV * (1 + r)^(n - 1); here n = 3.
        assert!(approx(future, present * 1.01 * 1.01));
    }

    #[test]
    fn irr_zeroes_the_npv() {
        let flows = money(&[-100.0, 60.0, 60.0]);
        let series = Cashflows::<Monthly>::new(&flows);
        let irr = series.internal_rate_of_return().unwrap();
        // Discounting at the IRR gives (approximately) zero NPV.
        assert!(within(series.net_present_value(irr).unwrap().value(), 1e-6));
        // Closed form: 3x^2 + 3x - 5 = 0, x = 1/(1+r), x = (-3 + sqrt(69))/6,
        // giving r = 0.130662386…
        assert!(approx(irr.value(), 0.130_662_386));
    }

    #[test]
    fn irr_on_empty_series_errors() {
        let flows: [Money; 0] = [];
        let series = Cashflows::<Monthly>::new(&flows);
        assert_eq!(
            series.internal_rate_of_return(),
            Err(TvmError::EmptyCashflows)
        );
    }

    #[test]
    fn irr_without_a_root_does_not_converge() {
        // All inflows: NPV is positive for every rate > -100%, so there is no IRR.
        let flows = money(&[100.0, 60.0, 60.0]);
        let series = Cashflows::<Monthly>::new(&flows);
        assert_eq!(
            series.internal_rate_of_return(),
            Err(TvmError::IrrDidNotConverge)
        );
    }

    #[test]
    fn irr_falls_back_to_bisection_from_a_bad_guess() {
        // A wildly off guess sends Newton out of the valid domain on its first
        // step; the bracketing fallback must still find the same root.
        let flows = money(&[-100.0, 60.0, 60.0]);
        let series = Cashflows::<Monthly>::new(&flows);
        let irr = series.internal_rate_of_return_from(1e6).unwrap();
        assert!(within(series.net_present_value(irr).unwrap().value(), 1e-6));
        assert!(approx(irr.value(), 0.130_662_386));
    }

    #[test]
    fn irr_recovers_a_large_rate() {
        // Root well above the 10% default guess: -1 now, +2 next period is a
        // 100% per-period return. Newton reaches it, but confirm the value.
        let flows = money(&[-1.0, 2.0, 0.0]);
        let series = Cashflows::<Monthly>::new(&flows);
        let irr = series.internal_rate_of_return().unwrap();
        assert!(within(series.net_present_value(irr).unwrap().value(), 1e-6));
        assert!(approx(irr.value(), 1.0));
    }

    #[test]
    fn npv_and_nfv_of_an_empty_series_are_zero() {
        // Convention (ADR-0021): nothing to discount or compound is `Ok(0)`.
        let empty: [Money; 0] = [];
        let series = Cashflows::<Monthly>::new(&empty);
        let rate = Rate::<Monthly>::new(0.05).unwrap();
        assert_eq!(series.net_present_value(rate).unwrap(), Money::ZERO);
        assert_eq!(series.net_future_value(rate).unwrap(), Money::ZERO);
    }

    #[test]
    fn npv_and_nfv_overflow_to_a_non_finite_result() {
        // Two near-max cashflows sum past `f64::MAX`, so both discounted and
        // compounded totals overflow — surfaced as an error, not a silent `inf`.
        let big = [Money::new(f64::MAX).unwrap(), Money::new(f64::MAX).unwrap()];
        let series = Cashflows::<Monthly>::new(&big);
        let rate = Rate::<Monthly>::new(0.0).unwrap();
        assert_eq!(series.net_present_value(rate), Err(TvmError::Overflow));
        assert_eq!(series.net_future_value(rate), Err(TvmError::Overflow));
    }

    #[test]
    fn irr_converges_for_large_magnitude_cashflows() {
        // Millions-scale: an absolute NPV tolerance would be unreachable, but the
        // magnitude-scaled tolerance (ADR-0021) converges to the same rate as the
        // unit-scale version of this series (irr_zeroes_the_npv).
        let flows = money(&[-1_000_000.0, 600_000.0, 600_000.0]);
        let series = Cashflows::<Monthly>::new(&flows);
        let irr = series.internal_rate_of_return().unwrap();
        assert!(approx(irr.value(), 0.130_662_386));
    }

    #[test]
    fn len_and_is_empty() {
        let flows = money(&[-1.0, 2.0, 3.0]);
        let series = Cashflows::<Monthly>::new(&flows);
        assert_eq!(series.len(), 3);
        assert!(!series.is_empty());

        let empty: [Money; 0] = [];
        assert!(Cashflows::<Monthly>::new(&empty).is_empty());
    }

    #[cfg(any(feature = "std", feature = "libm"))]
    mod mirr {
        use super::{approx, Cashflows, Money, Monthly, Rate, TvmError};

        fn rate(r: f64) -> Rate<Monthly> {
            Rate::<Monthly>::new(r).unwrap()
        }

        fn series(values: &[f64]) -> [Money; 4] {
            assert_eq!(values.len(), 4);
            [
                Money::new(values[0]).unwrap(),
                Money::new(values[1]).unwrap(),
                Money::new(values[2]).unwrap(),
                Money::new(values[3]).unwrap(),
            ]
        }

        #[test]
        fn matches_the_manual_formula() {
            // Outflows -1000 (t0) and -500 (t1) discounted at 10% -> PV -1454.5454;
            // inflows 800 (t2) and 900 (t3) compounded to t3 at 12% -> TV 1796;
            // MIRR = (1796 / 1454.5454)^(1/3) - 1 = 0.0728187…
            let flows = series(&[-1000.0, -500.0, 800.0, 900.0]);
            let mirr = Cashflows::<Monthly>::new(&flows)
                .modified_internal_rate_of_return(rate(0.10), rate(0.12))
                .unwrap();
            assert!(approx(mirr.value(), 0.072_818_724_6));
        }

        #[test]
        fn equal_rates_and_a_single_outflow() {
            // One outflow at t0, so the finance rate is inert; 500 each at t1..t3
            // compounded at 10% to t3 gives TV 1655, MIRR = 1.655^(1/3) - 1.
            let flows = series(&[-1000.0, 500.0, 500.0, 500.0]);
            let mirr = Cashflows::<Monthly>::new(&flows)
                .modified_internal_rate_of_return(rate(0.10), rate(0.10))
                .unwrap();
            assert!(approx(mirr.value(), 0.182_858_148_6));
        }

        #[test]
        fn empty_series_errors() {
            let empty: [Money; 0] = [];
            assert_eq!(
                Cashflows::<Monthly>::new(&empty)
                    .modified_internal_rate_of_return(rate(0.10), rate(0.10)),
                Err(TvmError::EmptyCashflows)
            );
        }

        #[test]
        fn single_cashflow_has_no_span() {
            let flows = [Money::new(-1000.0).unwrap()];
            assert_eq!(
                Cashflows::<Monthly>::new(&flows)
                    .modified_internal_rate_of_return(rate(0.10), rate(0.10)),
                Err(TvmError::Undefined)
            );
        }

        #[test]
        fn no_outflows_has_no_present_value_to_grow_from() {
            let flows = series(&[1000.0, 500.0, 500.0, 500.0]);
            assert_eq!(
                Cashflows::<Monthly>::new(&flows)
                    .modified_internal_rate_of_return(rate(0.10), rate(0.10)),
                Err(TvmError::Undefined)
            );
        }

        #[test]
        fn no_inflows_is_a_total_loss() {
            // No positive cashflow, so the terminal value is 0 and MIRR = -100%.
            let flows = series(&[-1000.0, -500.0, -500.0, -500.0]);
            assert_eq!(
                Cashflows::<Monthly>::new(&flows)
                    .modified_internal_rate_of_return(rate(0.10), rate(0.10)),
                Err(TvmError::RateOutOfRange)
            );
        }
    }
}
