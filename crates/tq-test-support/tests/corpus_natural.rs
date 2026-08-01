//! Natural-size preservation regression tests.

use tq_test_support::corpus::build_source_snapshot;

mod support;

#[test]
fn source_preparation_preserves_every_exact_byte_and_feature() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("natural.json");
    let natural = br#"{"type":"FeatureCollection","metadata":{"natural":true},"features":[{"id":1,"payload":"short"},{"id":2,"payload":"a much longer natural record"},{"id":3,"payload":null}]}"#;
    std::fs::write(&source, natural).expect("natural source");

    let manifest =
        build_source_snapshot(support::snapshot_input(source.clone())).expect("manifest");
    assert_eq!(manifest.artifacts.source_json.bytes, natural.len() as u64);
    assert_eq!(manifest.document.logical_records, 3);
    assert_eq!(std::fs::read(source).expect("source remains"), natural);
}

#[test]
fn source_schema_has_no_resizing_or_sampling_controls() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/corpus-source-v1.schema.json"
    ))
    .expect("source schema");
    let serialized = serde_json::to_string(&schema).expect("schema JSON");

    for forbidden in [
        "slice",
        "repeat",
        "pad",
        "sample",
        "truncate",
        "nominal_size",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "source schema must not expose {forbidden}"
        );
    }
}
