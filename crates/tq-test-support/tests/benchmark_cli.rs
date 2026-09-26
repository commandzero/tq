#![allow(missing_docs)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::{Value, json};
use tq_test_support::{
    benchmark::load_benchmark_catalog,
    corpus::{GeneratedArtifacts, SnapshotState, build_source_snapshot, write_snapshot_manifest},
};

fn write_report(path: &Path, final_status: &str) {
    write_report_for_host(path, final_status, "test", "test");
}

fn write_report_for_host(path: &Path, final_status: &str, os: &str, architecture: &str) {
    let report = json!({
        "schema_version": 1,
        "campaign_id": "test-campaign",
        "suite": "natural-corpus",
        "profile": "standard",
        "environment": {
            "collected_at": "2026-01-01T00:00:00Z",
            "os": os,
            "kernel": null,
            "architecture": architecture,
            "logical_cpus": 1,
            "physical_cpus": null,
            "cpu_model": null,
            "memory_bytes": null,
            "filesystem": null,
            "power_settings": null,
            "compiler_profile": "test",
            "machine_identity": "test"
        },
        "corpus": [],
        "tools": [],
        "cases": [],
        "comparability": {"comparable": true, "reasons": []},
        "regression_gate": {
            "evaluated": false,
            "thresholds": null,
            "failures": []
        },
        "final_status": final_status
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&report).expect("serialize report"),
    )
    .expect("write report");
}

#[test]
fn render_only_succeeds_for_historical_failed_reports() {
    let directory = tempfile::tempdir().expect("temporary directory");
    for status in ["observed-failures", "regression"] {
        let report = directory.path().join(format!("{status}.json"));
        let markdown = directory.path().join(status);
        write_report(&report, status);
        fs::create_dir(&markdown).expect("create markdown directory");

        let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
            .args([
                "--render-only",
                report.to_str().expect("report path"),
                "--markdown-dir",
                markdown.to_str().expect("markdown path"),
            ])
            .output()
            .expect("run render-only benchmark command");

        assert!(
            output.status.success(),
            "render-only failed for {status}: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let rendered = String::from_utf8_lossy(&output.stdout);
        let status_name = match status {
            "observed-failures" => "ObservedFailures",
            "regression" => "Regression",
            _ => unreachable!("test status should be known"),
        };
        assert!(rendered.contains(status_name));
    }
}

#[cfg(unix)]
#[test]
fn unknown_selected_case_aborts_without_publishing_an_empty_report() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let jq = fake_tool(directory.path(), "jq", "null");
    let yq = fake_tool(directory.path(), "yq", "null");
    let tq = fake_tool(directory.path(), "tq", "null");
    let report = directory.path().join("unknown.json");

    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--suite",
            "smoke",
            "--max-samples",
            "1",
            "--case",
            "benchmark.does-not-exist",
            "--output",
            report.to_str().expect("report path"),
        ])
        .env("TQ_JQ", jq)
        .env("TQ_YQ", yq)
        .env("TQ_BIN", tq)
        .output()
        .expect("run benchmark command");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        !report.exists(),
        "invalid selection must not publish a report"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("benchmark.does-not-exist"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("not present in the catalog"),
        "stderr: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn selected_case_without_suite_dataset_retains_only_incomplete_diagnostics() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let cache = directory.path().join("cache");
    let source = directory.path().join("source.json");
    fs::write(&source, br#"{"type":"FeatureCollection","features":[]}"#).expect("write source");

    let mut input = test_snapshot_input(source.clone());
    input.source_id = "usgs-all-month".to_owned();
    let mut manifest = build_source_snapshot(input).expect("build snapshot manifest");
    let source_file = cache.join("prepared/source.json");
    let download_file = cache.join("downloads/source.json");
    let yaml_file = cache.join("generated/source.yaml");
    let toon_file = cache.join("generated/source.toon");
    for path in [&source_file, &download_file, &yaml_file, &toon_file] {
        fs::create_dir_all(path.parent().expect("artifact parent"))
            .expect("create artifact parent");
    }
    fs::copy(&source, &source_file).expect("copy source artifact");
    fs::copy(&source, &download_file).expect("copy download artifact");
    fs::write(&yaml_file, b"null\n").expect("write YAML artifact");
    fs::write(&toon_file, b"null\n").expect("write TOON artifact");
    manifest.state = SnapshotState::CrossFormatValidated;
    manifest.artifacts.download = artifact_identity("downloads/source.json", &download_file);
    manifest.artifacts.source_json = artifact_identity("prepared/source.json", &source_file);
    manifest.artifacts.generated = Some(GeneratedArtifacts {
        yaml: artifact_identity("generated/source.yaml", &yaml_file),
        toon: artifact_identity("generated/source.toon", &toon_file),
    });
    manifest.validation.yaml_equivalent = Some(true);
    manifest.validation.toon_equivalent = Some(true);
    let manifest_path = directory.path().join("manifest.json");
    write_snapshot_manifest(&manifest_path, &manifest).expect("write snapshot manifest");

    let jq = fake_tool(directory.path(), "jq", "null");
    let yq = fake_tool(directory.path(), "yq", "null");
    let tq = fake_tool(directory.path(), "tq", "null");
    let report = directory.path().join("no-dataset.json");
    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--profile",
            "quick",
            "--case",
            "benchmark.startup",
            "--output",
            report.to_str().expect("report path"),
            "--cache-root",
            cache.to_str().expect("cache path"),
        ])
        .env("TQ_BENCH_MANIFESTS", &manifest_path)
        .env("TQ_JQ", jq)
        .env("TQ_YQ", yq)
        .env("TQ_BIN", tq)
        .output()
        .expect("run benchmark command");

    assert_eq!(output.status.code(), Some(2));
    let checkpoint: Value =
        serde_json::from_slice(&fs::read(report).expect("read early diagnostic checkpoint"))
            .expect("checkpoint JSON");
    assert_eq!(checkpoint["final_status"], "incomplete");
    assert_eq!(checkpoint["execution"]["complete"], false);
}

