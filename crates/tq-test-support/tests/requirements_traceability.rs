//! Guards the release mapping for every MVP capability-spec scenario.

use std::{fs, path::Path};

#[test]
fn every_mvp_scenario_has_a_reviewed_evidence_route() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let traceability = fs::read_to_string(root.join("docs/requirements-traceability.md"))
        .expect("requirements traceability document");
    let routes = [
        (
            "benchmark-corpus",
            16,
            "crates/tq-test-support/tests/corpus_manifest.rs",
        ),
        (
            "cross-tool-compatibility",
            20,
            "tests/compatibility/reviews/coverage-v1.json",
        ),
        ("jq-core-language", 37, "crates/tq-core/src"),
        ("performance-benchmarks", 26, "benchmarks/2026-08-06.md"),
        ("query-runtime", 21, "tests/fuzz/fuzz_targets/vm_program.rs"),
        ("resource-governance", 17, "benchmarks/2026-08-06.md"),
        ("toon-stream-io", 21, "crates/tq-toon/tests"),
        ("tq-cli", 38, "crates/tq-cli/src"),
    ];

    let specs = root.join("openspec/changes/build-tq-mvp/specs");
    let discovered = fs::read_dir(&specs)
        .expect("MVP specs")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join("spec.md").is_file())
        .count();
    assert_eq!(discovered, routes.len(), "unmapped MVP capability spec");

    for (name, expected_scenarios, evidence) in routes {
        let spec = fs::read_to_string(specs.join(name).join("spec.md"))
            .unwrap_or_else(|error| panic!("read {name} spec: {error}"));
        let actual_scenarios = spec
            .lines()
            .filter(|line| line.starts_with("#### Scenario:"))
            .count();
        assert_eq!(
            actual_scenarios, expected_scenarios,
            "{name} scenarios changed without a traceability review"
        );
        assert!(
            traceability.contains(&format!("| `{name}` | {expected_scenarios} |")),
            "missing traceability row for {name}"
        );
        assert!(root.join(evidence).exists(), "missing evidence: {evidence}");
    }
}
