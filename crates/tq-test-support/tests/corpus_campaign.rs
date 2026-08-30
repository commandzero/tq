//! Refreshed campaign and frozen replay contract tests.

use std::path::PathBuf;

use tq_test_support::corpus::{
    CampaignError, CampaignMode, GeneratedArtifacts, SnapshotState, SourceSnapshotInput,
    build_source_snapshot, discover_latest_validated_manifests, load_frozen_snapshot,
    write_snapshot_manifest,
};

mod support;

#[test]
fn campaign_mode_reuses_prepared_corpus_unless_frozen_manifest_is_explicit() {
    assert_eq!(CampaignMode::default(), CampaignMode::Prepared);
    assert_eq!(
        CampaignMode::from_frozen_manifest(None),
        CampaignMode::Prepared
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
    assert!(temp.path().join("verification-cache-v1.json").is_file());

    std::fs::write(&source_path, b"same size but wrong bytes..............")
        .expect("corrupt source");
    assert!(matches!(
        load_frozen_snapshot(&manifest_path, temp.path()),
        Err(CampaignError::SizeMismatch { .. } | CampaignError::DigestMismatch { .. })
    ));
}

#[test]
fn discovery_reuses_latest_admitted_snapshot_and_ignores_incomplete_refresh() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source.json");
    std::fs::write(&source, br#"{"type":"FeatureCollection","features":[]}"#).expect("source JSON");
    let mut ready = build_source_snapshot(support::snapshot_input(source)).expect("manifest");
    ready.state = SnapshotState::CrossFormatValidated;
    ready.artifacts.generated = Some(GeneratedArtifacts {
        yaml: support::artifact("campaigns/ready/usgs-all-hour/source.yaml", &{
            let path = temp.path().join("yaml");
            std::fs::write(&path, b"yaml").expect("yaml");
            path
        }),
        toon: support::artifact("campaigns/ready/usgs-all-hour/source.toon", &{
            let path = temp.path().join("toon");
            std::fs::write(&path, b"toon").expect("toon");
            path
        }),
    });
    ready.validation.yaml_equivalent = Some(true);
    ready.validation.toon_equivalent = Some(true);
    let ready_path = temp
        .path()
        .join("campaigns/ready/usgs-all-hour/manifest.json");
    write_snapshot_manifest(&ready_path, &ready).expect("ready manifest");

    let mut incomplete = ready;
    incomplete.retrieved_at = "2026-08-30T12:00:00Z".to_owned();
    incomplete.state = SnapshotState::SourceValidated;
    incomplete.artifacts.generated = None;
    incomplete.validation.yaml_equivalent = None;
    incomplete.validation.toon_equivalent = None;
    let incomplete_path = temp
        .path()
        .join("campaigns/incomplete/usgs-all-hour/manifest.json");
    write_snapshot_manifest(&incomplete_path, &incomplete).expect("incomplete manifest");

    assert_eq!(
        discover_latest_validated_manifests(temp.path()).expect("discovery"),
        vec![ready_path]
    );
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