#[cfg(unix)]
#[test]
fn structural_regression_failure_returns_regression_when_gate_is_not_evaluated() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let jq = fake_tool(directory.path(), "jq", "null");
    let yq = fake_tool(directory.path(), "yq", "null");
    let tq = fake_tool(directory.path(), "tq", "null");
    let seed = directory.path().join("seed.json");
    let baseline = directory.path().join("baseline.json");
    let candidate = directory.path().join("candidate.json");

    let seed_output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--suite",
            "smoke",
            "--max-samples",
            "1",
            "--case",
            "benchmark.startup",
            "--output",
            seed.to_str().expect("seed path"),
        ])
        .env("TQ_JQ", &jq)
        .env("TQ_YQ", &yq)
        .env("TQ_BIN", &tq)
        .output()
        .expect("run seed benchmark command");
    assert_eq!(seed_output.status.code(), Some(0));
    let mut baseline_value: Value =
        serde_json::from_slice(&fs::read(&seed).expect("read seed report"))
            .expect("parse seed report");
    baseline_value["cases"]
        .as_array_mut()
        .expect("seed rows")
        .pop();
    fs::write(
        &baseline,
        serde_json::to_vec_pretty(&baseline_value).expect("serialize baseline report"),
    )
    .expect("write baseline report");

    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--suite",
            "smoke",
            "--max-samples",
            "1",
            "--case",
            "benchmark.startup",
            "--baseline",
            baseline.to_str().expect("baseline path"),
            "--output",
            candidate.to_str().expect("candidate path"),
        ])
        .env("TQ_JQ", jq)
        .env("TQ_YQ", yq)
        .env("TQ_BIN", tq)
        .output()
        .expect("run benchmark command");

    assert_eq!(output.status.code(), Some(1));
    let report: Value =
        serde_json::from_slice(&fs::read(candidate).expect("read candidate report"))
            .expect("parse candidate report");
    assert_eq!(report["final_status"], "regression");
    assert_eq!(report["regression_gate"]["evaluated"], false);
    assert_eq!(
        report["regression_gate"]["failures"],
        json!(["benchmark row identities differ"])
    );
}

