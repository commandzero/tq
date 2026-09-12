//! Sampling-loop enforcement of correctness gates.

#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

use serde_json::json;
use tq_test_support::{
    benchmark::{
        BenchmarkAdapter, BenchmarkCase, BenchmarkCorpusIdentity, BenchmarkInvocation,
        BenchmarkLimits, BenchmarkOutcome, BenchmarkRunnerError, BenchmarkSampling, BenchmarkTool,
        ComparisonFamily, CorrectnessObservation, CorrectnessPayload, DatasetFamily,
        DatasetSelector, DatasetTier, ExecutionClass, InputFormat, MeasureError, OutputContract,
        OutputContractKind, normalize_correctness_run, run_correctness_limit_probe, run_gated_row,
        semantic_digest,
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
fn rss_limited_rows_keep_timing_and_enforcement_repetitions_separate() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = script(directory.path(), "correct-limited", "printf '1\\n'");
    let mut invocation = invocation(executable);
    invocation.rss_limit = Some(128 * 1024 * 1024);
    let mut case = case();
    case.limits.rss_bytes = invocation.rss_limit;
    let row = run_gated_row(
        &case,
        &adapter(),
        &corpus(),
        DatasetTier::Startup,
        &invocation,
        &reference(),
    )
    .expect("limited gated row");
    assert_eq!(row.outcome, BenchmarkOutcome::Timed);
    assert_eq!(row.samples.len(), 3);
    assert!(row.samples.iter().all(|sample| {
        sample
            .measurement_protocol
            .as_ref()
            .is_some_and(|protocol| protocol.rss_poll_interval_micros.is_none())
    }));
    let stored = serde_json::to_value(&row).unwrap();
    let instrumented = stored["instrumented_samples"]
        .as_array()
        .expect("separate enforcement samples");
    assert_eq!(instrumented.len(), 3);
    assert!(
        instrumented
            .iter()
            .all(|sample| sample["measurement_protocol"]["rss_poll_interval_micros"] == 25_000)
    );
    assert_eq!(row.summary.as_ref().unwrap().wall_time_micros.samples, 3);
}

#[test]
fn failed_sampled_enforcement_is_not_reported_as_a_timing_repetition() {
    let directory = tempfile::tempdir().unwrap();
    let executable = script(directory.path(), "over-limit", "printf '1\\n'");
    let mut invocation = invocation(executable);
    invocation.rss_limit = Some(1);
    let mut case = case();
    case.limits.rss_bytes = Some(1);
    let row = run_gated_row(
        &case,
        &adapter(),
        &corpus(),
        DatasetTier::Startup,
        &invocation,
        &reference(),
    )
    .unwrap();
    assert_eq!(row.outcome, BenchmarkOutcome::ResourceLimit);
    assert!(row.samples.is_empty());
    assert_eq!(row.instrumented_samples.len(), 1);
    assert!(row.summary.is_none());
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
    assert!(row.summary.is_none());
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
    assert!(row.summary.is_none());
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
    let mut toon_invocation = invocation(toon);
    toon_invocation.args = vec!["--seq".to_owned()];
    let candidate = normalize_correctness_run(
        &toon_invocation,
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

#[test]
fn default_toon_values_are_normalized_without_sequence_framing() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let toon = script(directory.path(), "toon-values", "printf 'a: 1\\n2\\n'");
    let row = run_gated_row(
        &case(),
        &tq_adapter(Vec::new()),
        &corpus(),
        DatasetTier::Startup,
        &invocation(toon),
        &reference_values(&[json!({"a": 1}), json!(2)]),
    )
    .expect("gated row");

    assert_eq!(row.outcome, BenchmarkOutcome::Timed);
}

#[test]
fn default_toon_values_preserve_adjacent_same_shaped_objects() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let toon = script(
        directory.path(),
        "adjacent-objects",
        "printf 'a: 1\\na: 2\\n'",
    );
    let row = run_gated_row(
        &case(),
        &tq_adapter(Vec::new()),
        &corpus(),
        DatasetTier::Startup,
        &invocation(toon),
        &reference_values(&[json!({"a": 1}), json!({"a": 2})]),
    )
    .expect("adjacent object row");

    assert_eq!(row.outcome, BenchmarkOutcome::Timed);
}

#[test]
fn default_toon_value_without_terminal_lf_is_reported_as_unnormalized() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let toon = script(directory.path(), "missing-terminal-lf", "printf 'a: 1'");
    let row = run_gated_row(
        &case(),
        &tq_adapter(Vec::new()),
        &corpus(),
        DatasetTier::Startup,
        &invocation(toon),
        &reference_values(&[json!({"a": 1})]),
    )
    .expect("missing terminal LF row");

    assert_eq!(row.outcome, BenchmarkOutcome::Incorrect);
    assert!(
        row.diagnostic
            .as_deref()
            .is_some_and(|diagnostic| diagnostic.contains("terminal LF"))
    );
}

