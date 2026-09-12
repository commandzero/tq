//! Correctness, environment, comparability, and regression tests.

use std::collections::BTreeMap;

use serde_json::json;
use tq_test_support::{
    benchmark::{
        BenchmarkCampaignReport, BenchmarkCorpusIdentity, BenchmarkFinalStatus, BenchmarkOutcome,
        BenchmarkRow, BenchmarkSample, Comparability, ComparisonFamily, CorrectnessDecision,
        CorrectnessObservation, CorrectnessPayload, ExecutionClass, InputFormat,
        LaunchIsolationEvidence, MeasurementProtocol, OutputContractKind, RegressionGate,
        RegressionThresholds, RssProvenance, SoftObjectiveStatus, WorkerIdentity,
        collect_environment, compare_reports, correctness_gate, evaluate_regression,
        populate_reference_ratios, semantic_digest, summarize_samples,
    },
    compatibility::{ProcessStatus, ToolIdentity, ToolKind},
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
fn different_operating_systems_are_not_regression_comparable() {
    let left = campaign("machine-a", "digest-a", 100, 1024);
    let mut right = left.clone();
    right.environment.os = if left.environment.os == "linux" {
        "macos".to_owned()
    } else {
        "linux".to_owned()
    };
    // Keep the host digest equal so this assertion covers the explicit OS
    // boundary rather than relying on identity construction details.
    right.environment.machine_identity = left.environment.machine_identity.clone();

    let comparison = compare_reports(&left, &right);
    assert!(!comparison.comparable);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("operating system")),
        "reasons: {:?}",
        comparison.reasons
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
fn changed_measurement_contracts_are_not_report_comparable() {
    let left = campaign("machine-a", "digest-a", 100, 1024);
    let mut right = left.clone();
    for sample in &mut right.cases[0].samples {
        sample.measurement_protocol = None;
        sample.rss_provenance = Some(RssProvenance::GnuTimeV);
    }
    right.cases[0].summary = summarize_samples(&right.cases[0].samples, 100, 1);

    let comparison = compare_reports(&left, &right);
    assert!(!comparison.comparable);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("measurement protocol"))
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

#[test]
fn issue30_disclosure_and_blocking_boundaries_use_unrounded_metrics() {
    let baseline = campaign("machine-a", "digest-a", 1000, 1000);
    for (value, notices, failures) in [(1200, 0, 0), (1201, 2, 0), (1500, 2, 0), (1501, 2, 2)] {
        let candidate = campaign(
            "machine-a",
            "digest-a",
            value,
            u64::try_from(value).unwrap(),
        );
        let gate = evaluate_regression(
            &baseline,
            &candidate,
            RegressionThresholds {
                wall_time_percent: 50.0,
                peak_rss_percent: 50.0,
                minimum_samples: 3,
            },
        );
        assert!(gate.evaluated);
        assert_eq!(gate.disclosures.len(), notices, "value {value}");
        assert_eq!(gate.failures.len(), failures, "value {value}");
        if notices > 0 {
            let disclosures = gate.disclosures.join("\n");
            assert!(disclosures.contains("baseline="), "value {value}");
            assert!(disclosures.contains("candidate="), "value {value}");
            assert!(disclosures.contains("baseline_samples=3"), "value {value}");
            assert!(disclosures.contains("candidate_samples=3"), "value {value}");
            assert!(disclosures.contains("dispersion="), "value {value}");
        }
    }
}

#[test]
fn missing_baseline_rows_are_not_evaluated_as_passing() {
    let mut baseline = campaign("machine-a", "digest-a", 1000, 1000);
    baseline.cases.clear();
    let candidate = campaign("machine-a", "digest-a", 1000, 1000);
    let gate = evaluate_regression(
        &baseline,
        &candidate,
        RegressionThresholds {
            wall_time_percent: 50.0,
            peak_rss_percent: 50.0,
            minimum_samples: 3,
        },
    );
    assert!(!gate.evaluated);
    assert_eq!(gate.unavailable.len(), 1);
}

#[test]
fn missing_authoritative_sample_is_unavailable_not_a_passing_gate() {
    let baseline = campaign("machine-a", "digest-a", 1000, 1000);
    let mut candidate = campaign("machine-a", "digest-a", 1000, 1000);
    candidate.cases[0].samples[1].peak_rss_bytes = None;
    candidate.cases[0].summary = summarize_samples(&candidate.cases[0].samples, 100, 1);

    let gate = evaluate_regression(
        &baseline,
        &candidate,
        RegressionThresholds {
            wall_time_percent: 50.0,
            peak_rss_percent: 50.0,
            minimum_samples: 3,
        },
    );
    assert!(!gate.evaluated);
    assert!(gate.failures.is_empty());
    assert!(!gate.unavailable.is_empty());
}

#[test]
fn self_regression_allows_a_different_tq_binary_but_requires_command_and_protocol_match() {
    let baseline = campaign("machine-a", "digest-a", 1000, 1000);
    let mut candidate = baseline.clone();
    candidate.cases[0].command[0] = "/other/build/tq".to_owned();
    let thresholds = RegressionThresholds {
        wall_time_percent: 50.0,
        peak_rss_percent: 50.0,
        minimum_samples: 3,
    };
    assert!(evaluate_regression(&baseline, &candidate, thresholds.clone()).evaluated);

    candidate.cases[0].command[1] = "different-query".to_owned();
    let gate = evaluate_regression(&baseline, &candidate, thresholds.clone());
    assert!(!gate.evaluated);
    assert!(
        gate.unavailable
            .iter()
            .any(|reason| reason.contains("contract"))
    );

    candidate = baseline.clone();
    for sample in &mut candidate.cases[0].samples {
        let mut protocol = native_protocol();
        protocol.input_delivery = "pipe".to_owned();
        sample.measurement_protocol = Some(protocol);
    }
    candidate.cases[0].summary = summarize_samples(&candidate.cases[0].samples, 100, 1);
    let gate = evaluate_regression(&baseline, &candidate, thresholds);
    assert!(!gate.evaluated);
    assert!(
        gate.unavailable
            .iter()
            .any(|reason| reason.contains("measurement protocol"))
    );
}

#[test]
fn relocated_corpus_paths_are_normalized_only_for_the_recorded_row_artifact() {
    let mut baseline = campaign("machine-a", "digest-a", 1000, 1000);
    let baseline_path = "/baseline/cache/source.json".to_owned();
    baseline.corpus[0].artifact.path.clone_from(&baseline_path);
    baseline.cases[0].command.push(baseline_path);

    let mut candidate = baseline.clone();
    let candidate_path = "/candidate/cache/source.json".to_owned();
    candidate.corpus[0]
        .artifact
        .path
        .clone_from(&candidate_path);
    candidate.cases[0].command[0] = "/candidate/build/tq".to_owned();
    candidate.cases[0].command[2] = candidate_path;

    assert!(compare_reports(&baseline, &candidate).comparable);
    assert!(
        evaluate_regression(
            &baseline,
            &candidate,
            RegressionThresholds {
                wall_time_percent: 50.0,
                peak_rss_percent: 50.0,
                minimum_samples: 3,
            },
        )
        .evaluated
    );
}

#[test]
fn self_regression_rejects_a_command_path_that_is_not_the_row_artifact() {
    let mut baseline = campaign("machine-a", "digest-a", 1000, 1000);
    let baseline_path = "/baseline/cache/source.json".to_owned();
    baseline.corpus[0].artifact.path.clone_from(&baseline_path);
    baseline.cases[0].command.push(baseline_path);

    let mut candidate = baseline.clone();
    let candidate_path = "/candidate/cache/source.json".to_owned();
    candidate.corpus[0]
        .artifact
        .path
        .clone_from(&candidate_path);
    candidate.cases[0].command[0] = "/candidate/build/tq".to_owned();
    candidate.cases[0].command[2] = "/candidate/cache/not-the-row-artifact.json".to_owned();

    let gate = evaluate_regression(
        &baseline,
        &candidate,
        RegressionThresholds {
            wall_time_percent: 50.0,
            peak_rss_percent: 50.0,
            minimum_samples: 3,
        },
    );
    assert!(!gate.evaluated);
    assert!(
        gate.unavailable
            .iter()
            .any(|reason| reason.contains("contract"))
    );
}

#[test]
fn corpus_relocation_does_not_hide_an_immutable_artifact_change() {
    let mut baseline = campaign("machine-a", "digest-a", 1000, 1000);
    baseline.corpus[0].artifact.path = "/baseline/source.json".to_owned();
    let mut candidate = baseline.clone();
    candidate.corpus[0].artifact.path = "/candidate/source.json".to_owned();
    candidate.corpus[0].artifact.bytes += 1;

    let comparison = compare_reports(&baseline, &candidate);
    assert!(!comparison.comparable);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("corpus"))
    );
}

