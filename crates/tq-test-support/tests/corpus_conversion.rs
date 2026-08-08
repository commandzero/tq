//! Cross-format materialization and ordered semantic-equivalence tests.

use serde_json::{Map, Value, json};
use tq_test_support::corpus::{
    ConversionError, DifferenceKind, compare_ordered, finalize_generated_representations,
    generate_representations, validate_generated_representations,
};

#[test]
fn json_is_generated_as_yaml_and_toon_outside_benchmark_execution() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source.json");
    let yaml = temp.path().join("source.yaml");
    let toon = temp.path().join("source.toon");
    std::fs::write(
        &source,
        br#"{"z":null,"name":"Alice","items":[{"id":1},{"id":2}]}"#,
    )
    .expect("source JSON");

    let generated = generate_representations(
        &source,
        &yaml,
        &toon,
        "prepared/source.yaml",
        "prepared/source.toon",
    )
    .expect("cross-format generation");

    assert!(yaml.is_file());
    assert!(toon.is_file());
    assert!(generated.yaml.bytes > 0);
    assert!(generated.toon.bytes > 0);
    validate_generated_representations(&source, &yaml, &toon)
        .expect("ordered representations agree");
}

#[test]
fn existing_representations_can_be_finalized_without_regeneration() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source.json");
    let yaml = temp.path().join("source.yaml");
    let toon = temp.path().join("source.toon");
    std::fs::write(&source, br#"{"value":[1,2,3]}"#).expect("source JSON");
    let generated = generate_representations(
        &source,
        &yaml,
        &toon,
        "campaign/source.yaml",
        "campaign/source.toon",
    )
    .expect("cross-format generation");
    let yaml_before = std::fs::read(&yaml).expect("YAML bytes");
    let toon_before = std::fs::read(&toon).expect("TOON bytes");

    let finalized = finalize_generated_representations(
        &source,
        &yaml,
        &toon,
        "campaign/source.yaml",
        "campaign/source.toon",
    )
    .expect("cross-format finalization");

    assert_eq!(finalized, generated);
    assert_eq!(std::fs::read(yaml).expect("YAML bytes"), yaml_before);
    assert_eq!(std::fs::read(toon).expect("TOON bytes"), toon_before);
}

#[test]
fn generated_yaml_uses_interoperable_numeric_scalars() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source.json");
    let yaml = temp.path().join("source.yaml");
    let toon = temp.path().join("source.toon");
    std::fs::write(
        &source,
        br#"{"integer":9007199254740993,"float":0.986208946133579}"#,
    )
    .expect("source JSON");

    generate_representations(
        &source,
        &yaml,
        &toon,
        "prepared/source.yaml",
        "prepared/source.toon",
    )
    .expect("cross-format generation");

    let text = std::fs::read_to_string(&yaml).expect("generated YAML");
    assert!(!text.contains("$serde_json::private::Number"));
    let value: yaml_serde::Value = yaml_serde::from_str(&text).expect("YAML model");
    assert!(value["integer"].is_number());
    assert!(value["float"].is_number());
    validate_generated_representations(&source, &yaml, &toon)
        .expect("ordered representations agree");
}

#[test]
fn arbitrary_decimal_yaml_uses_lossless_json_subset_profile() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source.json");
    let yaml = temp.path().join("source.yaml");
    let toon = temp.path().join("source.toon");
    let document = br#"{"coordinate":-80.976660999999993,"integral_float":34.0,"name":"building"}"#;
    std::fs::write(&source, document).expect("source JSON");

    generate_representations(
        &source,
        &yaml,
        &toon,
        "prepared/source.yaml",
        "prepared/source.toon",
    )
    .expect("lossless cross-format generation");

    assert_eq!(std::fs::read(&yaml).expect("YAML bytes"), document);
    assert!(
        std::fs::read_to_string(&toon)
            .expect("TOON text")
            .contains("-80.976660999999993")
    );
    let _: yaml_serde::Value = yaml_serde::from_slice(document).expect("JSON is YAML");
    validate_generated_representations(&source, &yaml, &toon)
        .expect("ordered representations agree exactly");
}

#[test]
fn semantic_comparison_rejects_type_value_and_array_order_changes() {
    let type_error =
        compare_ordered(&json!({"value": 1}), &json!({"value": "1"})).expect_err("type change");
    assert_eq!(type_error.path, "/value");
    assert_eq!(type_error.kind, DifferenceKind::Type);

    let value_error = compare_ordered(&json!({"value": true}), &json!({"value": false}))
        .expect_err("value change");
    assert_eq!(value_error.kind, DifferenceKind::Value);

    let order_error =
        compare_ordered(&json!([1, 2, 3]), &json!([1, 3, 2])).expect_err("array order change");
    assert_eq!(order_error.path, "/1");
}

#[test]
fn semantic_comparison_rejects_object_member_order_changes() {
    let mut expected = Map::new();
    expected.insert("first".to_owned(), json!(1));
    expected.insert("second".to_owned(), json!(2));
    let mut actual = Map::new();
    actual.insert("second".to_owned(), json!(2));
    actual.insert("first".to_owned(), json!(1));

    let difference = compare_ordered(&Value::Object(expected), &Value::Object(actual))
        .expect_err("object order change");
    assert_eq!(difference.kind, DifferenceKind::ObjectOrder);
    assert_eq!(difference.path, "");
}

#[test]
fn semantic_comparison_reports_numeric_fidelity_loss() {
    let exact: Value = serde_json::from_str("9007199254740993").expect("exact number");
    let rounded: Value = serde_json::from_str("9007199254740992").expect("rounded number");

    let difference = compare_ordered(&exact, &rounded).expect_err("numeric loss");
    assert_eq!(difference.kind, DifferenceKind::NumericFidelity);
    assert_eq!(difference.expected, "9007199254740993");
    assert_eq!(difference.actual, "9007199254740992");
}

#[test]
fn corrupted_generated_representation_is_rejected() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source.json");
    let yaml = temp.path().join("source.yaml");
    let toon = temp.path().join("source.toon");
    std::fs::write(&source, br#"{"a":1,"b":2}"#).expect("source");
    generate_representations(
        &source,
        &yaml,
        &toon,
        "prepared/source.yaml",
        "prepared/source.toon",
    )
    .expect("generation");

    std::fs::write(&yaml, "a: 1\nb: changed\n").expect("corrupt YAML");
    assert!(matches!(
        validate_generated_representations(&source, &yaml, &toon),
        Err(ConversionError::Semantic { format, .. }) if format == "yaml"
    ));
}
