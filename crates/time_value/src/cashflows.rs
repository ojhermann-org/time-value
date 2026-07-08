//! [`Cashflows`] — a periodicity-tagged cashflow series and the discrete
//! operations over it.

use core::marker::PhantomData;

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
/// let npv = series.net_present_value(rate);
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
    /// dependency-free build.
    #[must_use]
    pub fn net_present_value(self, rate: Rate<P>) -> Money {
        let discount = 1.0 / (1.0 + rate.value());
        let mut factor = 1.0; // discountᵗ
        let mut acc = 0.0;
        for cf in self.flows {
            acc += cf.value() * factor;
            factor *= discount;
        }
        Money::from_finite(acc)
    }

    /// The net future value of the series at its final period, compounded at
    /// `rate`.
    ///
    /// `NFV = Σₜ CFₜ (1 + r)ⁿ⁻¹⁻ᵗ` for a series of `n` cashflows, evaluated by
    /// Horner's method — again arithmetic-only. An empty series has value `0`.
    #[must_use]
    pub fn net_future_value(self, rate: Rate<P>) -> Money {
        let growth = 1.0 + rate.value();
        let mut acc = 0.0;
        for cf in self.flows {
            acc = acc * growth + cf.value();
        }
        Money::from_finite(acc)
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
        match self.newton(guess).or_else(|| self.bracket_and_bisect()) {
            Some(rate) => Rate::new(rate),
            None => Err(TvmError::IrrDidNotConverge),
        }
    }

    /// Newton–Raphson from `guess`. `None` if it does not reach a root within its
    /// iteration budget, the derivative goes flat, or an iterate leaves the valid
    /// domain (a rate ≤ −100%, or a non-finite value — `is_finite` also rejects
    /// `NaN`, so a diverging iterate fails cleanly rather than looping).
    fn newton(self, guess: f64) -> Option<f64> {
        const MAX_ITERATIONS: u32 = 128;
        const NPV_TOLERANCE: f64 = 1e-9;
        const MIN_DERIVATIVE: f64 = 1e-12;

        let mut rate = guess;
        for _ in 0..MAX_ITERATIONS {
            if !rate.is_finite() || rate <= -1.0 {
                return None;
            }
            let (npv, derivative) = self.npv_and_derivative(rate);
            if within(npv, NPV_TOLERANCE) {
                return Some(rate);
            }
            if within(derivative, MIN_DERIVATIVE) {
                return None;
            }
            rate -= npv / derivative;
        }
        None
    }

    /// Scan the valid rate domain (`r > −1`) for a sign change in the NPV and
    /// bisect the first bracket found. `None` if the NPV never changes sign (no
    /// real IRR). Samples `1 + r` geometrically from just above `0` upward, a
    /// ratio fine enough not to step over a lone root of a conventional series.
    fn bracket_and_bisect(self) -> Option<f64> {
        const NPV_TOLERANCE: f64 = 1e-9;
        const MAX_BISECTIONS: u32 = 200;
        const START: f64 = 1e-4; // 1 + r, i.e. r = -0.9999
        const RATIO: f64 = 1.25;
        const SAMPLES: u32 = 160; // reaches 1 + r ≈ 1e15

        let mut lo = START - 1.0;
        let mut f_lo = self.npv_at(lo);
        let mut growth = START;
        for _ in 0..SAMPLES {
            if within(f_lo, NPV_TOLERANCE) {
                return Some(lo);
            }
            growth *= RATIO;
            let hi = growth - 1.0;
            let f_hi = self.npv_at(hi);
            if opposite_signs(f_lo, f_hi) {
                return Some(bisect(
                    |r| self.npv_at(r),
                    lo,
                    hi,
                    f_lo,
                    NPV_TOLERANCE,
                    MAX_BISECTIONS,
                ));
            }
            lo = hi;
            f_lo = f_hi;
        }
        None
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

/// Bisect for the root of `f` in `[lo, hi]`, where `f` has opposite signs at the
/// ends (`f_lo` is `f(lo)`). Returns as soon as a sample is within `tol` of zero,
/// or the midpoint after `max` steps.
fn bisect(
    f: impl Fn(f64) -> f64,
    mut lo: f64,
    mut hi: f64,
    mut f_lo: f64,
    tol: f64,
    max: u32,
) -> f64 {
    for _ in 0..max {
        let mid = 0.5 * (lo + hi);
        let f_mid = f(mid);
        if within(f_mid, tol) {
            return mid;
        }
        if opposite_signs(f_lo, f_mid) {
            hi = mid;
        } else {
            lo = mid;
            f_lo = f_mid;
        }
    }
    0.5 * (lo + hi)
}

/// `|x| < tolerance`, without `f64::abs` (which is not in `core`).
fn within(x: f64, tolerance: f64) -> bool {
    x < tolerance && x > -tolerance
}

/// Whether `a` and `b` are both non-zero and of opposite sign.
fn opposite_signs(a: f64, b: f64) -> bool {
    (a < 0.0 && b > 0.0) || (a > 0.0 && b < 0.0)
}

#[cfg(test)]
mod tests {
    use super::within;
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
        assert!(approx(series.net_present_value(rate).value(), expected));
    }

    #[test]
    fn npv_at_zero_rate_is_the_plain_sum() {
        let flows = money(&[-100.0, 60.0, 60.0]);
        let series = Cashflows::<Monthly>::new(&flows);
        let rate = Rate::<Monthly>::new(0.0).unwrap();
        assert!(approx(series.net_present_value(rate).value(), 20.0));
    }

    #[test]
    fn nfv_is_npv_compounded_to_the_final_period() {
        let flows = money(&[-100.0, 60.0, 60.0]);
        let series = Cashflows::<Monthly>::new(&flows);
        let rate = Rate::<Monthly>::new(0.01).unwrap();
        let present = series.net_present_value(rate).value();
        let future = series.net_future_value(rate).value();
        // NFV = NPV * (1 + r)^(n - 1); here n = 3.
        assert!(approx(future, present * 1.01 * 1.01));
    }

    #[test]
    fn irr_zeroes_the_npv() {
        let flows = money(&[-100.0, 60.0, 60.0]);
        let series = Cashflows::<Monthly>::new(&flows);
        let irr = series.internal_rate_of_return().unwrap();
        // Discounting at the IRR gives (approximately) zero NPV.
        assert!(within(series.net_present_value(irr).value(), 1e-6));
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
        assert!(within(series.net_present_value(irr).value(), 1e-6));
        assert!(approx(irr.value(), 0.130_662_386));
    }

    #[test]
    fn irr_recovers_a_large_rate() {
        // Root well above the 10% default guess: -1 now, +2 next period is a
        // 100% per-period return. Newton reaches it, but confirm the value.
        let flows = money(&[-1.0, 2.0, 0.0]);
        let series = Cashflows::<Monthly>::new(&flows);
        let irr = series.internal_rate_of_return().unwrap();
        assert!(within(series.net_present_value(irr).value(), 1e-6));
        assert!(approx(irr.value(), 1.0));
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
}