#[test]
fn reference_tool_identity_changes_are_metadata_for_tq_self_regression() {
    let mut baseline = campaign("machine-a", "digest-a", 1000, 1000);
    baseline.tools = vec![reference_tool("jq-1")];
    let mut candidate = campaign("machine-a", "digest-a", 1000, 1000);
    candidate.tools = vec![reference_tool("jq-2")];

    let gate = evaluate_regression(
        &baseline,
        &candidate,
        RegressionThresholds {
            wall_time_percent: 50.0,
            peak_rss_percent: 50.0,
            minimum_samples: 3,
        },
    );
    assert!(gate.evaluated);
    assert!(gate.failures.is_empty());
}

#[test]
fn historical_wrapper_samples_are_retained_but_not_self_regression_evidence() {
    let mut baseline = campaign("machine-a", "digest-a", 1000, 1000);
    for sample in &mut baseline.cases[0].samples {
        sample.measurement_protocol = None;
        sample.rss_provenance = Some(RssProvenance::GnuTimeV);
    }
    baseline.cases[0].summary = summarize_samples(&baseline.cases[0].samples, 100, 1);
    let candidate = baseline.clone();

    let gate = evaluate_regression(
        &baseline,
        &candidate,
        RegressionThresholds {
            wall_time_percent: 50.0,
            peak_rss_percent: 50.0,
            minimum_samples: 3,
        },
    );
    assert!(!gate.evaluated);
    assert!(
        gate.unavailable
            .iter()
            .any(|reason| reason.contains("contract"))
    );
}

