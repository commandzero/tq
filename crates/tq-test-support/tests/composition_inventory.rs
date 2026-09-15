//! Source-linked inventory for bounded user-filter composition admission.

use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    io::Cursor,
    path::Path,
    sync::{Arc, Mutex},
};
use tq_core::{
    Compiled, Document, InputCursor, InputValue, JsonInput, JsonInputOptions, ResolveOptions,
    Value as RuntimeValue, Vm, VmError, VmLimits, analyze, parse, resolve,
};
use tq_test_support::compatibility::{
    CaseAdapter, CaseStatus, CompatibilityCase, ContractKind, FixtureFormat, InvocationMode,
    load_catalog, read_manual_review_case_ids,
};

type Cases<'a> = BTreeMap<&'a str, &'a CompatibilityCase>;
type DocumentPlan = tq_core::Plan<Compiled, Document>;

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
            ["managed", "kernel-only"].contains(&admission),
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
        assert_eq!(group["admission"].as_str(), Some("managed"));
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
            assert_eq!(signature["admission"].as_str(), Some("managed"));
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

#[derive(Debug, Eq, PartialEq)]
struct EmbeddedObservation {
    results: Vec<RuntimeValue>,
    error: Option<VmError>,
    effects: Vec<u8>,
}

fn composition_fixture_bytes(case: &CompatibilityCase, repo_root: &Path) -> Vec<u8> {
    if let Some(inline) = &case.fixture.inline {
        inline.as_bytes().to_vec()
    } else if let Some(path) = &case.fixture.path {
        std::fs::read(repo_root.join(path))
            .unwrap_or_else(|error| panic!("{} fixture {} cannot be read: {error}", case.id, path))
    } else {
        Vec::new()
    }
}

fn composition_fixture_inputs(case: &CompatibilityCase, repo_root: &Path) -> Vec<InputValue> {
    assert!(
        matches!(
            case.fixture.format,
            FixtureFormat::Json | FixtureFormat::Raw
        ),
        "{} has an embedded-incompatible fixture format {:?}",
        case.id,
        case.fixture.format
    );
    let bytes = composition_fixture_bytes(case, repo_root);
    let mut reader = JsonInput::new(Cursor::new(bytes.clone()), JsonInputOptions::default());
    let mut inputs = Vec::new();
    loop {
        let position = reader.position();
        let mut line_number = position.line;
        let mut offset = position.offset;
        while let Some(byte) = bytes.get(offset) {
            if !byte.is_ascii_whitespace() {
                break;
            }
            if *byte == b'\n' {
                line_number = line_number.saturating_add(1);
            }
            offset = offset.saturating_add(1);
        }
        let value = reader
            .next_value(&mut || Ok(()))
            .unwrap_or_else(|error| panic!("{} fixture JSON failed: {error}", case.id));
        let Some(value) = value else { break };
        inputs.push(InputValue {
            value,
            identity: Arc::from("<stdin>"),
            line_number: u64::try_from(line_number).expect("JSON line number fits in u64"),
        });
    }
    inputs
}

fn composition_variables(
    case: &CompatibilityCase,
    input: &InputValue,
) -> BTreeMap<Arc<str>, RuntimeValue> {
    let mut variables = BTreeMap::new();
    match case.id.as_str() {
        "manual.composition.arity.env.0" => {
            variables.insert(
                Arc::from("__tq_ambient_environment"),
                RuntimeValue::from_json(serde_json::json!({"PAGER": "less"}))
                    .expect("environment fixture converts"),
            );
        }
        "manual.composition.arity.input-filename.0" => {
            variables.insert(Arc::from("__tq_ambient_platform"), RuntimeValue::Bool(true));
            variables.insert(
                Arc::from("__tq_input_filename"),
                RuntimeValue::string("<stdin>"),
            );
        }
        "manual.composition.arity.localtime.0"
        | "manual.composition.arity.now.0"
        | "manual.composition.arity.strflocaltime.1" => {
            variables.insert(Arc::from("__tq_ambient_platform"), RuntimeValue::Bool(true));
        }
        "manual.composition.arity.input-line-number.0" => {
            variables.insert(
                Arc::from("__tq_input_line_number"),
                RuntimeValue::from_json(serde_json::json!(input.line_number))
                    .expect("line number converts"),
            );
        }
        _ => {}
    }
    variables
}

