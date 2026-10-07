//! Shared pinned-fixture loading and ordered semantic assertions.

use std::{collections::BTreeSet, fmt::Write as _, fs, path::PathBuf};

use serde_json::Value as JsonValue;
use sha2::{Digest as _, Sha256};
use tq_core::{Object, Value};
use tq_toon::{DecoderConfig, Delimiter, WriterConfig};

pub fn fixtures(category: &str) -> Vec<(String, JsonValue)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/spec-v4.1");
    let manifest: JsonValue =
        serde_json::from_str(include_str!("../fixtures/spec-v4.1/manifest.json")).unwrap();
    let records = manifest["files"].as_array().unwrap();
    let mut expected_paths = BTreeSet::new();
    let mut selected = Vec::new();
    for record in records {
        let path = record["path"].as_str().unwrap();
        assert!(
            expected_paths.insert(path.to_owned()),
            "duplicate manifest path: {path}"
        );
        let bytes = fs::read(root.join(path)).unwrap();
        let mut sha256 = String::with_capacity(64);
        for byte in Sha256::digest(&bytes) {
            write!(&mut sha256, "{byte:02x}").expect("writing to a string cannot fail");
        }
        assert_eq!(
            u64::try_from(bytes.len()).unwrap(),
            record["bytes"].as_u64().unwrap(),
            "{path} byte length"
        );
        assert_eq!(
            sha256,
            record["sha256"].as_str().unwrap(),
            "{path} upstream SHA-256"
        );
        if path.starts_with(&format!("{category}/")) {
            let fixture: JsonValue = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(fixture["category"].as_str().unwrap(), category, "{path}");
            let tests = fixture["tests"].as_array().unwrap();
            assert!(!tests.is_empty(), "{path} contains no cases");
            assert_eq!(
                u64::try_from(tests.len()).unwrap(),
                record["test_count"].as_u64().unwrap(),
                "{path} pinned membership"
            );
            for test in tests {
                match category {
                    "decode" => {
                        decoder_config(test);
                    }
                    "encode" => {
                        writer_config(test);
                    }
                    _ => panic!("unknown official fixture category: {category}"),
                }
            }
            selected.push((path.to_owned(), fixture));
        }
    }
    let mut actual_paths = BTreeSet::from(["LICENSE".to_owned()]);
    for directory in ["encode", "decode"] {
        for entry in fs::read_dir(root.join(directory)).unwrap() {
            let path = entry.unwrap().path();
            assert!(
                path.is_file(),
                "unexpected fixture directory: {}",
                path.display()
            );
            actual_paths.insert(format!(
                "{directory}/{}",
                path.file_name().unwrap().to_str().unwrap()
            ));
        }
    }
    assert_eq!(
        actual_paths, expected_paths,
        "complete vendored encode/decode membership"
    );
    assert!(
        !selected.is_empty(),
        "unknown or empty fixture category: {category}"
    );
    selected.sort_by(|left, right| left.0.cmp(&right.0));
    selected
}

pub fn decoder_config(test: &JsonValue) -> DecoderConfig {
    check_options(test, &["indentSize", "strict"]);
    DecoderConfig {
        strict: test["options"]["strict"].as_bool().unwrap_or(true),
        indent_size: test["options"]["indentSize"]
            .as_u64()
            .map_or(2, |n| usize::try_from(n).unwrap()),
        ..DecoderConfig::default()
    }
}

pub fn writer_config(test: &JsonValue) -> WriterConfig {
    check_options(test, &["indentSize", "delimiter"]);
    WriterConfig {
        indent_size: test["options"]["indentSize"]
            .as_u64()
            .map_or(2, |n| usize::try_from(n).unwrap()),
        delimiter: match test["options"]["delimiter"].as_str() {
            None | Some(",") => Delimiter::Comma,
            Some("\t") => Delimiter::Tab,
            Some("|") => Delimiter::Pipe,
            other => panic!("unsupported official delimiter: {other:?}"),
        },
    }
}

fn check_options(test: &JsonValue, supported: &[&str]) {
    if let Some(options) = test.get("options") {
        for name in options.as_object().unwrap().keys() {
            assert!(
                supported.contains(&name.as_str()),
                "unhandled official fixture option: {name}"
            );
        }
    }
}

