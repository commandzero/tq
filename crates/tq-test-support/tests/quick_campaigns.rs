//! Quick campaign regressions run apart from the broader script-fixture process load.

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

fn quick_dispatcher_fixture() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    copy_script(root, "campaign-run.sh");
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_tq-quick"), bin.join("tq-quick")).unwrap();
    executable(
        &bin.join("cargo"),
        r#"#!/bin/sh
set -eu
if [ "$1" = build ]; then
    case " $* " in *" tq-cli "*) sleep "${TEST_COMPILE_DELAY:-2}" ;; esac
    printf '{}\n'
    exit 0
fi
binary=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --bin) binary=$2; shift 2 ;;
        --) shift; exec "$TEST_BIN/$binary" "$@" ;;
        *) shift ;;
    esac
done
exit 2
"#,
    );
    executable(
        &bin.join("tq"),
        r#"#!/bin/sh
set -eu
if [ -n "${TEST_PARSER_DELAY:-}" ]; then
    : > "$TEST_ROOT/artifact-discovery-started"
    sleep "$TEST_PARSER_DELAY"
fi
while [ "$#" -gt 0 ]; do
    if [ "$1" = --arg ] && [ "$2" = name ]; then
        printf '%s/%s\n' "$TEST_BIN" "$3"
        exit 0
    fi
    shift
done
exit 2
"#,
    );
    executable(
        &bin.join("tq-corpus"),
        "#!/bin/sh\n: > \"$TEST_ROOT/preparation-started\"\nexec sleep 10\n",
    );
    executable(
        &bin.join("tq-bench"),
        "#!/bin/sh\nif [ \"$1\" = --preflight-only ]; then exit 0; fi\n: > \"$TEST_ROOT/measurement-started\"\n",
    );
    directory
}

#[test]
fn quick_deadline_includes_preparation_but_excludes_compilation() {
    let directory = quick_dispatcher_fixture();
    let root = directory.path();
    let bin = root.join("bin");
    let start = std::time::Instant::now();
    let output = Command::new("/bin/sh")
        .arg(root.join("scripts/campaign-run.sh"))
        .args(["benchmark", "natural-corpus", "quick"])
        .current_dir(root)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("CARGO", bin.join("cargo"))
        .env("TQ_HOST_TQ", bin.join("tq"))
        .env("TEST_ROOT", root)
        .env("TEST_BIN", &bin)
        .env("TQ_BENCHMARK_ARCHIVE_ROOT", root.join("archive"))
        // Compilation exceeds the work budget; leave room for rounded shell setup time.
        .env("TEST_COMPILE_DELAY", "4")
        .env("CAMPAIGN_BUDGET_SECONDS", "3")
        .env_remove("TQ_BIN")
        .env_remove("TQ_BENCH_WORKER")
        .env_remove("TQ_CAMPAIGN_COMPILED")
        .env_remove("TQ_BENCH_MANIFESTS")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(124), "{output:?}");
    assert!(start.elapsed() < std::time::Duration::from_secs(10));
    assert!(root.join("preparation-started").exists());
    assert!(!root.join("measurement-started").exists());
}

#[test]
fn quick_dispatcher_bounds_slow_artifact_discovery_after_cargo() {
    let directory = quick_dispatcher_fixture();
    let root = directory.path();
    let bin = root.join("bin");
    let started = std::time::Instant::now();
    let output = Command::new("/bin/sh")
        .arg(root.join("scripts/campaign-run.sh"))
        .args(["benchmark", "smoke", "quick"])
        .current_dir(root)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("CARGO", bin.join("cargo"))
        .env("TQ_HOST_TQ", bin.join("tq"))
        .env("TEST_ROOT", root)
        .env("TEST_BIN", &bin)
        .env("TEST_COMPILE_DELAY", "0")
        .env("TEST_PARSER_DELAY", "10")
        .env("TQ_BENCHMARK_ARCHIVE_ROOT", root.join("archive"))
        .env("CAMPAIGN_BUDGET_SECONDS", "3")
        .env_remove("TQ_BIN")
        .env_remove("TQ_CAMPAIGN_COMPILED")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(124), "{output:?}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(6),
        "elapsed {:?}; {output:?}",
        started.elapsed()
    );
    assert!(root.join("artifact-discovery-started").exists());
    assert!(!root.join("measurement-started").exists());
}