#[test]
fn uncalibrated_native_samples_are_not_comparison_evidence() {
    let baseline = campaign("machine-a", "digest-a", 1000, 1000);
    let mut candidate = baseline.clone();
    for sample in &mut candidate.cases[0].samples {
        sample
            .measurement_protocol
            .as_mut()
            .expect("native protocol")
            .validated_accuracy_micros = None;
    }
    candidate.cases[0].summary = summarize_samples(&candidate.cases[0].samples, 100, 1);

    let comparison = compare_reports(&baseline, &candidate);
    assert!(!comparison.comparable);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("timing controls are unvalidated")),
        "reasons: {:?}",
        comparison.reasons
    );

    let gate = evaluate_regression(
        &baseline,
        &candidate,
        RegressionThresholds {
            wall_time_percent: 50.0,
            peak_rss_percent: 50.0,
            minimum_samples: 3,
        },
    );
    assert!(!gate.evaluated);
    assert!(gate.failures.is_empty());
    assert!(gate.unavailable.iter().any(|reason| {
        reason.contains("timing precision") || reason.contains("measurement protocol")
    }));

    let mut ratio_report = candidate.clone();
    let mut reference = ratio_report.cases[0].clone();
    reference.adapter_id = "jq-json".to_owned();
    ratio_report.cases.push(reference);
    populate_reference_ratios(&mut ratio_report.cases, &["jq-json"]);
    assert!(ratio_report.cases[0].reference_ratios.is_empty());
    assert!(ratio_report.cases[0].reference_peak_rss_ratios.is_empty());
}

