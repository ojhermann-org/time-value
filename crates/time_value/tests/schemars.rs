//! The `schemars` `JsonSchema` shapes (ADR-0044) — the JSON-Schema companion to the
//! `serde` wire format (ADR-0042). Asserts each type's generated schema matches
//! the shape its `serde` impl serializes to.
//!
//! Gated on `schemars` **and** `std`/`libm` (the `Period` / `ContinuousRate` /
//! `DatedCashflow` types live behind the transcendental-math feature). The
//! published `schemars` support is `no_std`; that build is covered by CI's
//! `--no-default-features --features schemars` clippy check.
#![cfg(all(feature = "schemars", any(feature = "std", feature = "libm")))]

use schemars::schema_for;
use serde_json::Value;
use time_value::{ContinuousRate, Currency, DatedCashflow, FxRate, Money, Monthly, Period, Rate};

/// The generated schema for `T`, as a `serde_json::Value`.
macro_rules! schema {
    ($ty:ty) => {
        serde_json::to_value(schema_for!($ty)).unwrap()
    };
}

#[test]
fn bare_number_types_are_plain_numbers() {
    for schema in [
        schema!(Rate<Monthly>),
        schema!(Period<Monthly>),
        schema!(ContinuousRate),
    ] {
        assert_eq!(schema["type"], Value::from("number"), "schema: {schema}");
    }
}

#[test]
fn currency_is_a_string_enum_of_iso_codes() {
    let schema = schema!(Currency);
    assert_eq!(schema["type"], Value::from("string"));
    let codes = schema["enum"].as_array().expect("an enum array");
    assert!(codes.contains(&Value::from("USD")));
    assert!(codes.contains(&Value::from("XXX")));
    // The whole closed set, from `Currency::ALL`.
    assert_eq!(codes.len(), Currency::ALL.len());
}

#[test]
fn money_is_an_object_with_amount_and_currency() {
    let schema = schema!(Money);
    assert_eq!(schema["type"], Value::from("object"));
    let props = &schema["properties"];
    assert_eq!(props["amount"]["type"], Value::from("number"));
    // The currency sub-schema is the inlined code enum.
    assert_eq!(props["currency"]["type"], Value::from("string"));
    assert!(props["currency"]["enum"].is_array());
    let required = schema["required"].as_array().expect("required array");
    assert!(required.contains(&Value::from("amount")));
    assert!(required.contains(&Value::from("currency")));
}

#[test]
fn composite_structs_are_objects_with_their_fields() {
    let fx = schema!(FxRate);
    assert_eq!(fx["type"], Value::from("object"));
    for field in ["from", "to", "rate"] {
        assert!(fx["properties"].get(field).is_some(), "FxRate.{field}");
    }

    let dated = schema!(DatedCashflow);
    assert_eq!(dated["type"], Value::from("object"));
    assert!(dated["properties"].get("offset_years").is_some());
    assert!(dated["properties"].get("amount").is_some());
}
