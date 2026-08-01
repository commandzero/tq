//! Sampling-loop enforcement of correctness gates.

#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

use serde_json::json;
use tq_test_support::{
    benchmark::{
        BenchmarkAdapter, BenchmarkCase, BenchmarkCorpusIdentity, BenchmarkInvocation,
        BenchmarkLimits, BenchmarkOutcome, BenchmarkSampling, BenchmarkTool, ComparisonFamily,
        DatasetFamily, DatasetSelector, DatasetTier, ExecutionClass, InputFormat, OutputContract,
        OutputContractKind, run_gated_row,
    },
    compatibility::{NormalizedObservation, ProcessStatus},
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
    assert!(row.samples.is_empty());
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

fn reference() -> NormalizedObservation {
    NormalizedObservation {
        results: vec![json!(1)],
        raw_bytes: None,
        stderr: Vec::new(),
        process_status: ProcessStatus::Exited,
        exit_code: Some(0),
        error_class: None,
        notes: Vec::new(),
    }
}
