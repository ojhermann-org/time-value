//! Ordinary (end-of-period) annuities: a fixed payment each period.
//!
//! Every function takes an interest `rate` and a number of `periods`, and models
//! an **ordinary** annuity — payments fall at the *end* of each period. They are
//! available with the `std` or `libm` feature, like the single-sum operations
//! (`docs/adr/0014-transcendental-single-sum-operations.md`), and handle the
//! `r → 0` limit, where the annuity factors collapse to `n`
//! (`docs/adr/0015-annuities.md`). The factors compound with `powf`, so on
//! extreme rate/period magnitudes a value can overflow to a non-finite
//! [`Money`](crate::Money) (see its docs).

use crate::math::powf;
use crate::{Money, Period, Periodicity, Rate, TvmError};

/// Rate magnitude below which the `r → 0` limit is used instead of the closed
/// form (which is `0/0` at exactly `r = 0` and ill-conditioned near it).
const RATE_NEAR_ZERO: f64 = 1e-9;

fn near_zero(x: f64) -> bool {
    x < RATE_NEAR_ZERO && x > -RATE_NEAR_ZERO
}

/// The present-value annuity factor `(1 - (1 + r)⁻ⁿ) / r`, taking the limit `n`
/// as `r → 0`.
fn present_value_factor(rate: f64, periods: f64) -> f64 {
    if near_zero(rate) {
        periods
    } else {
        (1.0 - powf(1.0 + rate, -periods)) / rate
    }
}

/// The future-value annuity factor `((1 + r)ⁿ - 1) / r`, taking the limit `n` as
/// `r → 0`.
fn future_value_factor(rate: f64, periods: f64) -> f64 {
    if near_zero(rate) {
        periods
    } else {
        (powf(1.0 + rate, periods) - 1.0) / rate
    }
}

/// The present value of an ordinary annuity that pays `payment` at the end of
/// each of `periods` periods, discounted at `rate`.
///
/// `PV = PMT · (1 - (1 + r)⁻ⁿ) / r`, or `PV = PMT · n` when `r = 0`.
///
/// # Examples
///
/// ```
/// use time_value::{annuity, Money, Monthly, Period, Rate};
///
/// // 100 at the end of each month for a year, at 1% per month.
/// let pv = annuity::present_value(
///     Rate::<Monthly>::new(0.01)?,
///     Period::new(12.0)?,
///     Money::new(100.0)?,
/// )?;
/// assert!((pv.value() - 1125.508).abs() < 1e-2);
/// # Ok::<(), time_value::TvmError>(())
/// ```
///
/// # Errors
///
/// [`TvmError::NonFiniteResult`] if the discounted sum overflows to a non-finite
/// value on extreme rate/period magnitudes (ADR-0021).
pub fn present_value<P: Periodicity>(
    rate: Rate<P>,
    periods: Period,
    payment: Money,
) -> Result<Money, TvmError> {
    Money::from_operation(payment.value() * present_value_factor(rate.value(), periods.value()))
}

/// The future value of an ordinary annuity that pays `payment` at the end of
/// each of `periods` periods, compounded at `rate`.
///
/// `FV = PMT · ((1 + r)ⁿ - 1) / r`, or `FV = PMT · n` when `r = 0`.
///
/// # Examples
///
/// ```
/// use time_value::{annuity, Money, Monthly, Period, Rate};
///
/// let fv = annuity::future_value(
///     Rate::<Monthly>::new(0.01)?,
///     Period::new(12.0)?,
///     Money::new(100.0)?,
/// )?;
/// assert!((fv.value() - 1268.250).abs() < 1e-2);
/// # Ok::<(), time_value::TvmError>(())
/// ```
///
/// # Errors
///
/// [`TvmError::NonFiniteResult`] if the compounded sum overflows to a non-finite
/// value on extreme rate/period magnitudes (ADR-0021).
pub fn future_value<P: Periodicity>(
    rate: Rate<P>,
    periods: Period,
    payment: Money,
) -> Result<Money, TvmError> {
    Money::from_operation(payment.value() * future_value_factor(rate.value(), periods.value()))
}

