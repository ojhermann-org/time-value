//! # `time_value`
//!
//! Type-safe time-value-of-money (TVM) calculations.
//!
//! This crate is a deliberately type-heavy redesign of `time_value`, rebuilt
//! from scratch for the `1.0` line. The design goal is to make TVM mistakes —
//! applying an annual rate to monthly cashflows, discounting with an
//! economically meaningless rate — *compile errors* rather than silent
//! arithmetic, while keeping the common path ergonomic.
//!
//! The crate is `#![no_std]` and dependency-free.
//!
//! ## Status
//!
//! `1.0.0` is under active design. The public API is being built up
//! incrementally; this is the scaffolding baseline (error type only). The
//! validated newtypes (`Rate`, `Money`, `Period`), the `Cashflows` core type,
//! and the periodicity-tagged operations land in subsequent changes.

#![no_std]
#![forbid(unsafe_code)]

use core::fmt;

/// Errors produced when constructing or operating on time-value types.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TvmError {
    /// A rate was less than or equal to `-1.0` (i.e. ≤ −100%), which is
    /// economically meaningless for discounting and compounding.
    RateOutOfRange,
}

impl fmt::Display for TvmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::RateOutOfRange => f.write_str("rate must be greater than -1.0 (-100%)"),
        }
    }
}

impl core::error::Error for TvmError {}

#[cfg(test)]
mod tests {
    use super::TvmError;

    #[test]
    fn error_variant_is_comparable() {
        assert_eq!(TvmError::RateOutOfRange, TvmError::RateOutOfRange);
    }
}
