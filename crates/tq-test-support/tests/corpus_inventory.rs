//! Corpus inventory reporting tests.

use tq_test_support::corpus::{
    CorpusOrigin, build_source_snapshot, inventory_snapshots, write_snapshot_manifest,
};

mod support;

#[test]
fn inventory_shows_source_snapshot_sizes_counts_digests_and_validation() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source.json");
    std::fs::write(
        &source,
        br#"{"type":"FeatureCollection","features":[{},{}]}"#,
    )
    .expect("source");
    let manifest = build_source_snapshot(support::snapshot_input(source)).expect("manifest");
    let path = temp.path().join("snapshot.json");
    write_snapshot_manifest(&path, &manifest).expect("manifest write");

    let inventory = inventory_snapshots(CorpusOrigin::Refreshed, &[path]).expect("inventory");
    assert_eq!(inventory.origin, CorpusOrigin::Refreshed);
    assert_eq!(inventory.snapshots[0].source_id, "usgs-all-hour");
    assert_eq!(inventory.snapshots[0].logical_records, 2);
    assert!(inventory.snapshots[0].source_json.bytes > 0);
    assert_eq!(inventory.snapshots[0].source_json.sha256.len(), 64);
    assert_eq!(inventory.snapshots[0].validation.source_json, "valid");
    assert!(inventory.snapshots[0].generated.is_none());
}
