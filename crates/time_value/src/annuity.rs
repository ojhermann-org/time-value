//! Annuities: a fixed payment each period.
//!
//! The top-level functions model an **ordinary** annuity — payments fall at the
//! *end* of each period, the default in finance and the basis of loan
//! amortisation. The [`due`] submodule mirrors them for an **annuity-due**
//! (payments at the *start* of each period), whose factors are the ordinary
//! factors scaled by `(1 + r)`. [`perpetuity`] and [`growing_perpetuity`] give
//! the present value of a payment that continues forever.
//!
//! Every function takes an interest `rate`; the dated ones also take a number of
//! `periods`. They are available with the `std` or `libm` feature, like the
//! single-sum operations (`docs/adr/0014-transcendental-single-sum-operations.md`),
//! and handle the `r → 0` limit, where the annuity factors collapse to `n`
//! (`docs/adr/0015-annuities.md`). The factors compound with `powf`, so on
//! extreme rate/period magnitudes a value can overflow to a non-finite
//! [`Money`] (see its docs). A perpetuity instead diverges when its rate does not
//! exceed its growth rate, which its constructors reject.

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

/// The present value of a **level perpetuity** — a `payment` at the end of every
/// period, forever — discounted at `rate`.
///
/// `PV = PMT / r`. The sum converges only when `r > 0`; a non-positive rate makes
/// the series diverge, so it is rejected rather than returning the finite-looking
/// `PMT / r`. This is the `g = 0` case of [`growing_perpetuity`].
///
/// # Examples
///
/// ```
/// use time_value::{annuity, Money, Monthly, Rate};
///
/// // 100 at the end of every month, forever, discounted at 5% per month.
/// let pv = annuity::perpetuity(Rate::<Monthly>::new(0.05)?, Money::new(100.0)?)?;
/// assert!((pv.value() - 2000.0).abs() < 1e-9);
/// # Ok::<(), time_value::TvmError>(())
/// ```
///
/// # Errors
///
/// Returns [`TvmError::DivergentPerpetuity`] if `rate` is not strictly positive
/// (the present value diverges), or [`TvmError::NonFiniteResult`] if the division
/// overflows on extreme magnitudes (ADR-0021).
pub fn perpetuity<P: Periodicity>(rate: Rate<P>, payment: Money) -> Result<Money, TvmError> {
    growing_perpetuity(rate, Rate::from_valid(0.0), payment)
}

/// The present value of a **growing perpetuity** — a payment at the end of every
/// period, forever, growing at `growth` each period — discounted at `rate`.
///
/// `PV = PMT / (r - g)`, where `PMT` is the *first* payment (one period from now)
/// and `g` is the per-period growth rate. The sum converges only when `r > g`; if
/// `r <= g` the series diverges (`r = g` gives an infinity, `r < g` a finite but
/// meaningless value), so it is rejected. `rate` and `growth` share the
/// periodicity `P`, so mixing a monthly rate with an annual growth is a compile
/// error.
///
/// # Examples
///
/// ```
/// use time_value::{annuity, Money, Monthly, Rate};
///
/// // First payment 100 at month end, growing 2%/month, discounted at 5%/month.
/// let pv = annuity::growing_perpetuity(
///     Rate::<Monthly>::new(0.05)?,
///     Rate::<Monthly>::new(0.02)?,
///     Money::new(100.0)?,
/// )?;
/// assert!((pv.value() - 3333.333).abs() < 1e-3); // 100 / (0.05 - 0.02)
/// # Ok::<(), time_value::TvmError>(())
/// ```
///
/// # Errors
///
/// Returns [`TvmError::DivergentPerpetuity`] if `rate <= growth` (the present
/// value diverges), or [`TvmError::NonFiniteResult`] if the division overflows on
/// extreme magnitudes (ADR-0021).
pub fn growing_perpetuity<P: Periodicity>(
    rate: Rate<P>,
    growth: Rate<P>,
    payment: Money,
) -> Result<Money, TvmError> {
    if rate.value() <= growth.value() {
        return Err(TvmError::DivergentPerpetuity);
    }
    Money::from_operation(payment.value() / (rate.value() - growth.value()))
}

