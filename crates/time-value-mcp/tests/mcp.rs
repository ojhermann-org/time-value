//! End-to-end test of the MCP server: spawn the binary and drive a real stdio
//! JSON-RPC session — initialize, tools/list, tools/call (ADR-0011).

use assert_cmd::Command;
use predicates::prelude::*;

/// Wrap one or more `tools/call` request lines in a full initialised session.
fn session(calls: &str) -> String {
    format!(
        concat!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"test","version":"0"}}}}}}"#,
            "\n",
            r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#,
            "\n",
            "{calls}",
        ),
        calls = calls
    )
}

#[test]
fn stdio_session_lists_tools_and_computes_npv() {
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"npv","arguments":{"rate":0.01,"cashflows":[-100,60,60]}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        // The server identifies itself (not the rmcp crate) on initialise.
        .stdout(predicate::str::contains("\"name\":\"time-value-mcp\""))
        // tools/list exposes every tool with a JSON-Schema input.
        .stdout(predicate::str::contains("npv"))
        .stdout(predicate::str::contains("irr"))
        .stdout(predicate::str::contains("single_sum_present_value"))
        .stdout(predicate::str::contains("annuity_payment"))
        .stdout(predicate::str::contains("inputSchema"))
        // tools/call returns the computed NPV (~18.2237).
        .stdout(predicate::str::contains("18.22"));
}

#[test]
fn irr_tool_solves_the_series() {
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"irr","arguments":{"cashflows":[-100,60,60]}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        .stdout(predicate::str::contains("0.130"));
}

#[test]
fn xirr_tool_solves_dated_flows() {
    // Microsoft's XIRR example over ISO dates -> ~0.3734.
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"xirr","arguments":{"flows":[{"date":"2008-01-01","amount":-10000},{"date":"2008-03-01","amount":2750},{"date":"2008-10-30","amount":4250},{"date":"2009-02-15","amount":3250},{"date":"2009-04-01","amount":2750}]}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        .stdout(predicate::str::contains("0.373"));
}

#[test]
fn xnpv_tool_lists_and_computes() {
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"xnpv","arguments":{"rate":0.10,"flows":[{"date":"2020-01-01","amount":-100},{"date":"2021-01-01","amount":110}]}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        // The new tools are advertised alongside the originals.
        .stdout(predicate::str::contains("mirr"))
        .stdout(predicate::str::contains("xnpv"))
        .stdout(predicate::str::contains("xirr"));
}

#[test]
fn an_invalid_date_is_an_error() {
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"xirr","arguments":{"flows":[{"date":"2020-02-30","amount":-100},{"date":"2021-01-01","amount":110}]}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        .stdout(predicate::str::contains("invalid date"));
}

#[test]
fn single_sum_periods_tool_solves() {
    // 1000 → 1126.825 at 1%/period ≈ 12 periods.
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"single_sum_periods","arguments":{"rate":0.01,"present":1000,"future":1126.825}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        .stdout(predicate::str::contains("single_sum_periods"))
        .stdout(predicate::str::contains("11.9").or(predicate::str::contains("12.0")));
}

#[test]
fn annuity_perpetuity_and_due_tools() {
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"annuity_perpetuity","arguments":{"rate":0.05,"payment":100}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"annuity_due_present_value","arguments":{"rate":0.01,"periods":12,"payment":100}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        // Perpetuity 100/0.05 = 2000; annuity-due PV ≈ 1136.76.
        .stdout(predicate::str::contains("2000"))
        .stdout(predicate::str::contains("1136.7"));
}

#[test]
fn annuity_periods_requires_exactly_one_anchor() {
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"annuity_periods","arguments":{"rate":0.01,"payment":100}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        .stdout(predicate::str::contains("present").or(predicate::str::contains("future")));
}

#[test]
fn rate_conversion_tools() {
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"rate_effective_annual","arguments":{"rate":0.01,"periodicity":"monthly"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"rate_convert","arguments":{"rate":0.01,"from":"monthly","to":"quarterly"}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        // EAR of 1%/month = 0.126825…; monthly→quarterly = 0.030301…
        .stdout(predicate::str::contains("0.1268"))
        .stdout(predicate::str::contains("0.0303"));
}

#[test]
fn rate_rejects_an_unknown_periodicity() {
    // Periodicity is a typed enum (ADR-0039), so an unknown value is refused by
    // deserialization at the boundary, before the handler runs.
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"rate_effective_annual","arguments":{"rate":0.01,"periodicity":"fortnightly"}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        // The deserialize error names the bad value and lists the valid set.
        .stdout(predicate::str::contains("unknown variant"))
        .stdout(predicate::str::contains("fortnightly"))
        .stdout(predicate::str::contains("semi-annual"));
}

#[test]
fn amortize_tool_returns_a_schedule() {
    // 1000 at 10% paying 500 → three installments, last clears the balance.
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"amortize","arguments":{"rate":0.10,"principal":1000,"payment":500}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        .stdout(predicate::str::contains("amortize"))
        .stdout(predicate::str::contains("\"period\":3"))
        .stdout(predicate::str::contains("\"balance\":0"));
}

#[test]
fn amortize_requires_periods_or_payment() {
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"amortize","arguments":{"rate":0.10,"principal":1000}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        .stdout(predicate::str::contains("periods").or(predicate::str::contains("payment")));
}

#[test]
fn an_overflowing_result_is_an_error_not_null() {
    // Previously this returned `{"future_value":null}` with isError:false — a
    // silent non-answer. Now it is a proper error (ADR-0021).
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"single_sum_future_value","arguments":{"rate":1,"periods":2000,"present":1000000}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success() // the process exits cleanly; the error is in the JSON-RPC response
        .stdout(predicate::str::contains("finite"))
        .stdout(predicate::str::contains("\"single_sum_future_value\":null").not());
}

#[test]
fn an_invalid_rate_is_an_error() {
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"npv","arguments":{"rate":-1.5,"cashflows":[-100,60]}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success() // the process exits cleanly; the JSON-RPC response carries the error
        .stdout(predicate::str::contains("error"))
        .stdout(predicate::str::contains("rate"));
}

// ---- Currency (the `currency` input field): ADR-0034 ---------------------

#[test]
fn npv_echoes_the_currency_when_given() {
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"npv","arguments":{"rate":0.01,"cashflows":[-100,60,60],"currency":"USD"}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        .stdout(predicate::str::contains("18.22"))
        .stdout(predicate::str::contains("\"currency\":\"USD\""));
}

#[test]
fn npv_without_currency_has_no_currency_field() {
    // Omitting `currency` (XXX) keeps the pre-currency output shape.
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"npv","arguments":{"rate":0.01,"cashflows":[-100,60,60]}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        .stdout(predicate::str::contains("\"currency\"").not());
}

#[test]
fn a_rate_result_carries_no_currency() {
    // IRR is a rate, not money: the `currency` input is accepted but not echoed.
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"irr","arguments":{"cashflows":[-100,60,60],"currency":"USD"}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success()
        .stdout(predicate::str::contains("0.130"))
        .stdout(predicate::str::contains("\"currency\"").not());
}

#[test]
fn an_unknown_currency_code_is_an_error() {
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"npv","arguments":{"rate":0.01,"cashflows":[-100,60],"currency":"ZZZ"}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success() // process exits cleanly; the JSON-RPC response carries the error
        .stdout(predicate::str::contains("error"))
        .stdout(predicate::str::contains("ZZZ"));
}