#[test]
fn default_toon_value_with_trailing_extra_bytes_is_rejected() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let toon = script(directory.path(), "trailing-bytes", "printf 'a: 1\\n2\\n'");
    let row = run_gated_row(
        &case(),
        &tq_adapter(Vec::new()),
        &corpus(),
        DatasetTier::Startup,
        &invocation(toon),
        &reference_values(&[json!({"a": 1})]),
    )
    .expect("trailing bytes row");

    assert_eq!(row.outcome, BenchmarkOutcome::Incorrect);
    assert!(row.diagnostic.is_some());
}

#[test]
fn malformed_default_toon_multi_result_stream_is_rejected_with_bounded_fallback() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let toon = script(
        directory.path(),
        "malformed-multi-result",
        "printf '0\\n0\\n0\\n'",
    );
    let row = run_gated_row(
        &case(),
        &tq_adapter(Vec::new()),
        &corpus(),
        DatasetTier::Startup,
        &invocation(toon),
        &reference_values(&[json!(1), json!(2)]),
    )
    .expect("malformed multi-result row");

    assert_eq!(row.outcome, BenchmarkOutcome::Incorrect);
    assert!(row.diagnostic.is_some());
}

#[test]
fn large_first_toon_composite_and_following_scalar_use_linear_boundary_hint() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let items = (0..5_000)
        .map(|index| json!({"id": index, "nested": {"value": index}}))
        .collect::<Vec<_>>();
    let first = json!({"items": items});
    let toon_value = tq_core::Value::from_json(first.clone()).expect("TOON value");
    let mut output = tq_toon::encode(&toon_value, tq_toon::WriterConfig::default());
    output.push('\n');
    output.push_str("2\n");
    fs::write(directory.path().join("large-toon.output"), output).expect("TOON output");
    let executable = script(directory.path(), "large-toon", "cat \"$0.output\"");
    let mut large_case = case();
    large_case.limits.output_bytes = 1024 * 1024;
    large_case.sampling = BenchmarkSampling {
        warmups: 0,
        small: 1,
        medium: 1,
        large: 1,
    };
    let mut large_invocation = invocation(executable);
    large_invocation.output_limit = 1024 * 1024;
    let row = run_gated_row(
        &large_case,
        &tq_adapter(Vec::new()),
        &corpus(),
        DatasetTier::Startup,
        &large_invocation,
        &reference_values(&[first, json!(2)]),
    )
    .expect("large composite row");

    assert_eq!(row.outcome, BenchmarkOutcome::Timed);
}

#[test]
fn explicit_toon_sequence_and_json_output_are_normalized_from_arguments() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let sequence = script(
        directory.path(),
        "toon-sequence-values",
        "printf '\\036a: 1\\n\\0362\\n'",
    );
    let mut sequence_invocation = invocation(sequence);
    sequence_invocation.args = vec!["--seq".to_owned()];
    let sequence_row = run_gated_row(
        &case(),
        &tq_adapter(sequence_invocation.args.clone()),
        &corpus(),
        DatasetTier::Startup,
        &sequence_invocation,
        &reference_values(&[json!({"a": 1}), json!(2)]),
    )
    .expect("sequence gated row");
    assert_eq!(sequence_row.outcome, BenchmarkOutcome::Timed);

    let json_output = script(
        directory.path(),
        "json-output-values",
        "printf '%s\\n' '{\"a\":1}' '2'",
    );
    let fixture_output = std::process::Command::new(&json_output)
        .output()
        .expect("JSON fixture output");
    assert!(fixture_output.status.success());
    assert_eq!(fixture_output.stdout, b"{\"a\":1}\n2\n");
    let mut json_invocation = invocation(json_output);
    json_invocation.args = vec!["--output-format".to_owned(), "json".to_owned()];
    let json_row = run_gated_row(
        &case(),
        &tq_adapter(json_invocation.args.clone()),
        &corpus(),
        DatasetTier::Startup,
        &json_invocation,
        &reference_values(&[json!({"a": 1}), json!(2)]),
    )
    .expect("JSON gated row");
    assert_eq!(json_row.outcome, BenchmarkOutcome::Timed);
}

