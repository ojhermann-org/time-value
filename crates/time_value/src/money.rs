//! [`Money`] — a validated monetary amount.

use core::fmt;
use core::ops::Neg;

use crate::TvmError;

/// A monetary amount.
///
/// A plain newtype over `f64`; currency is intentionally **not** type-tagged in
/// the `1.0` line (see `docs/adr/0005-domain-modelling-and-strong-typing.md`).
///
/// Every `Money` is finite. The [`new`](Money::new) constructor rejects `NaN`
/// and the infinities, and every operation that could leave the finite range —
/// the TVM operations and the arithmetic below — returns a `Result` whose `Err`
/// is [`TvmError::Overflow`] (a real result too large for `f64`), or
/// [`TvmError::Undefined`] for a degenerate case such as division by zero,
/// rather than a non-finite `Money`
/// (`docs/adr/0021-fallible-operations-on-non-finite-results.md`,
/// `docs/adr/0031-split-non-finite-result-into-overflow-and-undefined.md`).
///
/// Cashflows are signed — an outflow is negative, an inflow positive.
///
/// # Arithmetic
///
/// Negation is a [`Neg`] operator: negating a finite amount is always finite, so
/// it cannot fail. Addition, subtraction and scaling *can* leave `f64` range, so
/// they are fallible [`try_add`](Self::try_add), [`try_sub`](Self::try_sub),
/// [`try_mul`](Self::try_mul) and [`try_div`](Self::try_div) methods rather than
/// operators — an operator cannot return a `Result`, and silently yielding an
/// infinity is the foot-gun this crate exists to avoid
/// (`docs/adr/0023-money-arithmetic-surface.md`).
///
/// ```
/// use time_value::Money;
///
/// let fee = Money::new(25.0)?;
/// let refund = -fee; // an inflow becomes an outflow
/// assert_eq!(refund.value(), -25.0);
///
/// let total = fee.try_add(Money::new(75.0)?)?;
/// let doubled = total.try_mul(2.0)?;
/// assert_eq!(doubled.value(), 200.0);
/// # Ok::<(), time_value::TvmError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Money(f64);

impl Money {
    /// Zero money.
    pub const ZERO: Self = Self(0.0);

    /// Wraps `amount`.
    ///
    /// # Errors
    ///
    /// Returns [`TvmError::NonFiniteAmount`] if `amount` is not finite
    /// (`NaN`, `+∞`, or `-∞`).
    pub fn new(amount: f64) -> Result<Self, TvmError> {
        if amount.is_finite() {
            Ok(Self(amount))
        } else {
            Err(TvmError::NonFiniteAmount)
        }
    }

    /// The wrapped amount as a plain `f64`.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }

    /// Constructs from the `f64` result of an operation, validating finiteness.
    ///
    /// This is the overflow funnel: a non-finite result reaching here is a real
    /// value that exceeded the representable `f64` range, so it is
    /// [`TvmError::Overflow`]. Mathematically undefined cases (e.g. an annuity
    /// payment over zero periods) are guarded at their call sites and return
    /// [`TvmError::Undefined`] before reaching this point (ADR-0021, ADR-0031).
    /// Both are distinct from the [`TvmError::NonFiniteAmount`] that
    /// [`new`](Self::new) returns for a non-finite value supplied by a *caller*.
    pub(crate) fn from_operation(amount: f64) -> Result<Self, TvmError> {
        if amount.is_finite() {
            Ok(Self(amount))
        } else {
            Err(TvmError::Overflow)
        }
    }

    /// Adds `rhs`.
    ///
    /// # Errors
    ///
    /// Returns [`TvmError::Overflow`] if the sum leaves the finite `f64` range.
    pub fn try_add(self, rhs: Self) -> Result<Self, TvmError> {
        Self::from_operation(self.0 + rhs.0)
    }

    /// Subtracts `rhs`.
    ///
    /// # Errors
    ///
    /// Returns [`TvmError::Overflow`] if the difference leaves the finite `f64`
    /// range.
    pub fn try_sub(self, rhs: Self) -> Result<Self, TvmError> {
        Self::from_operation(self.0 - rhs.0)
    }

    /// Scales by `factor` — e.g. `payment.try_mul(12.0)` for an annual total.
    ///
    /// # Errors
    ///
    /// Returns [`TvmError::Undefined`] if `factor` is itself `NaN` or infinite (no
    /// finite product is defined), or [`TvmError::Overflow`] if a finite factor
    /// pushes the product past the representable range.
    pub fn try_mul(self, factor: f64) -> Result<Self, TvmError> {
        if !factor.is_finite() {
            return Err(TvmError::Undefined);
        }
        Self::from_operation(self.0 * factor)
    }

