//! Repository campaign scripts preserve prerequisite and publication boundaries.

#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

fn executable(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn copy_script(root: &Path, name: &str) {
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scripts")
            .join(name),
        root.join("scripts").join(name),
    )
    .unwrap();
}

fn campaign_stub(bin: &Path) {
    executable(
        &bin.join("cargo"),
        "#!/bin/sh\nset -eu\nif [ \"$1\" = build ]; then\n    target_name=tq\n    for argument in \"$@\"; do\n        if [ \"$argument\" = tq-bench-worker ]; then\n            target_name=tq-bench-worker\n        fi\n    done\n    printf '{\"reason\":\"compiler-artifact\",\"target\":{\"name\":\"%s\"},\"executable\":\"%s/%s/release/%s\"}\\n' \\\n        \"$target_name\" \"$TEST_TARGET_DIR\" \"$TEST_TARGET_TRIPLE\" \"$target_name\"\n    exit 0\nfi\nif [ \"$1\" = run ] && [ \"$6\" = tq-cli ]; then\n    printf 'PARSER_RUN\\n' >> \"$TEST_COMMAND_LOG\"\n    exit 69\nfi\nprintf 'TQ_BIN=%s\\n' \"${TQ_BIN:-}\" >> \"$TEST_COMMAND_LOG\"\nprintf 'TQ_BENCH_WORKER=%s\\n' \"${TQ_BENCH_WORKER:-}\" >> \"$TEST_COMMAND_LOG\"\nprintf '%s\\n' \"$@\" >> \"$TEST_COMMAND_LOG\"\n",
    );
    executable(
        &bin.join("tq"),
        "#!/bin/sh\nset -eu\ninput=\nfor argument in \"$@\"; do\n    input=$argument\ndone\nif [ \"${TEST_BAD_ARTIFACT:-}\" = missing ]; then\n    exit 0\nfi\nif [ \"${TEST_BAD_ARTIFACT:-}\" = ambiguous ]; then\n    printf '%s/%s/release/tq\\n' \"$TEST_TARGET_DIR\" \"$TEST_TARGET_TRIPLE\"\n    printf '%s/%s/release/tq-other\\n' \"$TEST_TARGET_DIR\" \"$TEST_TARGET_TRIPLE\"\n    exit 0\nfi\nif [ \"${TEST_BAD_ARTIFACT:-}\" = malformed ]; then\n    exit 2\nfi\nif [ -f \"$input\" ] && grep -q tq-bench-worker \"$input\"; then\n    printf '%s/%s/release/tq-bench-worker\\n' \"$TEST_TARGET_DIR\" \"$TEST_TARGET_TRIPLE\"\nelse\n    printf '%s/%s/release/tq\\n' \"$TEST_TARGET_DIR\" \"$TEST_TARGET_TRIPLE\"\nfi\n",
    );
}

#[test]
fn preflight_checks_the_exact_okf_version_without_ripgrep() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    copy_script(root, "preflight.sh");
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    executable(&bin.join("cargo"), "#!/bin/sh\nexit 0\n");
    executable(
        &bin.join("openspec"),
        "#!/bin/sh\n[ \"$1\" != --version ] || echo 1.11.0\nexit 0\n",
    );
    executable(
        &bin.join("okf"),
        "#!/bin/sh\n[ \"$1\" != --version ] || echo \"$TEST_OKF_VERSION\"\nexit 0\n",
    );
    executable(
        &bin.join("rg"),
        "#!/bin/sh\necho called > \"$TEST_RG_MARKER\"\nexit 99\n",
    );
    for (version, expected_success) in [
        ("okf 0.2.7", true),
        ("okf 0.2.7 build", true),
        ("okf 0.2.70 build", false),
    ] {
        let output = Command::new("/bin/sh")
            .arg(root.join("scripts/preflight.sh"))
            .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
            .env("CARGO", bin.join("cargo"))
            .env("OPENSPEC", bin.join("openspec"))
            .env("OKF", bin.join("okf"))
            .env("TEST_OKF_VERSION", version)
            .env("TEST_RG_MARKER", root.join("rg-called"))
            .output()
            .unwrap();
        assert_eq!(output.status.success(), expected_success, "{output:?}");
        assert!(!root.join("rg-called").exists());
    }
}

