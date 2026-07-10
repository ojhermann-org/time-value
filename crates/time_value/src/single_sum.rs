//! Present and future value of a single amount.
//!
//! Both functions compound with `powf`; on extreme rate/period magnitudes the
//! result can overflow to a non-finite [`Money`] (see its docs).

use crate::math::powf;
use crate::{Money, Period, Periodicity, Rate, TvmError};

/// The present value of a single `future` amount received `periods` periods from
/// now, discounted at `rate`: `PV = FV / (1 + r)ⁿ`.
///
/// Unlike the discrete [`Cashflows`](crate::Cashflows) operations, this admits a
/// **fractional** number of periods, so it needs `powf` and is available only
/// with the `std` or `libm` feature
/// (`docs/adr/0014-transcendental-single-sum-operations.md`).
///
/// # Examples
///
/// ```
/// use time_value::{single_sum, Money, Monthly, Period, Rate};
///
/// // 1000 a year out, at 1% per month, is worth ~887.45 today.
/// let pv = single_sum::present_value(
///     Rate::<Monthly>::new(0.01)?,
///     Period::new(12.0)?,
///     Money::new(1000.0)?,
/// )?;
/// assert!((pv.value() - 887.449).abs() < 1e-3);
/// # Ok::<(), time_value::TvmError>(())
/// ```
///
/// # Errors
///
/// [`TvmError::NonFiniteResult`] if the
/// discounting overflows to a non-finite value on extreme rate/period magnitudes
/// (ADR-0021).
pub fn present_value<P: Periodicity>(
    rate: Rate<P>,
    periods: Period,
    future: Money,
) -> Result<Money, TvmError> {
    let growth = powf(1.0 + rate.value(), periods.value());
    Money::from_operation(future.value() / growth)
}

/// The future value of a single `present` amount after `periods` periods,
/// compounded at `rate`: `FV = PV (1 + r)ⁿ`.
///
/// Admits a **fractional** number of periods; available with the `std` or `libm`
/// feature (`docs/adr/0014-transcendental-single-sum-operations.md`).
///
/// # Examples
///
/// ```
/// use time_value::{single_sum, Money, Monthly, Period, Rate};
///
/// // 1000 today, at 1% per month for a year, grows to ~1126.83.
/// let fv = single_sum::future_value(
///     Rate::<Monthly>::new(0.01)?,
///     Period::new(12.0)?,
///     Money::new(1000.0)?,
/// )?;
/// assert!((fv.value() - 1126.825).abs() < 1e-3);
/// # Ok::<(), time_value::TvmError>(())
/// ```
///
/// # Errors
///
/// [`TvmError::NonFiniteResult`] if the
/// compounding overflows to a non-finite value on extreme rate/period magnitudes
/// (ADR-0021).
pub fn future_value<P: Periodicity>(
    rate: Rate<P>,
    periods: Period,
    present: Money,
) -> Result<Money, TvmError> {
    let growth = powf(1.0 + rate.value(), periods.value());
    Money::from_operation(present.value() * growth)
}

#[cfg(test)]
mod tests {
    use super::{future_value, present_value};
    use crate::{Money, Monthly, Period, Rate, TvmError};

    /// `no_std`-safe approximate equality (no `f64::abs`).
    fn approx(a: f64, b: f64) -> bool {
        let d = a - b;
        d < 1e-6 && d > -1e-6
    }

    fn setup() -> (Rate<Monthly>, Period, Money) {
        (
            Rate::<Monthly>::new(0.01).unwrap(),
            Period::new(12.0).unwrap(),
            Money::new(1000.0).unwrap(),
        )
    }

    #[test]
    fn present_and_future_value_are_inverses() {
        let (rate, periods, amount) = setup();
        let fv = future_value(rate, periods, amount).unwrap();
        let back = present_value(rate, periods, fv).unwrap();
        assert!(approx(back.value(), amount.value()));
    }

    #[test]
    fn future_value_matches_manual_compounding() {
        let (rate, periods, amount) = setup();
        let mut expected = amount.value();
        for _ in 0..12 {
            expected *= 1.01;
        }
        assert!(approx(
            future_value(rate, periods, amount).unwrap().value(),
            expected
        ));
    }

    #[test]
    fn zero_periods_is_the_amount_itself() {
        let (rate, _, amount) = setup();
        let now = Period::ZERO;
        assert!(approx(
            present_value(rate, now, amount).unwrap().value(),
            amount.value()
        ));
        assert!(approx(
            future_value(rate, now, amount).unwrap().value(),
            amount.value()
        ));
    }

    #[test]
    fn present_value_below_face_for_positive_rate() {
        let (rate, periods, amount) = setup();
        assert!(present_value(rate, periods, amount).unwrap().value() < amount.value());
    }

    #[test]
    fn future_value_overflow_is_a_non_finite_result() {
        // 2^2000 is well past f64::MAX, so compounding overflows — an error, not
        // a silent `inf` (ADR-0021).
        let rate = Rate::<Monthly>::new(1.0).unwrap(); // 100% per period
        let periods = Period::new(2000.0).unwrap();
        let amount = Money::new(1e6).unwrap();
        assert_eq!(
            future_value(rate, periods, amount),
            Err(TvmError::NonFiniteResult)
        );
    }
}
