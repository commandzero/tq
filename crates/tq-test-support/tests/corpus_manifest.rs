//! Snapshot-manifest population and atomic-write contract tests.

use std::path::PathBuf;

use jsonschema::Validator;
use tq_test_support::corpus::{
    ArchiveIdentity, ArtifactIdentity, ManifestError, Provenance, RequestIdentity, SnapshotState,
    SourceSnapshotInput, build_source_snapshot, write_snapshot_manifest,
};

const SNAPSHOT_SCHEMA: &str = include_str!("../../../schemas/snapshot-manifest-v1.schema.json");

fn input(source_json_file: PathBuf) -> SourceSnapshotInput {
    SourceSnapshotInput {
        campaign_id: "2026-07-31T08-00-00Z".to_owned(),
        source_id: "usgs-all-hour".to_owned(),
        retrieved_at: "2026-07-31T08:00:00Z".to_owned(),
        request: RequestIdentity {
            requested_url: "https://example.test/all_hour.geojson".to_owned(),
            final_url: "https://cdn.example.test/all_hour.geojson".to_owned(),
            status: 200,
            content_type: "application/geo+json".to_owned(),
            etag: Some("\"snapshot\"".to_owned()),
            last_modified: None,
        },
        archive: None,
        download: ArtifactIdentity {
            path: "downloads/all_hour.geojson".to_owned(),
            bytes: 123,
            sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_owned(),
        },
        source_json_file,
        source_json_path: "prepared/all_hour.json".to_owned(),
        provenance: Provenance {
            title: "USGS All Earthquakes, Past Hour".to_owned(),
            publisher: "U.S. Geological Survey".to_owned(),
            landing_page:
                "https://earthquake.usgs.gov/earthquakes/feed/v1.0/geojson.php"
                    .to_owned(),
            license: tq_test_support::corpus::LicenseIdentity {
                name: "USGS Government Work in the U.S. Public Domain".to_owned(),
                url: "https://www.usgs.gov/information-policies-and-instructions/copyrights-and-credits".to_owned(),
            },
        },
    }
}

#[test]
fn validated_source_populates_exact_manifest_identity() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("all_hour.json");
    let bytes = br#"{"type":"FeatureCollection","features":[{"id":1},{"id":2}]}"#;
    std::fs::write(&source, bytes).expect("source JSON");

    let manifest = build_source_snapshot(input(source)).expect("source manifest");
    assert_eq!(manifest.state, SnapshotState::SourceValidated);
    assert_eq!(manifest.artifacts.source_json.bytes, bytes.len() as u64);
    assert_eq!(manifest.document.root_type, "FeatureCollection");
    assert_eq!(manifest.document.logical_records, 2);
    assert_eq!(
        manifest.artifacts.source_json.sha256,
        "49478cf187be82b2573f066f57bc0642cfc5cf48b8987983c43cdf9b929db5f1"
    );

    let schema: serde_json::Value = serde_json::from_str(SNAPSHOT_SCHEMA).expect("snapshot schema");
    let validator = Validator::new(&schema).expect("schema validator");
    let value = serde_json::to_value(&manifest).expect("manifest JSON");
    assert!(validator.is_valid(&value), "source-stage manifest is valid");
}

#[test]
fn invalid_source_never_produces_a_manifest() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("invalid.json");
    std::fs::write(&source, br#"{"type":"FeatureCollection"}"#).expect("invalid source");

    assert!(matches!(
        build_source_snapshot(input(source)),
        Err(ManifestError::GeoJson(_))
    ));
}

#[test]
fn snapshot_manifest_write_is_atomic_and_newline_terminated() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source.json");
    let destination = temp.path().join("manifests/snapshot.json");
    std::fs::write(&source, br#"{"type":"FeatureCollection","features":[]}"#).expect("source");
    let manifest = build_source_snapshot(input(source)).expect("manifest");

    write_snapshot_manifest(&destination, &manifest).expect("manifest write");
    let bytes = std::fs::read(&destination).expect("written manifest");
    assert_eq!(bytes.last(), Some(&b'\n'));
    let decoded: serde_json::Value = serde_json::from_slice(&bytes).expect("manifest JSON");
    assert_eq!(decoded["state"], "source-validated");
}

#[test]
fn archive_identity_is_serialized_when_source_was_extracted() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("Georgia.geojson");
    std::fs::write(&source, br#"{"type":"FeatureCollection","features":[]}"#).expect("source");
    let mut manifest_input = input(source);
    manifest_input.source_id = "microsoft-us-buildings-georgia".to_owned();
    manifest_input.archive = Some(ArchiveIdentity {
        format: "zip".to_owned(),
        member: "Georgia.geojson".to_owned(),
    });

    let manifest = build_source_snapshot(manifest_input).expect("manifest");
    assert_eq!(
        manifest.archive.expect("archive identity").member,
        "Georgia.geojson"
    );
}