#[cfg(unix)]
fn admitted_snapshot(directory: &Path, source_id: &str) -> PathBuf {
    let cache = directory.join("cache");
    let source = directory.join(format!("{source_id}.json"));
    fs::write(&source, br#"{"type":"FeatureCollection","features":[]}"#).expect("write source");
    let mut input = test_snapshot_input(source.clone());
    source_id.clone_into(&mut input.source_id);
    let mut manifest = build_source_snapshot(input).expect("build snapshot manifest");
    let relative = |area: &str, ext: &str| format!("{area}/{source_id}.{ext}");
    let source_name = relative("prepared", "json");
    let download_name = relative("downloads", "json");
    let yaml_name = relative("generated", "yaml");
    let toon_name = relative("generated", "toon");
    let source_file = cache.join(&source_name);
    let download_file = cache.join(&download_name);
    let yaml_file = cache.join(&yaml_name);
    let toon_file = cache.join(&toon_name);
    for path in [&source_file, &download_file, &yaml_file, &toon_file] {
        fs::create_dir_all(path.parent().expect("artifact parent")).expect("create parent");
    }
    fs::copy(&source, &source_file).expect("copy source artifact");
    fs::copy(&source, &download_file).expect("copy download artifact");
    fs::write(&yaml_file, b"null\n").expect("write YAML artifact");
    fs::write(&toon_file, b"null\n").expect("write TOON artifact");
    manifest.state = SnapshotState::CrossFormatValidated;
    manifest.artifacts.download = artifact_identity(&download_name, &download_file);
    manifest.artifacts.source_json = artifact_identity(&source_name, &source_file);
    manifest.artifacts.generated = Some(GeneratedArtifacts {
        yaml: artifact_identity(&yaml_name, &yaml_file),
        toon: artifact_identity(&toon_name, &toon_file),
    });
    manifest.validation.yaml_equivalent = Some(true);
    manifest.validation.toon_equivalent = Some(true);
    let path = directory.join(format!("{source_id}-manifest.json"));
    write_snapshot_manifest(&path, &manifest).expect("write snapshot manifest");
    path
}

#[cfg(unix)]
#[test]
fn quick_uses_standard_sources_and_one_measured_invocation_per_row() {
    use std::collections::BTreeSet;

    let directory = tempfile::tempdir().expect("temporary directory");
    let hour = admitted_snapshot(directory.path(), "usgs-all-hour");
    let month = admitted_snapshot(directory.path(), "usgs-all-month");
    let manifests = std::env::join_paths([hour, month]).expect("join manifest paths");
    let jq = fake_tool(directory.path(), "jq", "null");
    let yq = fake_tool(directory.path(), "yq", "null");
    let tq = fake_tool(directory.path(), "tq", "null");
    let run = |profile: Option<&str>, log: &str| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_tq-bench"));
        command
            .arg("run")
            .args([
                "--case",
                "benchmark.array-construction",
                "--adapter",
                "tq-json",
            ])
            .arg("--cache-root")
            .arg(directory.path().join("cache"))
            .env("TQ_BENCH_MANIFESTS", &manifests)
            .env("TQ_JQ", &jq)
            .env("TQ_YQ", &yq)
            .env("TQ_BIN", &tq)
            .env("BENCH_CALL_LOG", directory.path().join(log))
            .env("TMPDIR", directory.path());
        if let Some(profile) = profile {
            command.args(["--profile", profile]);
        }
        let result = command.output().expect("run benchmark campaign");
        assert_eq!(
            result.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        result
    };
    run(None, "standard-calls");
    let quick = run(Some("quick"), "quick-calls");
    let reports = session_reports(directory.path());
    let (standard_path, standard) = &reports["standard"];
    let (quick_path, quick_report) = &reports["quick"];
    assert_eq!(
        standard_path.parent().expect("standard directory").parent(),
        Some(directory.path())
    );
    assert_eq!(
        quick_path.parent().expect("quick directory").parent(),
        Some(directory.path())
    );
    assert!(String::from_utf8_lossy(&quick.stderr).contains(&quick_path.display().to_string()));
    assert_eq!(standard["suite"], "natural-corpus");
    assert_eq!(quick_report["suite"], "natural-corpus");
    assert_eq!(
        report_row_identities(standard),
        report_row_identities(quick_report)
    );
    assert_eq!(
        report_row_identities(quick_report)
            .iter()
            .map(|(_, source, _)| source.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["usgs-all-hour", "usgs-all-month"])
    );
    for row in quick_report["cases"].as_array().expect("quick rows") {
        assert_eq!(row["outcome"], "timed");
        assert_eq!(row["warmups"], 0);
        assert_eq!(row["requested_samples"], 1);
        assert_eq!(row["samples"].as_array().expect("samples").len(), 1);
        assert!(
            row["instrumented_samples"]
                .as_array()
                .expect("instrumented")
                .is_empty()
        );
    }
    for row in standard["cases"].as_array().expect("standard rows") {
        assert_eq!(row["warmups"], 1);
        assert_eq!(row["requested_samples"], 3);
        assert_eq!(row["samples"].as_array().expect("samples").len(), 3);
    }
    let call_count = |name: &str| {
        fs::read_to_string(directory.path().join(name))
            .expect("invocation log")
            .lines()
            .count()
    };
    assert_eq!(call_count("quick-calls"), 2 * (1 + 2 * (1 + 1)));
    assert_eq!(call_count("standard-calls"), 2 * (1 + 2 * (1 + 1 + 3)));
}

#[cfg(unix)]
#[test]
fn quick_coordinator_without_completion_owner_keeps_checkpoint_incomplete() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let hour = admitted_snapshot(directory.path(), "usgs-all-hour");
    let month = admitted_snapshot(directory.path(), "usgs-all-month");
    let manifests = std::env::join_paths([hour, month]).expect("join manifest paths");
    let jq = fake_tool(directory.path(), "jq", "null");
    let yq = fake_tool(directory.path(), "yq", "null");
    let tq = fake_tool(directory.path(), "tq", "null");
    let output = directory.path().join("checkpoint.json");
    let result = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--profile",
            "quick",
            "--case",
            "benchmark.array-construction",
            "--adapter",
            "tq-json",
        ])
        .arg("--cache-root")
        .arg(directory.path().join("cache"))
        .arg("--output")
        .arg(&output)
        .env("TQ_BENCH_MANIFESTS", &manifests)
        .env("TQ_JQ", &jq)
        .env("TQ_YQ", &yq)
        .env("TQ_BIN", &tq)
        .env("TQ_QUICK_SUPERVISED", "1")
        .env_remove("TQ_QUICK_COMPLETION_STAGE")
        .output()
        .expect("run quick coordinator without completion owner");
    assert_eq!(result.status.code(), Some(2));
    let report: Value =
        serde_json::from_slice(&fs::read(output).expect("checkpoint")).expect("JSON");
    assert_eq!(report["execution"]["complete"], false);
    assert_eq!(report["final_status"], "incomplete");
    let rows = report["cases"].as_array().expect("retained observations");
    for source in ["usgs-all-hour", "usgs-all-month"] {
        let row = rows
            .iter()
            .find(|row| row["source_id"] == source && row["adapter_id"] == "tq-json")
            .expect("completed candidate observation");
        assert_eq!(row["outcome"], "timed");
        assert_eq!(row["samples"].as_array().unwrap().len(), 1);
    }
}