#[test]
fn stack_overflow_campaign_uses_emitted_artifacts_and_keeps_uncalibrated_pages() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    copy_script(root, "run-campaign.sh");
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    campaign_stub(&bin);
    let archive = root.join("archive");
    let log = root.join("commands");
    let target = root.join("custom target");
    let output = Command::new("/bin/sh")
        .arg(root.join("scripts/run-campaign.sh"))
        .args(["benchmark", "stack-overflow"])
        .current_dir(root)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("CARGO", bin.join("cargo"))
        .env("CARGO_TARGET_DIR", &target)
        .env("TEST_TARGET_TRIPLE", "aarch64-unknown-linux-gnu")
        .env("TEST_TARGET_DIR", &target)
        .env("TQ_BENCHMARK_ARCHIVE_ROOT", &archive)
        .env("TEST_COMMAND_LOG", &log)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let arguments = fs::read_to_string(log).unwrap();
    assert!(arguments.contains(&format!(
        "TQ_BIN={}\n",
        target.join("aarch64-unknown-linux-gnu/release/tq").display()
    )));
    assert!(arguments.contains(&format!(
        "TQ_BENCH_WORKER={}\n",
        target
            .join("aarch64-unknown-linux-gnu/release/tq-bench-worker")
            .display()
    )));
    assert!(arguments.contains(&format!(
        "--report-dir\n{}\n",
        archive.join(".work/stack-overflow-pages").display()
    )));
    assert!(!arguments.contains("docs/tests/stack-overflow"));
    assert!(!arguments.contains("PARSER_RUN\n"));
}

#[test]
fn campaign_preserves_explicit_binary_overrides() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    copy_script(root, "run-campaign.sh");
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    campaign_stub(&bin);
    let archive = root.join("archive");
    let log = root.join("commands");
    let target = root.join("custom target");
    let tq = root.join("provided tq");
    let worker = root.join("provided worker");
    let output = Command::new("/bin/sh")
        .arg(root.join("scripts/run-campaign.sh"))
        .args(["benchmark", "stack-overflow"])
        .current_dir(root)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("CARGO", bin.join("cargo"))
        .env("CARGO_TARGET_DIR", &target)
        .env("TEST_TARGET_DIR", &target)
        .env("TEST_TARGET_TRIPLE", "aarch64-unknown-linux-gnu")
        .env("TQ_BENCHMARK_ARCHIVE_ROOT", &archive)
        .env("TEST_COMMAND_LOG", &log)
        .env("TQ_BIN", &tq)
        .env("TQ_BENCH_WORKER", &worker)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let arguments = fs::read_to_string(log).unwrap();
    assert!(arguments.contains(&format!("TQ_BIN={}\n", tq.display())));
    assert!(arguments.contains(&format!("TQ_BENCH_WORKER={}\n", worker.display())));
    assert!(!arguments.contains("PARSER_RUN\n"));
}

#[test]
fn benchmark_smoke_forwards_timing_calibration() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    copy_script(root, "run-campaign.sh");
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    campaign_stub(&bin);
    let archive = root.join("archive");
    let log = root.join("commands");
    let target = root.join("custom target");
    let calibration = root.join("calibration.json");
    let output = Command::new("/bin/sh")
        .arg(root.join("scripts/run-campaign.sh"))
        .args(["benchmark", "smoke"])
        .current_dir(root)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("CARGO", bin.join("cargo"))
        .env("CARGO_TARGET_DIR", &target)
        .env("TEST_TARGET_DIR", &target)
        .env("TEST_TARGET_TRIPLE", "x86_64-unknown-linux-gnu")
        .env("TQ_BENCHMARK_ARCHIVE_ROOT", &archive)
        .env("TEST_COMMAND_LOG", &log)
        .env("TQ_TIMING_CALIBRATION", &calibration)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let arguments = fs::read_to_string(log).unwrap();
    assert!(arguments.contains(&format!(
        "--timing-calibration\n{}\n",
        calibration.display()
    )));
}