#[test]
fn equal_unknown_rss_provenance_is_not_comparable() {
    let mut baseline = campaign("machine-a", "digest-a", 1000, 1000);
    for sample in &mut baseline.cases[0].samples {
        sample.rss_provenance = None;
    }
    let candidate = baseline.clone();

    let comparison = compare_reports(&baseline, &candidate);
    assert!(!comparison.comparable);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| { reason.contains("unverified RSS provenance") })
    );
}

#[test]
fn uncalibrated_native_samples_remain_valid_diagnostic_records() {
    let mut report = campaign("machine-a", "digest-a", 1000, 1000);
    for sample in &mut report.cases[0].samples {
        sample
            .measurement_protocol
            .as_mut()
            .expect("native protocol")
            .validated_accuracy_micros = None;
    }
    assert!(report.validate_authoritative_rss().is_ok());
    let error = report
        .validate_for_publication()
        .expect_err("uncalibrated native samples cannot be published");
    assert!(error.contains("calibrated worker"), "{error}");
}

#[test]
fn native_publication_requires_worker_identity_floor_evidence_and_scope() {
    let mut report = campaign("machine-a", "digest-a", 1000, 1000);
    let protocol = report.cases[0].samples[0]
        .measurement_protocol
        .as_mut()
        .expect("native protocol");
    protocol.worker = None;
    let error = report
        .validate_for_publication()
        .expect_err("worker identity is required");
    assert!(error.contains("calibrated worker"), "{error}");

    let mut report = campaign("machine-a", "digest-a", 1000, 1000);
    for sample in &mut report.cases[0].samples {
        sample
            .measurement_protocol
            .as_mut()
            .expect("native protocol")
            .isolation_evidence = None;
    }
    assert!(
        report.validate_authoritative_rss().is_ok(),
        "diagnostic native reports may omit isolation evidence"
    );
    let error = report
        .validate_for_publication()
        .expect_err("launch isolation evidence is required");
    assert!(error.contains("launch-isolation evidence"), "{error}");

    let mut report = campaign("machine-a", "digest-a", 1000, 1000);
    report.cases[0].samples[0]
        .measurement_protocol
        .as_mut()
        .expect("native protocol")
        .isolation_evidence
        .as_mut()
        .expect("isolation evidence")
        .max_parent_delta_bytes = 2;
    let error = report
        .validate_for_publication()
        .expect_err("failed isolation tolerance is not publication evidence");
    assert!(error.contains("launch-isolation evidence"), "{error}");

    let mut report = campaign("machine-a", "digest-a", 1000, 1000);
    report.cases[0].samples[0]
        .measurement_protocol
        .as_mut()
        .expect("native protocol")
        .rss_scope = "wait4-child".to_owned();
    let error = report
        .validate_for_publication()
        .expect_err("lifetime scope is required");
    assert!(error.contains("lifetime-scope"), "{error}");
}

#[test]
fn worker_identity_mismatch_blocks_pooling_and_self_regression() {
    let baseline = campaign("machine-a", "digest-a", 1000, 1000);
    let mut candidate = baseline.clone();
    candidate.cases[0].samples[0]
        .measurement_protocol
        .as_mut()
        .expect("native protocol")
        .worker
        .as_mut()
        .expect("worker identity")
        .executable_sha256 = "different-worker".to_owned();

    let comparison = compare_reports(&baseline, &candidate);
    assert!(!comparison.comparable);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("measurement protocol"))
    );
    let gate = evaluate_regression(&baseline, &candidate, regression_thresholds());
    assert!(!gate.evaluated);
    assert!(
        gate.unavailable
            .iter()
            .any(|reason| reason.contains("measurement"))
    );
}

