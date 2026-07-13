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
        .stdout(predicate::str::contains("present_value"))
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
fn an_overflowing_result_is_an_error_not_null() {
    // Previously this returned `{"future_value":null}` with isError:false — a
    // silent non-answer. Now it is a proper error (ADR-0021).
    let calls = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"future_value","arguments":{"rate":1,"periods":2000,"present":1000000}}}"#,
        "\n",
    );

    Command::cargo_bin("time-value-mcp")
        .unwrap()
        .write_stdin(session(calls))
        .assert()
        .success() // the process exits cleanly; the error is in the JSON-RPC response
        .stdout(predicate::str::contains("finite"))
        .stdout(predicate::str::contains("\"future_value\":null").not());
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