/// Value's ordinary equality ignores object order; fixture equality must not.
pub fn assert_ordered(actual: &Value, expected: &Value, context: &str) {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => {
            assert_eq!(
                actual.len(),
                expected.len(),
                "{context}: object cardinality"
            );
            for ((actual_key, actual_value), (expected_key, expected_value)) in
                actual.iter().zip(expected.iter())
            {
                assert_eq!(
                    actual_key, expected_key,
                    "{context}: object encounter order"
                );
                assert_ordered(actual_value, expected_value, context);
            }
        }
        (Value::Array(actual), Value::Array(expected)) => {
            assert_eq!(actual.len(), expected.len(), "{context}: array cardinality");
            for (actual, expected) in actual.iter().zip(expected.iter()) {
                assert_ordered(actual, expected, context);
            }
        }
        _ => assert_eq!(actual, expected, "{context}: exact typed value"),
    }
}

/// Only eligible table values may adopt the first value's recursive key order.
/// Outer entry/field order, array order and anonymous list-object order stay exact.
pub fn header_ordered(value: &Value) -> Value {
    normalize(value, true)
}

fn normalize(value: &Value, table_position: bool) -> Value {
    match value {
        Value::Array(values) => {
            if table_position
                && values
                    .first()
                    .is_some_and(|first| values.iter().all(|value| same_schema(first, value)))
            {
                Value::array(
                    values
                        .iter()
                        .map(|value| reorder(value, &values[0]))
                        .collect::<Vec<_>>(),
                )
            } else {
                Value::array(
                    values
                        .iter()
                        .map(|value| normalize(value, false))
                        .collect::<Vec<_>>(),
                )
            }
        }
        Value::Object(values) => {
            let first = values.iter().next().map(|(_, value)| value);
            let keyed = table_position
                && values.len() >= 2
                && first
                    .is_some_and(|first| values.iter().all(|(_, value)| same_schema(first, value)));
            Value::object(
                values
                    .iter()
                    .map(|(key, value)| {
                        (
                            key.clone(),
                            if keyed {
                                reorder(value, first.unwrap())
                            } else {
                                normalize(value, true)
                            },
                        )
                    })
                    .collect::<Object>(),
            )
        }
        _ => value.clone(),
    }
}

fn same_schema(first: &Value, value: &Value) -> bool {
    let (Value::Object(first), Value::Object(value)) = (first, value) else {
        return false;
    };
    !first.is_empty()
        && first.len() == value.len()
        && first.iter().all(|(key, first)| {
            value.get(key).is_some_and(|value| match (first, value) {
                (Value::Object(_), Value::Object(_)) => same_schema(first, value),
                (Value::Object(_) | Value::Array(_), _)
                | (_, Value::Object(_) | Value::Array(_)) => false,
                _ => true,
            })
        })
}

fn reorder(value: &Value, schema: &Value) -> Value {
    match (value, schema) {
        (Value::Object(value), Value::Object(schema)) => Value::object(
            schema
                .iter()
                .map(|(key, schema)| (key.clone(), reorder(value.get(key).unwrap(), schema)))
                .collect::<Object>(),
        ),
        _ => value.clone(),
    }
}

#[test]
fn header_order_exception_is_recursive_and_table_position_specific() {
    let input: Value = serde_json::from_str(
        r#"{"z":[{"id":1,"meta":{"z":2,"a":3}},{"meta":{"a":4,"z":5},"id":6}],"a":[[{"z":1,"a":2},{"a":3,"z":4}]]}"#,
    ).unwrap();
    let expected: Value = serde_json::from_str(
        r#"{"z":[{"id":1,"meta":{"z":2,"a":3}},{"id":6,"meta":{"z":5,"a":4}}],"a":[[{"z":1,"a":2},{"a":3,"z":4}]]}"#,
    ).unwrap();
    assert_ordered(
        &header_ordered(&input),
        &expected,
        "recursive table-only equality",
    );
    let keyed: Value = serde_json::from_str(
        r#"{"z":{"id":1,"meta":{"z":2,"a":3}},"a":{"meta":{"a":4,"z":5},"id":6}}"#,
    )
    .unwrap();
    let expected: Value = serde_json::from_str(
        r#"{"z":{"id":1,"meta":{"z":2,"a":3}},"a":{"id":6,"meta":{"z":5,"a":4}}}"#,
    )
    .unwrap();
    assert_ordered(
        &header_ordered(&keyed),
        &expected,
        "keyed values, not outer entry order",
    );
    let invalid: Value =
        serde_json::from_str(r#"[{"z":1,"a":2},{"a":3,"z":4,"extra":5}]"#).unwrap();
    assert_ordered(
        &header_ordered(&invalid),
        &invalid,
        "invalid schema must preserve ordinary list order",
    );
}