#[test]
fn worker_protocol_mismatch_cannot_hide_in_another_row_contract() {
    let mut baseline = campaign("machine-a", "digest-a", 1000, 1000);
    let mut peer = baseline.cases[0].clone();
    peer.adapter_id = "tq-yaml".to_owned();
    for sample in &mut peer.samples {
        sample
            .measurement_protocol
            .as_mut()
            .expect("native protocol")
            .worker
            .as_mut()
            .expect("worker identity")
            .launch_protocol = "peer-worker-v1".to_owned();
    }
    baseline.cases.push(peer);

    let mut candidate = baseline.clone();
    let baseline_worker = worker_identity();
    let peer_worker = WorkerIdentity {
        launch_protocol: "peer-worker-v1".to_owned(),
        ..baseline_worker.clone()
    };
    for sample in &mut candidate.cases[0].samples {
        sample
            .measurement_protocol
            .as_mut()
            .expect("native protocol")
            .worker = Some(peer_worker.clone());
    }
    for sample in &mut candidate.cases[1].samples {
        sample
            .measurement_protocol
            .as_mut()
            .expect("native protocol")
            .worker = Some(baseline_worker.clone());
    }

    let comparison = compare_reports(&baseline, &candidate);
    assert!(!comparison.comparable);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("measurement protocol"))
    );
}

#[test]
fn a_faster_metric_cannot_hide_an_independent_regression() {
    let baseline = campaign("machine-a", "digest-a", 1000, 1000);
    for (wall, rss) in [(500, 1501), (1501, 500)] {
        let candidate = campaign("machine-a", "digest-a", wall, rss);
        let gate = evaluate_regression(
            &baseline,
            &candidate,
            RegressionThresholds {
                wall_time_percent: 50.0,
                peak_rss_percent: 50.0,
                minimum_samples: 3,
            },
        );
        assert!(gate.evaluated);
        assert_eq!(gate.disclosures.len(), 1);
        assert_eq!(gate.failures.len(), 1);
    }
}

#[test]
fn incompatible_collection_methods_do_not_support_regression_or_reference_ratios() {
    let baseline = campaign("machine-a", "digest-a", 1000, 1000);
    let mut candidate = baseline.clone();
    for sample in &mut candidate.cases[0].samples {
        sample.rss_provenance = Some(RssProvenance::BsdTimeL);
    }
    let gate = evaluate_regression(
        &baseline,
        &candidate,
        RegressionThresholds {
            wall_time_percent: 50.0,
            peak_rss_percent: 50.0,
            minimum_samples: 3,
        },
    );
    assert!(!gate.evaluated);
    assert_eq!(gate.unavailable.len(), 1);
    let mut reference = baseline.cases[0].clone();
    reference.adapter_id = "jq-json".to_owned();
    candidate.cases.push(reference);
    populate_reference_ratios(&mut candidate.cases, &["jq-json"]);
    assert!(candidate.cases[0].reference_ratios.is_empty());
    assert!(candidate.cases[0].reference_peak_rss_ratios.is_empty());
    let mut samples = baseline.cases[0].samples.clone();
    samples[1].rss_provenance = Some(RssProvenance::BsdTimeL);
    assert!(summarize_samples(&samples, 100, 1).is_none());
}

#[test]
fn reference_ratios_do_not_pool_mixed_protocol_samples() {
    let mut report = campaign("machine-a", "digest-a", 1000, 1000);
    let mut jq = report.cases[0].clone();
    jq.adapter_id = "jq-json".to_owned();
    let mut changed = native_protocol();
    changed.input_delivery = "pipe".to_owned();
    jq.samples[1].measurement_protocol = Some(changed);
    jq.summary = summarize_samples(&jq.samples, 100, 1);
    assert!(jq.summary.is_none());
    report.cases.push(jq);

    populate_reference_ratios(&mut report.cases, &["jq-json"]);
    assert!(report.cases[0].reference_ratios.is_empty());
    assert!(report.cases[0].reference_peak_rss_ratios.is_empty());
}