/// Annuity-due variants: a fixed payment at the *start* of each period.
///
/// These mirror the ordinary (end-of-period) functions in the parent module —
/// same signatures, same `r → 0` and degenerate-`n` handling — but each factor is
/// scaled by `(1 + r)`, because every payment is brought forward one period.
/// `PV_due = PV · (1 + r)`, `FV_due = FV · (1 + r)`, and [`payment`](self::payment)
/// inverts `present_value` here just as the ordinary `payment` inverts the
/// ordinary `present_value` (`docs/adr/0015-annuities.md`).
pub mod due {
    use super::{future_value_factor, present_value_factor};
    use crate::{Money, Period, Periodicity, Rate, TvmError};

    /// The present value of an annuity-due that pays `payment` at the *start* of
    /// each of `periods` periods, discounted at `rate`.
    ///
    /// `PV = PMT · (1 + r) · (1 - (1 + r)⁻ⁿ) / r`, or `PV = PMT · n` when `r = 0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use time_value::{annuity, Money, Monthly, Period, Rate};
    ///
    /// // 100 at the start of each month for a year, at 1% per month.
    /// let pv = annuity::due::present_value(
    ///     Rate::<Monthly>::new(0.01)?,
    ///     Period::new(12.0)?,
    ///     Money::new(100.0)?,
    /// )?;
    /// assert!((pv.value() - 1136.763).abs() < 1e-2); // ordinary 1125.508 × 1.01
    /// # Ok::<(), time_value::TvmError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`TvmError::NonFiniteResult`] if the discounted sum overflows to a
    /// non-finite value on extreme rate/period magnitudes (ADR-0021).
    pub fn present_value<P: Periodicity>(
        rate: Rate<P>,
        periods: Period,
        payment: Money,
    ) -> Result<Money, TvmError> {
        let factor = present_value_factor(rate.value(), periods.value()) * (1.0 + rate.value());
        Money::from_operation(payment.value() * factor)
    }

    /// The future value of an annuity-due that pays `payment` at the *start* of
    /// each of `periods` periods, compounded at `rate`.
    ///
    /// `FV = PMT · (1 + r) · ((1 + r)ⁿ - 1) / r`, or `FV = PMT · n` when `r = 0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use time_value::{annuity, Money, Monthly, Period, Rate};
    ///
    /// let fv = annuity::due::future_value(
    ///     Rate::<Monthly>::new(0.01)?,
    ///     Period::new(12.0)?,
    ///     Money::new(100.0)?,
    /// )?;
    /// assert!((fv.value() - 1280.933).abs() < 1e-2); // ordinary 1268.250 × 1.01
    /// # Ok::<(), time_value::TvmError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`TvmError::NonFiniteResult`] if the compounded sum overflows to a
    /// non-finite value on extreme rate/period magnitudes (ADR-0021).
    pub fn future_value<P: Periodicity>(
        rate: Rate<P>,
        periods: Period,
        payment: Money,
    ) -> Result<Money, TvmError> {
        let factor = future_value_factor(rate.value(), periods.value()) * (1.0 + rate.value());
        Money::from_operation(payment.value() * factor)
    }

    /// The level payment, made at the *start* of each period, that amortises a
    /// `present` value over `periods` periods at `rate` — the inverse of
    /// [`present_value`].
    ///
    /// `PMT = PV / [(1 + r) · (1 - (1 + r)⁻ⁿ) / r]`, or `PMT = PV / n` when
    /// `r = 0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use time_value::{annuity, Money, Monthly, Period, Rate};
    ///
    /// // Amortise a 1136.763 loan over a year at 1%/month with start-of-month
    /// // payments -> ~100 per month.
    /// let pmt = annuity::due::payment(
    ///     Rate::<Monthly>::new(0.01)?,
    ///     Period::new(12.0)?,
    ///     Money::new(1136.763)?,
    /// )?;
    /// assert!((pmt.value() - 100.0).abs() < 1e-2);
    /// # Ok::<(), time_value::TvmError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`TvmError::NonFiniteResult`] if the amortisation is degenerate — in
    /// particular when `periods` is zero, so the factor is `0` and the division is
    /// non-finite — or if it overflows on extreme magnitudes (ADR-0021).
    pub fn payment<P: Periodicity>(
        rate: Rate<P>,
        periods: Period,
        present: Money,
    ) -> Result<Money, TvmError> {
        let factor = present_value_factor(rate.value(), periods.value()) * (1.0 + rate.value());
        Money::from_operation(present.value() / factor)
    }
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

