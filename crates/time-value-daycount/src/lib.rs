//! ACT/365 day-count for the `time-value` binaries.
//!
//! The core [`time_value`] library takes year-offsets, not a date type
//! (`docs/adr/0029-dated-cashflows-xnpv-xirr.md`); the CLI and MCP binaries accept
//! real ISO `YYYY-MM-DD` dates and convert them to year-offsets here. This crate
//! is the single, self-contained home for that conversion, shared by both binaries
//! so the day-count is defined once (`docs/adr/0030-shared-day-count-support-crate.md`).
//!
//! It is dependency-free: the calendar arithmetic is Howard Hinnant's
//! days-from-civil algorithm, so no date/time crate reaches the binaries.
//!
//! ```
//! use time_value_daycount::{act365_year_fraction, iso_to_day};
//!
//! let start = iso_to_day("2020-01-01")?;
//! let end = iso_to_day("2021-01-01")?;
//! // 2020 is a leap year (366 days), so the ACT/365 fraction is just over 1.
//! assert!((act365_year_fraction(start, end) - 366.0 / 365.0).abs() < 1e-12);
//! # Ok::<(), time_value_daycount::ParseDateError>(())
//! ```

#![forbid(unsafe_code)]

use core::fmt;

/// An ISO date that could not be parsed into a valid calendar date.
///
/// Carries the offending text so the message is self-contained; each binary maps
/// it into its own error type (`anyhow` context for the CLI, an `invalid_params`
/// error for the MCP server) via [`Display`](fmt::Display).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDateError {
    input: String,
    kind: DateErrorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DateErrorKind {
    /// Not three `-`-separated fields, or a field was not an integer.
    Malformed,
    /// Month not in `1..=12`, or day not valid for that month and year.
    OutOfRange,
}

impl fmt::Display for ParseDateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            DateErrorKind::Malformed => {
                write!(f, "invalid date `{}` (expected YYYY-MM-DD)", self.input)
            }
            DateErrorKind::OutOfRange => write!(
                f,
                "invalid date `{}` (month 1-12, day valid for the month)",
                self.input
            ),
        }
    }
}

impl std::error::Error for ParseDateError {}

/// Parse an ISO `YYYY-MM-DD` date into a serial day number — days since the Unix
/// epoch in the proleptic Gregorian calendar (1970-01-01 is `0`).
///
/// Only the *difference* between two day numbers is meaningful to callers (see
/// [`act365_year_fraction`]); the epoch is an arbitrary shared origin.
///
/// # Errors
///
/// [`ParseDateError`] if `text` is not three `-`-separated integers, or if the
/// month is outside `1..=12` or the day is not valid for that month and year.
pub fn iso_to_day(text: &str) -> Result<i64, ParseDateError> {
    let malformed = || ParseDateError {
        input: text.to_owned(),
        kind: DateErrorKind::Malformed,
    };
    let out_of_range = || ParseDateError {
        input: text.to_owned(),
        kind: DateErrorKind::OutOfRange,
    };

    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() != 3 {
        return Err(malformed());
    }
    let year: i64 = parts[0].parse().map_err(|_| malformed())?;
    let month: i64 = parts[1].parse().map_err(|_| malformed())?;
    let day: i64 = parts[2].parse().map_err(|_| malformed())?;
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return Err(out_of_range());
    }
    Ok(days_from_civil(year, month, day))
}

/// The ACT/365 year-fraction from `reference` to `day`, where both are serial day
/// numbers from [`iso_to_day`]: `(day − reference) / 365`.
///
/// This is the day-count convention the binaries feed to the core's dated-cashflow
/// operations (XNPV/XIRR) — actual elapsed days over a fixed 365-day year.
#[must_use]
pub fn act365_year_fraction(reference: i64, day: i64) -> f64 {
    // Day-count differences for real calendar dates are far below 2^53, so this
    // conversion is exact despite the lint's worst-case warning.
    #[allow(clippy::cast_precision_loss)]
    let years = (day - reference) as f64 / 365.0;
    years
}

/// Days since the epoch (proleptic Gregorian) via Howard Hinnant's days-from-civil
/// algorithm. `month` is `1..=12`, `day` valid for the month.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let year_of_era = y - era * 400; // [0, 399]
    let month_index = (month + 9) % 12; // Mar = 0 … Feb = 11
    let day_of_year = (153 * month_index + 2) / 5 + day - 1; // [0, 365]
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_day_zero() {
        assert_eq!(iso_to_day("1970-01-01").unwrap(), 0);
    }

    #[test]
    fn consecutive_new_years_span_the_year_length() {
        // 2020 is a leap year, 2021 is not.
        let d2020 = iso_to_day("2020-01-01").unwrap();
        let d2021 = iso_to_day("2021-01-01").unwrap();
        let d2022 = iso_to_day("2022-01-01").unwrap();
        assert_eq!(d2021 - d2020, 366);
        assert_eq!(d2022 - d2021, 365);
    }

    #[test]
    fn leap_year_rule() {
        assert!(is_leap_year(2000)); // divisible by 400
        assert!(!is_leap_year(1900)); // divisible by 100 but not 400
        assert!(is_leap_year(2020));
        assert!(!is_leap_year(2021));
    }

    #[test]
    fn february_29_is_valid_only_in_a_leap_year() {
        assert!(iso_to_day("2020-02-29").is_ok());
        assert_eq!(
            iso_to_day("2021-02-29").unwrap_err().kind,
            DateErrorKind::OutOfRange
        );
    }

    #[test]
    fn out_of_range_month_and_day_are_rejected() {
        assert_eq!(
            iso_to_day("2020-13-01").unwrap_err().kind,
            DateErrorKind::OutOfRange
        );
        assert_eq!(
            iso_to_day("2020-04-31").unwrap_err().kind, // April has 30 days
            DateErrorKind::OutOfRange
        );
        assert_eq!(
            iso_to_day("2020-01-00").unwrap_err().kind,
            DateErrorKind::OutOfRange
        );
    }

    #[test]
    fn malformed_shapes_are_rejected() {
        for bad in [
            "2020-01",
            "2020/01/01",
            "not-a-date",
            "2020-01-01-01",
            "20a0-01-01",
        ] {
            assert_eq!(
                iso_to_day(bad).unwrap_err().kind,
                DateErrorKind::Malformed,
                "expected `{bad}` to be malformed"
            );
        }
    }

    #[test]
    fn year_fraction_is_act_365() {
        let start = iso_to_day("2021-01-01").unwrap(); // non-leap year ahead
        let end = iso_to_day("2022-01-01").unwrap();
        assert!((act365_year_fraction(start, end) - 1.0).abs() < 1e-12);
        assert!(act365_year_fraction(start, start).abs() < 1e-12);
    }

    #[test]
    fn error_messages_name_the_input() {
        let err = iso_to_day("2020-02-30").unwrap_err();
        assert!(err.to_string().contains("invalid date"));
        assert!(err.to_string().contains("2020-02-30"));
    }
}
