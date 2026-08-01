//! Smoke-corpus discovery and report-label tests.

use tq_test_support::corpus::{CorpusOrigin, discover_smoke_corpus};

#[test]
fn examples_are_validated_and_unambiguously_labeled_smoke() {
    let temp = tempfile::tempdir().expect("temporary directory");
    std::fs::write(
        temp.path().join("all_hour.geojson"),
        br#"{"type":"FeatureCollection","features":[{"id":1}]}"#,
    )
    .expect("GeoJSON example");
    std::fs::write(temp.path().join("all_hour.toon"), "ignored: true").expect("generated example");

    let corpus = discover_smoke_corpus(temp.path()).expect("smoke corpus");
    assert_eq!(corpus.origin, CorpusOrigin::Smoke);
    assert_eq!(corpus.snapshots.len(), 1);
    assert_eq!(corpus.snapshots[0].source_id, "all-hour");
    assert_eq!(corpus.snapshots[0].document.logical_records, 1);
}

#[test]
fn report_serialization_cannot_omit_smoke_origin() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let corpus = discover_smoke_corpus(temp.path()).expect("empty smoke corpus");
    let report = serde_json::to_value(corpus).expect("smoke report JSON");
    assert_eq!(report["origin"], "smoke");
}
