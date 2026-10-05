//! Opt-in Windows native accounting controls, independent of the worker implementation.
//!
//! Run on native Windows (release builds are preferred for retained evidence):
//! `cargo test --release -p tq-test-support --test benchmark_windows_native_validation -- --ignored --nocapture`
//! Set `TQ_NATIVE_VALIDATION_OUT` to durable storage to retain the JSON records.
//! These controls use a separate safe CreateProcess/GetProcessTimes/PSAPI path,
//! not the benchmark collector or a sampled metrics fallback. Both paths still
//! rely on the same Windows accounting authority; this is implementation-level
//! independence, not an independent operating-system measurement tool.
//!
//! The output is deliberately NOT a calibration summary. Coordinator allocation,
//! prepared stdin, high/low request isolation, and sampler controls remain absent.

#![cfg(windows)]

use std::{
    env,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde::Serialize;
use sha2::{Digest as _, Sha256};
use tq_test_support::benchmark::{
    BenchmarkInvocation, MeasuredOutcome, MeasuredStatus, RssProvenance, collector_source_sha256,
    measure_process_worker_uninstrumented,
};
use winsafe::{self as w, co};

const PROBE: &str = env!("CARGO_BIN_EXE_tq-bench-probe");
const REPETITIONS: usize = 20;
const TIMEOUT_MILLIS: u32 = 5_000;

struct Case {
    name: &'static str,
    args: &'static [&'static str],
    touched_bytes: u64,
    duration_micros: Option<u64>,
    busy: bool,
}

const CASES: &[Case] = &[
    Case {
        name: "noop",
        args: &["noop"],
        touched_bytes: 0,
        duration_micros: None,
        busy: false,
    },
    Case {
        name: "burst-4m",
        args: &["allocate-burst", "4194304"],
        touched_bytes: 4 * 1024 * 1024,
        duration_micros: None,
        busy: false,
    },
    Case {
        name: "burst-16m",
        args: &["allocate-burst", "16777216"],
        touched_bytes: 16 * 1024 * 1024,
        duration_micros: None,
        busy: false,
    },
    Case {
        name: "threads-4m-x2",
        args: &["allocate-threads", "4194304", "2"],
        touched_bytes: 8 * 1024 * 1024,
        duration_micros: None,
        busy: false,
    },
    Case {
        name: "threads-2m-x4",
        args: &["allocate-threads", "2097152", "4"],
        touched_bytes: 8 * 1024 * 1024,
        duration_micros: None,
        busy: false,
    },
    Case {
        name: "known-duration-20ms",
        args: &["known-duration", "20"],
        touched_bytes: 0,
        duration_micros: Some(20_000),
        busy: false,
    },
    Case {
        name: "known-duration-100ms",
        args: &["known-duration", "100"],
        touched_bytes: 0,
        duration_micros: Some(100_000),
        busy: false,
    },
    Case {
        name: "known-duration-250ms",
        args: &["known-duration", "250"],
        touched_bytes: 0,
        duration_micros: Some(250_000),
        busy: false,
    },
    Case {
        name: "busy-200ms",
        args: &["busy-duration", "200"],
        touched_bytes: 0,
        duration_micros: Some(200_000),
        busy: true,
    },
];

#[derive(Serialize)]
struct Control {
    process_id: u32,
    exit_code: u32,
    wrapper_wall_micros: u128,
    process_lifetime_micros: u64,
    user_cpu_micros: u64,
    system_cpu_micros: u64,
    user_cpu_100ns_ticks: u64,
    system_cpu_100ns_ticks: u64,
    peak_working_set_bytes: u64,
}

#[derive(Serialize)]
struct Record {
    case: &'static str,
    repetition: usize,
    native: MeasuredOutcome,
    independent_control: Control,
    rss_tolerance_bytes: u64,
    cpu_tolerance_micros: u64,
    native_duration_error_micros: Option<u128>,
}

#[derive(Serialize)]
struct Evidence {
    protocol: &'static str,
    calibration_status: &'static str,
    missing_calibration_controls: &'static [&'static str],
    control_method: &'static str,
    host_os: &'static str,
    host_arch: &'static str,
    build_profile: &'static str,
    page_size_bytes: u64,
    repetitions: usize,
    probe_sha256: String,
    collector_source_sha256: String,
    records: Vec<Record>,
    validation_failures: Vec<String>,
}

