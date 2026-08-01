//! Versioned corpus source and snapshot schema contract tests.

use jsonschema::Validator;
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, path::Path};

const SOURCE_SCHEMA: &str = include_str!("../../../schemas/corpus-source-v1.schema.json");
const SNAPSHOT_SCHEMA: &str = include_str!("../../../schemas/snapshot-manifest-v1.schema.json");

fn schema(source: &str) -> Value {
    serde_json::from_str(source).expect("schema must be JSON")
}

fn validator(source: &str) -> Validator {
    let schema = schema(source);
    assert!(
        jsonschema::meta::is_valid(&schema),
        "schema must satisfy its declared meta-schema"
    );
    jsonschema::validator_for(&schema).expect("schema must compile")
}

fn valid_source() -> Value {
    json!({
        "schema_version": 1,
        "id": "usgs-all-day",
        "kind": "geojson",
        "format": "json",
        "campaigns": ["standard"],
        "fetch": {
            "url": "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/all_day.geojson",
            "refresh": "mutable",
            "redirect_limit": 5,
            "expected_content_types": [
                "application/geo+json",
                "application/json"
            ]
        },
        "document": {
            "root_type": "FeatureCollection",
            "records_pointer": "/features"
        },
        "provenance": {
            "title": "USGS All Earthquakes, Past Day",
            "publisher": "U.S. Geological Survey",
            "landing_page": "https://earthquake.usgs.gov/earthquakes/feed/v1.0/geojson.php",
            "license": {
                "name": "USGS data policy",
                "url": "https://www.usgs.gov/information-policies-and-instructions/copyrights-and-credits"
            }
        }
    })
}

fn valid_snapshot() -> Value {
    json!({
        "schema_version": 1,
        "campaign_id": "2026-07-30T12-00-00Z",
        "source_id": "usgs-all-day",
        "retrieved_at": "2026-07-30T12:00:00Z",
        "request": {
            "requested_url": "https://earthquake.usgs.gov/all_day.geojson",
            "final_url": "https://earthquake.usgs.gov/all_day.geojson",
            "status": 200,
            "content_type": "application/geo+json",
            "etag": "\"example\"",
            "last_modified": "Wed, 30 Jul 2026 11:59:00 GMT"
        },
        "archive": null,
        "artifacts": {
            "download": {
                "path": "downloads/all_day.geojson",
                "bytes": 1024,
                "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            },
            "source_json": {
                "path": "prepared/all_day.json",
                "bytes": 1024,
                "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            },
            "generated": {
                "yaml": {
                    "path": "prepared/all_day.yaml",
                    "bytes": 1100,
                    "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                },
                "toon": {
                    "path": "prepared/all_day.toon",
                    "bytes": 900,
                    "sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                }
            }
        },
        "document": {
            "root_type": "FeatureCollection",
            "logical_records": 12
        },
        "validation": {
            "source_json": "valid",
            "yaml_equivalent": true,
            "toon_equivalent": true
        },
        "provenance": {
            "title": "USGS All Earthquakes, Past Day",
            "publisher": "U.S. Geological Survey",
            "landing_page": "https://earthquake.usgs.gov/earthquakes/feed/v1.0/geojson.php",
            "license": {
                "name": "USGS Government Work in the U.S. Public Domain",
                "url": "https://www.usgs.gov/information-policies-and-instructions/copyrights-and-credits"
            }
        }
    })
}

#[test]
fn schemas_are_valid_draft_2020_12() {
    validator(SOURCE_SCHEMA);
    validator(SNAPSHOT_SCHEMA);
}

#[test]
fn source_schema_accepts_a_versioned_natural_source() {
    assert!(validator(SOURCE_SCHEMA).is_valid(&valid_source()));
}

#[test]
fn source_schema_rejects_unknown_versions_and_incomplete_archives() {
    let validator = validator(SOURCE_SCHEMA);

    let mut wrong_version = valid_source();
    wrong_version["schema_version"] = json!(2);
    assert!(!validator.is_valid(&wrong_version));

    let mut incomplete_archive = valid_source();
    incomplete_archive["archive"] = json!({"format": "zip"});
    assert!(!validator.is_valid(&incomplete_archive));
}

#[test]
fn source_schema_rejects_unreviewed_fields() {
    let validator = validator(SOURCE_SCHEMA);
    let mut source = valid_source();
    source["nominal_size_bytes"] = json!(1_000_000);
    assert!(!validator.is_valid(&source));
}

#[test]
fn snapshot_schema_accepts_a_self_describing_snapshot() {
    assert!(validator(SNAPSHOT_SCHEMA).is_valid(&valid_snapshot()));
}

#[test]
fn snapshot_schema_rejects_invalid_digests_and_missing_http_identity() {
    let validator = validator(SNAPSHOT_SCHEMA);

    let mut bad_digest = valid_snapshot();
    bad_digest["artifacts"]["download"]["sha256"] = json!("abc");
    assert!(!validator.is_valid(&bad_digest));

    let mut missing_final_url = valid_snapshot();
    missing_final_url["request"]
        .as_object_mut()
        .expect("request object")
        .remove("final_url");
    assert!(!validator.is_valid(&missing_final_url));
}

#[test]
fn snapshot_schema_rejects_absolute_artifact_paths_and_extra_fields() {
    let validator = validator(SNAPSHOT_SCHEMA);

    let mut absolute_path = valid_snapshot();
    absolute_path["artifacts"]["source_json"]["path"] = json!("/tmp/all_day.json");
    assert!(!validator.is_valid(&absolute_path));

    let mut extra = valid_snapshot();
    extra["resized_to_bytes"] = json!(1_000_000);
    assert!(!validator.is_valid(&extra));
}

#[test]
fn registered_natural_sources_are_valid_and_uniquely_identified() {
    let validator = validator(SOURCE_SCHEMA);
    let source_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/sources");
    let mut ids = BTreeSet::new();

    let mut paths = fs::read_dir(source_dir)
        .expect("source registry must exist")
        .map(|entry| entry.expect("source directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();

    for path in paths {
        let source: Value =
            serde_json::from_slice(&fs::read(&path).expect("registered source must be readable"))
                .expect("registered source must be JSON");
        assert!(
            validator.is_valid(&source),
            "{} must satisfy the source schema",
            path.display()
        );
        let id = source["id"].as_str().expect("source id must be a string");
        assert!(ids.insert(id.to_owned()), "duplicate source id: {id}");
    }

    assert_eq!(
        ids,
        [
            "microsoft-us-buildings-georgia",
            "usgs-all-day",
            "usgs-all-hour",
            "usgs-all-month",
            "usgs-all-week"
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    );
}
