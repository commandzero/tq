//! Cross-format materialization and ordered semantic-equivalence tests.

use serde_json::{Map, Value, json};
#[cfg(unix)]
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use tq_test_support::corpus::{
    ArtifactIdentity, GeneratedArtifacts, validate_generated_representations_cached,
    validate_generated_representations_with_tq,
};
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
fn semantic_comparison_rejects_reordered_object_members() {
    let mut expected = Map::new();
    expected.insert("first".to_owned(), json!(1));
    expected.insert("second".to_owned(), json!(2));
    let mut actual = Map::new();
    actual.insert("second".to_owned(), json!(2));
    actual.insert("first".to_owned(), json!(1.0));

    let difference = compare_ordered(&Value::Object(expected), &Value::Object(actual))
        .expect_err("object member order matters");
    assert_eq!(difference.kind, DifferenceKind::ObjectOrder);
    assert_eq!(difference.expected, "first,second");
    assert_eq!(difference.actual, "second,first");
}

#[test]
fn semantic_comparison_normalizes_integral_decimal_without_rounding_precision() {
    let integral: Value = serde_json::from_str("34").expect("integral number");
    let decimal: Value = serde_json::from_str("34.0").expect("decimal number");
    compare_ordered(&integral, &decimal).expect("34.0 equals 34");

    let exact: Value = serde_json::from_str("9007199254740993").expect("exact number");
    let rounded: Value = serde_json::from_str("9007199254740992").expect("rounded number");
    let difference = compare_ordered(&exact, &rounded).expect_err("precision loss");
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

#[cfg(unix)]
#[test]
fn canonical_stream_gate_rejects_reordered_members_and_malformed_output() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source.json");
    let yaml = temp.path().join("source.yaml");
    let toon = temp.path().join("source.toon");
    let tq = temp.path().join("tq-fake.sh");
    fs::write(&source, br#"{"outer":{"first":34.0,"second":[1,2]}}"#).expect("source");
    fs::write(&yaml, br#"{"outer":{"first":34,"second":[1,2]}}"#).expect("yaml");
    fs::write(&toon, br#"{"outer":{"first":34,"second":[1,2]}}"#).expect("toon");
    fs::write(
        &tq,
        "#!/bin/sh\nfor arg do input=\"$arg\"; done\ncat \"$input\"\n",
    )
    .expect("fake tq");
    let mut permissions = fs::metadata(&tq).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&tq, permissions).expect("fake executable");

    validate_generated_representations_with_tq(&tq, &source, &yaml, &toon)
        .expect("nested numeric spelling is equivalent");
    fs::write(&yaml, br#"{"outer":{"second":[1,2],"first":34}}"#).expect("reordered yaml");
    assert!(matches!(
        validate_generated_representations_with_tq(&tq, &source, &yaml, &toon),
        Err(ConversionError::TqSemantic { format, .. }) if format == "yaml"
    ));

    fs::write(&yaml, br#"{"outer":{"first":34,"second":[1,2]}}"#).expect("restore yaml");
    fs::write(&toon, b"{\"outer\":\x0c{\"first\":34,\"second\":[1,2]}}")
        .expect("malformed toon stream");
    assert!(matches!(
        validate_generated_representations_with_tq(&tq, &source, &yaml, &toon),
        Err(ConversionError::Tq { format, .. }) if format == "toon"
    ));
}

#[cfg(unix)]
#[test]
fn semantic_validation_cache_hits_and_invalidates_on_identity_change() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source.json");
    let yaml = temp.path().join("source.yaml");
    let toon = temp.path().join("source.toon");
    let count = temp.path().join("invocations");
    let tq = temp.path().join("tq-fake.sh");
    let document = br#"{"a":34.0,"items":[1,2]}"#;
    for path in [&source, &yaml, &toon] {
        fs::write(path, document).expect("representation");
    }
    fs::write(
        &tq,
        format!(
            "#!/bin/sh\nprintf x >> '{}'\nfor arg do input=\"$arg\"; done\ncat \"$input\"\n",
            count.display()
        ),
    )
    .expect("fake tq");
    let mut permissions = fs::metadata(&tq).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&tq, permissions).expect("fake executable");

    let source_identity = identity(&source, "source.json");
    let generated = GeneratedArtifacts {
        yaml: identity(&yaml, "source.yaml"),
        toon: identity(&toon, "source.toon"),
    };
    let validate = |artifacts| {
        validate_generated_representations_cached(
            temp.path(),
            &tq,
            &source,
            &yaml,
            &toon,
            &source_identity,
            artifacts,
        )
        .expect("semantic validation");
    };
    validate(&generated);
    assert_eq!(fs::read(&count).expect("invocation count").len(), 3);

    validate(&generated);
    assert_eq!(fs::read(&count).expect("cached invocation count").len(), 3);

    let cache_path = temp.path().join("semantic-validation-cache-v1.json");
    let stale_cache = fs::read_to_string(&cache_path)
        .expect("semantic cache")
        .replace("tq-semantic-equivalence-v3", "tq-semantic-equivalence-v2");
    fs::write(&cache_path, stale_cache).expect("stale semantic policy");
    validate(&generated);
    assert_eq!(
        fs::read(&count).expect("policy invalidation count").len(),
        6
    );

    let mut changed = generated.clone();
    changed.yaml.sha256 = "identity-changed".to_owned();
    validate(&changed);
    assert_eq!(
        fs::read(&count)
            .expect("invalidated invocation count")
            .len(),
        9
    );

    let mut script = fs::read(&tq).expect("fake script bytes");
    script.extend_from_slice(b"\n# binary identity change\n");
    fs::write(&tq, script).expect("changed fake binary");
    validate(&generated);
    assert_eq!(
        fs::read(&count).expect("binary invalidation count").len(),
        12
    );
}

#[cfg(unix)]
fn identity(path: &std::path::Path, manifest_path: &str) -> ArtifactIdentity {
    use std::fmt::Write as _;
    let bytes = fs::read(path).expect("identity bytes");
    let mut sha256 = String::with_capacity(64);
    for byte in Sha256::digest(&bytes) {
        write!(&mut sha256, "{byte:02x}").expect("write digest");
    }
    ArtifactIdentity {
        path: manifest_path.to_owned(),
        bytes: bytes.len() as u64,
        sha256,
    }
}
