//! Robust one-dimensional root finding, shared across the crate.
//!
//! The bracketing bisection here is arithmetic-only (no transcendental math), so
//! it lives in the default `no_std`, zero-dependency build alongside its first
//! caller, [`Cashflows::internal_rate_of_return`](crate::Cashflows). The
//! transcendental solve-for-rate operations (`annuity::rate`) reuse the same
//! bracketing fallback (`docs/adr/0020-robust-irr-newton-with-bisection-fallback.md`,
//! `docs/adr/0025-solve-for-periods-and-rate.md`).

/// `|x| < tolerance`, without `f64::abs` (which is not in `core`).
pub(crate) fn within(x: f64, tolerance: f64) -> bool {
    x < tolerance && x > -tolerance
}

/// `|x|`, without `f64::abs` (which is not in `core`).
pub(crate) fn abs(x: f64) -> f64 {
    if x < 0.0 {
        -x
    } else {
        x
    }
}

/// Whether `a` and `b` are both non-zero and of opposite sign.
pub(crate) fn opposite_signs(a: f64, b: f64) -> bool {
    (a < 0.0 && b > 0.0) || (a > 0.0 && b < 0.0)
}

/// Bisect for the root of `f` in `[lo, hi]`, where `f` has opposite signs at the
/// ends (`f_lo` is `f(lo)`). Returns as soon as a sample is within `tol` of zero,
/// or the midpoint after `max` steps.
pub(crate) fn bisect(
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

/// Scan the valid rate domain (`r > −1`) for a sign change in `f` and bisect the
/// first bracket found, returning the lowest bracketed root. `None` if `f` never
/// changes sign over the scan (no root).
///
/// `f` is a residual in the per-period rate `r` — for the IRR it is the NPV, for a
/// solve-for-rate it is `value(r) − target`. `1 + r` is sampled geometrically from
/// just above `0` upward, a ratio fine enough not to step over a lone root of a
/// monotone residual.
pub(crate) fn bracket_and_bisect(f: impl Fn(f64) -> f64, tolerance: f64) -> Option<f64> {
    const MAX_BISECTIONS: u32 = 200;
    const START: f64 = 1e-4; // 1 + r, i.e. r = -0.9999
    const RATIO: f64 = 1.25;
    const SAMPLES: u32 = 160; // reaches 1 + r ≈ 1e15

    let mut lo = START - 1.0;
    let mut f_lo = f(lo);
    let mut growth = START;
    for _ in 0..SAMPLES {
        if within(f_lo, tolerance) {
            return Some(lo);
        }
        growth *= RATIO;
        let hi = growth - 1.0;
        let f_hi = f(hi);
        if opposite_signs(f_lo, f_hi) {
            return Some(bisect(&f, lo, hi, f_lo, tolerance, MAX_BISECTIONS));
        }
        lo = hi;
        f_lo = f_hi;
    }
    None
}
