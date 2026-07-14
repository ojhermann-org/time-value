//! Transcendental helpers behind the `std` / `libm` features.
//!
//! The core is `no_std` and dependency-free by default
//! (`docs/adr/0009-no_std-and-optional-libm.md`); the operations here need
//! `powf`, which lives in `std` or — for `no_std` builds — in `libm`. `std` is
//! preferred when both features are enabled.

/// `base` raised to the power `exponent`, via `std`.
#[cfg(feature = "std")]
#[inline]
pub(crate) fn powf(base: f64, exponent: f64) -> f64 {
    base.powf(exponent)
}

/// `base` raised to the power `exponent`, via `libm` (for `no_std` builds).
#[cfg(all(not(feature = "std"), feature = "libm"))]
#[inline]
pub(crate) fn powf(base: f64, exponent: f64) -> f64 {
    libm::pow(base, exponent)
}

/// `e` raised to the power `x`, via `std`.
#[cfg(feature = "std")]
#[inline]
pub(crate) fn exp(x: f64) -> f64 {
    x.exp()
}

/// `e` raised to the power `x`, via `libm` (for `no_std` builds).
#[cfg(all(not(feature = "std"), feature = "libm"))]
#[inline]
pub(crate) fn exp(x: f64) -> f64 {
    libm::exp(x)
}

/// The natural logarithm of `x`, via `std`.
#[cfg(feature = "std")]
#[inline]
pub(crate) fn ln(x: f64) -> f64 {
    x.ln()
}

/// The natural logarithm of `x`, via `libm` (for `no_std` builds).
#[cfg(all(not(feature = "std"), feature = "libm"))]
#[inline]
pub(crate) fn ln(x: f64) -> f64 {
    libm::log(x)
}

/// `x` rounded to the nearest integer (half away from zero), via `std`.
#[cfg(feature = "std")]
#[inline]
pub(crate) fn round(x: f64) -> f64 {
    x.round()
}

/// `x` rounded to the nearest integer (half away from zero), via `libm` (for
/// `no_std` builds).
#[cfg(all(not(feature = "std"), feature = "libm"))]
#[inline]
pub(crate) fn round(x: f64) -> f64 {
    libm::round(x)
}
