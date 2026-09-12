//! The executable manual corpus needs only committed fixtures, not its source checkout.

use std::{fs, path::Path, time::Duration};
use tq_test_support::compatibility::{ExecutableConfig, compare_manual, load_catalog};

#[test]
#[ignore = "requires built tq and the pinned jq reference"]
fn complete_manual_corpus_executes_from_a_relocated_fixture_only_checkout() {
    let original = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let relocated = tempfile::tempdir().unwrap();
    for subtree in [
        "tests/fixtures",
        "tests/compatibility/cases",
        "tests/compatibility/reviews/jq-manual",
    ] {
        copy_tree(&original.join(subtree), &relocated.path().join(subtree));
    }
    assert!(!relocated.path().join("jq-manual").exists());
    let config = ExecutableConfig {
        jq: Some(original.join("target/reference-build/jq/jq")),
        tq: Some(original.join("target/debug/tq")),
        yq: None,
    };
    let catalog = load_catalog(&relocated.path().join("tests/compatibility/cases")).unwrap();
    let report =
        compare_manual(&catalog, &config, relocated.path(), Duration::from_secs(5)).unwrap();
    assert!(report["cases"].as_array().unwrap().len() >= 518);
    for row in report["cases"].as_array().unwrap() {
        for tool in ["jq", "tq"] {
            assert_eq!(
                row[tool]["state"], "executed",
                "{} {tool}: {}",
                row["id"], row[tool]
            );
        }
    }
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let path = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &path);
        } else {
            fs::copy(entry.path(), path).unwrap();
        }
    }
}
