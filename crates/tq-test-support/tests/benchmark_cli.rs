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
        "profile": "rapid",
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

#[test]
fn render_only_accepts_multiple_host_reports_without_overwriting_host_sections() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let linux_report = directory.path().join("linux.json");
    let macos_report = directory.path().join("macos.json");
    let markdown = directory.path().join("markdown");
    fs::create_dir(&markdown).expect("create markdown directory");
    write_report_for_host(&linux_report, "passed", "linux", "x86_64");
    write_report_for_host(&macos_report, "passed", "macos", "aarch64");

    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "--render-only",
            linux_report.to_str().expect("linux report path"),
            "--render-only",
            macos_report.to_str().expect("macos report path"),
            "--markdown-dir",
            markdown.to_str().expect("markdown path"),
        ])
        .output()
        .expect("run multi-host render-only benchmark command");

    assert!(
        output.status.success(),
        "multi-host render-only failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let index = fs::read_to_string(markdown.join("index.md")).expect("rendered index");
    assert_eq!(index.matches("### Host: linux / x86_64 / rapid").count(), 1);
    assert_eq!(
        index.matches("### Host: macos / aarch64 / rapid").count(),
        1
    );
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
            "--profile",
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
fn selected_case_without_profile_dataset_aborts_without_publishing_an_empty_report() {
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
            "rapid",
            "--max-samples",
            "1",
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
    assert!(
        !report.exists(),
        "empty selected matrix must not publish a report"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("benchmark.startup"), "stderr: {stderr}");
    assert!(stderr.contains("no prepared datasets"), "stderr: {stderr}");
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
            "--profile",
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
            "--profile",
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
fn failed_run_writes_report_returns_failure_and_reports_row_progress() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let jq = fake_tool(directory.path(), "jq", "null");
    let yq = fake_tool(directory.path(), "yq", "null");
    let tq = fake_tool(directory.path(), "tq", "1");
    let report = directory.path().join("run.json");

    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--profile",
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "workload=benchmark.startup dataset=startup adapter=tq-json outcome=Incorrect"
        )
    );
    assert!(
        stderr.contains("workload=benchmark.startup dataset=startup adapter=yq-json outcome=Timed")
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
            "--profile",
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
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'fake-{name} 1.0\\n'\n  exit 0\nfi\nprintf '{output}\\n'\n"
    );
    fs::write(&path, script).expect("write fake executable");
    let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake executable");
    path
}