    #[test]
    fn due_present_value_is_ordinary_scaled_by_one_plus_r() {
        let periods = Period::new(12.0).unwrap();
        let payment = Money::new(100.0).unwrap();
        let ordinary = annuity::present_value(rate(0.01), periods, payment).unwrap();
        let due = annuity::due::present_value(rate(0.01), periods, payment).unwrap();
        assert!(approx(due.value(), ordinary.value() * 1.01, 1e-9));
    }

    #[test]
    fn due_future_value_is_ordinary_scaled_by_one_plus_r() {
        let periods = Period::new(12.0).unwrap();
        let payment = Money::new(100.0).unwrap();
        let ordinary = annuity::future_value(rate(0.01), periods, payment).unwrap();
        let due = annuity::due::future_value(rate(0.01), periods, payment).unwrap();
        assert!(approx(due.value(), ordinary.value() * 1.01, 1e-9));
    }

    #[test]
    fn due_payment_inverts_due_present_value() {
        let payment = Money::new(100.0).unwrap();
        let periods = Period::new(24.0).unwrap();
        let pv = annuity::due::present_value(rate(0.015), periods, payment).unwrap();
        let recovered = annuity::due::payment(rate(0.015), periods, pv).unwrap();
        assert!(approx(recovered.value(), payment.value(), 1e-9));
    }

    #[test]
    fn due_zero_rate_matches_ordinary_limit() {
        // At r = 0 the (1 + r) scaling is 1, so due == ordinary == payment * n.
        let periods = Period::new(10.0).unwrap();
        let payment = Money::new(50.0).unwrap();
        assert!(approx(
            annuity::due::present_value(rate(0.0), periods, payment)
                .unwrap()
                .value(),
            500.0,
            1e-9,
        ));
        assert!(approx(
            annuity::due::future_value(rate(0.0), periods, payment)
                .unwrap()
                .value(),
            500.0,
            1e-9,
        ));
    }

    #[test]
    fn due_payment_over_zero_periods_is_degenerate() {
        let result = annuity::due::payment(rate(0.01), Period::ZERO, Money::new(1000.0).unwrap());
        assert_eq!(result, Err(TvmError::NonFiniteResult));
    }

    #[test]
    fn perpetuity_is_payment_over_rate() {
        let pv = annuity::perpetuity(rate(0.05), Money::new(100.0).unwrap()).unwrap();
        assert!(approx(pv.value(), 2000.0, 1e-9));
    }

    #[test]
    fn perpetuity_is_the_zero_growth_growing_perpetuity() {
        let pv = annuity::perpetuity(rate(0.05), Money::new(100.0).unwrap()).unwrap();
        let grown =
            annuity::growing_perpetuity(rate(0.05), rate(0.0), Money::new(100.0).unwrap()).unwrap();
        assert!(approx(pv.value(), grown.value(), 1e-9));
    }

    #[test]
    fn growing_perpetuity_discounts_by_the_spread() {
        // 100 / (0.05 - 0.02) = 3333.333...
        let pv = annuity::growing_perpetuity(rate(0.05), rate(0.02), Money::new(100.0).unwrap())
            .unwrap();
        assert!(approx(pv.value(), 3_333.333_333_333_333, 1e-6));
    }

    #[test]
    fn perpetuity_with_non_positive_rate_diverges() {
        let payment = Money::new(100.0).unwrap();
        assert_eq!(
            annuity::perpetuity(rate(0.0), payment),
            Err(TvmError::DivergentPerpetuity),
        );
        assert_eq!(
            annuity::perpetuity(rate(-0.01), payment),
            Err(TvmError::DivergentPerpetuity),
        );
    }

    #[test]
    fn growing_perpetuity_diverges_when_rate_does_not_exceed_growth() {
        let payment = Money::new(100.0).unwrap();
        // r = g: an infinity from division by zero.
        assert_eq!(
            annuity::growing_perpetuity(rate(0.03), rate(0.03), payment),
            Err(TvmError::DivergentPerpetuity),
        );
        // r < g: a finite but meaningless value, still rejected.
        assert_eq!(
            annuity::growing_perpetuity(rate(0.02), rate(0.05), payment),
            Err(TvmError::DivergentPerpetuity),
        );
    }
}