// Fixed internal arguments contain only mode names and decimal integers. Supplying
// application_name separately avoids executable-path parsing ambiguity. Refuse
// non-Unicode paths rather than silently hashing/launching a lossy replacement.
fn independent_control(args: &[&str]) -> io::Result<Control> {
    let executable = Path::new(PROBE)
        .to_str()
        .ok_or_else(|| io::Error::other("control executable path is not Unicode"))?;
    let command_line = format!("\"{executable}\" {}", args.join(" "));
    let started = Instant::now();
    let process = w::CreateProcess(
        Some(executable),
        Some(&command_line),
        None,
        None,
        false,
        co::CREATE::NO_WINDOW,
        &[],
        None,
        &mut w::STARTUPINFO::default(),
    )
    .map_err(io::Error::other)?;
    let wait = process.hProcess.WaitForSingleObject(Some(TIMEOUT_MILLIS));
    if !matches!(wait, Ok(co::WAIT::OBJECT_0)) {
        // CloseHandle alone does not terminate a child. Complete bounded cleanup
        // before reporting a wait failure; never turn it into an accepted sample.
        process
            .hProcess
            .TerminateProcess(1)
            .map_err(io::Error::other)?;
        let cleanup = process
            .hProcess
            .WaitForSingleObject(Some(TIMEOUT_MILLIS))
            .map_err(io::Error::other)?;
        if cleanup != co::WAIT::OBJECT_0 {
            return Err(io::Error::other("independent control cleanup timed out"));
        }
        return Err(io::Error::other(format!(
            "independent control wait failed: {wait:?}"
        )));
    }
    let wrapper_wall_micros = started.elapsed().as_micros();
    let (created, exited, kernel, user) = process
        .hProcess
        .GetProcessTimes()
        .map_err(io::Error::other)?;
    let memory = process
        .hProcess
        .GetProcessMemoryInfo()
        .map_err(io::Error::other)?;
    let lifetime = u64::from(exited)
        .checked_sub(u64::from(created))
        .ok_or_else(|| io::Error::other("control exit time precedes creation"))?;
    Ok(Control {
        process_id: process.dwProcessId,
        exit_code: process
            .hProcess
            .GetExitCodeProcess()
            .map_err(io::Error::other)?,
        wrapper_wall_micros,
        process_lifetime_micros: lifetime / 10,
        user_cpu_micros: u64::from(user) / 10,
        system_cpu_micros: u64::from(kernel) / 10,
        user_cpu_100ns_ticks: u64::from(user),
        system_cpu_100ns_ticks: u64::from(kernel),
        peak_working_set_bytes: u64::try_from(memory.PeakWorkingSetSize)
            .map_err(io::Error::other)?,
    })
}

fn invocation(case: &Case) -> BenchmarkInvocation {
    BenchmarkInvocation {
        cancellation: None,
        executable: PathBuf::from(PROBE),
        args: case.args.iter().map(|arg| (*arg).to_owned()).collect(),
        stdin: Vec::new(),
        current_dir: None,
        timeout: Duration::from_millis(u64::from(TIMEOUT_MILLIS)),
        output_limit: 1024 * 1024,
        rss_limit: None,
        retain_output: false,
    }
}

fn median(mut values: Vec<u64>) -> Option<u64> {
    values.sort_unstable();
    values.get(values.len() / 2).copied()
}

fn cpu_outside_tolerance(native: Option<u128>, control: u64, tolerance: u64) -> bool {
    native.is_none_or(|cpu| cpu.abs_diff(u128::from(control)) > u128::from(tolerance))
}

#[test]
fn discrete_cpu_differences_remain_failures_without_tolerance_relaxation() {
    assert!(cpu_outside_tolerance(None, 0, 20_000));
    assert!(!cpu_outside_tolerance(Some(0), 0, 20_000));
    assert!(!cpu_outside_tolerance(Some(20_000), 0, 20_000));
    assert!(cpu_outside_tolerance(Some(20_001), 0, 20_000));
    for difference in [31_250, 46_875, 62_500] {
        assert!(cpu_outside_tolerance(Some(difference), 0, 20_000));
        assert!(cpu_outside_tolerance(
            Some(0),
            u64::try_from(difference).unwrap(),
            20_000
        ));
    }
}

