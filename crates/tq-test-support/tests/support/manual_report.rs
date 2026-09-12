//! Small, explicit manual-report fixtures for integrity and renderer tests.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};
use tq_test_support::compatibility::{
    CaseStatus, CompatibilityCatalog, ContractKind, FixtureFormat, ToolKind, TqContract,
    case_fingerprint, encode_hex, load_catalog, read_gap_inventory, read_manual_review_case_ids,
    summarize_manual_comparison,
};

/// Repository root used by integration-test fixtures.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Builds a passing report from the executable catalog and source-review IDs.
///
/// The observations are deliberately tiny and synthetic. These rows exercise
/// report shape and gate policy; they are not campaign evidence.
pub fn passing_report() -> Value {
    let root = root();
    let catalog = load_catalog(&root.join("tests/compatibility/cases")).expect("case catalog");
    let review_ids =
        read_manual_review_case_ids(&root.join("tests/compatibility/reviews/jq-manual"))
            .expect("review case IDs");
    passing_report_for(&catalog, &review_ids)
}

/// Builds the historical baseline from the pin's original IDs and gap labels.
pub fn historical_baseline_report() -> Value {
    let root = root();
    let catalog = load_catalog(&root.join("tests/compatibility/cases")).expect("case catalog");
    let review_ids =
        read_manual_review_case_ids(&root.join("tests/compatibility/reviews/jq-manual"))
            .expect("review case IDs");
    let pin: tq_test_support::compatibility::ManualReferencePin =
        tq_test_support::fixture_data::read(
            &root.join("tests/compatibility/reviews/jq-manual/reference-pin.toon"),
        )
        .expect("reference pin");
    let inventory =
        read_gap_inventory(&root.join("tests/compatibility/reviews/jq-manual/gap-inventory.toon"))
            .expect("gap inventory");
    let mut report = passing_report_for(&catalog, &review_ids);
    let baseline_ids = pin
        .baseline_cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    report["cases"] = report["cases"]
        .as_array()
        .expect("synthetic report cases")
        .iter()
        .filter(|case| {
            case["id"]
                .as_str()
                .is_some_and(|id| baseline_ids.contains(id))
        })
        .map(|case| {
            let mut case = case.clone();
            if let Some(entry) = inventory
                .entries
                .iter()
                .find(|entry| Some(entry.case_id.as_str()) == case["id"].as_str())
            {
                case["verdict"] = serde_json::to_value(entry.baseline_verdict)
                    .expect("baseline verdict encoding");
            }
            case
        })
        .collect();
    report
}

fn passing_report_for(catalog: &CompatibilityCatalog, review_ids: &BTreeSet<String>) -> Value {
    let mut ids = catalog
        .cases
        .iter()
        .filter(|case| case.id.starts_with("manual.") || case.id.starts_with("manual-"))
        .filter(|case| executable(case))
        .map(|case| case.id.clone())
        .collect::<BTreeSet<_>>();
    ids.extend(
        review_ids
            .iter()
            .filter(|id| {
                catalog
                    .cases
                    .iter()
                    .find(|case| case.id == **id)
                    .is_some_and(executable)
            })
            .cloned(),
    );
    let mut report = report_for_catalog_ids(catalog, &ids);
    summarize_manual_comparison(&mut report).expect("synthetic report summary");
    report
}

fn report_for_catalog_ids(catalog: &CompatibilityCatalog, ids: &BTreeSet<String>) -> Value {
    let cases = ids
        .iter()
        .map(|id| {
            let case = catalog
                .cases
                .iter()
                .find(|case| case.id == *id)
                .unwrap_or_else(|| panic!("unknown synthetic catalog case: {id}"));
            synthetic_case(case)
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": 1,
        "corpus": catalog.identity,
        "tools": [synthetic_identity(ToolKind::Jq), synthetic_identity(ToolKind::Tq)],
        "method": "Synthetic report fixture: compact observations exercise manual gate and renderer contracts without campaign evidence.",
        "cases": cases,
    })
}

fn executable(case: &tq_test_support::compatibility::CompatibilityCase) -> bool {
    case.status == CaseStatus::Mvp && case.adapters.jq.supported && case.adapters.tq.supported
}

