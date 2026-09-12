//! Frozen source and reference identities reject unreviewed corpus drift.

use std::path::Path;
use tq_test_support::compatibility::{
    ManualReferencePin, ToolIdentity, ToolKind, validate_manual_reference,
    validate_manual_source_checkout, validate_manual_source_inventory,
    validate_pinned_manual_coverage,
};

fn pinned_baseline_report(pin: &ManualReferencePin) -> serde_json::Value {
    serde_json::json!({
        "cases": pin.baseline_cases.iter().map(|case| serde_json::json!({
            "id": case.id,
            "verdict": if case.exact_match { "match" } else { "failure" },
        })).collect::<Vec<_>>(),
    })
}

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn pin() -> ManualReferencePin {
    tq_test_support::fixture_data::read(
        &root().join("tests/compatibility/reviews/jq-manual/reference-pin.toon"),
    )
    .unwrap()
}

#[test]
fn source_inventory_is_frozen_by_bytes_and_section_identity() {
    let pin = pin();
    let bytes =
        std::fs::read(root().join("tests/compatibility/reviews/jq-manual/source-examples.toon"))
            .unwrap();
    validate_manual_source_inventory(&pin, &bytes).unwrap();
    let mut changed = bytes;
    changed.push(b'\n');
    assert!(validate_manual_source_inventory(&pin, &changed).is_err());
    let mut changed_pin = pin.clone();
    changed_pin.sections.pop();
    assert!(validate_manual_source_inventory(&changed_pin, &changed[..changed.len() - 1]).is_err());
}

#[test]
fn reference_identity_is_path_independent_but_not_version_or_binary_independent() {
    let pin = pin();
    let reference = &pin.references[0];
    let identity = ToolIdentity {
        tool: ToolKind::Jq,
        path: "relocated/jq".into(),
        version: reference.version.clone(),
        executable: tq_test_support::corpus::ArtifactIdentity {
            path: "relocated/jq".into(),
            bytes: reference.bytes,
            sha256: reference.sha256.clone(),
        },
        build_features: reference
            .build_configuration
            .lines()
            .map(str::to_owned)
            .collect(),
        runtime_libraries: reference.runtime_libraries.clone(),
    };
    validate_manual_reference(&pin, &identity, &reference.target).unwrap();
    for changed in [
        ToolIdentity {
            version: "jq-1.8.2".into(),
            ..identity.clone()
        },
        ToolIdentity {
            build_features: vec!["different configuration".into()],
            ..identity.clone()
        },
        ToolIdentity {
            executable: tq_test_support::corpus::ArtifactIdentity {
                sha256: "00".repeat(32),
                ..identity.executable.clone()
            },
            ..identity.clone()
        },
        ToolIdentity {
            runtime_libraries: vec![tq_test_support::corpus::ArtifactIdentity {
                path: "/usr/lib/libjq.so.1".into(),
                bytes: 1,
                sha256: "11".repeat(32),
            }],
            ..identity.clone()
        },
    ] {
        assert!(validate_manual_reference(&pin, &changed, &reference.target).is_err());
    }
    assert!(validate_manual_reference(&pin, &identity, "unverified-target").is_err());
}

#[test]
fn missing_host_reference_evidence_blocks_the_release_claim() {
    let mut pin = pin();
    let target = tq_test_support::compatibility::manual_host_target();
    pin.references.clear();
    let error = validate_manual_reference(
        &pin,
        &ToolIdentity {
            tool: ToolKind::Jq,
            path: "missing/jq".into(),
            version: "jq-1.8.1".into(),
            executable: tq_test_support::corpus::ArtifactIdentity {
                path: "missing/jq".into(),
                bytes: 0,
                sha256: String::new(),
            },
            build_features: Vec::new(),
            runtime_libraries: Vec::new(),
        },
        &target,
    )
    .expect_err("an unpinned host cannot claim compatibility");
    assert!(error.to_string().contains("unverified or ambiguous"));
}

#[test]
fn source_checkout_verification_rejects_changed_and_missing_section_files() {
    let directory = tempfile::tempdir().unwrap();
    let mut pin = pin();
    pin.sections.truncate(1);
    let section = &mut pin.sections[0];
    section.file = "section.md".into();
    section.sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into();
    std::fs::write(directory.path().join("section.md"), b"").unwrap();
    validate_manual_source_checkout(&pin, directory.path()).unwrap();
    std::fs::write(directory.path().join("section.md"), b"changed").unwrap();
    assert!(validate_manual_source_checkout(&pin, directory.path()).is_err());
    std::fs::remove_file(directory.path().join("section.md")).unwrap();
    assert!(validate_manual_source_checkout(&pin, directory.path()).is_err());
}

#[test]
fn replacing_an_original_case_cannot_preserve_coverage_by_count_alone() {
    let pin = pin();
    let mut report = pinned_baseline_report(&pin);
    validate_pinned_manual_coverage(&pin, &report).unwrap();
    let original = pin
        .baseline_cases
        .first()
        .expect("pinned original cases are non-empty");
    let row = report["cases"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] == original.id)
        .expect("first pinned original case is present in the report");
    row["id"] = "replacement-with-the-same-count".into();
    assert!(
        validate_pinned_manual_coverage(&pin, &report)
            .unwrap_err()
            .to_string()
            .contains("original case missing")
    );
}

#[test]
fn original_matches_cannot_be_reclassified_as_disparities() {
    let pin = pin();
    let mut report = pinned_baseline_report(&pin);
    let original = pin
        .baseline_cases
        .iter()
        .find(|case| case.exact_match)
        .unwrap();
    let row = report["cases"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] == original.id)
        .unwrap();
    row["verdict"] = "disparity".into();
    assert!(
        validate_pinned_manual_coverage(&pin, &report)
            .unwrap_err()
            .to_string()
            .contains("original exact match regressed")
    );
}

#[test]
fn documented_arities_are_source_linked_and_executed_without_adapter_rewrites() {
    let ledger: serde_json::Value = tq_test_support::fixture_data::read(
        &root().join("tests/compatibility/reviews/jq-manual/arity-inventory.toon"),
    )
    .unwrap();
    let rows = ledger["requirements"].as_array().unwrap();
    assert_eq!(rows.len(), 220);
    let signatures = rows
        .iter()
        .map(|row| row["signature"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        signatures
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        220
    );
    let source_pin = pin();
    for row in rows {
        assert!(
            source_pin
                .sections
                .iter()
                .any(|source| row["source"] == source.file)
        );
        assert!(row["source_line"].as_u64().unwrap() > 0);
    }
    let catalog =
        tq_test_support::compatibility::load_catalog(&root().join("tests/compatibility/cases"))
            .unwrap();
    let case = catalog
        .cases
        .iter()
        .find(|case| case.id == "manual.audit.documented-arities")
        .unwrap();
    assert_eq!(
        case.query,
        format!("{} - builtins", serde_json::to_string(&signatures).unwrap())
    );
    assert_eq!(ledger["examples"][0]["query"], case.query);
    assert_eq!(ledger["examples"][0]["case_id"], case.id);
    assert!(case.adapters.jq.query.is_none());
    assert!(case.adapters.tq.query.is_none());
}