fn check_record(case: &Case, record: &Record, failures: &mut Vec<String>) {
    let native = &record.native;
    let control = &record.independent_control;
    let mut fail = |message: &str| {
        failures.push(format!(
            "{} repetition {}: {message}",
            case.name, record.repetition
        ));
    };
    if native.status != MeasuredStatus::Exited
        || native.exit_code != Some(0)
        || control.exit_code != 0
    {
        fail("probe did not exit successfully");
    }
    if native.rss_provenance != RssProvenance::WindowsPeakWorkingSet {
        fail("native RSS is not Windows peak working set");
    }
    if native.peak_rss_bytes.is_none_or(|rss| {
        rss == 0 || rss.abs_diff(control.peak_working_set_bytes) > record.rss_tolerance_bytes
    }) || control.peak_working_set_bytes == 0
    {
        fail("peak working set absent, zero, or outside comparison tolerance");
    }
    for (metric, native_cpu, control_cpu, control_ticks) in [
        (
            "user",
            native.user_cpu_micros,
            control.user_cpu_micros,
            control.user_cpu_100ns_ticks,
        ),
        (
            "system",
            native.system_cpu_micros,
            control.system_cpu_micros,
            control.system_cpu_100ns_ticks,
        ),
    ] {
        if cpu_outside_tolerance(native_cpu, control_cpu, record.cpu_tolerance_micros) {
            fail(&format!(
                "CPU accounting absent or outside comparison tolerance: {metric}; native={native_cpu:?} us, separate control PID={} value={control_cpu} us ({control_ticks} raw 100ns ticks), absolute difference={:?} us, tolerance={} us; these are distinct executions, not two queries of one process",
                control.process_id,
                native_cpu.map(|cpu| cpu.abs_diff(u128::from(control_cpu))),
                record.cpu_tolerance_micros,
            ));
        }
    }
    if case.busy
        && (control.user_cpu_micros + control.system_cpu_micros == 0
            || native.user_cpu_micros.unwrap_or(0) + native.system_cpu_micros.unwrap_or(0) == 0)
    {
        fail("busy control has no accounted CPU time");
    }
    if let Some(duration) = case.duration_micros
        && (native.wall_time_micros < u128::from(duration)
            || control.process_lifetime_micros < duration)
    {
        fail("known-duration probe finished before its requested duration");
    }
    if native
        .measurement_protocol
        .worker
        .as_ref()
        .is_none_or(|worker| worker.launch_protocol != "tq-bench-worker-windows-v1")
        || native
            .measurement_protocol
            .validated_accuracy_micros
            .is_some()
        || native.measurement_protocol.isolation_evidence.is_some()
    {
        fail("raw Windows worker identity missing or unexpectedly calibrated");
    }
}