fn synthetic_case(case: &tq_test_support::compatibility::CompatibilityCase) -> Value {
    let contract = case.expected.contract;
    let input_format = (case.fixture.format != FixtureFormat::None).then_some(case.fixture.format);
    let identity_output = case
        .expected
        .tq_contract
        .map_or("synthetic\n", identity_fixture);
    let (results, stdout) = match contract {
        ContractKind::ResultSequence => (vec![json!(null)], "null\n"),
        ContractKind::Error => (Vec::new(), "synthetic error\n"),
        ContractKind::RawBytes | ContractKind::ExitStatus => (Vec::new(), identity_output),
    };
    let jq_stdout = if case.expected.tq_contract.is_some() {
        "synthetic jq identity\n"
    } else {
        stdout
    };
    let jq = observation(ToolKind::Jq, input_format, &results, jq_stdout, contract);
    let tq = observation(ToolKind::Tq, input_format, &results, stdout, contract);
    // Keep the independent observations present even for raw rows: a focused
    // disparity test may intentionally reclassify one row as result-sequence
    // to exercise approval evidence without borrowing a campaign dump.
    let result_contract = matches!(contract, ContractKind::ResultSequence | ContractKind::Error);
    json!({
        "id": case.id,
        "input": case.query,
        "case_fingerprint": case_fingerprint(case).expect("case fingerprint"),
        "contract": contract,
        "verdict": "match",
        "reason": "Synthetic report fixture; no campaign evidence is claimed.",
        "json_equivalent": contract == ContractKind::ResultSequence,
        "toon_sequence": false,
        "toon_equivalent": result_contract,
        "tokens": Value::Null,
        "toon_contract_match": true,
        "compact": Some(json!({
            "exact": true,
            "jq": observation(ToolKind::Jq, input_format, &results, stdout, contract),
            "tq": observation(ToolKind::Tq, input_format, &results, stdout, contract),
            "differences": [],
        })),
        "differences": [],
        "jq": jq,
        "tq": tq,
        "tq_toon": Some(observation(
            ToolKind::Tq,
            input_format,
            &results,
            stdout,
            contract,
        )),
    })
}

fn observation(
    tool: ToolKind,
    input_format: Option<FixtureFormat>,
    results: &[Value],
    stdout: &str,
    contract: ContractKind,
) -> Value {
    let raw = matches!(contract, ContractKind::RawBytes | ContractKind::ExitStatus);
    let error = contract == ContractKind::Error;
    json!({
        "tool": tool,
        "input_format": input_format,
        "state": "executed",
        "results": results,
        "stdout_hex": encode_hex(stdout.as_bytes()),
        "raw_stdout_hex": raw.then(|| encode_hex(stdout.as_bytes())),
        "stderr_hex": Value::Null,
        "process_status": "exited",
        "exit_code": i32::from(error),
        "error_class": Value::Null,
    })
}

fn synthetic_identity(tool: ToolKind) -> Value {
    let (path, version) = match tool {
        ToolKind::Jq => ("synthetic/jq", "jq-1.8.1"),
        ToolKind::Tq => ("synthetic/tq", "tq 0.1.0 (synthetic)"),
        ToolKind::Yq => ("synthetic/yq", "yq (synthetic)"),
    };
    json!({
        "tool": tool,
        "path": path,
        "version": version,
        "executable": {"path": path, "bytes": 1, "sha256": "00".repeat(32)},
        "build_features": [],
        "runtime_libraries": [],
    })
}

fn identity_fixture(contract: TqContract) -> &'static str {
    match contract {
        TqContract::Version => "tq 0.1.0 (TOON v3; jq target 1.8.x; synthetic)\n",
        TqContract::BuildConfiguration => "target=synthetic\nformats=toon\njq-target=1.8.x\n",
        TqContract::Help => concat!(
            "tq - jq-compatible queries over TOON\nUsage: tq\n",
            "-i, --input-format FORMAT\n-o, --output-format FORMAT\n",
            "-n, --null-input\n-R, --raw-input\n-s, --slurp\n",
            "-c, --compact-output\n-r, --raw-output\n--raw-output0\n",
            "-j, --join-output\n-a, --ascii-output\n-S, --sort-keys\n",
            "-C, --color-output\n-M, --monochrome-output\n--tab\n",
            "--indent N\n--unbuffered\n--allow-environment\n--allow-platform\n",
            "--stream\n--stream-errors\n-x, --proxy-on-error\n--seq\n",
            "-f, --from-file FILE\n-L, --library-path DIR\n",
            "--arg NAME VALUE\n--argjson NAME JSON\n--argtoon NAME TOON\n",
            "--slurpfile NAME FILE\n--rawfile NAME FILE\n--args\n--jsonargs\n",
            "-e, --exit-status\n-b, --binary\n-V, --version\n",
            "--build-configuration\n--run-tests [FILE]\n-h, --help\n",
            "Formats: -i, --input-format auto|toon|yaml|json|json5|jsonl|toon-seq|json-seq\n",
            "-o, --output-format toon|yaml|json|jsonl\nselect TOON\nemit compact JSON\n",
        ),
    }
}
