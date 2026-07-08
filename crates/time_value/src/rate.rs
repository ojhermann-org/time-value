//! [`Rate`] — a periodicity-tagged interest rate.

use core::fmt;
use core::marker::PhantomData;

use crate::{Periodicity, TvmError};

/// A per-period interest rate, tagged with its [`Periodicity`].
///
/// `Rate<Monthly>` and `Rate<Annual>` are **distinct types**, so an operation
/// that discounts monthly cashflows will not accept an annual rate: the
/// periodicity mismatch is a compile error, not a silent arithmetic bug (see
/// `docs/adr/0005-domain-modelling-and-strong-typing.md`).
///
/// A `Rate` is always finite and strictly greater than `-1.0` (−100%); rates at
/// or below that are economically meaningless for discounting and compounding.
///
/// The value is the plain per-period rate: `0.01` is 1% per period.
#[derive(Clone, Copy, PartialEq)]
pub struct Rate<P: Periodicity> {
    per_period: f64,
    marker: PhantomData<P>,
}

impl<P: Periodicity> Rate<P> {
    /// Wraps a per-period `rate` (e.g. `0.01` for 1% per period).
    ///
    /// # Errors
    ///
    /// Returns [`TvmError::RateOutOfRange`] if `rate` is not finite or is
    /// `<= -1.0` (≤ −100%).
    pub fn new(rate: f64) -> Result<Self, TvmError> {
        if rate.is_finite() && rate > -1.0 {
            Ok(Self::from_valid(rate))
        } else {
            Err(TvmError::RateOutOfRange)
        }
    }

    /// The per-period rate as a plain `f64`.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.per_period
    }

    /// The number of periods of this rate's periodicity in one year.
    #[must_use]
    pub const fn periods_per_year(self) -> u16 {
        P::PERIODS_PER_YEAR
    }

    /// Constructs from an `f64` already known to satisfy the domain (internal use
    /// — e.g. a solved [`Cashflows::internal_rate_of_return`] result that is
    /// guarded to stay above −100%).
    ///
    /// [`Cashflows::internal_rate_of_return`]: crate::Cashflows::internal_rate_of_return
    pub(crate) const fn from_valid(rate: f64) -> Self {
        Self {
            per_period: rate,
            marker: PhantomData,
        }
    }
}

// `Debug`/`Display` are hand-written so the periodicity shows as its name rather
// than a `PhantomData`. `Clone`/`Copy`/`PartialEq` are derived (a derived
// `PartialEq` is exempt from `clippy::float_cmp`, unlike a hand-written one).
impl<P: Periodicity> fmt::Debug for Rate<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rate")
            .field("per_period", &self.per_period)
            .field("periodicity", &P::NAME)
            .finish()
    }
}

impl<P: Periodicity> fmt::Display for Rate<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.per_period, P::NAME)
    }
}

#[cfg(test)]
mod tests {
    // These tests round-trip exactly-representable values, so exact `==` is
    // correct here.
    #![allow(clippy::float_cmp)]

    use crate::{Annual, Monthly, Rate, TvmError};

    #[test]
    fn accepts_rates_above_minus_one() {
        assert_eq!(Rate::<Monthly>::new(0.05).unwrap().value(), 0.05);
        assert_eq!(Rate::<Monthly>::new(-0.5).unwrap().value(), -0.5);
    }

    #[test]
    fn rejects_meaningless_rates() {
        assert_eq!(Rate::<Monthly>::new(-1.0), Err(TvmError::RateOutOfRange));
        assert_eq!(Rate::<Monthly>::new(-1.5), Err(TvmError::RateOutOfRange));
        assert_eq!(
            Rate::<Monthly>::new(f64::NAN),
            Err(TvmError::RateOutOfRange)
        );
    }

    #[test]
    fn periods_per_year_comes_from_the_tag() {
        assert_eq!(Rate::<Monthly>::new(0.01).unwrap().periods_per_year(), 12);
        assert_eq!(Rate::<Annual>::new(0.01).unwrap().periods_per_year(), 1);
    }
}