#[test]
fn selected_sort_quick_wrapper_uses_session_path_without_implicit_long_budgets() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../benchmarks/cases/parallel-selected-json.sh");
    let tq = root.join("tq");
    let input = root.join("input.json");
    let archive = root.join("archive");
    executable(&tq, "#!/bin/sh\nexit 0\n");
    fs::write(&input, "[]").unwrap();
    fs::create_dir(&archive).unwrap();
    let archived = archive.join("report.json");
    fs::write(&archived, "historical extended evidence").unwrap();
    let output = Command::new("/bin/bash")
        .arg(script)
        .args([&input, &tq])
        .env("SAMPLING", "quick")
        .env("TQ_BENCH", env!("CARGO_BIN_EXE_tq-bench"))
        .env("TQ_JQ", &tq)
        .env("TQ_BENCHMARK_ARCHIVE_ROOT", &archive)
        .env("TMPDIR", root)
        .env_remove("CAMPAIGN_BUDGET_SECONDS")
        .env_remove("CASE_BUDGET_SECONDS")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let sessions: Vec<_> = fs::read_dir(root)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("tq-parallel-selected-quick.")
        })
        .collect();
    assert_eq!(sessions.len(), 1);
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(sessions[0].join("report.json")).unwrap()).unwrap();
    assert_eq!(report["execution"]["sampling"], "quick");
    assert_eq!(report["execution"]["complete"], false);
    assert_eq!(
        fs::read_to_string(archived).unwrap(),
        "historical extended evidence"
    );
}

#[test]
fn smoke_sampling_override_runs_quick_policy_and_keeps_archive_untouched() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    copy_script(root, "campaign-run.sh");
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    let tool = bin.join("tool");
    executable(&tool, "#!/bin/sh\nprintf 'null\\n'\n");
    let archive = root.join("archive");
    fs::create_dir_all(archive.join(".work")).unwrap();
    let archived = archive.join(".work/smoke-extended.json");
    fs::write(&archived, "historical").unwrap();
    let output = Command::new("/bin/sh")
        .arg(root.join("scripts/campaign-run.sh"))
        .args(["benchmark", "smoke", "extended"])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .env("TQ_CAMPAIGN_COMPILED", "1")
        .env("TQ_BENCH_BIN", env!("CARGO_BIN_EXE_tq-bench"))
        .env("TQ_BENCH_WORKER", env!("CARGO_BIN_EXE_tq-bench-worker"))
        .env("TQ_BIN", &tool)
        .env("TQ_JQ", &tool)
        .env("TQ_YQ", &tool)
        .env("SAMPLING", "quick")
        .env("TMPDIR", root)
        .env("TQ_BENCHMARK_ARCHIVE_ROOT", &archive)
        .output()
        .unwrap();
    let sessions: Vec<_> = fs::read_dir(root)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("tq-benchmark-smoke-extended.")
        })
        .collect();
    assert_eq!(sessions.len(), 1, "{output:?}");
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(sessions[0].join("report.json")).unwrap()).unwrap();
    assert_eq!(report["execution"]["sampling"], "quick");
    assert_eq!(report["profile"], "extended");
    let rows = report["cases"].as_array().unwrap();
    for row in rows {
        assert_eq!(row["warmups"], 0);
        assert_eq!(row["requested_samples"], 1);
        if row["outcome"] == "timed" {
            assert_eq!(row["samples"].as_array().unwrap().len(), 1);
        }
    }
    assert_eq!(fs::read_to_string(archived).unwrap(), "historical");
}