#[cfg(unix)]
fn session_reports(directory: &Path) -> std::collections::BTreeMap<String, (PathBuf, Value)> {
    fs::read_dir(directory)
        .expect("temporary reports")
        .map(|entry| entry.expect("report directory").path().join("report.json"))
        .filter(|path| path.is_file())
        .map(|path| {
            let report: Value = serde_json::from_slice(&fs::read(&path).expect("read report"))
                .expect("report JSON");
            (
                report["profile"].as_str().expect("profile").to_owned(),
                (path, report),
            )
        })
        .collect()
}

#[cfg(unix)]
fn report_row_identities(report: &Value) -> std::collections::BTreeSet<(String, String, String)> {
    report["cases"]
        .as_array()
        .expect("report rows")
        .iter()
        .map(|row| {
            (
                row["case_id"].as_str().expect("case").to_owned(),
                row["source_id"].as_str().expect("source").to_owned(),
                row["adapter_id"].as_str().expect("adapter").to_owned(),
            )
        })
        .collect()
}

#[test]
fn quick_rejects_publication_and_extra_measurements() {
    for forbidden in [
        vec!["--sampling", "screen"],
        vec!["--max-samples", "1"],
        vec!["--instrument-rss"],
        vec!["--baseline", "baseline.json"],
        vec!["--wall-regression-percent", "10"],
        vec!["--rss-regression-percent", "10"],
        vec!["--minimum-regression-samples", "1"],
        vec!["--markdown-dir", "published"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
            .args(["run", "--preflight-only", "--profile", "quick"])
            .args(&forbidden)
            .output()
            .expect("validate quick options");
        assert_eq!(
            result.status.code(),
            Some(2),
            "accepted forbidden quick option {forbidden:?}"
        );
    }
    for profile in ["rapid", "large", "smoke", "extra-large", "stack-overflow"] {
        let removed_profile = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
            .args(["run", "--preflight-only", "--profile", profile])
            .output()
            .expect("reject removed profile");
        assert_eq!(removed_profile.status.code(), Some(2));
    }
    for suite in [
        "quick",
        "standard",
        "extended",
        "extra-large",
        "stack-overflow",
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
            .args(["run", "--preflight-only", "--suite", suite])
            .output()
            .expect("reject profile used as a suite");
        assert_eq!(
            result.status.code(),
            Some(2),
            "accepted removed suite {suite}"
        );
    }
}

#[cfg(unix)]
#[test]
fn quick_deadline_covers_blocking_tool_discovery_and_keeps_incomplete_checkpoint() {
    use std::{
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };

    let directory = tempfile::tempdir().expect("temp");
    let slow_jq = directory.path().join("slow-jq");
    fs::write(&slow_jq, "#!/bin/sh\nexec sleep 10\n").expect("slow discovery executable");
    fs::set_permissions(&slow_jq, fs::Permissions::from_mode(0o755)).expect("executable");
    let checkpoint = directory.path().join("discovery.json");
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--suite",
            "smoke",
            "--profile",
            "quick",
            "--case",
            "benchmark.startup",
            "--adapter",
            "jq-json",
            "--campaign-budget-seconds",
            "1",
            "--case-budget-seconds",
            "1",
            "--output",
        ])
        .arg(&checkpoint)
        .env("TQ_JQ", &slow_jq)
        .output()
        .expect("run bounded discovery");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "unbounded tool discovery"
    );
    assert!(
        !output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&fs::read(&checkpoint).expect("checkpoint"))
        .expect("valid checkpoint");
    assert_eq!(report["final_status"], "incomplete");
    assert_eq!(report["execution"]["complete"], false);
    assert!(String::from_utf8_lossy(&output.stdout).contains("| Suite | Profile | Status |"));
    assert!(String::from_utf8_lossy(&output.stderr).contains(checkpoint.to_str().unwrap()));
}

