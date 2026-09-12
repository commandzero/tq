//! Trace the official manual's input/output examples to runnable catalog cases.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::Duration,
};

use serde_json::Value;
use tq_test_support::compatibility::{
    BaselinePolicy, CaseStatus, ContractKind, Invocation, InvocationMode, ProcessOutcome,
    ProcessStatus, load_catalog, manual_case_ids, normalize_jq, read_manual_ledger,
    run_process_with_environment,
};

#[test]
#[ignore = "requires the jq 1.8 reference binary; run explicitly after downloading it"]
fn jq_reference_matches_the_published_manual_results() {
    let source: Value = tq_test_support::fixture_data::from_toon(include_bytes!(
        "../../../tests/compatibility/reviews/jq-manual/source-examples.toon"
    ))
    .expect("manual source inventory");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let executable = std::env::var_os("TQ_JQ").map_or_else(
        || root.join("target/reference-build/jq/jq"),
        |path| root.join(path),
    );
    let pin = tq_test_support::fixture_data::read(
        &root.join("tests/compatibility/reviews/jq-manual/reference-pin.toon"),
    )
    .expect("reviewed manual reference pin");
    let identity = tq_test_support::compatibility::discover_tool(
        tq_test_support::compatibility::ToolKind::Jq,
        &tq_test_support::compatibility::ExecutableConfig {
            jq: Some(executable.clone()),
            ..Default::default()
        },
        &root,
    )
    .expect("discover jq")
    .expect("jq reference is required");
    tq_test_support::compatibility::validate_manual_reference(
        &pin,
        &identity,
        &tq_test_support::compatibility::manual_host_target(),
    )
    .expect("reference must match the reviewed build");
    let mut failures = Vec::new();
    for section in source["sections"].as_array().expect("sections") {
        for example in section["table_examples"].as_array().expect("examples") {
            let expected = ProcessOutcome {
                status: ProcessStatus::Exited,
                exit_code: Some(0),
                signal: None,
                stdout: example["reference_stdout"]
                    .as_str()
                    .or_else(|| example["expected_stdout"].as_str())
                    .expect("published output")
                    .as_bytes()
                    .to_vec(),
                stderr: Vec::new(),
                wall_time_micros: 0,
                recorded_command: Vec::new(),
            };
            let expected = normalize_jq(&expected).expect("published JSON result sequence");
            if example.get("reference_stdout").is_some() {
                assert!(
                    example["reference_note"]
                        .as_str()
                        .is_some_and(|note| !note.is_empty())
                );
            }
            let actual = run_process_with_environment(
                &Invocation {
                    executable: executable.clone(),
                    args: vec![example["query"].as_str().expect("query").to_owned()],
                    stdin: example["input"]
                        .as_str()
                        .expect("input")
                        .as_bytes()
                        .to_vec(),
                    timeout: Duration::from_secs(5),
                    current_dir: Some(root.clone()),
                    environment: BTreeMap::new(),
                },
                &BTreeMap::from([("PAGER".to_owned(), "less".to_owned())]),
            )
            .expect("run jq reference");
            let actual = normalize_jq(&actual).expect("jq JSON output");
            if actual.results != expected.results || actual.exit_code != Some(0) {
                failures.push(format!(
                    "{}:{} expected {:?}, observed {:?}, exit {:?}",
                    section["file"],
                    example["line"],
                    expected.results,
                    actual.results,
                    actual.exit_code
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_manual_input_output_example_has_an_executable_case() {
    let source: Value = tq_test_support::fixture_data::from_toon(include_bytes!(
        "../../../tests/compatibility/reviews/jq-manual/source-examples.toon"
    ))
    .expect("manual source inventory");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = load_catalog(&root.join("tests/compatibility/cases")).expect("case catalog");
    let sections = source["sections"].as_array().expect("manual sections");
    assert_eq!(
        sections.len(),
        13,
        "all manual sections must be inventoried"
    );
    let mut missing = Vec::new();
    let mut count = 0;
    for section in sections {
        for example in section["table_examples"]
            .as_array()
            .expect("table examples")
        {
            count += 1;
            let query = example["query"].as_str().expect("source query");
            let input = example["input"].as_str().expect("source input");
            let covered = catalog.cases.iter().any(|case| {
                case.query == query
                    && case.adapters.jq.query.as_deref().unwrap_or(&case.query) == query
                    && case.adapters.tq.query.as_deref().unwrap_or(&case.query) == query
                    && case.fixture.inline.as_deref().map(str::trim) == Some(input.trim())
                    && case.status == CaseStatus::Mvp
                    && case.adapters.jq.supported
                    && case.adapters.tq.supported
            });
            if !covered {
                missing.push(format!(
                    "{}:{}: {query}",
                    section["file"].as_str().expect("source file"),
                    example["line"]
                ));
            }
        }
    }
    assert_eq!(count, 251, "manual snapshot example count changed");
    assert!(
        missing.is_empty(),
        "{} manual examples lack executable jq/tq cases:\n{}",
        missing.len(),
        missing.join("\n")
    );
}

#[test]
fn every_section_audit_accounts_for_fences_and_references_real_cases() {
    let source: Value = tq_test_support::fixture_data::from_toon(include_bytes!(
        "../../../tests/compatibility/reviews/jq-manual/source-examples.toon"
    ))
    .expect("manual source inventory");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = load_catalog(&root.join("tests/compatibility/cases")).expect("case catalog");
    let introduction = serde_json::json!({
        "section": "introduction",
        "fenced_examples": []
    });
    for section in source["sections"]
        .as_array()
        .expect("sections")
        .iter()
        .chain(std::iter::once(&introduction))
    {
        let name = section["section"].as_str().expect("section name");
        let path = root.join(format!("tests/compatibility/reviews/jq-manual/{name}.toon"));
        let audit = read_manual_ledger(&path).expect("section audit ledger");
        let examples = audit["examples"].as_array().expect("audit examples");
        for fence in section["fenced_examples"].as_array().expect("fences") {
            let line = fence["line"].as_u64().expect("fence line");
            assert!(
                examples
                    .iter()
                    .chain(audit["coverage_notes"].as_array().into_iter().flatten())
                    .any(|example| {
                        example["source_line"].as_u64() == Some(line)
                            || example["source_fence_line"].as_u64() == Some(line)
                    }),
                "{name}:{line}: fenced snippet missing from audit"
            );
        }
        for example in examples
            .iter()
            .chain(audit["coverage_notes"].as_array().into_iter().flatten())
        {
            let mut ids = manual_case_ids(example).expect("case references");
            ids.extend(
                audit["coverage_evidence"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|edge| edge["note_id"] == example["id"])
                    .map(|edge| edge["case_id"].as_str().expect("evidence case ID")),
            );
            if matches!(
                example["disposition"].as_str(),
                Some("new" | "covered" | "adapted")
            ) {
                assert!(
                    !ids.is_empty(),
                    "{name}: {} claims coverage without executable case evidence",
                    example["id"]
                );
                assert!(
                    ids.iter().any(|id| catalog.cases.iter().any(|case| {
                        case.id == *id
                            && case.status == CaseStatus::Mvp
                            && case.adapters.jq.supported
                            && case.adapters.tq.supported
                    })),
                    "{name}: {} claims coverage without an enabled jq/tq case",
                    example["id"]
                );
            }
            if ids.is_empty() {
                assert!(
                    example["reason"]
                        .as_str()
                        .is_some_and(|reason| !reason.is_empty()),
                    "{name}: an example without cases must explain its exclusion"
                );
            }
            for id in ids {
                assert!(
                    catalog.cases.iter().any(|case| case.id == id),
                    "{name}: audit references missing case {id}"
                );
            }
        }
    }
}

#[test]
fn corrected_math_model_is_tabular_equivalent_and_preserves_case_coverage() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("tests/compatibility/reviews/jq-manual/math.toon");
    let toon = read_manual_ledger(&path).unwrap();
    let json: Value = serde_json::from_slice(&serde_json::to_vec(&toon).unwrap()).unwrap();
    assert_eq!(json, toon, "comparison JSON must mirror canonical TOON");
    assert_eq!(toon["schema_version"], 2);
    let text = fs::read_to_string(path.with_extension("toon")).unwrap();
    assert!(text.contains("examples[62]{"));
    assert!(text.contains("coverage_notes[6]{"));
    let examples = toon["examples"].as_array().unwrap();
    let catalog = load_catalog(&root.join("tests/compatibility/cases")).unwrap();
    let mut ids = BTreeSet::new();
    for example in examples {
        assert!(example["kind"] != "prose");
        assert!(example.get("case_ids").is_none());
        assert!(
            example
                .as_object()
                .unwrap()
                .values()
                .all(|v| !v.is_array() && !v.is_object())
        );
        let case_id = example["case_id"].as_str().unwrap();
        assert!(ids.insert(case_id), "one row per executable scenario");
        let case = catalog
            .cases
            .iter()
            .find(|case| case.id == case_id)
            .unwrap();
        assert_eq!(Some(case.query.as_str()), example["query"].as_str());
        assert_eq!(
            case.fixture.inline.as_deref().map(str::trim),
            example["input"].as_str().map(str::trim)
        );
    }
    assert_eq!(ids.len(), 62);
    let notes = toon["coverage_notes"].as_array().unwrap();
    assert_eq!(notes.len(), 6);
    assert_eq!(
        notes
            .iter()
            .filter(|n| n["evidence_case_id"].is_null())
            .count(),
        3
    );
    for note in notes {
        if let Some(id) = note["evidence_case_id"].as_str() {
            assert!(ids.contains(id));
        }
    }
}

#[test]
fn invalid_canonical_toon_never_falls_back_to_json() {
    let directory = tempfile::tempdir().unwrap();
    let json = directory.path().join("manual-example.json");
    fs::write(&json, "{}").unwrap();
    fs::write(json.with_extension("toon"), "items[2]{id}:\n  1\n").unwrap();
    assert!(read_manual_ledger(&json).is_err());
}

#[test]
fn legacy_case_reference_arrays_are_rejected() {
    for ids in [
        serde_json::json!([]),
        serde_json::json!(["a"]),
        serde_json::json!(["a", "b"]),
    ] {
        assert!(manual_case_ids(&serde_json::json!({"case_ids":ids})).is_err());
    }
}

#[test]
fn every_section_uses_scalar_examples_notes_and_explicit_evidence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = load_catalog(&root.join("tests/compatibility/cases")).unwrap();
    let mut sections = 0;
    for entry in fs::read_dir(root.join("tests/compatibility/reviews/jq-manual")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "toon") {
            continue;
        }
        let ledger = read_manual_ledger(&path).unwrap();
        let Some(examples) = ledger["examples"].as_array() else {
            continue;
        };
        sections += usize::from(ledger["section"].is_string());
        assert_eq!(ledger["schema_version"], 2, "{}", path.display());
        let text = fs::read_to_string(&path).unwrap();
        let mut ids = BTreeSet::new();
        for name in ["examples", "coverage_notes"] {
            let rows = ledger[name].as_array().expect("required collection");
            if !rows.is_empty() {
                assert!(
                    text.contains(&format!("{name}[{}]{{", rows.len())),
                    "{path:?}: {name} must be tabular"
                );
            }
            for row in rows {
                assert!(row.get("case_ids").is_none(), "{path:?}: legacy case_ids");
                assert!(
                    row.as_object()
                        .unwrap()
                        .values()
                        .all(|v| !v.is_object() && !v.is_array()),
                    "{path:?}: non-scalar {name} row"
                );
                assert!(ids.insert(row["id"].as_str().unwrap()));
            }
        }
        let mut cases = BTreeSet::new();
        for example in examples {
            let id = example["case_id"]
                .as_str()
                .expect("required scalar case_id");
            assert!(cases.insert(id), "{path:?}: duplicate executable case {id}");
            let case = catalog.cases.iter().find(|case| case.id == id).unwrap();
            assert_eq!(example["query"].as_str(), Some(case.query.as_str()), "{id}");
            if let Some(hex) = example["input_hex"].as_str() {
                assert!(example["input"].is_null());
                assert_eq!(
                    hex,
                    tq_test_support::compatibility::encode_hex(
                        case.fixture.inline.as_ref().unwrap().as_bytes()
                    )
                );
            } else {
                assert_eq!(
                    example["input"].as_str().map(str::trim),
                    case.fixture.inline.as_deref().map(str::trim),
                    "{id}"
                );
            }
        }
        let notes = ledger["coverage_notes"].as_array().unwrap();
        for note in notes {
            assert!(note.get("evidence_case_id").is_some());
            if let Some(id) = note["evidence_case_id"].as_str() {
                assert!(
                    cases.contains(id) || catalog.cases.iter().any(|case| case.id == id),
                    "{path:?}: missing evidence case {id}"
                );
            }
        }
        let mut edges = BTreeSet::new();
        for edge in ledger["coverage_evidence"].as_array().into_iter().flatten() {
            let note = notes
                .iter()
                .find(|note| note["id"] == edge["note_id"])
                .expect("evidence owner");
            assert!(note["evidence_case_id"].is_null());
            let case = edge["case_id"].as_str().unwrap();
            assert!(cases.contains(case));
            assert!(edges.insert((edge["note_id"].as_str().unwrap(), case)));
        }
        for value in ledger["source_values"].as_array().into_iter().flatten() {
            assert!(ids.contains(value["entry_id"].as_str().unwrap()));
            assert!(value["value"].is_array() || value["value"].is_object());
        }
    }
    assert_eq!(sections, 14, "all 13 manual sections plus introduction");
}

#[test]
fn source_corrections_retain_invalid_math_probes_and_add_valid_arities() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = load_catalog(&root.join("tests/compatibility/cases")).unwrap();
    let ledger =
        read_manual_ledger(&root.join("tests/compatibility/reviews/jq-manual/math.toon")).unwrap();
    let examples = ledger["examples"].as_array().unwrap();

    for (name, invalid_id, valid_id, invalid_query, valid_input) in [
        (
            "frexp",
            "manual.math.frexp-reference-unavailable",
            "manual.math.frexp-actual-arity",
            "frexp(8;0)",
            "8",
        ),
        (
            "modf",
            "manual.math.modf-reference-unavailable",
            "manual.math.modf-actual-arity",
            "modf(3;0)",
            "3.5",
        ),
    ] {
        let invalid = catalog
            .cases
            .iter()
            .find(|case| case.id == invalid_id)
            .unwrap_or_else(|| panic!("missing invalid {name}/2 case"));
        assert_eq!(invalid.query, invalid_query);
        assert_eq!(invalid.expected.contract, ContractKind::Error);
        assert_eq!(invalid.expected.baseline, BaselinePolicy::Required);
        assert_eq!(
            invalid.expected.error_class.as_deref(),
            Some("query-compile")
        );
        assert!(
            invalid
                .capabilities
                .iter()
                .any(|capability| capability == "manual.math.invalid-arity")
        );
        assert!(invalid.adapters.jq.supported);
        assert!(invalid.adapters.tq.supported);

        let valid = catalog
            .cases
            .iter()
            .find(|case| case.id == valid_id)
            .unwrap_or_else(|| panic!("missing valid {name}/0 case"));
        assert_eq!(valid.query, name);
        assert_eq!(valid.fixture.inline.as_deref(), Some(valid_input));
        assert_eq!(valid.expected.contract, ContractKind::ResultSequence);
        assert_eq!(valid.expected.baseline, BaselinePolicy::Required);
        assert!(
            valid
                .capabilities
                .iter()
                .any(|capability| capability == "manual.math.arity-correction")
        );
        assert!(valid.adapters.jq.supported);
        assert!(valid.adapters.tq.supported);

        for id in [invalid_id, valid_id] {
            assert!(examples.iter().any(|example| example["case_id"] == id));
        }
    }
}

#[test]
fn source_whitespace_corrections_keep_published_and_reference_values() {
    let source: Value = tq_test_support::fixture_data::from_toon(include_bytes!(
        "../../../tests/compatibility/reviews/jq-manual/source-examples.toon"
    ))
    .expect("manual source inventory");
    let corrections = source["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|section| section["table_examples"].as_array().into_iter().flatten())
        .filter(|example| example.get("reference_note").is_some())
        .collect::<Vec<_>>();
    assert_eq!(corrections.len(), 2);
    for (line, query, published, observed) in [
        (
            1122,
            "join(\" \")",
            "\"a 1 2.3 true false\"\n",
            "\"a 1 2.3 true  false\"\n",
        ),
        (
            117,
            "match(\"foo (?<bar123>bar)? foo\"; \"ig\")",
            "{\"offset\": 0, \"length\": 11, \"string\": \"foo bar foo\", \"captures\": [{\"offset\": 4, \"length\": 3, \"string\": \"bar\", \"name\": \"bar123\"}]}\n{\"offset\": 12, \"length\": 8, \"string\": \"foo foo\", \"captures\": [{\"offset\": -1, \"length\": 0, \"string\": null, \"name\": \"bar123\"}]}\n",
            "{\"offset\": 0, \"length\": 11, \"string\": \"foo bar foo\", \"captures\": [{\"offset\": 4, \"length\": 3, \"string\": \"bar\", \"name\": \"bar123\"}]}\n{\"offset\": 12, \"length\": 8, \"string\": \"foo  foo\", \"captures\": [{\"offset\": -1, \"length\": 0, \"string\": null, \"name\": \"bar123\"}]}\n",
        ),
    ] {
        let example = corrections
            .iter()
            .find(|example| example["line"] == line && example["query"] == query)
            .unwrap_or_else(|| panic!("missing whitespace correction at line {line}"));
        assert_eq!(example["expected_stdout"], published);
        assert_eq!(example["reference_stdout"], observed);
        assert_ne!(published, observed);
        let note = example["reference_note"].as_str().unwrap();
        assert!(note.contains("Verified locally"));
        assert!(note.contains("published text remains"));
    }
}

#[test]
fn regex_null_input_fences_preserve_their_explicit_jq_n_mode() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = load_catalog(&root.join("tests/compatibility/cases")).unwrap();
    for id in [
        "manual.regex.fence-whitespace-extended",
        "manual.regex.fence-inline-flags",
    ] {
        let case = catalog.cases.iter().find(|case| case.id == id).unwrap();
        assert_eq!(case.invocation_mode, InvocationMode::NullInput);
        assert_eq!(case.adapters.jq.args, ["-n"]);
        assert_eq!(case.adapters.tq.args, ["-n"]);
    }
}