fn composition_plan(case: &CompatibilityCase, query: &str, repo_root: &Path) -> DocumentPlan {
    let parsed = parse(query).unwrap_or_else(|error| panic!("{} query parses: {error}", case.id));
    let mut options = ResolveOptions::default();
    if case.id == "manual.composition.arity.modulemeta.0" {
        options
            .module_roots
            .push(repo_root.join("tests/fixtures/manual-modules"));
    }
    let resolved = resolve(parsed, &options)
        .unwrap_or_else(|error| panic!("{} query resolves: {error}", case.id));
    analyze(resolved)
        .compile()
        .unwrap_or_else(|error| panic!("{} query compiles: {error}", case.id))
        .document_plan()
}

fn execute_composition_query(
    case: &CompatibilityCase,
    query: &str,
    repo_root: &Path,
) -> EmbeddedObservation {
    assert_eq!(case.invocation_mode, InvocationMode::Stdin, "{}", case.id);
    let plan = composition_plan(case, query, repo_root);
    let inputs = composition_fixture_inputs(case, repo_root);
    let null_input = case
        .adapters
        .tq
        .args
        .iter()
        .any(|argument| argument == "-n");
    let source = Arc::new(Mutex::new(VecDeque::from(inputs)));
    let cursor_source = Arc::clone(&source);
    let cursor = InputCursor::from_provider(move || {
        let mut source = cursor_source.lock().map_err(|_| VmError::Runtime {
            message: Arc::from("composition input source is unavailable"),
        })?;
        Ok(source.pop_front())
    });
    let mut pending_roots = if null_input {
        VecDeque::from([InputValue {
            value: RuntimeValue::Null,
            identity: Arc::from("<stdin>"),
            line_number: 1,
        }])
    } else {
        VecDeque::new()
    };
    if !null_input
        && let Some(first_root) = source
            .lock()
            .expect("composition input source is available")
            .pop_front()
    {
        pending_roots.push_back(first_root);
    }

    let mut results = Vec::new();
    let mut effects = Vec::new();
    while let Some(input) = pending_roots.pop_front() {
        let variables = composition_variables(case, &input);
        let mut vm = Vm::new_with_variables(&plan, input.value, VmLimits::default(), variables)
            .with_input_cursor(cursor.clone());
        let mut error = None;
        loop {
            match vm.next_result() {
                Ok(Some(value)) => results.push(value),
                Ok(None) => break,
                Err(runtime_error) => {
                    error = Some(runtime_error);
                    break;
                }
            }
        }
        effects.extend(vm.take_effects());
        if error.is_none()
            && !null_input
            && let Some(next_root) = source
                .lock()
                .expect("composition input source is available")
                .pop_front()
        {
            pending_roots.push_back(next_root);
        }
        if error.is_some() {
            return EmbeddedObservation {
                results,
                error,
                effects,
            };
        }
    }
    EmbeddedObservation {
        results,
        error: None,
        effects,
    }
}

fn direct_composition_query(query: &str) -> Option<String> {
    for name in ["__manual_operation_witness", "__manual_composition_witness"] {
        let prefix = format!("def {name}: ");
        let Some(start) = query.find(&prefix) else {
            continue;
        };
        let body = query[start + prefix.len()..].strip_suffix(&format!("; {name}"))?;
        return Some(format!("{}{}", &query[..start], body));
    }
    None
}