#[test]
fn quick_budget_overrides_cannot_extend_the_work_deadline() {
    for (profile_args, budget_args) in [
        (
            vec!["--profile", "quick"],
            vec!["--campaign-budget-seconds", "51"],
        ),
        (
            vec!["--profile", "quick"],
            vec!["--case-budget-seconds", "51"],
        ),
        (
            vec!["--profile", "standard", "--sampling", "quick"],
            vec!["--campaign-budget-seconds", "51"],
        ),
        (
            vec!["--profile", "standard", "--sampling", "quick"],
            vec!["--case-budget-seconds", "51"],
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
            .args(["run", "--preflight-only", "--suite", "smoke"])
            .args(&profile_args)
            .args(&budget_args)
            .output()
            .expect("reject oversized quick budget");
        assert_eq!(
            output.status.code(),
            Some(2),
            "{profile_args:?} {budget_args:?}"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("cannot exceed 50s"));
    }
}

#[cfg(unix)]
#[test]
fn extended_measures_five_samples_in_fast_and_exhaustive_modes() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let manifest = admitted_snapshot(directory.path(), "usgs-all-month");
    let buildings = admitted_snapshot(directory.path(), "microsoft-us-buildings-georgia");
    let jq = fake_tool(directory.path(), "jq", "null");
    let yq = fake_tool(directory.path(), "yq", "null");
    let tq = fake_tool(directory.path(), "tq", "null");
    for mode in ["fast", "exhaustive"] {
        let output_path = directory.path().join(format!("{mode}.json"));
        let result = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
            .args([
                "run",
                "--profile",
                "extended",
                "--mode",
                mode,
                "--case",
                "benchmark.parse-discard",
                "--adapter",
                "tq-json",
            ])
            .arg("--manifest")
            .arg(&manifest)
            .arg("--manifest")
            .arg(&buildings)
            .arg("--cache-root")
            .arg(directory.path().join("cache"))
            .arg("--output")
            .arg(&output_path)
            .env("TQ_JQ", &jq)
            .env("TQ_YQ", &yq)
            .env("TQ_BIN", &tq)
            .output()
            .expect("run extended campaign");
        assert_eq!(
            result.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let report: Value =
            serde_json::from_slice(&fs::read(output_path).expect("read extended report"))
                .expect("report JSON");
        assert_eq!(report["suite"], "natural-corpus");
        assert_eq!(report["profile"], "extended");
        let rows = report["cases"].as_array().expect("measured rows");
        assert_eq!(rows.len(), 2);
        for row in rows {
            assert_eq!(row["source_id"], "usgs-all-month");
            assert_eq!(row["outcome"], "timed");
            assert_eq!(row["warmups"], 1);
            assert_eq!(row["requested_samples"], 5);
            assert_eq!(row["samples"].as_array().expect("samples").len(), 5);
        }
    }
}

#[cfg(unix)]
#[test]
fn large_input_profile_changes_sampling_and_extended_adds_selected_sort() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let manifest = admitted_snapshot(directory.path(), "microsoft-us-buildings-georgia");
    let input = directory.path().join("provided.geojson");
    fs::write(
        &input,
        br#"{"type":"FeatureCollection","features":[{"properties":{"release":1}}]}"#,
    )
    .expect("write provided input");
    let jq = fake_tool(directory.path(), "jq", "null");
    let yq = fake_tool(directory.path(), "yq", "null");
    let tq = fake_tool(directory.path(), "tq", "null");

    for (profile, warmups, samples, expected_rows) in [
        ("quick", 0, 1, 6),
        ("standard", 1, 3, 6),
        ("extended", 1, 5, 8),
    ] {
        for use_manifest in [false, true] {
            let output_path = directory
                .path()
                .join(format!("{profile}-{use_manifest}.json"));
            let mut command = Command::new(env!("CARGO_BIN_EXE_tq-bench"));
            command
                .args(["run", "--suite", "large-input", "--profile", profile])
                .arg(if use_manifest {
                    "--manifest"
                } else {
                    "--input"
                })
                .arg(if use_manifest { &manifest } else { &input })
                .arg("--cache-root")
                .arg(directory.path().join("cache"))
                .arg("--output")
                .arg(&output_path)
                .env("TQ_JQ", &jq)
                .env("TQ_YQ", &yq)
                .env("TQ_BIN", &tq);
            let result = command.output().expect("run large-input suite");
            assert_eq!(
                result.status.code(),
                Some(0),
                "{profile} {use_manifest}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            let report: Value =
                serde_json::from_slice(&fs::read(output_path).expect("read report"))
                    .expect("parse report");
            assert_eq!(report["suite"], "large-input");
            assert_eq!(report["profile"], profile);
            let rows = report["cases"].as_array().expect("measured rows");
            assert_eq!(rows.len(), expected_rows);
            assert_eq!(
                rows.iter()
                    .any(|row| row["case_id"] == "benchmark.large-selected-sort"),
                profile == "extended"
            );
            for row in rows {
                assert_eq!(row["outcome"], "timed");
                assert_eq!(row["warmups"], warmups);
                assert_eq!(row["requested_samples"], samples);
                assert_eq!(row["samples"].as_array().expect("samples").len(), samples);
                assert!(
                    ["jq-json", "tq-json", "yq-json"]
                        .contains(&row["adapter_id"].as_str().unwrap())
                );
                assert_eq!(
                    row["source_id"],
                    if use_manifest {
                        "microsoft-us-buildings-georgia"
                    } else {
                        "large-input-selected-json"
                    }
                );
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn large_input_case_and_exhaustive_selection_override_profile_coverage() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let input = directory.path().join("provided.geojson");
    fs::write(
        &input,
        br#"{"type":"FeatureCollection","features":[{"properties":{"release":1}}]}"#,
    )
    .expect("write provided input");
    let jq = fake_tool(directory.path(), "jq", "null");
    let yq = fake_tool(directory.path(), "yq", "null");
    let tq = fake_tool(directory.path(), "tq", "null");
    for profile in ["quick", "standard"] {
        let output_path = directory.path().join(format!("selected-{profile}.json"));
        let result = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
            .args(["run", "--suite", "large-input", "--profile", profile])
            .arg("--input")
            .arg(&input)
            .args(["--case", "benchmark.large-selected-sort"])
            .arg("--output")
            .arg(&output_path)
            .env("TQ_JQ", &jq)
            .env("TQ_YQ", &yq)
            .env("TQ_BIN", &tq)
            .output()
            .expect("run explicitly selected sort");
        assert_eq!(
            result.status.code(),
            Some(0),
            "{profile}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let report: Value = serde_json::from_slice(&fs::read(output_path).expect("read report"))
            .expect("parse report");
        let rows = report["cases"].as_array().expect("selected sort rows");
        assert_eq!(rows.len(), 2);
        assert!(
            rows.iter()
                .all(|row| row["case_id"] == "benchmark.large-selected-sort")
        );
    }

    let exhaustive_report = directory.path().join("exhaustive.json");
    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--suite",
            "large-input",
            "--profile",
            "quick",
            "--mode",
            "exhaustive",
        ])
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&exhaustive_report)
        .env("TQ_JQ", &jq)
        .env("TQ_YQ", &yq)
        .env("TQ_BIN", &tq)
        .output()
        .expect("run exhaustive large-input suite");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(exhaustive_report).expect("read exhaustive report"))
            .expect("parse report");
    let rows = report["cases"].as_array().expect("exhaustive rows");
    assert!(
        rows.iter()
            .any(|row| row["case_id"] == "benchmark.scalar-extraction")
    );
    assert!(
        rows.iter()
            .any(|row| row["case_id"] == "benchmark.large-selected-sort")
    );
    assert!(
        rows.iter()
            .all(|row| row["source_id"] == "large-input-selected-json")
    );
}

#[test]
fn render_only_cannot_publish_a_quick_session() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let report = directory.path().join("quick.json");
    let markdown = directory.path().join("markdown");
    write_report(&report, "passed");
    let mut value: Value =
        serde_json::from_slice(&fs::read(&report).expect("read report")).expect("report JSON");
    value["profile"] = json!("quick");
    fs::write(
        &report,
        serde_json::to_vec(&value).expect("serialize report"),
    )
    .expect("write report");
    let result = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "--render-only",
            report.to_str().expect("report path"),
            "--markdown-dir",
            markdown.to_str().expect("markdown path"),
        ])
        .output()
        .expect("attempt quick publication");
    assert_eq!(result.status.code(), Some(2));
    assert!(!markdown.join("index.md").exists());
}

