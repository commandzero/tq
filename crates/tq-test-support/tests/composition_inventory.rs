//! Source-linked inventory for bounded user-filter composition admission.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tq_core::{ResolveOptions, Value as RuntimeValue, Vm, VmLimits, analyze, parse, resolve};
use tq_test_support::compatibility::{
    CaseAdapter, CaseStatus, CompatibilityCase, load_catalog, read_manual_review_case_ids,
};

type Cases<'a> = BTreeMap<&'a str, &'a CompatibilityCase>;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn source_operation_variants(source: &str) -> Vec<String> {
    let start = source
        .find("pub(crate) enum Operation {")
        .expect("Operation enum");
    let end = source[start..]
        .find("\n}\n\n#[derive(Clone, Debug)]")
        .map(|offset| start + offset)
        .expect("Operation enum end");
    source[start..end]
        .lines()
        .skip(1)
        .filter_map(|line| {
            let line = line.trim();
            let name = line
                .split(['(', '{', ','])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            (!name.is_empty() && name.as_bytes()[0].is_ascii_uppercase()).then(|| name.to_owned())
        })
        .collect()
}

fn validate_operations(
    rows: &[Value],
    source_operations: &BTreeSet<String>,
    operation_witnesses: &BTreeMap<String, &Value>,
    cases: &Cases<'_>,
    review_case_ids: &BTreeSet<String>,
) {
    let listed = rows
        .iter()
        .map(|row| row["name"].as_str().expect("operation name").to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(&listed, source_operations);
    assert_eq!(rows.len(), source_operations.len());
    assert_eq!(
        operation_witnesses.len(),
        rows.iter()
            .filter(|row| row["admission"].as_str() != Some("kernel-only"))
            .count()
    );
    for row in rows {
        let name = row["name"].as_str().expect("operation name");
        let admission = row["admission"].as_str().expect("admission");
        assert!(
            ["managed", "limited", "kernel-only"].contains(&admission),
            "{name} has unknown admission {admission}"
        );
        assert_ne!(row["family"].as_str().unwrap_or_default(), "");
        assert_ne!(row["rationale"].as_str().unwrap_or_default(), "");
        let Some(evidence) = row["evidence_case"].as_str() else {
            assert_eq!(
                admission, "kernel-only",
                "{name} lacks a composition rationale"
            );
            continue;
        };
        assert_ne!(admission, "kernel-only", "{name} has a kernel witness");
        let witness = operation_witnesses
            .get(name)
            .unwrap_or_else(|| panic!("{name}: missing operation witness"));
        assert_eq!(Some(evidence), witness["case_id"].as_str());
        let fragment = witness["query_fragment"]
            .as_str()
            .expect("operation fragment");
        assert!(!fragment.is_empty(), "{name}: empty operation fragment");
        let case = cases
            .get(evidence)
            .unwrap_or_else(|| panic!("{name}: missing {evidence}"));
        assert_eq!(case.status, CaseStatus::Mvp);
        assert!(case.adapters.jq.supported && case.adapters.tq.supported);
        assert!(
            review_case_ids.contains(evidence),
            "{evidence} is not source-linked"
        );
        assert!(
            case.query.contains(fragment),
            "{name}: operation witness omits {fragment:?}"
        );
        assert_ne!(witness["source"].as_str().unwrap_or_default(), "");
        assert!(witness["source_line"].as_u64().is_some());
    }
}

fn validate_contexts(
    contexts: &[Value],
    cases: &Cases<'_>,
    review_case_ids: &BTreeSet<String>,
) -> BTreeSet<String> {
    let required = [
        "def-body",
        "around-user-call",
        "filter-parameter",
        "value-parameter",
    ];
    assert_eq!(contexts.len(), 48);
    let mut families = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for context in contexts {
        let family = context["family"].as_str().expect("context family");
        let mode = context["context"].as_str().expect("context kind");
        families.insert(family.to_owned());
        assert!(
            required.contains(&mode),
            "{family} has unknown context {mode}"
        );
        assert!(
            seen.insert((family.to_owned(), mode.to_owned())),
            "duplicate {family}/{mode}"
        );
        let case_id = context["case_id"].as_str().expect("context case ID");
        let case = cases
            .get(case_id)
            .unwrap_or_else(|| panic!("{family}/{mode}: missing case {case_id}"));
        assert_eq!(case.status, CaseStatus::Mvp);
        assert!(case.adapters.jq.supported && case.adapters.tq.supported);
        assert!(
            review_case_ids.contains(case_id),
            "{case_id} is not source-linked"
        );
        assert_ne!(context["operation"].as_str().unwrap_or_default(), "");
        assert_ne!(context["source"].as_str().unwrap_or_default(), "");
        let fragment = context["operation_fragment"]
            .as_str()
            .expect("context operation fragment");
        assert!(
            case.query.contains(fragment),
            "{family}/{mode}: operation witness omits {fragment:?}"
        );
        match mode {
            "def-body" => assert!(case.query.contains("def ")),
            "around-user-call" => assert!(
                case.query.contains("f | "),
                "{case_id} lacks a real caller suffix"
            ),
            "filter-parameter" => {
                assert!(case.query.contains("def apply(g):"));
                assert!(case.query.contains("apply("));
            }
            "value-parameter" => {
                assert!(case.query.contains("def f($x):"));
                assert!(case.query.contains("f("));
            }
            _ => unreachable!(),
        }
    }
    assert_eq!(families.len(), 12);
    for family in &families {
        for mode in required {
            assert!(seen.contains(&(family.clone(), mode.to_owned())));
        }
    }
    families
}

fn assert_adapter_setup_preserved(generated: &CaseAdapter, original: &CaseAdapter, label: &str) {
    assert_eq!(
        generated.query, original.query,
        "{label}: adapter query changed"
    );
    assert_eq!(
        generated.args, original.args,
        "{label}: adapter args changed"
    );
    assert_eq!(
        generated.trailing_args, original.trailing_args,
        "{label}: trailing args changed"
    );
    assert_eq!(
        generated.omit_query, original.omit_query,
        "{label}: omit-query changed"
    );
    assert_eq!(generated.env, original.env, "{label}: environment changed");
    assert_eq!(
        generated.env_paths, original.env_paths,
        "{label}: environment paths changed"
    );
    assert_eq!(
        generated.supported, original.supported,
        "{label}: support changed"
    );
    assert_eq!(
        generated.note, original.note,
        "{label}: adapter note changed"
    );
}

fn validate_arity_witnesses(
    ledger: &Value,
    cases: &Cases<'_>,
    review_case_ids: &BTreeSet<String>,
) -> BTreeMap<String, String> {
    let rows = ledger["arity_witnesses"]
        .as_array()
        .expect("per-arity witnesses");
    assert_eq!(rows.len(), 220);
    let mut by_signature = BTreeMap::new();
    let mut case_ids = BTreeSet::new();
    for row in rows {
        let signature = row["signature"].as_str().expect("witness signature");
        let case_id = row["case_id"].as_str().expect("witness case ID");
        assert!(
            by_signature
                .insert(signature.to_owned(), case_id.to_owned())
                .is_none(),
            "duplicate per-arity witness {signature}"
        );
        assert!(
            case_ids.insert(case_id.to_owned()),
            "duplicate witness case {case_id}"
        );
        let case = cases
            .get(case_id)
            .unwrap_or_else(|| panic!("{signature}: missing witness case {case_id}"));
        assert_eq!(case.status, CaseStatus::Mvp);
        assert!(case.adapters.jq.supported && case.adapters.tq.supported);
        assert!(
            review_case_ids.contains(case_id),
            "{case_id} is not source-linked"
        );
        assert!(case.query.contains("def __manual_composition_witness:"));
        let original_ref = row["original_evidence_ref"]
            .as_str()
            .expect("original evidence reference");
        let witness = row["witness"].as_str().expect("original witness");
        assert_ne!(original_ref, "");
        assert_ne!(witness, "");
        if let Some(original) = cases.get(original_ref) {
            assert_eq!(
                case.fixture.format, original.fixture.format,
                "{signature}: fixture format changed"
            );
            assert_eq!(
                case.fixture.inline, original.fixture.inline,
                "{signature}: fixture changed"
            );
            assert_eq!(
                case.fixture.path, original.fixture.path,
                "{signature}: fixture path changed"
            );
            assert_eq!(
                case.invocation_mode, original.invocation_mode,
                "{signature}: input mode changed"
            );
            assert_adapter_setup_preserved(&case.adapters.jq, &original.adapters.jq, signature);
            assert_adapter_setup_preserved(&case.adapters.tq, &original.adapters.tq, signature);
            let original_query = original.query.trim();
            let query_body = original_query
                .split_once(';')
                .filter(|(prefix, _)| prefix.trim_start().starts_with("import "))
                .map_or(original_query, |(_, body)| body.trim());
            assert!(
                case.query.contains(query_body),
                "{signature}: original query body was replaced"
            );
        } else {
            assert!(
                case.query.contains(witness),
                "{signature}: focused witness query was replaced"
            );
        }
        assert_ne!(row["reason"].as_str().unwrap_or_default(), "");
    }
    by_signature
}

fn validate_arity_groups(
    groups: &[Value],
    family_ids: &BTreeSet<String>,
    cases: &Cases<'_>,
    witness_by_signature: &BTreeMap<String, String>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    assert_ne!(groups, [] as [Value; 0]);
    let mut names = BTreeSet::new();
    let mut signatures = BTreeSet::new();
    for group in groups {
        let family = group["family"].as_str().expect("arity family");
        assert!(family_ids.contains(family), "unknown arity family {family}");
        assert!(
            ["managed-allowlist", "composition-deferred"]
                .contains(&group["admission"].as_str().expect("arity admission"))
        );
        let case_id = group["case_id"].as_str().expect("arity case ID");
        assert!(
            cases.contains_key(case_id),
            "arity group case missing: {case_id}"
        );
        for name in group["names"].as_array().expect("arity names") {
            assert!(names.insert(name.as_str().expect("arity function name").to_owned()));
        }
        assert_ne!(group["rationale"].as_str().unwrap_or_default(), "");
        for signature in group["signatures"].as_array().expect("arity signatures") {
            let text = signature["signature"].as_str().expect("signature");
            assert!(
                signatures.insert(text.to_owned()),
                "duplicate arity signature {text}"
            );
            assert_eq!(
                signature["case_id"].as_str(),
                witness_by_signature.get(text).map(String::as_str),
                "{text} has no normalized witness mapping"
            );
            assert!(
                ["managed-allowlist", "composition-deferred"].contains(
                    &signature["admission"]
                        .as_str()
                        .expect("signature admission")
                )
            );
            assert_ne!(signature["rationale"].as_str().unwrap_or_default(), "");
        }
    }
    (names, signatures)
}

fn validate_documented_arities(
    repo_root: &Path,
    grouped_names: &BTreeSet<String>,
    grouped_signatures: &BTreeSet<String>,
) {
    let arities: Value = tq_test_support::fixture_data::read(
        &repo_root.join("tests/compatibility/reviews/jq-manual/completeness.toon"),
    )
    .expect("documented arity inventory");
    let requirements = arities["requirements"]
        .as_array()
        .expect("documented signatures");
    assert_eq!(requirements.len(), 220);
    let mut signatures = BTreeSet::new();
    for requirement in requirements {
        let signature = requirement["signature"]
            .as_str()
            .expect("documented signature");
        assert!(
            signatures.insert(signature.to_owned()),
            "duplicate {signature}"
        );
        assert_eq!(requirement["status"], "semantic-witness", "{signature}");
        assert_ne!(requirement["evidence_ref"].as_str().unwrap_or_default(), "");
        assert_ne!(requirement["witness"].as_str().unwrap_or_default(), "");
        let name = signature.split('/').next().unwrap();
        assert!(
            grouped_names.contains(name),
            "{signature} has no family classification"
        );
        assert!(
            grouped_signatures.contains(signature),
            "{signature} has no per-arity classification"
        );
    }
    let required_names = requirements
        .iter()
        .map(|row| {
            row["signature"]
                .as_str()
                .unwrap()
                .split('/')
                .next()
                .unwrap()
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(grouped_names, &required_names);
    assert_eq!(grouped_signatures, &signatures);
}

fn documented_signatures(repo_root: &Path) -> Vec<(String, usize)> {
    let inventory: Value = tq_test_support::fixture_data::read(
        &repo_root.join("tests/compatibility/reviews/jq-manual/completeness.toon"),
    )
    .expect("documented arity inventory");
    let requirements = inventory["requirements"]
        .as_array()
        .expect("documented signatures");
    let mut signatures = BTreeSet::new();
    let mut parsed = Vec::with_capacity(requirements.len());
    for requirement in requirements {
        let signature = requirement["signature"]
            .as_str()
            .expect("documented signature");
        let (name, arity) = signature
            .rsplit_once('/')
            .unwrap_or_else(|| panic!("malformed documented signature {signature}"));
        let arity = arity
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("malformed documented arity {signature}"));
        assert!(
            signatures.insert(signature),
            "duplicate documented signature {signature}"
        );
        parsed.push((name.to_owned(), arity));
    }
    assert_eq!(parsed.len(), 220);
    parsed
}

fn admission_query(name: &str, arity: usize) -> String {
    if arity == 0 {
        format!("def __admission: {name}; __admission")
    } else {
        let arguments = std::iter::repeat_n(".", arity)
            .collect::<Vec<_>>()
            .join("; ");
        format!("def __admission: {name}({arguments}); __admission")
    }
}

fn evaluate_document_filter(query: &str) -> Vec<RuntimeValue> {
    let parsed = parse(query).expect("builtins availability query parses");
    let resolved =
        resolve(parsed, &ResolveOptions::default()).expect("builtins availability query resolves");
    let analyzed = analyze(resolved);
    let plan = analyzed
        .compile()
        .expect("builtins availability query compiles")
        .document_plan();
    let mut vm = Vm::new(&plan, RuntimeValue::Null, VmLimits::default());
    let mut values = Vec::new();
    loop {
        let result = vm
            .next_result()
            .expect("builtins availability query evaluates");
        let Some(value) = result else { break };
        values.push(value);
    }
    values
}

#[test]
fn builtins_composition_witness_checks_documented_availability() {
    let repo_root = root();
    let catalog = load_catalog(&repo_root.join("tests/compatibility/cases")).expect("catalog");
    let case = catalog
        .cases
        .iter()
        .find(|case| case.id == "manual.composition.arity.builtins.0")
        .expect("builtins arity witness");
    let query = case.query.trim();
    assert!(query.contains("def __manual_composition_witness:"));
    assert!(query.contains("builtins"));
    assert!(
        query.ends_with("__manual_composition_witness"),
        "availability witness must invoke its called definition"
    );

    let inventory: Value = tq_test_support::fixture_data::read(
        &repo_root.join("tests/compatibility/reviews/jq-manual/arity-inventory.toon"),
    )
    .expect("documented arity inventory");
    let required = inventory["requirements"]
        .as_array()
        .expect("documented signatures")
        .iter()
        .map(|row| row["signature"].as_str().expect("signature"))
        .collect::<Vec<_>>();
    assert_eq!(required.len(), 220);
    for signature in &required {
        assert!(
            query.contains(&format!("\"{signature}\"")),
            "availability witness omits documented signature {signature}"
        );
    }
    assert!(!query.contains("get_prog_origin/0"));
    assert!(!query.contains("get_jq_origin/0"));
    assert!(!query.contains("get_search_list/0"));

    let witness_values = evaluate_document_filter(query);
    assert_eq!(
        witness_values.len(),
        1,
        "availability witness must emit exactly one result"
    );
    assert_eq!(witness_values, [RuntimeValue::array(Vec::new())]);

    let builtin_values = evaluate_document_filter("builtins");
    assert_eq!(
        builtin_values.len(),
        1,
        "builtins must emit exactly one inventory result"
    );
    let RuntimeValue::Array(entries) = builtin_values.first().expect("builtins result") else {
        panic!("builtins must return one array result");
    };
    let mut seen = BTreeSet::new();
    for entry in entries.iter() {
        let RuntimeValue::String(signature) = entry else {
            panic!("builtins entry is not a string: {entry:?}");
        };
        let Some((name, arity)) = signature.rsplit_once('/') else {
            panic!("builtins entry lacks name/arity shape: {signature}");
        };
        assert!(!name.is_empty(), "builtins entry has an empty name");
        assert!(
            arity.parse::<usize>().is_ok(),
            "builtins entry has a non-numeric arity: {signature}"
        );
        assert!(
            seen.insert(signature.to_string()),
            "duplicate builtins entry {signature}"
        );
    }
    for signature in required {
        assert!(
            seen.contains(signature),
            "documented signature missing from builtins: {signature}"
        );
    }
}

#[test]
fn composition_inventory_admits_documented_signatures_without_execution() {
    let signatures = documented_signatures(root());
    let mut failures = BTreeMap::<&str, Vec<String>>::new();
    for (name, arity) in signatures {
        let signature = format!("{name}/{arity}");
        let query = admission_query(&name, arity);
        let parsed = match parse(&query) {
            Ok(parsed) => parsed,
            Err(error) => {
                failures
                    .entry("parse")
                    .or_default()
                    .push(format!("{signature}: {error}"));
                continue;
            }
        };
        let resolved = match resolve(parsed, &ResolveOptions::default()) {
            Ok(resolved) => resolved,
            Err(error) => {
                failures
                    .entry("resolve")
                    .or_default()
                    .push(format!("{signature}: {error}"));
                continue;
            }
        };
        let analyzed = analyze(resolved);
        if let Err(error) = analyzed.compile() {
            failures
                .entry("compile")
                .or_default()
                .push(format!("{signature}: {error}"));
        }
    }

    assert!(
        failures.is_empty(),
        "documented signature admission failures (no VM execution performed):\n{}",
        failures
            .into_iter()
            .map(|(stage, signatures)| format!("{stage}:\n{}", signatures.join("\n")))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn composition_inventory_covers_operations_arities_and_contexts() {
    let repo_root = root();
    let ledger: Value = tq_test_support::fixture_data::read(
        &repo_root.join("tests/compatibility/reviews/jq-manual/composition-inventory.toon"),
    )
    .expect("composition inventory");
    assert_eq!(ledger["schema_version"], 1);
    assert_ne!(ledger["witness_policy"].as_str().unwrap_or_default(), "");

    let catalog = load_catalog(&repo_root.join("tests/compatibility/cases")).expect("catalog");
    let cases = catalog
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<BTreeMap<_, _>>();
    let review_case_ids =
        read_manual_review_case_ids(&repo_root.join("tests/compatibility/reviews/jq-manual"))
            .expect("manual review mappings");

    let source = std::fs::read_to_string(repo_root.join("crates/tq-core/src/bytecode.rs"))
        .expect("Operation source");
    let source_operations = source_operation_variants(&source)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let operation_witnesses = ledger["operation_witnesses"]
        .as_array()
        .expect("operation witnesses")
        .iter()
        .map(|row| {
            (
                row["name"].as_str().expect("operation name").to_owned(),
                row,
            )
        })
        .collect::<BTreeMap<_, _>>();
    validate_operations(
        ledger["operation_variants"]
            .as_array()
            .expect("operation inventory rows"),
        &source_operations,
        &operation_witnesses,
        &cases,
        &review_case_ids,
    );
    let family_ids = validate_contexts(
        ledger["contexts"].as_array().expect("composition contexts"),
        &cases,
        &review_case_ids,
    );
    let witness_by_signature = validate_arity_witnesses(&ledger, &cases, &review_case_ids);
    let (grouped_names, grouped_signatures) = validate_arity_groups(
        ledger["arity_groups"].as_array().expect("arity groups"),
        &family_ids,
        &cases,
        &witness_by_signature,
    );
    validate_documented_arities(repo_root, &grouped_names, &grouped_signatures);
}