#[test]
fn benchmark_standard_requires_timing_calibration() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    copy_script(root, "run-campaign.sh");
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    campaign_stub(&bin);
    let output = Command::new("/bin/sh")
        .arg(root.join("scripts/run-campaign.sh"))
        .args(["benchmark", "standard"])
        .current_dir(root)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("CARGO", bin.join("cargo"))
        .env("TQ_BENCHMARK_ARCHIVE_ROOT", root.join("archive"))
        .env("TEST_COMMAND_LOG", root.join("commands"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&output.stderr).contains("TQ_TIMING_CALIBRATION"));
}

#[test]
fn malformed_cargo_artifacts_fail_closed_and_clean_capture_files() {
    for bad_artifact in ["missing", "ambiguous", "malformed"] {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        copy_script(root, "run-campaign.sh");
        let bin = root.join("bin");
        let temporary = root.join("temporary");
        fs::create_dir(&bin).unwrap();
        fs::create_dir(&temporary).unwrap();
        campaign_stub(&bin);
        let target = root.join("custom target");
        let log = root.join("commands");
        let output = Command::new("/bin/sh")
            .arg(root.join("scripts/run-campaign.sh"))
            .args(["benchmark", "stack-overflow"])
            .current_dir(root)
            .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
            .env("CARGO", bin.join("cargo"))
            .env("CARGO_TARGET_DIR", &target)
            .env("TEST_TARGET_DIR", &target)
            .env("TEST_TARGET_TRIPLE", "aarch64-unknown-linux-gnu")
            .env("TEST_COMMAND_LOG", &log)
            .env("TEST_BAD_ARTIFACT", bad_artifact)
            .env("TMPDIR", &temporary)
            .output()
            .unwrap();
        if bad_artifact == "malformed" {
            assert_eq!(output.status.code(), Some(2), "{bad_artifact}: {output:?}");
        } else {
            assert_eq!(output.status.code(), Some(69), "{bad_artifact}: {output:?}");
            assert!(String::from_utf8_lossy(&output.stderr).contains("one executable"));
        }
        assert!(!log.exists(), "{bad_artifact}: campaign launched");
        assert_eq!(
            fs::read_dir(&temporary).unwrap().count(),
            0,
            "{bad_artifact}"
        );
    }
}

#[test]
fn campaign_manifest_parsing_uses_host_tq_instead_of_jq() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = fs::read_to_string(root.join("scripts/run-campaign.sh")).unwrap();
    assert!(!script.contains("$(jq "));
    assert!(script.contains("host_tq"));
    assert!(script.contains("TQ_HOST_TQ"));
}