#[test]
fn resource_failure_is_not_misclassified_as_incorrect() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = script(
        directory.path(),
        "resource-limit",
        "printf 'tq: VM resource limit exceeded: vm-steps\\n' >&2; exit 5",
    );
    let row = run_gated_row(
        &case(),
        &tq_adapter(Vec::new()),
        &corpus(),
        DatasetTier::Startup,
        &invocation(executable),
        &reference(),
    )
    .expect("resource row");

    assert_eq!(row.outcome, BenchmarkOutcome::ResourceLimit);
    assert!(
        row.diagnostic
            .as_deref()
            .is_some_and(|diagnostic| diagnostic.contains("Resource") && diagnostic.contains('5'))
    );
}

#[test]
fn successful_empty_selection_is_timed_without_first_output_latency() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = script(directory.path(), "empty-selection", "true");
    let mut empty_case = case();
    empty_case.measure_first_result = true;
    let row = run_gated_row(
        &empty_case,
        &tq_adapter(Vec::new()),
        &corpus(),
        DatasetTier::Startup,
        &invocation(executable),
        &reference_values(&[]),
    )
    .expect("empty selection row");

    assert_eq!(row.outcome, BenchmarkOutcome::Timed);
    assert_eq!(row.samples.len(), 3);
    assert!(
        row.summary
            .is_some_and(|summary| summary.first_result_micros.is_none())
    );
}

#[test]
fn nonzero_exit_after_correctness_never_becomes_a_valid_timing() {
    for warmups in [0, 1] {
        let directory = tempfile::tempdir().expect("temporary directory");
        let executable = script(
            directory.path(),
            "flaky-status",
            "if test -f invoked; then printf '1\\n'; exit 3; fi\n: > invoked\nprintf '1\\n'",
        );
        let mut invocation = invocation(executable);
        invocation.current_dir = Some(directory.path().to_owned());
        let mut case = case();
        case.sampling.warmups = warmups;
        let row = run_gated_row(
            &case,
            &adapter(),
            &corpus(),
            DatasetTier::Startup,
            &invocation,
            &reference(),
        )
        .expect("gated row");
        assert_eq!(row.outcome, BenchmarkOutcome::Incorrect);
        assert!(row.summary.is_none());
    }
}

#[test]
fn capture_limit_on_candidate_is_a_resource_failure() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = script(directory.path(), "capture-limit", "head -c 40000 /dev/zero");
    let row = run_gated_row(
        &case(),
        &adapter(),
        &corpus(),
        DatasetTier::Startup,
        &invocation(executable),
        &reference(),
    )
    .expect("gated capture limit");
    assert_eq!(row.outcome, BenchmarkOutcome::ResourceLimit);
    assert!(row.summary.is_none());
    assert!(row.diagnostic.is_some_and(|reason| reason.contains("1024")));
}

#[test]
fn capture_limit_on_reference_keeps_successful_probe_unverified() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = script(directory.path(), "small-probe", "printf '1\\n'");
    let row = run_correctness_limit_probe(
        &case(),
        &adapter(),
        &corpus(),
        DatasetTier::Startup,
        &invocation(executable),
    )
    .expect("reference-limited probe");
    assert_eq!(row.outcome, BenchmarkOutcome::ResourceLimit);
    assert!(row.summary.is_none());
    assert!(
        row.diagnostic
            .is_some_and(|reason| reason.contains("reference"))
    );
}

#[test]
fn candidate_measurement_infrastructure_failure_aborts_before_gate_decision() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = script(directory.path(), "missing-rss", "printf '1\\n'");
    let mut measured = invocation(executable);
    measured.current_dir = Some(directory.path().join("does-not-exist"));
    let result = run_gated_row(
        &case(),
        &adapter(),
        &corpus(),
        DatasetTier::Startup,
        &measured,
        &reference(),
    );
    let Err(BenchmarkRunnerError::Measure(MeasureError::Collection {
        source,
        stdout_path,
        stderr_path,
        ..
    })) = result
    else {
        panic!("missing working directory must abort as a collection failure");
    };
    assert!(matches!(source.as_ref(), MeasureError::Io(_)));
    assert!(source.to_string().contains("isolated worker target failed"));
    assert!(stdout_path.exists());
    assert!(stderr_path.exists());
    let _ = fs::remove_file(stdout_path);
    let _ = fs::remove_file(stderr_path);
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
        cancellation: None,
        executable,
        args: Vec::new(),
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
        timeout_seconds: 20,
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
        unsupported_reason: None,
        args: Vec::new(),
        query: None,
        comparison_families: vec![ComparisonFamily::SameFormat],
    }
}

fn tq_adapter(args: Vec<String>) -> BenchmarkAdapter {
    BenchmarkAdapter {
        id: "tq-json".to_owned(),
        tool: BenchmarkTool::Tq,
        input_format: InputFormat::Json,
        applicable: true,
        unsupported_reason: None,
        args,
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