    /// Divides by `divisor` — e.g. `total.try_div(12.0)` for a monthly share.
    ///
    /// # Errors
    ///
    /// Returns [`TvmError::Undefined`] if `divisor` is zero or `NaN` (the quotient
    /// has no defined value), or [`TvmError::Overflow`] if dividing a large amount
    /// by a tiny one leaves the finite range. An *infinite* divisor is not an
    /// error: the quotient is zero, which is finite.
    pub fn try_div(self, divisor: f64) -> Result<Self, TvmError> {
        if divisor == 0.0 || divisor.is_nan() {
            return Err(TvmError::Undefined);
        }
        Self::from_operation(self.0 / divisor)
    }
}

/// Flips the sign — an inflow becomes an outflow, and vice versa.
///
/// Infallible: the negation of a finite amount is finite (ADR-0021).
impl Neg for Money {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    // These tests round-trip exactly-representable values, so exact `==` is
    // correct here.
    #![allow(clippy::float_cmp)]

    use crate::{Money, TvmError};

    #[test]
    fn accepts_finite_values() {
        assert_eq!(Money::new(42.5).unwrap().value(), 42.5);
        assert_eq!(Money::new(-42.5).unwrap().value(), -42.5);
        assert_eq!(Money::ZERO.value(), 0.0);
    }

    #[test]
    fn rejects_non_finite_values() {
        assert_eq!(Money::new(f64::NAN), Err(TvmError::NonFiniteAmount));
        assert_eq!(Money::new(f64::INFINITY), Err(TvmError::NonFiniteAmount));
        assert_eq!(
            Money::new(f64::NEG_INFINITY),
            Err(TvmError::NonFiniteAmount)
        );
    }

    /// The largest finite `f64`; doubling it overflows.
    fn huge() -> Money {
        Money::new(f64::MAX).unwrap()
    }

    #[test]
    fn negation_flips_the_sign() {
        assert_eq!((-Money::new(42.5).unwrap()).value(), -42.5);
        assert_eq!((-Money::new(-42.5).unwrap()).value(), 42.5);
        assert_eq!(-(-huge()), huge());
    }

    #[test]
    fn adds_and_subtracts() {
        let a = Money::new(100.0).unwrap();
        let b = Money::new(25.0).unwrap();
        assert_eq!(a.try_add(b).unwrap().value(), 125.0);
        assert_eq!(a.try_sub(b).unwrap().value(), 75.0);
        assert_eq!(b.try_sub(a).unwrap().value(), -75.0);
    }

    #[test]
    fn add_and_sub_report_overflow() {
        assert_eq!(huge().try_add(huge()), Err(TvmError::Overflow));
        assert_eq!(huge().try_sub(-huge()), Err(TvmError::Overflow));
    }

    #[test]
    fn scales_by_a_factor() {
        let payment = Money::new(250.0).unwrap();
        assert_eq!(payment.try_mul(12.0).unwrap().value(), 3000.0);
        assert_eq!(payment.try_mul(0.0).unwrap().value(), 0.0);
        assert_eq!(payment.try_mul(-1.0).unwrap().value(), -250.0);
    }

    #[test]
    fn mul_rejects_a_non_finite_result() {
        // A finite factor that overflows the range is an Overflow; a non-finite
        // factor has no defined product, so it is Undefined (ADR-0031).
        assert_eq!(huge().try_mul(2.0), Err(TvmError::Overflow));
        assert_eq!(
            Money::new(1.0).unwrap().try_mul(f64::INFINITY),
            Err(TvmError::Undefined)
        );
        assert_eq!(
            Money::new(1.0).unwrap().try_mul(f64::NAN),
            Err(TvmError::Undefined)
        );
    }

    #[test]
    fn divides_by_a_divisor() {
        let total = Money::new(3000.0).unwrap();
        assert_eq!(total.try_div(12.0).unwrap().value(), 250.0);
        assert_eq!(total.try_div(-12.0).unwrap().value(), -250.0);
        // An infinite divisor yields zero, which is finite — not an error.
        assert_eq!(total.try_div(f64::INFINITY).unwrap().value(), 0.0);
    }

    #[test]
    fn div_rejects_a_non_finite_result() {
        let total = Money::new(3000.0).unwrap();
        // Division by zero or NaN is undefined; a finite divisor that overflows
        // the range is an Overflow (ADR-0031).
        assert_eq!(total.try_div(0.0), Err(TvmError::Undefined));
        assert_eq!(total.try_div(f64::NAN), Err(TvmError::Undefined));
        // 0 / 0 is undefined, not zero.
        assert_eq!(Money::ZERO.try_div(0.0), Err(TvmError::Undefined));
        assert_eq!(huge().try_div(0.5), Err(TvmError::Overflow));
    }
}
