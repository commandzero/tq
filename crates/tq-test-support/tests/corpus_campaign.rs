//! Refreshed campaign and frozen replay contract tests.

use std::path::PathBuf;

use tq_test_support::corpus::{
    CampaignError, CampaignMode, SourceSnapshotInput, build_source_snapshot, load_frozen_snapshot,
    write_snapshot_manifest,
};

mod support;

#[test]
fn campaign_mode_is_refreshed_unless_frozen_manifest_is_explicit() {
    assert_eq!(CampaignMode::default(), CampaignMode::Refreshed);
    assert_eq!(
        CampaignMode::from_frozen_manifest(None),
        CampaignMode::Refreshed
    );
    assert_eq!(
        CampaignMode::from_frozen_manifest(Some(PathBuf::from("snapshot.json"))),
        CampaignMode::Frozen(PathBuf::from("snapshot.json"))
    );
}

#[test]
fn frozen_replay_requires_every_artifact_to_match_manifest() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source_path = temp.path().join("prepared/source.json");
    std::fs::create_dir_all(source_path.parent().expect("source parent"))
        .expect("source directory");
    std::fs::write(
        &source_path,
        br#"{"type":"FeatureCollection","features":[]}"#,
    )
    .expect("source JSON");
    let download_path = temp.path().join("downloads/source.json");
    std::fs::create_dir_all(download_path.parent().expect("download parent"))
        .expect("download directory");
    std::fs::copy(&source_path, &download_path).expect("download copy");

    let mut input: SourceSnapshotInput = support::snapshot_input(source_path.clone());
    input.download = support::artifact("downloads/source.json", &download_path);
    let manifest = build_source_snapshot(input).expect("manifest");
    let manifest_path = temp.path().join("snapshot.json");
    write_snapshot_manifest(&manifest_path, &manifest).expect("manifest write");

    let frozen = load_frozen_snapshot(&manifest_path, temp.path()).expect("verified replay");
    assert_eq!(frozen.manifest.source_id, "usgs-all-hour");

    std::fs::write(&source_path, b"same size but wrong bytes..............")
        .expect("corrupt source");
    assert!(matches!(
        load_frozen_snapshot(&manifest_path, temp.path()),
        Err(CampaignError::SizeMismatch { .. } | CampaignError::DigestMismatch { .. })
    ));
}

#[test]
fn frozen_replay_rejects_missing_artifacts() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source_path = temp.path().join("source.json");
    std::fs::write(
        &source_path,
        br#"{"type":"FeatureCollection","features":[]}"#,
    )
    .expect("source JSON");
    let manifest = build_source_snapshot(support::snapshot_input(source_path)).expect("manifest");
    let manifest_path = temp.path().join("snapshot.json");
    write_snapshot_manifest(&manifest_path, &manifest).expect("manifest write");

    assert!(matches!(
        load_frozen_snapshot(&manifest_path, temp.path()),
        Err(CampaignError::MissingArtifact(_))
    ));
}