fn campaign(machine: &str, digest: &str, wall: u128, rss: u64) -> BenchmarkCampaignReport {
    let samples = (0..3)
        .map(|_| BenchmarkSample {
            measurement_protocol: Some(native_protocol()),
            wall_time_micros: wall,
            user_cpu_micros: Some(1),
            system_cpu_micros: Some(1),
            peak_rss_bytes: Some(rss),
            rss_provenance: Some(RssProvenance::LinuxWait4),
            process_group_peak_rss_bytes: None,
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
            case_id: "benchmark.issue5-identity".to_owned(),
            adapter_id: "tq-json".to_owned(),
            source_id: "source".to_owned(),
            tier: "small".to_owned(),
            input_format: InputFormat::Json,
            execution_class: ExecutionClass::Document,
            comparison_families: vec![ComparisonFamily::SameFormat],
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
            instrumented_samples: Vec::new(),
            summary,
            reference_ratios: BTreeMap::new(),
            reference_peak_rss_ratios: BTreeMap::new(),
            soft_performance_objective: None,
            diagnostic: None,
        }],
        comparability: Comparability::default(),
        regression_gate: RegressionGate::default(),
        final_status: BenchmarkFinalStatus::Passed,
    }
}

fn native_protocol() -> MeasurementProtocol {
    MeasurementProtocol {
        timing_method: "direct-spawn-to-exit-observation".to_owned(),
        input_delivery: "prepared-seekable-stdin-file".to_owned(),
        rss_scope: "wait4-child-lifetime-including-pre-exec-and-waited-descendants-and-threads"
            .to_owned(),
        exit_poll_interval_micros: 100,
        rss_poll_interval_micros: None,
        validated_accuracy_micros: Some(1_000),
        worker: Some(worker_identity()),
        isolation_evidence: Some(isolation_evidence()),
    }
}

fn worker_identity() -> WorkerIdentity {
    WorkerIdentity {
        executable_sha256: "worker".to_owned(),
        launch_protocol: "direct-target-v1".to_owned(),
        collector_source_sha256: "collector".to_owned(),
    }
}

fn isolation_evidence() -> LaunchIsolationEvidence {
    LaunchIsolationEvidence {
        summary_sha256: "summary".to_owned(),
        control_peak_rss_bytes: 1,
        max_parent_delta_bytes: 0,
        tolerance_bytes: 1,
    }
}