#[test]
#[ignore = "requires native Windows process accounting; retains incomplete calibration controls"]
#[allow(
    clippy::too_many_lines,
    reason = "collect, check, and retain one explicit control run"
)]
fn windows_native_accounting_controls_write_incomplete_evidence() {
    let page_size = u64::from(w::GetSystemInfo().dwPageSize);
    assert!(page_size > 0, "Windows reported zero page size");
    let output_root =
        env::var_os("TQ_NATIVE_VALIDATION_OUT").map_or_else(env::temp_dir, PathBuf::from);
    fs::create_dir_all(&output_root).expect("create evidence root");
    let directory = tempfile::Builder::new()
        .prefix("windows-native-controls-")
        .tempdir_in(output_root)
        .expect("create evidence directory")
        .keep();
    let mut evidence = Evidence {
        protocol: "windows-native-accounting-controls-v1",
        calibration_status: "incomplete",
        missing_calibration_controls: &[
            "coordinator allocations 0/32MiB/128MiB and residual floor",
            "prepared stdin isolation (loader requires at least 1MiB; Unix controls use 2MiB)",
            "high/low request isolation",
            "verified allocation burst of at least 1MiB below 25ms",
            "RSS sampler distortion and separate instrumented timing controls",
            "full 75% allocation-delta checks and before/after noop isolation",
            "release machine-linked calibration summary and observed duration error bound",
            "independently verified worker isolation linked to the same executable digest",
        ],
        control_method: "separate CreateProcess, retained exact handle, GetProcessTimes and PSAPI PeakWorkingSetSize; no Job Object or benchmark collector reuse",
        host_os: env::consts::OS,
        host_arch: env::consts::ARCH,
        build_profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        page_size_bytes: page_size,
        repetitions: REPETITIONS,
        probe_sha256: Sha256::digest(fs::read(PROBE).expect("hash probe"))
            .iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                write!(hex, "{byte:02x}").expect("write probe digest");
                hex
            }),
        collector_source_sha256: collector_source_sha256(),
        records: Vec::new(),
        validation_failures: Vec::new(),
    };
    for case in CASES {
        for repetition in 0..REPETITIONS {
            // Alternate order to avoid systematically assigning warm-cache runs
            // to one implementation. Errors are retained and fail the test.
            let pair = if repetition % 2 == 0 {
                independent_control(case.args).and_then(|control| {
                    measure_process_worker_uninstrumented(&invocation(case))
                        .map(|native| (native, control))
                        .map_err(io::Error::other)
                })
            } else {
                measure_process_worker_uninstrumented(&invocation(case))
                    .map_err(io::Error::other)
                    .and_then(|native| {
                        independent_control(case.args).map(|control| (native, control))
                    })
            };
            let (native, control) = match pair {
                Ok(pair) => pair,
                Err(error) => {
                    evidence
                        .validation_failures
                        .push(format!("{} repetition {repetition}: {error}", case.name));
                    continue;
                }
            };
            let record = Record {
                rss_tolerance_bytes: (8 * page_size).max(control.peak_working_set_bytes / 10),
                // This compares different executions, including their startup
                // costs. FILETIME's 100ns units do not establish CPU accounting
                // resolution. Keep the strict tolerance: discretely different
                // counters remain failures, not approved accounting accuracy.
                cpu_tolerance_micros: 20_000_u64.max(case.duration_micros.unwrap_or(0) / 5),
                native_duration_error_micros: case
                    .duration_micros
                    .map(|duration| native.wall_time_micros.abs_diff(u128::from(duration))),
                case: case.name,
                repetition,
                native,
                independent_control: control,
            };
            check_record(case, &record, &mut evidence.validation_failures);
            evidence.records.push(record);
        }
    }
    let noop_native = median(
        evidence
            .records
            .iter()
            .filter(|r| r.case == "noop")
            .filter_map(|r| r.native.peak_rss_bytes)
            .collect(),
    );
    let noop_control = median(
        evidence
            .records
            .iter()
            .filter(|r| r.case == "noop")
            .map(|r| r.independent_control.peak_working_set_bytes)
            .collect(),
    );
    for case in CASES {
        let records: Vec<_> = evidence
            .records
            .iter()
            .filter(|r| r.case == case.name)
            .collect();
        if records.len() != REPETITIONS {
            evidence
                .validation_failures
                .push(format!("{}: incomplete repetitions", case.name));
        }
        if case.touched_bytes > 0 && !records.is_empty() {
            let native = median(
                records
                    .iter()
                    .filter_map(|r| r.native.peak_rss_bytes)
                    .collect(),
            );
            let control = median(
                records
                    .iter()
                    .map(|r| r.independent_control.peak_working_set_bytes)
                    .collect(),
            );
            if native
                .zip(noop_native)
                .is_none_or(|(peak, noop)| peak.saturating_sub(noop) < case.touched_bytes / 2)
                || control
                    .zip(noop_control)
                    .is_none_or(|(peak, noop)| peak.saturating_sub(noop) < case.touched_bytes / 2)
            {
                evidence.validation_failures.push(format!(
                    "{}: allocation delta missing from native or independent control",
                    case.name
                ));
            }
        }
    }
    let first_worker = evidence
        .records
        .first()
        .and_then(|record| record.native.measurement_protocol.worker.as_ref());
    if first_worker.is_none()
        || evidence.records.iter().any(|record| {
            let worker = record.native.measurement_protocol.worker.as_ref();
            worker != first_worker
                || worker.is_none_or(|worker| {
                    worker.collector_source_sha256 != evidence.collector_source_sha256
                        || worker.executable_sha256.is_empty()
                })
        })
    {
        evidence.validation_failures.push(
            "worker identity missing, changed, or linked to stale collector sources".to_owned(),
        );
    }
    let path = directory.join("controls.json");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&evidence).expect("serialize controls"),
    )
    .expect("retain evidence");
    eprintln!(
        "Windows controls (NOT calibration evidence): {}",
        path.display()
    );
    assert!(
        evidence.validation_failures.is_empty(),
        "{}",
        evidence.validation_failures.join("\n")
    );
}