#[test]
fn release_workflow_provisions_a_host_parser_before_the_manual_gate() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let workflow: yaml_serde::Value = yaml_serde::from_str(
        &fs::read_to_string(repository.join(".github/workflows/regex-date-platform.yml")).unwrap(),
    )
    .unwrap();
    let steps = workflow["jobs"]["release-host-contract"]["steps"]
        .as_sequence()
        .unwrap();
    let provision = steps
        .iter()
        .position(|step| step["name"].as_str() == Some("Provision the host tq parser"))
        .expect("host parser provisioning step");
    let gate = steps
        .iter()
        .position(|step| step["name"].as_str() == Some("Run the pinned manual release gate"))
        .unwrap();
    assert!(provision < gate);
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    executable(
        &bin.join("rustc"),
        "#!/bin/sh\nprintf 'host: test-native-host\\n'\n",
    );
    executable(
        &bin.join("cargo"),
        r#"#!/bin/sh
set -eu
printf '%s\n' "$@" > "$TEST_COMMAND_LOG"
while [ "$#" -gt 0 ]; do
    if [ "$1" = --root ]; then
        shift
        install_root=$1
    fi
    shift
done
mkdir -p "$install_root/bin"
printf '#!/bin/sh\nexit 0\n' > "$install_root/bin/tq"
chmod +x "$install_root/bin/tq"
"#,
    );
    let runner_temp = root.join("runner temp");
    let environment = root.join("github-env");
    let log = root.join("commands");
    let output = Command::new("/bin/bash")
        .args(["-e", "-o", "pipefail", "-c"])
        .arg(steps[provision]["run"].as_str().unwrap())
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("RUNNER_TEMP", &runner_temp)
        .env("GITHUB_ENV", &environment)
        .env("TEST_COMMAND_LOG", &log)
        .env("CARGO_BUILD_TARGET", "not-the-host")
        .env("CARGO_TARGET_DIR", root.join("custom target"))
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let arguments = fs::read_to_string(log).unwrap();
    assert!(arguments.starts_with("install\n"));
    assert!(arguments.contains("--locked\n"));
    assert!(arguments.contains("--path\ncrates/tq-cli\n"));
    assert!(arguments.contains("--target\ntest-native-host\n"));
    let parser = runner_temp.join("tq-host/bin/tq");
    assert_eq!(
        fs::read_to_string(environment).unwrap(),
        format!("TQ_HOST_TQ={}\n", parser.display())
    );
    assert!(
        Command::new(parser)
            .arg("--version")
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn release_workflow_has_checked_in_official_reference_defaults() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let workflow = fs::read_to_string(root.join(".github/workflows/regex-date-platform.yml"))
        .expect("release-host workflow");
    for (target, url, sha256) in [
        (
            "x86_64-linux",
            "https://github.com/jqlang/jq/releases/download/jq-1.8.1/jq-linux-amd64",
            "020468de7539ce70ef1bceaf7cde2e8c4f2ca6c3afb84642aabc5c97d9fc2a0d",
        ),
        (
            "aarch64-macos",
            "https://github.com/jqlang/jq/releases/download/jq-1.8.1/jq-macos-arm64",
            "a9fe3ea2f86dfc72f6728417521ec9067b343277152b114f4e98d8cb0e263603",
        ),
    ] {
        assert!(workflow.contains(&format!("reference_target: {target}")));
        assert!(workflow.contains(&format!("reference_jq_url: {url}")));
        assert!(workflow.contains(&format!("reference_jq_sha256: {sha256}")));
    }
    assert!(workflow.contains("matrix.reference_jq_url"));
    assert!(workflow.contains("matrix.reference_jq_sha256"));
}

#[test]
fn release_workflow_retains_manual_evidence_even_when_the_gate_fails() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let workflow: yaml_serde::Value = yaml_serde::from_str(
        &fs::read_to_string(root.join(".github/workflows/regex-date-platform.yml")).unwrap(),
    )
    .unwrap();
    let steps = workflow["jobs"]["release-host-contract"]["steps"]
        .as_sequence()
        .unwrap();
    let gate = steps
        .iter()
        .position(|step| step["name"].as_str() == Some("Run the pinned manual release gate"))
        .unwrap();
    let upload = steps
        .iter()
        .position(|step| {
            step["uses"]
                .as_str()
                .is_some_and(|action| action.starts_with("actions/upload-artifact@"))
        })
        .expect("retain the native manual gate's evidence");
    assert!(upload > gate);
    assert_eq!(steps[upload]["if"].as_str(), Some("always()"));
    assert_eq!(
        steps[upload]["with"]["path"].as_str(),
        Some("target/compatibility/")
    );
    let name = steps[upload]["with"]["name"].as_str().unwrap();
    assert!(name.contains("matrix.reference_target"));
    assert!(name.contains("github.run_attempt"));
    assert_eq!(
        steps[upload]["with"]["if-no-files-found"].as_str(),
        Some("error")
    );
    assert_ne!(steps[gate]["continue-on-error"].as_bool(), Some(true));
}