fn reference_tool(version: &str) -> ToolIdentity {
    ToolIdentity {
        tool: ToolKind::Jq,
        path: version.into(),
        version: version.to_owned(),
        executable: ArtifactIdentity {
            path: version.to_owned(),
            bytes: 1,
            sha256: version.to_owned(),
        },
        build_features: Vec::new(),
        runtime_libraries: Vec::new(),
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
    assert!(
        report.cases[0]
            .reference_peak_rss_ratios
            .contains_key("jq-json")
    );
    let objective = report.cases[0]
        .soft_performance_objective
        .as_ref()
        .expect("tq/jq objective");
    assert_eq!(objective.wall_time_ratio, Some(1.0));
    assert_eq!(objective.peak_rss_ratio, Some(1.0));
    assert_eq!(
        objective.wall_time,
        tq_test_support::benchmark::SoftObjectiveStatus::Met
    );
    assert_eq!(
        objective.peak_rss,
        tq_test_support::benchmark::SoftObjectiveStatus::Met
    );
    assert!(
        report
            .render_human()
            .contains("soft jq target: time Met, rss Met")
    );
    let value = serde_json::to_value(&report.cases[0]).expect("row JSON");
    assert!(value.get("composite_score").is_none());
}

#[test]
fn issue_6_rows_report_the_soft_jq_objective() {
    for case_id in [
        "benchmark.recurse-bounded",
        "benchmark.walk-structural",
        "benchmark.label-early-break",
    ] {
        let mut report = campaign("machine-a", "digest-a", 100, 1024);
        report.cases[0].case_id = case_id.to_owned();
        let mut jq = report.cases[0].clone();
        jq.adapter_id = "jq-json".to_owned();
        report.cases.push(jq);
        populate_reference_ratios(&mut report.cases, &["jq-json"]);
        let objective = report.cases[0]
            .soft_performance_objective
            .as_ref()
            .unwrap_or_else(|| panic!("missing objective for {case_id}"));
        assert_eq!(objective.wall_time, SoftObjectiveStatus::Met);
        assert_eq!(objective.peak_rss, SoftObjectiveStatus::Met);
    }
}

#[test]
fn native_format_objectives_use_the_matching_reference_and_require_rss() {
    for (format, reference) in [
        ("json-seq", "jq-json-seq"),
        ("csv", "yq-csv"),
        ("tsv", "yq-tsv"),
    ] {
        let mut report = campaign("machine-a", "digest-a", 100, 1024);
        report.cases[0].case_id = format!("benchmark.native-{format}");
        report.cases[0].adapter_id = format!("tq-{format}");
        let mut peer = report.cases[0].clone();
        peer.adapter_id = reference.to_owned();
        report.cases.push(peer);
        populate_reference_ratios(&mut report.cases, &[reference]);
        let objective = report.cases[0]
            .soft_performance_objective
            .as_ref()
            .expect("native objective");
        assert_eq!(objective.wall_time, SoftObjectiveStatus::Met);
        assert_eq!(objective.peak_rss, SoftObjectiveStatus::Met);

        report.cases[1].summary.as_mut().unwrap().peak_rss_bytes = None;
        report.cases[0].reference_peak_rss_ratios.clear();
        populate_reference_ratios(&mut report.cases, &[reference]);
        let objective = report.cases[0].soft_performance_objective.as_ref().unwrap();
        assert_eq!(objective.peak_rss, SoftObjectiveStatus::NotComparable);
    }
}

#[test]
fn soft_jq_objective_rejects_incorrect_or_policy_mismatched_rows() {
    let mut report = campaign("machine-a", "digest-a", 100, 1024);
    let mut jq = report.cases[0].clone();
    jq.adapter_id = "jq-json".to_owned();
    jq.outcome = BenchmarkOutcome::Incorrect;
    report.cases.push(jq);
    populate_reference_ratios(&mut report.cases, &["jq-json"]);
    let objective = report.cases[0]
        .soft_performance_objective
        .as_ref()
        .expect("issue 5 objective");
    assert_eq!(objective.wall_time, SoftObjectiveStatus::NotComparable);
    assert_eq!(objective.peak_rss, SoftObjectiveStatus::NotComparable);
    assert!(report.cases[0].reference_ratios.is_empty());

    report.cases[1].outcome = BenchmarkOutcome::Timed;
    report.cases[1].limits.output_bytes += 1;
    populate_reference_ratios(&mut report.cases, &["jq-json"]);
    assert!(report.cases[0].reference_ratios.is_empty());
}

#[test]
fn new_reports_reject_missing_or_zero_rss_provenance() {
    let mut report = campaign("machine-a", "digest-a", 100, 1024);
    report.cases[0].samples[0].rss_provenance = None;
    let error = report
        .validate_authoritative_rss()
        .expect_err("legacy-style samples must not validate as new measurements");
    assert!(error.contains("RSS provenance"));

    for sample in &mut report.cases[0].samples {
        sample.rss_provenance = Some(RssProvenance::GnuTimeV);
    }
    assert!(report.validate_authoritative_rss().is_ok());

    report.cases[0].samples[0].peak_rss_bytes = Some(0);
    let error = report
        .validate_authoritative_rss()
        .expect_err("zero RSS must not validate");
    assert!(error.contains("positive authoritative RSS"));
}

#[test]
fn rss_limited_timed_rows_require_valid_separate_instrumented_evidence() {
    let baseline = limited_campaign(1000, 1000);
    assert!(baseline.validate_authoritative_rss().is_ok());

    let mut missing = baseline.clone();
    missing.cases[0].instrumented_samples.clear();
    let error = missing
        .validate_authoritative_rss()
        .expect_err("timed RSS-limited rows need enforcement repetitions");
    assert!(
        error.contains("instrumented RSS-limit repetitions"),
        "{error}"
    );
    let gate = evaluate_regression(&baseline, &missing, regression_thresholds());
    assert!(!gate.evaluated);
    assert!(gate.failures.is_empty());
    assert!(
        gate.unavailable
            .iter()
            .any(|reason| reason.contains("instrumented"))
    );

    let mut invalid = baseline.clone();
    invalid.cases[0].instrumented_samples[1].measurement_protocol = Some(native_protocol());
    let error = invalid
        .validate_authoritative_rss()
        .expect_err("instrumented repetitions need a sampler protocol");
    assert!(error.contains("instrumented"), "{error}");
    let gate = evaluate_regression(&baseline, &invalid, regression_thresholds());
    assert!(!gate.evaluated);
    assert!(gate.failures.is_empty());
    assert!(
        gate.unavailable
            .iter()
            .any(|reason| reason.contains("instrumented"))
    );

    let mut mixed = baseline.clone();
    for sample in &mut mixed.cases[0].samples {
        sample.measurement_protocol = Some(instrumented_protocol());
    }
    mixed.cases[0].summary = summarize_samples(&mixed.cases[0].samples, 100, 1);
    let error = mixed
        .validate_authoritative_rss()
        .expect_err("timing samples cannot carry the sampler contract");
    assert!(
        error.contains("mixes instrumented and timing samples"),
        "{error}"
    );
}

#[test]
fn instrumented_contracts_remain_separate_and_must_match_across_reports() {
    let baseline = limited_campaign(1000, 1000);
    let mut candidate = baseline.clone();
    let mut changed = instrumented_protocol();
    changed.rss_poll_interval_micros = Some(50_000);
    for sample in &mut candidate.cases[0].instrumented_samples {
        sample.measurement_protocol = Some(changed.clone());
    }
    assert!(candidate.validate_authoritative_rss().is_ok());
    let comparison = compare_reports(&baseline, &candidate);
    assert!(!comparison.comparable);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("measurement protocol"))
    );
    let gate = evaluate_regression(&baseline, &candidate, regression_thresholds());
    assert!(!gate.evaluated);
    assert!(gate.failures.is_empty());
    assert!(
        gate.unavailable
            .iter()
            .any(|reason| reason.contains("measurement"))
    );
}

