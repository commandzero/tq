//! Repository fixture storage must stay TOON-first.

use std::{fs, path::Path};
use tq_test_support::compatibility::load_catalog;

#[test]
fn full_execution_reports_stay_out_of_fixture_storage() {
    fn visit(path: &Path) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                visit(&path);
            } else {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                assert!(
                    !(name.starts_with("comparison")
                        || name.starts_with("manual-comparison.")
                        || name.starts_with("manual-execution.")
                        || name == "execution.toon"
                        || name.starts_with("coverage-v")),
                    "write full execution reports to ignored target/ storage, not {}",
                    path.display()
                );
            }
        }
    }
    visit(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/compatibility/reviews"));
}

#[test]
fn root_test_tree_keeps_jsonl_catalogs_but_no_json_documents() {
    fn visit(path: &Path) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                visit(&path);
            } else {
                assert!(
                    !path
                        .extension()
                        .is_some_and(|ext| matches!(ext.to_str(), Some("json" | "ndjson"))),
                    "store repository fixtures as TOON: {}",
                    path.display()
                );
            }
        }
    }
    visit(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests"));
    let catalogs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/compatibility/cases");
    let paths: Vec<_> = fs::read_dir(&catalogs)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert!(
        !paths.is_empty(),
        "the compatibility catalog must have fixtures"
    );
    assert!(
        paths
            .iter()
            .all(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
    );
    let catalog = load_catalog(&catalogs).expect("every JSONL catalog is schema-valid");
    assert!(!catalog.cases.is_empty(), "the catalog must contain cases");
}

#[test]
fn fixture_literal_tags_reject_invalid_values() {
    for text in [
        "$tq.fixture.utf8hex: abc\n",
        "$tq.fixture.utf8hex: zz\n",
        "$tq.fixture.utf8hex: ff\n",
        "$tq.fixture.number: nope\n",
    ] {
        assert!(
            tq_test_support::fixture_data::from_toon::<serde_json::Value>(text.as_bytes()).is_err()
        );
    }
}

#[test]
fn exact_number_lexemes_survive_typed_storage() {
    let value: serde_json::Value = serde_json::from_str(
        "[123456789012345670000000000000000000000000000000000000000000,1.000,1e1000001]",
    )
    .unwrap();
    let encoded = tq_test_support::fixture_data::to_toon(&value).unwrap();
    let decoded: serde_json::Value =
        tq_test_support::fixture_data::from_toon(encoded.as_bytes()).unwrap();
    assert_eq!(decoded, value);
    assert!(
        tq_test_support::fixture_data::to_toon(&serde_json::json!({"$tq.fixture.number":"123"}))
            .is_err()
    );
}