#[test]
fn native_delimited_raw_contract_unwraps_reference_scalars() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog =
        load_benchmark_catalog(&root.join("benchmarks/cases")).expect("load benchmark catalog");
    for (case_id, adapter_id) in [
        ("benchmark.native-csv", "yq-csv"),
        ("benchmark.native-tsv", "yq-tsv"),
    ] {
        let case = catalog
            .cases
            .iter()
            .find(|case| case.id == case_id)
            .expect("native case");
        let adapter = case
            .adapters
            .iter()
            .find(|adapter| adapter.id == adapter_id)
            .expect("native reference adapter");
        assert!(
            adapter
                .args
                .iter()
                .any(|argument| argument == "--unwrapScalar"),
            "{case_id}/{adapter_id} must emit scalar bytes compatible with tq --raw-output"
        );
    }
}

fn test_snapshot_input(source_json_file: PathBuf) -> tq_test_support::corpus::SourceSnapshotInput {
    tq_test_support::corpus::SourceSnapshotInput {
        campaign_id: "2026-09-13T00:00:00Z".to_owned(),
        source_id: "usgs-all-month".to_owned(),
        retrieved_at: "2026-09-13T00:00:00Z".to_owned(),
        request: tq_test_support::corpus::RequestIdentity {
            requested_url: "https://example.test/source.json".to_owned(),
            final_url: "https://example.test/source.json".to_owned(),
            status: 200,
            content_type: "application/json".to_owned(),
            etag: None,
            last_modified: None,
        },
        archive: None,
        download: tq_test_support::corpus::ArtifactIdentity {
            path: "downloads/source.json".to_owned(),
            bytes: 0,
            sha256: String::new(),
        },
        source_json_file,
        source_json_path: "prepared/source.json".to_owned(),
        provenance: tq_test_support::corpus::Provenance {
            title: "Test source".to_owned(),
            publisher: "Test publisher".to_owned(),
            landing_page: "https://example.test/source".to_owned(),
            license: tq_test_support::corpus::LicenseIdentity {
                name: "Test license".to_owned(),
                url: "https://example.test/license".to_owned(),
            },
        },
    }
}

