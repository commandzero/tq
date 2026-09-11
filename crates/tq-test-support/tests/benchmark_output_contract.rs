#![allow(missing_docs)]
#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

use serde_json::json;
use tq_test_support::{
    benchmark::{
        BenchmarkAdapter, BenchmarkCase, BenchmarkCorpusIdentity, BenchmarkInvocation,
        BenchmarkLimits, BenchmarkOutcome, BenchmarkSampling, BenchmarkTool, ComparisonFamily,
        CorrectnessObservation, CorrectnessPayload, DatasetFamily, DatasetSelector, DatasetTier,
        ExecutionClass, InputFormat, OutputContract, OutputContractKind, run_gated_row,
        semantic_digest,
    },
    compatibility::{ErrorClass, ProcessStatus},
    corpus::ArtifactIdentity,
};

#[test]
fn default_toon_values_match_in_order_and_consume_all_output() {
    let cases = [
        ("valid", "printf '1\\n2\\n'", BenchmarkOutcome::Timed),
        (
            "extra",
            "printf '1\\n2\\n3\\n'",
            BenchmarkOutcome::Incorrect,
        ),
        ("missing", "printf '1\\n'", BenchmarkOutcome::Incorrect),
        (
            "reordered",
            "printf '2\\n1\\n'",
            BenchmarkOutcome::Incorrect,
        ),
    ];
    for (name, body, expected_outcome) in cases {
        let directory = tempfile::tempdir().expect("temporary directory");
        let executable = script(directory.path(), name, body);
        let row = run_gated_row(
            &case(),
            &tq_adapter(Vec::<String>::new()),
            &corpus(),
            DatasetTier::Startup,
            &invocation(executable, Vec::new()),
            &reference_values(&[json!(1), json!(2)]),
        )
        .expect("gated row");

        assert_eq!(row.outcome, expected_outcome, "case {name}");
    }
}

#[test]
fn json_sequence_requires_lf_on_every_record() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = script(directory.path(), "missing-lf", "printf '\\0361'");
    let args = ["--output-format", "json", "--seq"];
    let row = run_gated_row(
        &case(),
        &tq_adapter(args),
        &corpus(),
        DatasetTier::Startup,
        &invocation(executable, args.map(str::to_owned).to_vec()),
        &reference_values(&[json!(1)]),
    )
    .expect("gated row");

    assert_eq!(row.outcome, BenchmarkOutcome::Incorrect);
}

#[test]
fn last_toon_framing_option_controls_normalization() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = script(directory.path(), "unframed", "printf 'a: 1\\n'");
    let args = ["--seq", "--unframed"];
    let row = run_gated_row(
        &case(),
        &tq_adapter(args),
        &corpus(),
        DatasetTier::Startup,
        &invocation(executable, args.map(str::to_owned).to_vec()),
        &reference_values(&[json!({"a": 1})]),
    )
    .expect("gated row");

    assert_eq!(row.outcome, BenchmarkOutcome::Timed);
}

#[test]
fn nonzero_candidate_process_is_not_timed_when_reference_has_same_error() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = script(
        directory.path(),
        "compile-error",
        "printf 'compile error\\n' >&2; exit 3",
    );
    for contract in [
        OutputContractKind::SemanticSequence,
        OutputContractKind::ExitOnly,
    ] {
        let mut case = case();
        case.output_contract.kind = contract;
        let mut reference = error_reference();
        if contract == OutputContractKind::ExitOnly {
            reference.payload = CorrectnessPayload::ExitOnly;
        }
        let row = run_gated_row(
            &case,
            &tq_adapter(Vec::<String>::new()),
            &corpus(),
            DatasetTier::Startup,
            &invocation(executable.clone(), Vec::new()),
            &reference,
        )
        .expect("gated row");
        assert_eq!(row.outcome, BenchmarkOutcome::Incorrect);
    }
}

fn script(directory: &std::path::Path, name: &str, body: &str) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("script");
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("permissions");
    path
}

fn invocation(executable: PathBuf, args: Vec<String>) -> BenchmarkInvocation {
    BenchmarkInvocation {
        executable,
        args,
        stdin: Vec::new(),
        current_dir: None,
        // Process startup under instrumented CI is not the behavior under test.
        timeout: Duration::from_secs(20),
        output_limit: 1024,
        rss_limit: None,
        retain_output: false,
    }
}

fn case() -> BenchmarkCase {
    BenchmarkCase {
        schema_version: 1,
        id: "benchmark.output-contract".to_owned(),
        compatibility_gate: "common.identity.number".to_owned(),
        dataset_selector: DatasetSelector {
            family: DatasetFamily::SyntheticHelper,
            tiers: vec![DatasetTier::Startup],
        },
        query: ".".to_owned(),
        execution_class: ExecutionClass::Startup,
        measure_first_result: false,
        sampling: BenchmarkSampling {
            warmups: 0,
            small: 1,
            medium: 1,
            large: 1,
        },
        timeout_seconds: 20,
        limits: BenchmarkLimits {
            output_bytes: 1024,
            rss_bytes: None,
        },
        output_contract: OutputContract {
            kind: OutputContractKind::SemanticSequence,
            reference_adapter: "jq-json".to_owned(),
        },
        adapters: vec![tq_adapter(Vec::<String>::new())],
    }
}

fn tq_adapter<I, S>(args: I) -> BenchmarkAdapter
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    BenchmarkAdapter {
        id: "tq-json".to_owned(),
        tool: BenchmarkTool::Tq,
        input_format: InputFormat::Json,
        applicable: true,
        unsupported_reason: None,
        args: args.into_iter().map(Into::into).collect(),
        query: None,
        comparison_families: vec![ComparisonFamily::SameFormat],
    }
}

fn reference_values(values: &[serde_json::Value]) -> CorrectnessObservation {
    CorrectnessObservation {
        payload: CorrectnessPayload::SemanticSequence(
            semantic_digest(values).expect("semantic digest"),
        ),
        process_status: ProcessStatus::Exited,
        exit_code: Some(0),
        error_class: None,
    }
}

fn error_reference() -> CorrectnessObservation {
    CorrectnessObservation {
        payload: CorrectnessPayload::SemanticSequence(
            semantic_digest(std::iter::empty()).expect("empty semantic digest"),
        ),
        process_status: ProcessStatus::Exited,
        exit_code: Some(3),
        error_class: Some(ErrorClass::QueryCompile),
    }
}

fn corpus() -> BenchmarkCorpusIdentity {
    BenchmarkCorpusIdentity {
        origin: "smoke".to_owned(),
        source_id: "helper".to_owned(),
        tier: "startup".to_owned(),
        format: InputFormat::Json,
        artifact: ArtifactIdentity {
            path: "inline".to_owned(),
            bytes: 4,
            sha256: "a".repeat(64),
        },
        logical_records: 1,
        manifest_sha256: "b".repeat(64),
    }
}
