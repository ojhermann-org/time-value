//! [`Money`] — a validated monetary amount.

use core::fmt;

use crate::TvmError;

/// A monetary amount.
///
/// A plain newtype over `f64`; currency is intentionally **not** type-tagged in
/// the `1.0` line (see `docs/adr/0005-domain-modelling-and-strong-typing.md`).
///
/// The [`new`](Money::new) constructor rejects `NaN` and the infinities, so a
/// `Money` obtained from `new` is finite. The TVM *operations* (present/future
/// value, NPV, …) assume finite inputs and do not re-validate their result: with
/// extreme inputs the underlying `f64` arithmetic can overflow, so the returned
/// `Money` may be non-finite. Call `money.value().is_finite()` when you feed in
/// magnitudes that might overflow.
///
/// Cashflows are signed — an outflow is negative, an inflow positive.
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

    /// Constructs from an `f64` already known to be finite (internal use — the
    /// results of validated arithmetic on validated inputs).
    pub(crate) const fn from_finite(amount: f64) -> Self {
        Self(amount)
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
}