fn artifact_identity(path: &str, file: &Path) -> tq_test_support::corpus::ArtifactIdentity {
    use sha2::{Digest as _, Sha256};
    use std::fmt::Write as _;

    let bytes = fs::read(file).expect("read artifact");
    let digest = Sha256::digest(&bytes);
    tq_test_support::corpus::ArtifactIdentity {
        path: path.to_owned(),
        bytes: bytes.len() as u64,
        sha256: digest
            .iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                write!(hex, "{byte:02x}").expect("write digest");
                hex
            }),
    }
}

#[cfg(unix)]
#[test]
fn failed_run_preserves_correctness_outcomes() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let jq = fake_tool(directory.path(), "jq", "null");
    let yq = fake_tool(directory.path(), "yq", "null");
    let tq = fake_tool(directory.path(), "tq", "1");
    let report = directory.path().join("run.json");

    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--suite",
            "smoke",
            "--max-samples",
            "1",
            "--case",
            "benchmark.startup",
            "--output",
            report.to_str().expect("report path"),
        ])
        .env("TQ_JQ", jq)
        .env("TQ_YQ", yq)
        .env("TQ_BIN", tq)
        .output()
        .expect("run benchmark command");

    assert_eq!(output.status.code(), Some(1));
    assert!(report.is_file(), "failed run should preserve its report");
    let report: Value =
        serde_json::from_slice(&fs::read(report).expect("read report")).expect("parse report");
    assert_eq!(report["final_status"], "observed-failures");
    let rows = report["cases"].as_array().expect("report rows");
    assert!(
        rows.iter()
            .any(|row| row["adapter_id"] == "tq-json" && row["outcome"] == "incorrect")
    );
    assert!(
        rows.iter()
            .any(|row| row["adapter_id"] == "yq-json" && row["outcome"] == "timed")
    );
}

#[cfg(unix)]
#[test]
fn missing_campaign_executable_override_aborts_before_writing_a_report() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let report = directory.path().join("preflight.json");

    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--suite",
            "smoke",
            "--output",
            report.to_str().expect("report path"),
        ])
        // Discovery also searches repository-local reference builds, so an
        // empty PATH alone does not make the executable unavailable.
        .env("TQ_JQ", directory.path().join("missing-jq"))
        .output()
        .expect("run benchmark preflight");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        !report.exists(),
        "failed executable discovery must not write a report"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("configured Jq executable is not usable:"),
        "unexpected executable-discovery stderr: {stderr}"
    );
}

#[cfg(unix)]
fn fake_tool(directory: &Path, name: &str, output: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = directory.join(name);
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'fake-{name} 1.0\\n'\n  exit 0\nfi\nif [ -n \"$BENCH_CALL_LOG\" ] && [ \"$1\" != \"--build-configuration\" ] && [ \"$1\" != \"--help\" ]; then printf 'call\\n' >> \"$BENCH_CALL_LOG\"; fi\nprintf '{output}\\n'\n"
    );
    fs::write(&path, script).expect("write fake executable");
    let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake executable");
    path
}

#[cfg(unix)]
fn deadline_command(directory: &Path, campaign_seconds: &str, case_seconds: &str) -> Command {
    use std::os::unix::fs::PermissionsExt;
    let jq = fake_tool(directory, "jq", "null");
    let yq = fake_tool(directory, "yq", "null");
    let tq = directory.join("slow-tq");
    fs::write(&tq, "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'slow-tq 1.0\\n'; exit 0; fi\nprintf '%s' \"$$\" > \"$BENCH_CHILD_PID\"\nexec sleep 60\n").expect("write slow child");
    fs::set_permissions(&tq, fs::Permissions::from_mode(0o755)).expect("executable");
    let input = directory.join("input.json");
    fs::write(
        &input,
        br#"{"type":"FeatureCollection","features":[{"properties":{"release":1}}]}"#,
    )
    .expect("input");
    let mut command = Command::new(env!("CARGO_BIN_EXE_tq-bench"));
    command
        .arg("--input")
        .arg(input)
        .args([
            "run",
            "--suite",
            "large-input",
            "--mode",
            "fast",
            "--sampling",
            "screen",
            "--adapter",
            "jq-json",
            "--adapter",
            "tq-json",
            "--campaign-budget-seconds",
            campaign_seconds,
            "--case-budget-seconds",
            case_seconds,
            "--output",
        ])
        .arg(directory.join("checkpoint.json"))
        .env("TQ_JQ", jq)
        .env("TQ_BIN", tq)
        .env("TQ_YQ", yq)
        .env("TQ_BENCH_WORKER", env!("CARGO_BIN_EXE_tq-bench-worker"))
        .env("BENCH_CHILD_PID", directory.join("child.pid"));
    command
}

