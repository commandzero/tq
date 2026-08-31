//! Guards the one-to-one release mapping for every MVP scenario.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[derive(Debug, Eq, PartialEq)]
struct Scenario {
    spec: String,
    requirement: String,
    title: String,
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "the bidirectional traceability audit is intentionally one linear invariant"
)]
fn every_mvp_scenario_has_one_explicit_existing_evidence_route() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = fs::read_to_string(root.join("docs/requirements-traceability.tsv"))
        .expect("scenario traceability manifest");
    let mut lines = manifest.lines();
    assert_eq!(
        lines.next(),
        Some("scenario_id\tspec\trequirement\tscenario\tevidence")
    );

    let mut mapped = BTreeMap::new();
    for (offset, line) in lines.enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(
            fields.len(),
            5,
            "traceability row {} must have five TSV fields",
            offset + 2
        );
        let id = fields[0].to_owned();
        let scenario = Scenario {
            spec: fields[1].to_owned(),
            requirement: fields[2].to_owned(),
            title: fields[3].to_owned(),
        };
        let evidence = fields[4].to_owned();
        let (kind, locator) = evidence
            .split_once(':')
            .unwrap_or_else(|| panic!("{id} evidence must be kind:path#symbol: {evidence}"));
        assert!(
            matches!(kind, "test" | "report" | "manual"),
            "{id} has unsupported evidence kind {kind}"
        );
        let (path, symbol) = locator
            .rsplit_once('#')
            .unwrap_or_else(|| panic!("{id} evidence must name a symbol: {evidence}"));
        assert_ne!(symbol, "", "{id} has an empty evidence symbol");
        let evidence_path = root.join(path);
        assert!(
            evidence_path.is_file(),
            "{id} names missing or non-file evidence path: {path}"
        );
        let evidence_source = fs::read_to_string(&evidence_path)
            .unwrap_or_else(|error| panic!("read evidence for {id}: {error}"));
        let anchored = match kind {
            "test" | "report" => {
                evidence_source.contains(&format!("fn {symbol}("))
                    || (symbol == "fuzz_target" && evidence_source.contains("fuzz_target!("))
            }
            "manual" => evidence_source
                .lines()
                .any(|line| line.trim_start_matches('#').trim() == symbol),
            _ => unreachable!("evidence kind was validated"),
        };
        assert!(
            anchored,
            "{id} evidence symbol {symbol:?} is absent from {path}"
        );
        assert!(
            mapped.insert(id.clone(), (scenario, evidence)).is_none(),
            "duplicate traceability ID: {id}"
        );
    }

    let specs = root.join("openspec/changes/archive/2026-08-07-build-tq-mvp/specs");
    let mut discovered = BTreeMap::new();
    let mut spec_names = BTreeSet::new();
    for entry in fs::read_dir(&specs).expect("MVP specs") {
        let path = entry.expect("spec directory entry").path().join("spec.md");
        if !path.is_file() {
            continue;
        }
        let spec = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|value| value.to_str())
            .expect("UTF-8 spec name")
            .to_owned();
        spec_names.insert(spec.clone());
        let contents = fs::read_to_string(&path).expect("read MVP spec");
        let mut requirement = None;
        for line in contents.lines() {
            if let Some(title) = line.strip_prefix("### Requirement: ") {
                requirement = Some(title.to_owned());
            } else if let Some(title) = line.strip_prefix("#### Scenario: ") {
                let scenario = Scenario {
                    spec: spec.clone(),
                    requirement: requirement.clone().expect("scenario follows requirement"),
                    title: title.to_owned(),
                };
                let id = format!("{}.{}", spec, slug(title));
                assert!(
                    discovered.insert(id.clone(), scenario).is_none(),
                    "scenario IDs must be unique within the change: {id}"
                );
            }
        }
    }

    assert_eq!(
        mapped.keys().collect::<BTreeSet<_>>(),
        discovered.keys().collect::<BTreeSet<_>>(),
        "traceability rows must exactly match the OpenSpec scenarios"
    );
    for (id, expected) in discovered {
        assert_eq!(
            &mapped[&id].0, &expected,
            "traceability title or requirement drifted for {id}"
        );
    }
    assert_eq!(
        mapped
            .values()
            .map(|(scenario, _)| &scenario.spec)
            .collect::<BTreeSet<_>>(),
        spec_names.iter().collect(),
        "every capability spec must have mapped scenarios"
    );
}

fn slug(title: &str) -> String {
    let mut output = String::new();
    let mut separator = false;
    for character in title.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            if separator && !output.is_empty() {
                output.push('-');
            }
            output.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    output
}
