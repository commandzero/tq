//! Data-driven compatibility catalog schema and coverage tests.

use std::{collections::BTreeSet, fs, path::Path};

use serde_json::Value;

const SCHEMA: &str = include_str!("../../../schemas/compatibility-case-v1.schema.json");

#[test]
fn every_catalog_case_is_schema_valid_and_uniquely_identified() {
    let schema: Value = serde_json::from_str(SCHEMA).expect("case schema");
    let validator = jsonschema::Validator::new(&schema).expect("case validator");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../compatibility/cases");
    let mut ids = BTreeSet::new();
    let mut count = 0;
    for entry in fs::read_dir(root).expect("case directory") {
        let path = entry.expect("case entry").path();
        for (line_index, line) in fs::read_to_string(&path)
            .expect("case file")
            .lines()
            .enumerate()
        {
            if line.trim().is_empty() {
                continue;
            }
            let case: Value = serde_json::from_str(line)
                .unwrap_or_else(|error| panic!("{}:{}: {error}", path.display(), line_index + 1));
            assert!(
                validator.is_valid(&case),
                "{}:{} is not schema-valid",
                path.display(),
                line_index + 1
            );
            let id = case["id"].as_str().expect("case id");
            assert!(ids.insert(id.to_owned()), "duplicate case id {id}");
            count += 1;
        }
    }
    assert!(
        count >= 20,
        "catalog should contain the initial common surface"
    );
}

#[test]
fn common_and_navigation_capability_groups_are_present() {
    let cases = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../compatibility/cases/common.jsonl"),
    )
    .expect("common cases");
    for capability in [
        "value.null",
        "value.boolean",
        "value.number",
        "value.string",
        "value.array",
        "value.object",
        "cardinality.zero",
        "cardinality.one",
        "cardinality.many",
    ] {
        assert!(cases.contains(capability), "missing {capability}");
    }

    let navigation = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../compatibility/cases/navigation.jsonl"),
    )
    .expect("navigation cases");
    for capability in [
        "navigation.field",
        "navigation.computed",
        "navigation.index",
        "navigation.negative-index",
        "navigation.slice",
        "navigation.iteration",
        "navigation.optional",
        "error.runtime-type",
    ] {
        assert!(navigation.contains(capability), "missing {capability}");
    }
}
