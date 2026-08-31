//! Sampling-loop enforcement of correctness gates.

#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

use serde_json::json;
use tq_test_support::{
    benchmark::{
        BenchmarkAdapter, BenchmarkCase, BenchmarkCorpusIdentity, BenchmarkInvocation,
        BenchmarkLimits, BenchmarkOutcome, BenchmarkSampling, BenchmarkTool, ComparisonFamily,
        CorrectnessObservation, CorrectnessPayload, DatasetFamily, DatasetSelector, DatasetTier,
        ExecutionClass, InputFormat, OutputContract, OutputContractKind, normalize_correctness_run,
        run_correctness_limit_probe, run_gated_row, semantic_digest,
    },
    compatibility::{ProcessStatus, ToolKind},
    corpus::ArtifactIdentity,
};

#[test]
fn incorrect_candidate_never_receives_timing_samples() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = script(directory.path(), "wrong", "printf '2\\n'");
    let row = run_gated_row(
        &case(),
        &adapter(),
        &corpus(),
        DatasetTier::Startup,
        &invocation(executable),
        &reference(),
    )
    .expect("gated row");
    assert_eq!(row.outcome, BenchmarkOutcome::Incorrect);
    assert_eq!(row.samples.len(), 0);
    assert!(row.summary.is_none());
}

#[test]
fn correct_candidate_runs_warmup_and_requested_samples() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = script(directory.path(), "correct", "printf '1\\n'");
    let row = run_gated_row(
        &case(),
        &adapter(),
        &corpus(),
        DatasetTier::Startup,
        &invocation(executable),
        &reference(),
    )
    .expect("gated row");
    assert_eq!(row.outcome, BenchmarkOutcome::Timed);
    assert_eq!(row.samples.len(), 3);
    assert!(row.summary.is_some());
}

#[test]
fn signaled_correctness_candidate_is_not_misclassified_as_incorrect() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = script(directory.path(), "signaled", "kill -TERM $$");
    let row = run_gated_row(
        &case(),
        &adapter(),
        &corpus(),
        DatasetTier::Startup,
        &invocation(executable),
        &reference(),
    )
    .expect("gated row");
    assert_eq!(row.outcome, BenchmarkOutcome::OomOrSignal);
    assert_eq!(row.samples.len(), 1);
    assert!(row.summary.is_some());
}

#[test]
fn oversized_correctness_output_becomes_a_bounded_resource_row() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = script(directory.path(), "oversized", "head -c 40000000 /dev/zero");
    let mut invocation = invocation(executable);
    invocation.output_limit = 64 * 1024 * 1024;
    let row = run_correctness_limit_probe(
        &case(),
        &adapter(),
        &corpus(),
        DatasetTier::Startup,
        &invocation,
    )
    .expect("bounded probe row");

    assert_eq!(row.outcome, BenchmarkOutcome::ResourceLimit);
    assert_eq!(row.limits.output_bytes, 32 * 1024 * 1024);
    assert_eq!(row.samples.len(), 1);
    assert!(row.summary.is_some());
}

#[test]
fn semantic_digest_matches_json_and_toon_without_retaining_result_sequences() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let json = script(
        directory.path(),
        "json-sequence",
        "printf '1\\n{\"a\":2}\\n'",
    );
    let toon = script(
        directory.path(),
        "toon-sequence",
        "printf '\\0361\\n\\036a: 2\\n'",
    );
    let reference = normalize_correctness_run(
        &invocation(json),
        ToolKind::Jq,
        OutputContractKind::SemanticSequence,
    )
    .expect("JSON semantic digest");
    let candidate = normalize_correctness_run(
        &invocation(toon),
        ToolKind::Tq,
        OutputContractKind::SemanticSequence,
    )
    .expect("TOON semantic digest");

    assert_eq!(reference.payload, candidate.payload);
    assert!(matches!(
        reference.payload,
        CorrectnessPayload::SemanticSequence(ref digest) if digest.result_count == 2
    ));
}

fn script(directory: &std::path::Path, name: &str, body: &str) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("script");
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("permissions");
    path
}

fn invocation(executable: PathBuf) -> BenchmarkInvocation {
    BenchmarkInvocation {
        executable,
        args: Vec::new(),
        stdin: Vec::new(),
        current_dir: None,
        timeout: Duration::from_secs(2),
        output_limit: 1024,
        rss_limit: None,
        retain_output: false,
    }
}

fn case() -> BenchmarkCase {
    BenchmarkCase {
        schema_version: 1,
        id: "benchmark.test".to_owned(),
        compatibility_gate: "common.identity.number".to_owned(),
        dataset_selector: DatasetSelector {
            family: DatasetFamily::SyntheticHelper,
            tiers: vec![DatasetTier::Startup],
        },
        query: ".".to_owned(),
        execution_class: ExecutionClass::Startup,
        measure_first_result: true,
        sampling: BenchmarkSampling {
            warmups: 1,
            small: 3,
            medium: 3,
            large: 3,
        },
        timeout_seconds: 2,
        limits: BenchmarkLimits {
            output_bytes: 1024,
            rss_bytes: None,
        },
        output_contract: OutputContract {
            kind: OutputContractKind::SemanticSequence,
            reference_adapter: "jq-json".to_owned(),
        },
        adapters: vec![adapter()],
    }
}

fn adapter() -> BenchmarkAdapter {
    BenchmarkAdapter {
        id: "jq-json".to_owned(),
        tool: BenchmarkTool::Jq,
        input_format: InputFormat::Json,
        applicable: true,
        args: Vec::new(),
        query: None,
        comparison_families: vec![ComparisonFamily::SameFormat],
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

fn reference() -> CorrectnessObservation {
    let values = [json!(1)];
    CorrectnessObservation {
        payload: CorrectnessPayload::SemanticSequence(
            semantic_digest(&values).expect("semantic digest"),
        ),
        process_status: ProcessStatus::Exited,
        exit_code: Some(0),
        error_class: None,
    }
}