fn regression_thresholds() -> RegressionThresholds {
    RegressionThresholds {
        wall_time_percent: 50.0,
        peak_rss_percent: 50.0,
        minimum_samples: 3,
    }
}

fn limited_campaign(wall: u128, rss: u64) -> BenchmarkCampaignReport {
    let mut report = campaign("machine-a", "digest-a", wall, rss);
    report.cases[0].limits.rss_bytes = Some(128 * 1024 * 1024);
    report.cases[0].instrumented_samples = report.cases[0]
        .samples
        .iter()
        .cloned()
        .map(|mut sample| {
            sample.measurement_protocol = Some(instrumented_protocol());
            sample
        })
        .collect();
    report
}

fn instrumented_protocol() -> MeasurementProtocol {
    MeasurementProtocol {
        timing_method: "direct-spawn-to-exit-observation-with-rss-sampler".to_owned(),
        input_delivery: "prepared-seekable-stdin-file".to_owned(),
        rss_scope: "process-group-lifetime-including-pre-exec-and-waited-descendants-and-threads"
            .to_owned(),
        exit_poll_interval_micros: 100,
        rss_poll_interval_micros: Some(25_000),
        validated_accuracy_micros: Some(1_000),
        worker: Some(worker_identity()),
        isolation_evidence: Some(isolation_evidence()),
    }
}
