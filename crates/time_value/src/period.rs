//! [`Period`] — a count of periods.

use core::fmt;

use crate::TvmError;

/// A number of periods, expressed in the periodicity of the [`Rate`] it is used
/// with.
///
/// May be fractional (e.g. `1.5` periods); always finite and non-negative. It
/// carries no periodicity tag of its own — the [`Rate`] supplies the clock, so
/// `n` is simply "how many periods of that rate".
///
/// `Period` is available with the `std` or `libm` feature, alongside the
/// operations that consume it (`docs/adr/0014-transcendental-single-sum-operations.md`).
///
/// [`Rate`]: crate::Rate
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Period(f64);

impl Period {
    /// No periods.
    pub const ZERO: Self = Self(0.0);

    /// Wraps a period count.
    ///
    /// # Errors
    ///
    /// Returns [`TvmError::NegativePeriods`] if `periods` is negative or not
    /// finite.
    pub fn new(periods: f64) -> Result<Self, TvmError> {
        if periods.is_finite() && periods >= 0.0 {
            Ok(Self(periods))
        } else {
            Err(TvmError::NegativePeriods)
        }
    }

    /// The number of periods as a plain `f64`.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }

    /// Constructs from the `f64` result of a solve (e.g. a solved NPER),
    /// distinguishing a non-finite result — an overflow or a mathematically
    /// undefined case — from a finite but negative one.
    ///
    /// A non-finite value is [`TvmError::NonFiniteResult`] (the mirror of
    /// [`Money::from_operation`](crate::Money) and [`Rate::from_operation`], per
    /// ADR-0021); a finite negative count — a period count solved into the past —
    /// is [`TvmError::NegativePeriods`], the same variant [`new`](Self::new) uses.
    ///
    /// [`Rate::from_operation`]: crate::Rate
    pub(crate) fn from_operation(periods: f64) -> Result<Self, TvmError> {
        if periods.is_finite() {
            Self::new(periods)
        } else {
            Err(TvmError::NonFiniteResult)
        }
    }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    // Exactly-representable round-trips, so exact `==` is correct here.
    #![allow(clippy::float_cmp)]

    use crate::{Period, TvmError};

    #[test]
    fn accepts_non_negative_finite_counts() {
        assert_eq!(Period::new(0.0).unwrap().value(), 0.0);
        assert_eq!(Period::new(12.0).unwrap().value(), 12.0);
        assert_eq!(Period::new(1.5).unwrap().value(), 1.5);
    }

    #[test]
    fn rejects_negative_or_non_finite_counts() {
        assert_eq!(Period::new(-1.0), Err(TvmError::NegativePeriods));
        assert_eq!(Period::new(f64::NAN), Err(TvmError::NegativePeriods));
        assert_eq!(Period::new(f64::INFINITY), Err(TvmError::NegativePeriods));
    }
}
