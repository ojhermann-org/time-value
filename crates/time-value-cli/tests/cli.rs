//! Integration tests: drive the compiled `time-value` binary and assert on its
//! stdout / stderr / exit status (ADR-0010, ADR-0011 testing strategy).

use assert_cmd::Command;
use predicates::prelude::*;

fn time_value() -> Command {
    Command::cargo_bin("time-value").unwrap()
}

#[test]
fn npv_of_a_simple_series() {
    // -100 now, +60, +60 at 1% per period -> ~18.22.
    time_value()
        .args(["series", "npv", "--rate", "0.01", "-100", "60", "60"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("18.22"));
}

#[test]
fn nfv_of_a_simple_series() {
    time_value()
        .args(["series", "nfv", "--rate", "0.01", "-100", "60", "60"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("18.5"));
}

#[test]
fn irr_of_a_simple_series() {
    time_value()
        .args(["series", "irr", "-100", "60", "60"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("0.130"));
}

#[test]
fn mirr_of_a_simple_series() {
    // Outflows -1000, -500; inflows 800, 900 at finance 10% / reinvest 12%.
    time_value()
        .args([
            "series",
            "mirr",
            "--finance",
            "0.10",
            "--reinvest",
            "0.12",
            "-1000",
            "-500",
            "800",
            "900",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("0.072"));
}

#[test]
fn xnpv_of_dated_flows() {
    // -100 now, +110 exactly one year later at 10%/yr -> ~0.
    time_value()
        .args([
            "series",
            "xnpv",
            "--rate",
            "0.10",
            "2020-01-01:-100",
            "2021-01-01:110",
        ])
        .assert()
        .success()
        // 2020 is a leap year (366 days), so the offset is 366/365 -> XNPV slightly
        // above zero, but small.
        .stdout(predicate::str::starts_with("0.0").or(predicate::str::starts_with("-0.0")));
}

#[test]
fn xirr_of_the_excel_reference() {
    // Microsoft's XIRR example -> ~0.3734.
    time_value()
        .args([
            "series",
            "xirr",
            "2008-01-01:-10000",
            "2008-03-01:2750",
            "2008-10-30:4250",
            "2009-02-15:3250",
            "2009-04-01:2750",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("0.373"));
}

#[test]
fn an_invalid_date_fails() {
    time_value()
        .args(["series", "xirr", "2020-02-30:-100", "2021-01-01:110"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid date"));
}

#[test]
fn present_value_of_a_single_sum() {
    time_value()
        .args([
            "pv",
            "--rate",
            "0.01",
            "--periods",
            "12",
            "--future",
            "1000",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("887.4"));
}

#[test]
fn future_value_of_a_single_sum() {
    time_value()
        .args([
            "fv",
            "--rate",
            "0.01",
            "--periods",
            "12",
            "--present",
            "1000",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("1126.8"));
}

#[test]
fn annuity_present_value() {
    time_value()
        .args([
            "annuity",
            "pv",
            "--rate",
            "0.01",
            "--periods",
            "12",
            "--payment",
            "100",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("1125.5"));
}

#[test]
fn annuity_payment_amortises_a_present_value() {
    time_value()
        .args([
            "annuity",
            "payment",
            "--rate",
            "0.01",
            "--periods",
            "12",
            "--present",
            "1125.508",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("99.99").or(predicate::str::starts_with("100")));
}

#[test]
fn json_output_is_keyed_by_operation() {
    time_value()
        .args([
            "--json", "series", "npv", "--rate", "0.01", "-100", "60", "60",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"npv\""));
}

#[test]
fn an_invalid_rate_fails() {
    time_value()
        .args(["series", "npv", "--rate", "-1.5", "-100", "60"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("rate"));
}

#[test]
fn an_overflowing_result_fails_instead_of_printing_inf() {
    // 2^2000 overflows f64; the CLI must error, not print `inf` with exit 0.
    time_value()
        .args(["fv", "--rate", "1", "--periods", "2000", "--present", "1e6"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("finite"));
}

#[test]
fn an_overflowing_result_fails_in_json_mode_too() {
    // Previously this printed `{"fv":null}` with exit 0; now it is an error.
    time_value()
        .args([
            "--json",
            "fv",
            "--rate",
            "1",
            "--periods",
            "2000",
            "--present",
            "1e6",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("null").not());
}

#[test]
fn a_nonconvergent_irr_fails() {
    // All inflows: NPV is positive for every rate, so there is no IRR.
    time_value()
        .args(["series", "irr", "100", "60", "60"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("internal rate of return"));
}
