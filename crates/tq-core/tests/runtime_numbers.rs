//! Runtime non-finite values retain numeric identity until output projection.

use tq_core::{Number, NumberLimits, Value};

#[test]
fn computed_negative_zero_keeps_its_sign_in_output() {
    let number = Number::from_runtime_f64(-0.0);
    assert!(number.as_f64().is_sign_negative());
    assert_eq!(number.to_string(), "-0");
    let value = Value::Number(number);
    assert!(
        value
            .to_json()
            .unwrap()
            .as_f64()
            .unwrap()
            .is_sign_negative()
    );
    let serialized = serde_json::to_string(&value).unwrap();
    assert!(
        serde_json::from_str::<f64>(&serialized)
            .unwrap()
            .is_sign_negative()
    );
}

#[test]
fn runtime_nan_projects_to_json_null_without_becoming_runtime_null() {
    let value = Value::Number(Number::from_runtime_f64(f64::NAN));
    assert_eq!(value.kind(), tq_core::ValueKind::Number);
    assert_eq!(value.to_json().unwrap(), serde_json::Value::Null);
    assert_eq!(serde_json::to_string(&value).unwrap(), "null");
    assert!(Number::from_f64(f64::NAN).is_err());
    assert!(Number::parse("NaN").is_err());
}

#[test]
fn runtime_infinity_remains_infinite_until_json_projection() {
    let number = Number::from_runtime_f64(f64::INFINITY);
    assert!(number.as_f64().is_infinite());
    let value = Value::Number(number);
    let projected = value.to_json().unwrap();
    assert_eq!(projected.as_f64(), Some(f64::MAX));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&serde_json::to_string(&value).unwrap()).unwrap(),
        projected
    );
    assert!(Number::from_f64(f64::INFINITY).is_err());
}

#[test]
fn decimal_identity_and_semantic_normalization_are_separate() {
    let literal = Number::parse("100e-2").unwrap();
    assert_eq!(literal.to_string(), "1.00");
    assert_eq!(literal.canonical_numeric(), "1");
    assert_eq!(
        Number::parse("1.0").unwrap().canonical_numeric(),
        literal.canonical_numeric()
    );
    assert_eq!(
        Number::parse("1E1234567890").unwrap().to_string(),
        "1.7976931348623157e+308"
    );
}

#[test]
fn lossless_literal_normalization_bypasses_runtime_projection() {
    assert_eq!(Number::canonicalize_literal_numeric("30").unwrap(), "30");
    assert_eq!(Number::canonicalize_literal_numeric("1e3").unwrap(), "1000");
    assert_ne!(
        Number::canonicalize_literal_numeric("1e1234567890").unwrap(),
        Number::canonicalize_literal_numeric("2e1234567890").unwrap()
    );
}

#[test]
fn zero_literals_still_obey_rendered_byte_limits() {
    let limits = NumberLimits {
        rendered_bytes: 0,
        ..NumberLimits::default()
    };
    assert!(Number::parse_with_limits("0", limits).is_err());
    assert!(Number::parse_with_limits("-0", limits).is_err());
}

#[test]
fn computed_numbers_use_jq_compact_exponent_spelling() {
    for (value, expected) in [
        (1e-7, "1e-07"),
        (1e-6, "1e-06"),
        (1e20, "1e+20"),
        (1e21, "1e+21"),
        (1e22, "1e+22"),
        (1.2e100, "1.2e+100"),
        (2.0, "2"),
    ] {
        assert_eq!(Number::from_runtime_f64(value).to_string(), expected);
    }
}