/// The level payment that amortises a `present` value over `periods` periods at
/// `rate` — the inverse of [`present_value`].
///
/// `PMT = PV · r / (1 - (1 + r)⁻ⁿ)`, or `PMT = PV / n` when `r = 0`.
///
/// # Examples
///
/// ```
/// use time_value::{annuity, Money, Monthly, Period, Rate};
///
/// // Amortise a 1125.508 loan over a year at 1% per month -> ~100 per month.
/// let pmt = annuity::payment(
///     Rate::<Monthly>::new(0.01)?,
///     Period::new(12.0)?,
///     Money::new(1125.508)?,
/// )?;
/// assert!((pmt.value() - 100.0).abs() < 1e-2);
/// # Ok::<(), time_value::TvmError>(())
/// ```
///
/// # Errors
///
/// Returns [`TvmError::NonFiniteResult`] if the amortisation is degenerate — in
/// particular when `periods` is zero, so there is nothing to amortise over and
/// the payment is undefined (the factor is `0`, so the division is non-finite) —
/// or if it overflows on extreme magnitudes (ADR-0021).
pub fn payment<P: Periodicity>(
    rate: Rate<P>,
    periods: Period,
    present: Money,
) -> Result<Money, TvmError> {
    let factor = present_value_factor(rate.value(), periods.value());
    Money::from_operation(present.value() / factor)
}

#[cfg(test)]
mod tests {
    use crate::{annuity, Money, Monthly, Period, Rate, TvmError};

    /// `no_std`-safe approximate equality (no `f64::abs`).
    fn approx(a: f64, b: f64, tolerance: f64) -> bool {
        let d = a - b;
        d < tolerance && d > -tolerance
    }

    fn rate(r: f64) -> Rate<Monthly> {
        Rate::<Monthly>::new(r).unwrap()
    }

    #[test]
    fn present_value_matches_closed_form() {
        let pv = annuity::present_value(
            rate(0.01),
            Period::new(12.0).unwrap(),
            Money::new(100.0).unwrap(),
        )
        .unwrap();
        assert!(approx(pv.value(), 1125.508, 1e-2));
    }

    #[test]
    fn payment_inverts_present_value() {
        let payment = Money::new(100.0).unwrap();
        let periods = Period::new(24.0).unwrap();
        let pv = annuity::present_value(rate(0.015), periods, payment).unwrap();
        let recovered = annuity::payment(rate(0.015), periods, pv).unwrap();
        assert!(approx(recovered.value(), payment.value(), 1e-9));
    }

    #[test]
    fn future_value_is_present_value_compounded() {
        let periods = Period::new(12.0).unwrap();
        let pv = annuity::present_value(rate(0.01), periods, Money::new(100.0).unwrap()).unwrap();
        let fv = annuity::future_value(rate(0.01), periods, Money::new(100.0).unwrap()).unwrap();
        // FV = PV * (1 + r)^n; compound manually to avoid needing powf here.
        let mut growth = 1.0;
        for _ in 0..12 {
            growth *= 1.01;
        }
        assert!(approx(fv.value(), pv.value() * growth, 1e-6));
    }

    #[test]
    fn zero_rate_uses_the_limit() {
        let periods = Period::new(10.0).unwrap();
        let payment = Money::new(50.0).unwrap();
        // At r = 0 both factors are n, so PV = FV = payment * n.
        assert!(approx(
            annuity::present_value(rate(0.0), periods, payment)
                .unwrap()
                .value(),
            500.0,
            1e-9,
        ));
        assert!(approx(
            annuity::future_value(rate(0.0), periods, payment)
                .unwrap()
                .value(),
            500.0,
            1e-9,
        ));
    }

    #[test]
    fn payment_over_zero_periods_is_degenerate() {
        let result = annuity::payment(rate(0.01), Period::ZERO, Money::new(1000.0).unwrap());
        assert_eq!(result, Err(TvmError::NonFiniteResult));
    }
}