fn expected_inventory_results(case_id: &str) -> Vec<RuntimeValue> {
    let expected = match case_id {
        "manual.composition.inventory.constructors.def" => {
            serde_json::json!([{"head": 1, "whole": [1, 2]}])
        }
        "manual.composition.inventory.constructors.around" => serde_json::json!([{ "head": 1 }]),
        "manual.composition.inventory.constructors.filter" => {
            serde_json::json!([[{ "value": 1 }]])
        }
        "manual.composition.inventory.constructors.value" => {
            serde_json::json!([{ "value": [1, 2], "first": 1 }])
        }
        "manual.composition.inventory.navigation.def" => serde_json::json!(["a", "b"]),
        "manual.composition.inventory.navigation.around"
        | "manual.composition.inventory.folds.around"
        | "manual.composition.inventory.math.def"
        | "manual.composition.inventory.math.around"
        | "manual.composition.inventory.effects.def"
        | "manual.composition.inventory.effects.filter" => serde_json::json!([1]),
        "manual.composition.inventory.navigation.filter" => serde_json::json!([["a", "b"]]),
        "manual.composition.inventory.navigation.value" => serde_json::json!(["a"]),
        "manual.composition.inventory.scalar-control.def"
        | "manual.composition.inventory.scalar-control.around"
        | "manual.composition.inventory.math.value" => serde_json::json!([4]),
        "manual.composition.inventory.scalar-control.filter"
        | "manual.composition.inventory.scalar-control.value"
        | "manual.composition.inventory.bindings.around"
        | "manual.composition.inventory.paths.around"
        | "manual.composition.inventory.callbacks.around" => serde_json::json!([2]),
        "manual.composition.inventory.bindings.def" => serde_json::json!([[1, null]]),
        "manual.composition.inventory.bindings.filter" => serde_json::json!([[1]]),
        "manual.composition.inventory.bindings.value" => serde_json::json!([[1, 1]]),
        "manual.composition.inventory.folds.def" | "manual.composition.inventory.folds.filter" => {
            serde_json::json!([3])
        }
        "manual.composition.inventory.folds.value" => serde_json::json!([[1, 3]]),
        "manual.composition.inventory.paths.def" | "manual.composition.inventory.paths.filter" => {
            serde_json::json!([{ "a": 2 }])
        }
        "manual.composition.inventory.paths.value" => serde_json::json!([{ "a": 2, "b": 2 }]),
        "manual.composition.inventory.callbacks.def"
        | "manual.composition.inventory.callbacks.filter" => serde_json::json!([[2, 3]]),
        "manual.composition.inventory.callbacks.value" => serde_json::json!([[11, 12]]),
        "manual.composition.inventory.generators.def"
        | "manual.composition.inventory.generators.value" => serde_json::json!([[0, 1, 2]]),
        "manual.composition.inventory.generators.around" => serde_json::json!([10, 11, 12]),
        "manual.composition.inventory.generators.filter" => serde_json::json!([[0, 1]]),
        "manual.composition.inventory.math.filter" => serde_json::json!([[0, 0]]),
        "manual.composition.inventory.regex.def" | "manual.composition.inventory.regex.value" => {
            serde_json::json!([true])
        }
        "manual.composition.inventory.regex.around" => serde_json::json!([false]),
        "manual.composition.inventory.regex.filter" => serde_json::json!(["xbx"]),
        "manual.composition.inventory.effects.around" => serde_json::json!(["1"]),
        "manual.composition.inventory.effects.value"
        | "manual.composition.inventory.errors.def"
        | "manual.composition.inventory.errors.around"
        | "manual.composition.inventory.errors.filter"
        | "manual.composition.inventory.errors.value" => serde_json::json!(["x"]),
        _ => panic!("missing independent expected witness for {case_id}"),
    };
    expected
        .as_array()
        .expect("inventory expected sequence")
        .iter()
        .cloned()
        .map(|value| RuntimeValue::from_json(value).expect("inventory expected value converts"))
        .collect()
}

fn expected_composition_effects(case_id: &str) -> Option<&'static [u8]> {
    match case_id {
        "manual.composition.arity.debug.0" => Some(b"[\"DEBUG:\",42]\n"),
        "manual.composition.inventory.effects.def"
        | "manual.composition.inventory.effects.around"
        | "manual.composition.inventory.effects.filter" => Some(b"[\"DEBUG:\",1]\n"),
        "manual.composition.arity.debug.1" => Some(b"[\"DEBUG:\",\"message\"]\n"),
        "manual.composition.arity.stderr.0" => Some(b"hello"),
        "manual.composition.inventory.effects.value" => Some(b"[\"DEBUG:\",\"x\"]\n"),
        _ => None,
    }
}

fn expected_special_results(case_id: &str) -> Option<Vec<RuntimeValue>> {
    let expected = match case_id {
        "manual.composition.arity.env.0" => serde_json::json!(["less"]),
        "manual.composition.arity.input-filename.0" => serde_json::json!(["<stdin>"]),
        "manual.composition.arity.input-line-number.0" => serde_json::json!([1, 2]),
        "manual.composition.arity.input.0" => serde_json::json!([[1, 2], [3, 4]]),
        "manual.composition.arity.inputs.0" => serde_json::json!([6]),
        "manual.composition.arity.modulemeta.0" => serde_json::json!([{
            "homepage": "https://example.invalid/basic",
            "deps": [],
            "defs": ["value/0"]
        }]),
        _ => return None,
    };
    Some(
        expected
            .as_array()
            .expect("special expected sequence")
            .iter()
            .cloned()
            .map(|value| RuntimeValue::from_json(value).expect("special expected value converts"))
            .collect(),
    )
}