#[cfg(unix)]
fn assert_cancelled_child_reaped(directory: &Path) {
    use nix::{errno::Errno, sys::signal::kill, unistd::Pid};
    let pid: i32 = fs::read_to_string(directory.join("child.pid"))
        .expect("child ran")
        .parse()
        .expect("child PID");
    assert_eq!(kill(Pid::from_raw(pid), None), Err(Errno::ESRCH));
}

#[cfg(unix)]
#[test]
fn campaign_deadline_retains_completed_rows_and_reaps_child() {
    let directory = tempfile::tempdir().expect("temp");
    let started = std::time::Instant::now();
    let output = deadline_command(directory.path(), "5", "30")
        .args(["--case", "benchmark.large-selected-sort"])
        .output()
        .expect("run bounded campaign");
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(15));
    let report: Value = serde_json::from_slice(
        &fs::read(directory.path().join("checkpoint.json")).expect("durable report"),
    )
    .expect("complete JSON");
    assert_eq!(report["final_status"], "incomplete");
    assert_eq!(report["execution"]["complete"], false);
    let rows = report["cases"].as_array().expect("rows");
    assert!(
        rows.iter()
            .any(|row| row["adapter_id"] == "jq-json" && row["outcome"] == "timed")
    );
    let planned = usize::try_from(report["execution"]["planned_rows"].as_u64().expect("plan"))
        .expect("plan fits usize");
    assert!(rows.len() < planned);
    assert_cancelled_child_reaped(directory.path());
}

#[cfg(unix)]
#[test]
fn case_deadline_does_not_cancel_later_cases() {
    let directory = tempfile::tempdir().expect("temp");
    let output = deadline_command(directory.path(), "30", "2")
        .args([
            "--case",
            "benchmark.dead-sort-length",
            "--case",
            "benchmark.large-selected-sort",
        ])
        .output()
        .expect("run bounded cases");
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(
        &fs::read(directory.path().join("checkpoint.json")).expect("report"),
    )
    .expect("JSON");
    let rows = report["cases"].as_array().expect("rows");
    for case in [
        "benchmark.dead-sort-length",
        "benchmark.large-selected-sort",
    ] {
        assert!(
            rows.iter().any(|row| row["case_id"] == case
                && row["adapter_id"] == "jq-json"
                && row["outcome"] == "timed"),
            "{report:#}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert_eq!(
        report["execution"]["interruptions"]
            .as_array()
            .expect("interruptions")
            .len(),
        2
    );
    assert_cancelled_child_reaped(directory.path());
}

#[cfg(unix)]
#[test]
fn signal_cancellation_keeps_the_last_completed_row() {
    use nix::{
        sys::signal::{Signal, kill},
        unistd::Pid,
    };
    use std::{
        process::Stdio,
        time::{Duration, Instant},
    };
    let directory = tempfile::tempdir().expect("temp");
    let child = deadline_command(directory.path(), "30", "30")
        .args(["--case", "benchmark.large-selected-sort"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start campaign");
    let deadline = Instant::now() + Duration::from_secs(15);
    while !directory.path().join("child.pid").is_file() {
        assert!(Instant::now() < deadline, "slow adapter did not start");
        std::thread::sleep(Duration::from_millis(10));
    }
    let before: Value = serde_json::from_slice(
        &fs::read(directory.path().join("checkpoint.json")).expect("live checkpoint"),
    )
    .expect("atomic JSON");
    assert!(
        before["cases"]
            .as_array()
            .expect("rows")
            .iter()
            .any(|row| row["adapter_id"] == "jq-json" && row["outcome"] == "timed")
    );
    let pid = i32::try_from(child.id()).expect("valid process ID");
    kill(Pid::from_raw(pid), Signal::SIGTERM).expect("cancel campaign");
    assert_eq!(
        child
            .wait_with_output()
            .expect("reaped coordinator")
            .status
            .code(),
        Some(2)
    );
    let after: Value = serde_json::from_slice(
        &fs::read(directory.path().join("checkpoint.json")).expect("final checkpoint"),
    )
    .expect("JSON");
    assert_eq!(after["final_status"], "incomplete");
    assert_eq!(after["cases"], before["cases"]);
    assert_cancelled_child_reaped(directory.path());
}
