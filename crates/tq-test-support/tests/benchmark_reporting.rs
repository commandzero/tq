//! Correctness, environment, comparability, and regression tests.

use std::collections::BTreeMap;

use serde_json::json;
use tq_test_support::{
    benchmark::{
        BenchmarkCampaignReport, BenchmarkCorpusIdentity, BenchmarkFinalStatus, BenchmarkOutcome,
        BenchmarkRow, BenchmarkSample, Comparability, CorrectnessDecision, CorrectnessObservation,
        CorrectnessPayload, ExecutionClass, InputFormat, OutputContractKind, RegressionGate,
        RegressionThresholds, collect_environment, compare_reports, correctness_gate,
        evaluate_regression, populate_reference_ratios, semantic_digest, summarize_samples,
    },
    compatibility::ProcessStatus,
    corpus::ArtifactIdentity,
};

fn observation(results: &[serde_json::Value]) -> CorrectnessObservation {
    CorrectnessObservation {
        payload: CorrectnessPayload::SemanticSequence(
            semantic_digest(results).expect("semantic digest"),
        ),
        process_status: ProcessStatus::Exited,
        exit_code: Some(0),
        error_class: None,
    }
}

#[test]
fn correctness_gate_rejects_semantic_and_normalization_failures() {
    let reference = observation(&[json!(1), json!(2)]);
    assert_eq!(
        correctness_gate(
            OutputContractKind::SemanticSequence,
            &reference,
            Ok(&observation(&[json!(1), json!(2)]))
        ),
        CorrectnessDecision::Passed
    );
    assert!(matches!(
        correctness_gate(
            OutputContractKind::SemanticSequence,
            &reference,
            Ok(&observation(&[json!(2), json!(1)]))
        ),
        CorrectnessDecision::Incorrect(_)
    ));
    assert!(matches!(
        correctness_gate(
            OutputContractKind::SemanticSequence,
            &reference,
            Err("malformed YAML")
        ),
        CorrectnessDecision::Unnormalized(_)
    ));
}

#[test]
fn environment_has_stable_identity_and_explicit_optional_fields() {
    let environment = collect_environment("release-benchmark");
    assert_eq!(environment.machine_identity.len(), 64);
    assert_ne!(environment.os, "");
    assert_ne!(environment.architecture, "");
    assert_eq!(environment.compiler_profile, "release-benchmark");
    let value = serde_json::to_value(environment).expect("environment JSON");
    for optional in [
        "kernel",
        "physical_cpus",
        "cpu_model",
        "memory_bytes",
        "filesystem",
        "power_settings",
    ] {
        assert!(value.get(optional).is_some(), "missing explicit {optional}");
    }
}

#[test]
fn non_comparable_machine_and_corpus_are_visibly_separated() {
    let left = campaign("machine-a", "digest-a", 100, 1024);
    let right = campaign("machine-b", "digest-b", 100, 1024);
    let comparison = compare_reports(&left, &right);
    assert!(!comparison.comparable);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("machine"))
    );
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("corpus"))
    );
}

#[test]
fn different_campaign_profiles_are_not_regression_comparable() {
    let left = campaign("machine-a", "digest-a", 100, 1024);
    let mut right = left.clone();
    right.profile = "large".to_owned();
    let comparison = compare_reports(&left, &right);
    assert!(!comparison.comparable);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("profile"))
    );
}

#[test]
fn tq_regression_gate_uses_configured_self_thresholds_only() {
    let baseline = campaign("machine-a", "digest-a", 100, 1024);
    let candidate = campaign("machine-a", "digest-a", 130, 2048);
    let gate = evaluate_regression(
        &baseline,
        &candidate,
        RegressionThresholds {
            wall_time_percent: 20.0,
            peak_rss_percent: 50.0,
            minimum_samples: 3,
        },
    );
    assert!(gate.evaluated);
    assert_eq!(gate.failures.len(), 2);
}

fn campaign(machine: &str, digest: &str, wall: u128, rss: u64) -> BenchmarkCampaignReport {
    let samples = (0..3)
        .map(|_| BenchmarkSample {
            wall_time_micros: wall,
            user_cpu_micros: Some(1),
            system_cpu_micros: Some(1),
            peak_rss_bytes: Some(rss),
            first_result_micros: Some(1),
            output_bytes: 1,
        })
        .collect::<Vec<_>>();
    let summary = summarize_samples(&samples, 100, 1);
    let mut environment = collect_environment("release-benchmark");
    machine.clone_into(&mut environment.machine_identity);
    BenchmarkCampaignReport {
        schema_version: 1,
        campaign_id: "test".to_owned(),
        profile: "standard".to_owned(),
        environment,
        corpus: vec![BenchmarkCorpusIdentity {
            origin: "refreshed".to_owned(),
            source_id: "source".to_owned(),
            tier: "small".to_owned(),
            format: InputFormat::Json,
            artifact: ArtifactIdentity {
                path: "source.json".to_owned(),
                bytes: 100,
                sha256: digest.to_owned(),
            },
            logical_records: 1,
            manifest_sha256: digest.to_owned(),
        }],
        tools: Vec::new(),
        cases: vec![BenchmarkRow {
            case_id: "benchmark.identity".to_owned(),
            adapter_id: "tq-json".to_owned(),
            source_id: "source".to_owned(),
            tier: "small".to_owned(),
            input_format: InputFormat::Json,
            execution_class: ExecutionClass::Document,
            comparison_families: Vec::new(),
            command: vec!["tq".to_owned(), ".".to_owned()],
            outcome: BenchmarkOutcome::Timed,
            warmups: 2,
            requested_samples: 3,
            timeout_seconds: 10,
            limits: tq_test_support::benchmark::BenchmarkLimits {
                output_bytes: 1024,
                rss_bytes: None,
            },
            samples,
            summary,
            reference_ratios: BTreeMap::new(),
        }],
        comparability: Comparability::default(),
        regression_gate: RegressionGate::default(),
        final_status: BenchmarkFinalStatus::Passed,
    }
}

#[test]
fn reference_ratios_are_independent_and_have_no_composite_score() {
    let mut report = campaign("machine-a", "digest-a", 100, 1024);
    let mut jq = report.cases[0].clone();
    jq.adapter_id = "jq-json".to_owned();
    jq.summary = summarize_samples(&jq.samples, 100, 1);
    report.cases.push(jq);
    populate_reference_ratios(&mut report.cases, &["jq-json"]);
    assert!(report.cases[0].reference_ratios.contains_key("jq-json"));
    let value = serde_json::to_value(&report.cases[0]).expect("row JSON");
    assert!(value.get("composite_score").is_none());
}