fn assert_composition_contract(case: &CompatibilityCase, observation: &EmbeddedObservation) {
    match case.expected.contract {
        ContractKind::ResultSequence => {
            if case.id == "manual.composition.arity.halt.0" {
                assert!(matches!(
                    &observation.error,
                    Some(VmError::Halt { status: 0, stderr }) if stderr.is_empty()
                ));
            } else {
                assert!(
                    observation.error.is_none(),
                    "{} unexpectedly failed: {:?}",
                    case.id,
                    observation.error
                );
            }
        }
        ContractKind::Error => {
            let Some(error) = &observation.error else {
                panic!("{} must produce its intentional runtime error", case.id)
            };
            assert!(!matches!(error, VmError::Halt { .. }), "{} halted", case.id);
            match case.id.as_str() {
                "manual.composition.arity.error.1" => assert!(matches!(
                    error,
                    VmError::Raised {
                        value: RuntimeValue::String(value),
                        ..
                    } if value.as_ref() == "boom"
                )),
                "manual.composition.arity.upper-join.2" => assert!(matches!(
                    error,
                    VmError::Runtime { message }
                        if message.as_ref() == "field access cannot be applied to string"
                )),
                _ => panic!("{} has no typed embedded error contract", case.id),
            }
        }
        ContractKind::ExitStatus => {
            let Some(VmError::Halt { status, stderr }) = &observation.error else {
                panic!("{} must produce a typed halt", case.id)
            };
            let expected_status = if case.id.ends_with("halt-error.0") {
                5
            } else {
                7
            };
            assert_eq!(*status, expected_status, "{} halt status", case.id);
            let expected_stderr = if case.id.ends_with("halt-error.0") {
                b"".as_slice()
            } else {
                b"failure".as_slice()
            };
            assert_eq!(stderr.as_ref(), expected_stderr, "{} halt stderr", case.id);
        }
        ContractKind::RawBytes => panic!("{} is not an embedded value case", case.id),
    }
}

fn assert_same_composition_observation(
    case: &CompatibilityCase,
    composed: &EmbeddedObservation,
    direct: &EmbeddedObservation,
) {
    assert_eq!(
        composed.results, direct.results,
        "{} result sequence",
        case.id
    );
    assert_eq!(composed.error, direct.error, "{} runtime outcome", case.id);
    assert_eq!(composed.effects, direct.effects, "{} effect bytes", case.id);
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
fn composition_inventory_executes_all_catalog_witnesses_through_embedded_vm() {
    let repo_root = root();
    let catalog = load_catalog(&repo_root.join("tests/compatibility/cases")).expect("catalog");
    let cases = catalog
        .cases
        .iter()
        .filter(|case| case.id.starts_with("manual.composition."))
        .collect::<Vec<_>>();
    assert_eq!(cases.len(), 297, "composition catalog denominator");

    let mut direct_comparisons = 0;
    let mut inventory_expectations = 0;
    for case in cases {
        assert_eq!(case.status, CaseStatus::Mvp, "{} status", case.id);
        assert!(case.adapters.tq.supported, "{} tq adapter", case.id);
        assert_eq!(
            case.invocation_mode,
            InvocationMode::Stdin,
            "{} mode",
            case.id
        );

        let composed = execute_composition_query(case, &case.query, repo_root);
        assert_composition_contract(case, &composed);
        if let Some(expected) = expected_special_results(&case.id) {
            assert_eq!(
                composed.results, expected,
                "{} independent input/module witness",
                case.id
            );
        }
        if let Some(expected_effects) = expected_composition_effects(&case.id) {
            assert_eq!(
                composed.effects, expected_effects,
                "{} effect bytes",
                case.id
            );
        } else {
            assert!(
                composed.effects.is_empty(),
                "{} unexpected effects",
                case.id
            );
        }

        if let Some(direct_query) = direct_composition_query(&case.query) {
            let direct = execute_composition_query(case, &direct_query, repo_root);
            assert_same_composition_observation(case, &composed, &direct);
            direct_comparisons += 1;
        } else if case.id.starts_with("manual.composition.inventory.") {
            assert_eq!(
                composed.error, None,
                "{} inventory witness unexpectedly failed",
                case.id
            );
            assert_eq!(
                composed.results,
                expected_inventory_results(&case.id),
                "{} independent inventory witness",
                case.id
            );
            inventory_expectations += 1;
        } else {
            panic!("{} has no independent execution witness", case.id);
        }
    }
    assert_eq!(
        direct_comparisons, 249,
        "operation and arity direct comparisons"
    );
    assert_eq!(inventory_expectations, 48, "inventory expected witnesses");
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
