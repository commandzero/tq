//! Explicit native-accounting validation on macOS and Linux.
//!
//! This is an ignored, evidence-producing test rather than a campaign. It
//! runs the same Rust probe through `measure_process` and through the host's
//! `/usr/bin/time`, then keeps the native result, raw time output, and derived
//! summaries under one run directory. The test is intentionally opt-in: it
//! must never make an ordinary workspace test depend on native permissions or
//! on the availability of BSD/GNU time.
//!
//! Run outside a restricted sandbox with native child-accounting permission:
//!
//! ```text
//! TQ_NATIVE_VALIDATION_OUT=/tmp/tq-native-validation \
//!   cargo test -p tq-test-support --test benchmark_native_validation -- \
//!   --ignored --exact native_accounting_validation_writes_retained_evidence --nocapture
//! ```
//!
//! The test selects BSD `time -l` on macOS and GNU `time -v` on Linux. Use an
//! output directory on durable storage when retaining evidence for review.
//! `TQ_NATIVE_VALIDATION_REPETITIONS` may increase the default 20 samples, but
//! values below 20 are rejected.

#![cfg(any(target_os = "macos", target_os = "linux"))]
#![allow(missing_docs)]

use std::{
    collections::BTreeMap,
    env,
    fmt::Write as _,
    fs::{self, File, OpenOptions},
    io::{self, Write as _},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Output, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use sha2::{Digest as _, Sha256};
use tq_test_support::benchmark::{
    BenchmarkInvocation, EnvironmentManifest, MeasuredOutcome, MeasuredStatus, MeasurementProtocol,
    RssProvenance, collect_environment, collector_source_sha256, measure_process,
    measure_process_uninstrumented, measure_process_worker,
};

const PROBE: &str = env!("CARGO_BIN_EXE_tq-bench-probe");
const DEFAULT_REPETITIONS: usize = 20;
const MIN_REPETITIONS: usize = 20;
const MEANINGFUL_BURST_BYTES: u64 = 1 << 20;
const SHORT_BURST_LIMIT_MICROS: u128 = 25_000;
const ALLOCATION_DELTA_NUMERATOR: u64 = 3;
const ALLOCATION_DELTA_DENOMINATOR: u64 = 4;
const ALLOCATION_NOOP_NOISE_NUMERATOR: u64 = 1;
const ALLOCATION_NOOP_NOISE_DENOMINATOR: u64 = 10;
const TIME_CPU_TOLERANCE_MICROS: u128 = 10_000;
const TIMEOUT: Duration = Duration::from_secs(10);
const OUTPUT_LIMIT: u64 = 64 * 1024;
const SAMPLER_RSS_LIMIT_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Clone, Debug)]
struct Case {
    name: String,
    args: Vec<String>,
    touched_bytes: Option<u64>,
    page_rounded_bytes: Option<u64>,
    expected_duration_micros: Option<u128>,
    rss_limit_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
struct Metadata {
    protocol: &'static str,
    host_os: &'static str,
    host_arch: &'static str,
    page_size_bytes: u64,
    repetitions: usize,
    build_profile: &'static str,
    time_program: &'static str,
    time_mode: &'static str,
    rustc: Option<String>,
    time_version: Option<String>,
    time_sha256: Option<String>,
    started_unix_micros: Option<u128>,
    probe_path: String,
    probe_sha256: String,
    runner_executable_path: Option<String>,
    runner_executable_sha256: Option<String>,
    collector_source_sha256: String,
    environment: EnvironmentManifest,
}

#[derive(Debug, Serialize)]
struct TimeObservation {
    mode: &'static str,
    success: bool,
    exit_code: Option<i32>,
    signal: Option<i32>,
    wrapper_wall_micros: u128,
    parsed_wall_micros: Option<u128>,
    parsed_user_cpu_micros: Option<u128>,
    parsed_system_cpu_micros: Option<u128>,
    parsed_peak_rss_bytes: Option<u64>,
    stdout_file: String,
    stderr_file: String,
}

#[derive(Debug, Serialize)]
struct RssComparison {
    native_bytes: u64,
    time_bytes: u64,
    absolute_difference_bytes: u64,
    tolerance_bytes: u64,
    within_page_aware_tolerance: bool,
}

#[derive(Debug, Serialize)]
struct CpuComparison {
    native_micros: u128,
    time_micros: u128,
    absolute_difference_micros: u128,
    tolerance_micros: u128,
    within_tolerance: bool,
}

#[derive(Debug, Serialize)]
struct CpuComparisons {
    user: CpuComparison,
    system: CpuComparison,
}

#[derive(Debug, Serialize)]
struct Record {
    case: String,
    args: Vec<String>,
    repetition: usize,
    touched_bytes: Option<u64>,
    page_rounded_bytes: Option<u64>,
    expected_duration_micros: Option<u128>,
    rss_limit_bytes: Option<u64>,
    native: MeasuredOutcome,
    independent_time: TimeObservation,
    rss_comparison: Option<RssComparison>,
    cpu_comparison: Option<CpuComparisons>,
}

#[derive(Debug, Serialize)]
struct CaseSummary {
    case: String,
    repetitions: usize,
    touched_bytes: Option<u64>,
    page_rounded_bytes: Option<u64>,
    expected_duration_micros: Option<u128>,
    rss_limit_bytes: Option<u64>,
    native_wall_median_micros: u128,
    native_wall_mad_micros: u128,
    native_duration_error_bound_micros: Option<u128>,
    native_duration_excess_median_micros: Option<u128>,
    native_user_cpu_median_micros: Option<u128>,
    native_user_cpu_mad_micros: Option<u128>,
    native_system_cpu_median_micros: Option<u128>,
    native_system_cpu_mad_micros: Option<u128>,
    native_peak_rss_median_bytes: u64,
    native_peak_rss_mad_bytes: u64,
    time_wall_median_micros: Option<u128>,
    time_user_cpu_median_micros: Option<u128>,
    time_system_cpu_median_micros: Option<u128>,
    time_peak_rss_median_bytes: Option<u64>,
    rss_tolerance_bytes: Option<u64>,
    rss_comparisons_within_tolerance: Option<usize>,
    cpu_comparisons_within_tolerance: Option<usize>,
}

#[derive(Debug, Serialize)]
struct NoopIsolation {
    before_median_rss_bytes: u64,
    after_median_rss_bytes: u64,
    absolute_difference_bytes: u64,
    tolerance_bytes: u64,
    within_tolerance: bool,
}

#[derive(Debug, Serialize)]
struct AllocationDeltaCheck {
    case: String,
    native_peak_rss_median_bytes: u64,
    noop_median_rss_bytes: u64,
    observed_delta_bytes: u64,
    page_rounded_bytes: u64,
    minimum_delta_bytes: u64,
    runtime_noise_slack_bytes: u64,
    within_tolerance: bool,
}

#[derive(Debug, Serialize)]
struct ShortBurstCheck {
    bytes_threshold: u64,
    duration_limit_micros: u128,
    observed: bool,
    cases: Vec<String>,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct CalibrationLinkage {
    measurement_protocol: Option<MeasurementProtocol>,
    rss_limit_bytes: Option<u64>,
    machine_identity: String,
    probe_sha256: String,
    runner_build_profile: &'static str,
    collector_source_sha256: String,
    time_mode: &'static str,
    conservative_observed_duration_error_bound_micros: Option<u128>,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct WorkerAllocationControl {
    coordinator_allocation_bytes: u64,
    repetitions: usize,
    native_rss_median_bytes: u64,
    independent_time_rss_median_bytes: Option<u64>,
    rss_comparisons_within_tolerance: usize,
    cpu_comparisons_within_tolerance: usize,
    max_delta_from_zero_bytes: u64,
    tolerance_bytes: u64,
}

#[derive(Debug, Serialize)]
struct WorkerIsolationEvidence {
    collector_source_sha256: String,
    coordinator_controls: Vec<WorkerAllocationControl>,
    residual_floor_bytes: u64,
    prepared_stdin_bytes: u64,
    prepared_stdin_repetitions: usize,
    prepared_stdin_verified: bool,
    high_low_request_repetitions: usize,
    high_low_request_verified: bool,
    independent_time_repetitions: usize,
    independent_time_verified: bool,
}

#[derive(Debug, Serialize)]
struct SamplerDistortion {
    normal_case: String,
    sampler_case: String,
    normal_wall_median_micros: Option<u128>,
    normal_wall_mad_micros: u128,
    sampler_wall_median_micros: Option<u128>,
    wall_median_delta_micros: Option<u128>,
    wall_delta_within_normal_mad: Option<bool>,
    normal_user_cpu_median_micros: Option<u128>,
    normal_user_cpu_mad_micros: Option<u128>,
    sampler_user_cpu_median_micros: Option<u128>,
    user_cpu_median_delta_micros: Option<u128>,
    user_cpu_delta_within_normal_mad: Option<bool>,
    normal_system_cpu_median_micros: Option<u128>,
    normal_system_cpu_mad_micros: Option<u128>,
    sampler_system_cpu_median_micros: Option<u128>,
    system_cpu_median_delta_micros: Option<u128>,
    system_cpu_delta_within_normal_mad: Option<bool>,
    normal_peak_rss_median_bytes: Option<u64>,
    normal_peak_rss_mad_bytes: u64,
    sampler_peak_rss_median_bytes: Option<u64>,
    peak_rss_median_delta_bytes: Option<u64>,
    peak_rss_delta_within_normal_mad: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ErrorRecord {
    case: String,
    args: Vec<String>,
    repetition: usize,
    stage: String,
    error: String,
}

#[derive(Debug, Serialize)]
struct Summary {
    metadata: Metadata,
    evidence_directory: String,
    cases: Vec<CaseSummary>,
    native_measurement_protocol: Option<MeasurementProtocol>,
    native_noop_wall_median_micros: Option<u128>,
    native_noop_wall_mad_micros: Option<u128>,
    noop_isolation: Option<NoopIsolation>,
    allocation_delta_checks: Vec<AllocationDeltaCheck>,
    short_burst_check: ShortBurstCheck,
    sampler_distortion_checks: Vec<SamplerDistortion>,
    worker_isolation: Option<WorkerIsolationEvidence>,
    calibration_linkage: CalibrationLinkage,
    sampler_calibration_linkage: CalibrationLinkage,
    validation_failures: Vec<String>,
    protocol_notes: Vec<&'static str>,
}

#[test]
#[ignore = "explicit native validation; run with --ignored on macOS/Linux"]
fn native_accounting_validation_writes_retained_evidence() {
    let output_directory = run_validation().expect("native validation run");
    eprintln!("native validation evidence: {}", output_directory.display());
}

#[test]
#[ignore = "explicit worker-interface validation; run with --ignored on macOS/Linux"]
fn worker_public_measurement_returns_fresh_worker_identity() {
    let invocation = BenchmarkInvocation {
        cancellation: None,
        executable: PathBuf::from(PROBE),
        args: vec!["noop".to_owned()],
        stdin: Vec::new(),
        current_dir: None,
        timeout: TIMEOUT,
        output_limit: OUTPUT_LIMIT,
        rss_limit: None,
        retain_output: false,
    };
    let outcome = measure_process_worker(&invocation).expect("worker measurement");
    let worker = outcome
        .measurement_protocol
        .worker
        .expect("worker identity in public measurement protocol");
    assert!(!worker.executable_sha256.is_empty());
    assert!(!worker.launch_protocol.is_empty());
    assert_eq!(worker.collector_source_sha256, collector_source_sha256());
    assert!(outcome.peak_rss_bytes.is_some_and(|rss| rss > 0));
}

#[allow(clippy::too_many_lines)]
fn run_validation() -> io::Result<PathBuf> {
    let repetitions = configured_repetitions()?;
    let page_size = native_page_size()?;
    if page_size == 0 {
        return Err(io::Error::other("native page size was zero"));
    }
    let base_directory = env::var_os("TQ_NATIVE_VALIDATION_OUT")
        .map_or_else(|| PathBuf::from("target/native-validation"), PathBuf::from);
    fs::create_dir_all(&base_directory)?;
    let run_directory = base_directory.join(format!(
        "{}-{}",
        unix_micros().unwrap_or(0),
        std::process::id()
    ));
    fs::create_dir(&run_directory)?;

    let probe_path = Path::new(PROBE);
    let runner_executable = env::current_exe().ok();
    let metadata = Metadata {
        protocol: "native-accounting-v1",
        host_os: env::consts::OS,
        host_arch: env::consts::ARCH,
        page_size_bytes: page_size,
        repetitions,
        build_profile: build_profile(),
        time_program: "/usr/bin/time",
        time_mode: time_mode(),
        rustc: command_version("rustc", &["--version"]),
        time_version: command_version("/usr/bin/time", &["--version"]),
        time_sha256: file_sha256(Path::new("/usr/bin/time")).ok(),
        started_unix_micros: unix_micros(),
        probe_path: probe_path.display().to_string(),
        probe_sha256: file_sha256(probe_path)?,
        runner_executable_path: runner_executable
            .as_ref()
            .map(|path| path.display().to_string()),
        runner_executable_sha256: runner_executable
            .as_deref()
            .and_then(|path| file_sha256(path).ok()),
        collector_source_sha256: collector_source_sha256(),
        environment: collect_environment("native-validation"),
    };
    write_json(&run_directory.join("metadata.json"), &metadata)?;
    let mut records_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(run_directory.join("records.jsonl"))?;
    let mut errors_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(run_directory.join("errors.jsonl"))?;

    let cases = validation_cases(page_size)?;
    let mut records = Vec::with_capacity(cases.len() * repetitions);
    for case in cases {
        for repetition in 0..repetitions {
            let native = match run_native(&case.args, case.rss_limit_bytes) {
                Ok(native) => native,
                Err(error) => {
                    return Err(fail_after_recording(
                        &mut errors_file,
                        &case,
                        repetition,
                        "native-measurement",
                        error.to_string(),
                    ));
                }
            };
            if native.status != MeasuredStatus::Exited || native.exit_code != Some(0) {
                return Err(fail_after_recording(
                    &mut errors_file,
                    &case,
                    repetition,
                    "native-status",
                    format!(
                        "probe did not exit successfully: {:?} {:?}",
                        native.status, native.exit_code
                    ),
                ));
            }
            if native.rss_provenance != expected_provenance() {
                return Err(fail_after_recording(
                    &mut errors_file,
                    &case,
                    repetition,
                    "native-provenance",
                    format!(
                        "unexpected RSS provenance {}",
                        native.rss_provenance.label()
                    ),
                ));
            }
            let native_rss = native.peak_rss_bytes.ok_or_else(|| {
                fail_after_recording(
                    &mut errors_file,
                    &case,
                    repetition,
                    "native-rss",
                    "native RSS missing",
                )
            })?;
            if native_rss == 0 {
                return Err(fail_after_recording(
                    &mut errors_file,
                    &case,
                    repetition,
                    "native-rss",
                    "native RSS was zero",
                ));
            }
            let Some(native_user_cpu) = native.user_cpu_micros else {
                return Err(fail_after_recording(
                    &mut errors_file,
                    &case,
                    repetition,
                    "native-user-cpu",
                    "native user CPU was missing",
                ));
            };
            let Some(native_system_cpu) = native.system_cpu_micros else {
                return Err(fail_after_recording(
                    &mut errors_file,
                    &case,
                    repetition,
                    "native-system-cpu",
                    "native system CPU was missing",
                ));
            };

            let (time_output, mut independent_time) = match run_independent_time(&case.args) {
                Ok(value) => value,
                Err(error) => {
                    return Err(fail_after_recording(
                        &mut errors_file,
                        &case,
                        repetition,
                        "independent-time",
                        error.to_string(),
                    ));
                }
            };
            let (stdout_file, stderr_file) =
                match save_time_output(&run_directory, &case.name, repetition, &time_output) {
                    Ok(files) => files,
                    Err(error) => {
                        return Err(fail_after_recording(
                            &mut errors_file,
                            &case,
                            repetition,
                            "independent-time-artifacts",
                            error.to_string(),
                        ));
                    }
                };
            independent_time.stdout_file = stdout_file;
            independent_time.stderr_file = stderr_file;
            if !independent_time.success {
                return Err(fail_after_recording(
                    &mut errors_file,
                    &case,
                    repetition,
                    "independent-time-status",
                    format!("exited unsuccessfully under {}", time_mode()),
                ));
            }
            let Some(time_rss) = independent_time.parsed_peak_rss_bytes else {
                return Err(fail_after_recording(
                    &mut errors_file,
                    &case,
                    repetition,
                    "independent-time-rss",
                    format!(
                        "did not emit parseable {} RSS; raw stderr is {}",
                        time_mode(),
                        independent_time.stderr_file
                    ),
                ));
            };
            let Some(time_user_cpu) = independent_time.parsed_user_cpu_micros else {
                return Err(fail_after_recording(
                    &mut errors_file,
                    &case,
                    repetition,
                    "independent-time-user-cpu",
                    format!(
                        "did not emit parseable {} user CPU; raw stderr is {}",
                        time_mode(),
                        independent_time.stderr_file
                    ),
                ));
            };
            let Some(time_system_cpu) = independent_time.parsed_system_cpu_micros else {
                return Err(fail_after_recording(
                    &mut errors_file,
                    &case,
                    repetition,
                    "independent-time-system-cpu",
                    format!(
                        "did not emit parseable {} system CPU; raw stderr is {}",
                        time_mode(),
                        independent_time.stderr_file
                    ),
                ));
            };
            let comparison = rss_comparison(native_rss, time_rss, page_size);
            let cpu_comparison = CpuComparisons {
                user: cpu_comparison(native_user_cpu, time_user_cpu),
                system: cpu_comparison(native_system_cpu, time_system_cpu),
            };

            let record = Record {
                case: case.name.clone(),
                args: case.args.clone(),
                repetition,
                touched_bytes: case.touched_bytes,
                page_rounded_bytes: case.page_rounded_bytes,
                expected_duration_micros: case.expected_duration_micros,
                rss_limit_bytes: case.rss_limit_bytes,
                native,
                independent_time,
                rss_comparison: Some(comparison),
                cpu_comparison: Some(cpu_comparison),
            };
            serde_json::to_writer(&mut records_file, &record).map_err(io::Error::other)?;
            records_file.write_all(b"\n")?;
            records_file.flush()?;
            records.push(record);
        }
    }

    let summaries = summarize(&records, page_size);
    let noop_wall = summaries
        .iter()
        .find(|summary| summary.case == "noop")
        .map(|summary| {
            (
                summary.native_wall_median_micros,
                summary.native_wall_mad_micros,
            )
        });
    let noop_isolation = noop_isolation(&summaries, page_size);
    let allocation_delta_checks = allocation_delta_checks(&summaries, page_size);
    let short_burst_check = short_burst_check(&records);
    let sampler_distortion_checks = sampler_distortion_checks(&summaries);
    // The worker-backed controls populate this block once the worker seam is
    // enabled. Keeping it absent is intentional: direct-launch evidence must
    // never be presented as worker-isolated calibration.
    let worker_isolation = None;
    let native_measurement_protocol = records
        .iter()
        .find(|record| record.rss_limit_bytes.is_none())
        .map(|record| record.native.measurement_protocol.clone());
    let mut validation_failures = rss_comparison_failures(&records);
    validation_failures.extend(cpu_comparison_failures(&records));
    if let Some(isolation) = &noop_isolation {
        if !isolation.within_tolerance {
            validation_failures.push(format!(
                "noop isolation exceeded tolerance: {} bytes",
                isolation.absolute_difference_bytes
            ));
        }
    } else {
        validation_failures.push("noop isolation summary was unavailable".to_owned());
    }
    validation_failures.extend(
        allocation_delta_checks
            .iter()
            .filter(|check| !check.within_tolerance)
            .map(|check| {
                format!(
                    "{} allocation delta too small: observed {}, required {}",
                    check.case, check.observed_delta_bytes, check.minimum_delta_bytes
                )
            }),
    );
    if allocation_delta_checks.len() != 4 {
        validation_failures.push(format!(
            "allocation delta checks incomplete: expected 4, found {}",
            allocation_delta_checks.len()
        ));
    }
    if !short_burst_check.observed {
        validation_failures.push(format!(
            "no meaningful allocation burst completed within {} micros",
            short_burst_check.duration_limit_micros
        ));
    }
    let worker_isolation_verified = worker_isolation.as_ref().is_some_and(|evidence| {
        worker_isolation_passes(evidence, &metadata, repetitions, page_size)
    });
    let calibration_verified = validation_failures.is_empty() && worker_isolation_verified;
    let calibration_linkage =
        make_calibration_linkage(&metadata, &records, &summaries, None, calibration_verified);
    let sampler_calibration_linkage = make_calibration_linkage(
        &metadata,
        &records,
        &summaries,
        Some(SAMPLER_RSS_LIMIT_BYTES),
        calibration_verified,
    );
    let evidence_directory = run_directory.display().to_string();
    let summary = Summary {
        metadata,
        evidence_directory: evidence_directory.clone(),
        cases: summaries,
        native_measurement_protocol,
        native_noop_wall_median_micros: noop_wall.map(|value| value.0),
        native_noop_wall_mad_micros: noop_wall.map(|value| value.1),
        noop_isolation,
        allocation_delta_checks,
        short_burst_check,
        sampler_distortion_checks,
        worker_isolation,
        calibration_linkage,
        sampler_calibration_linkage,
        validation_failures: validation_failures.clone(),
        protocol_notes: vec![
            "Each record measures one fresh probe child through native wait4 and a separate fresh child through /usr/bin/time.",
            "Allocation probes touch one byte per 4096-byte chunk, so expected bytes are rounded to the host page size for interpretation.",
            "RSS comparison tolerance is max(4 pages, 25% of the larger value); it is a diagnostic gate, not a byte-equality claim.",
            "No-op median and MAD quantify spawn, polling, and exit-observation overhead; known-duration cases retain the unadjusted intervals.",
            "Known-duration error bound is the largest absolute native wall-time error across repetitions; excess includes spawn and exit-observation overhead.",
            "The parsed BSD/GNU elapsed time and separately measured wrapper interval are retained beside every native record.",
            "Records are independent children. The allocation-before/noop-after ordering checks that one child's accounting does not leak into another.",
            "No-op isolation compares the before/after medians with max(8 pages, 25% of the larger median).",
            "Key allocation cases must exceed the no-op median by at least 75% of page-rounded touched bytes, minus max(8 pages, 10% of the no-op median) runtime noise.",
            "At least one allocation burst larger than 1 MiB should finish under 25 ms to prove a short release-before-exit burst is covered; otherwise the result is unverified rather than silently passing.",
            "Artifact set: metadata.json, records.jsonl, errors.jsonl, summary.json, and one raw time stdout/stderr pair per completed comparison.",
            "The native result is authoritative only when its provenance is DarwinWait4 or LinuxWait4 and RSS is positive.",
            "Sampler-on controls use an explicit 128 MiB RSS limit and are separately labeled; their wall, CPU, and RSS distortion is reported but they never contribute to the no-sampler calibration bound.",
            "Sampler-on calibration uses its own sampler protocol and 20/100/250 ms controls; it must not be applied to no-sampler or differently instrumented records.",
            "Sampler distortion is compared with the matching no-sampler median and MAD; within-MAD is an observed bound for these repetitions, not proof of zero overhead or a universal accuracy guarantee.",
            "Any RSS/CPU counter, no-op isolation, allocation-delta, or short-burst validation failure marks both calibration linkages unverified; retained evidence is diagnostic only.",
            "Worker calibration requires retained 0/32/128 MiB coordinator controls, residual-floor data, large prepared-stdin and high-to-low request controls, and independent time agreement; direct-launch records cannot satisfy this gate.",
            "The calibration linkage is host/build/source specific. A loader must hash this summary artifact and include that digest in timing_method; the observed bound is not a universal timing-accuracy guarantee.",
        ],
    };
    write_json(&run_directory.join("summary.json"), &summary)?;
    if validation_failures.is_empty() {
        Ok(run_directory)
    } else {
        Err(io::Error::other(format!(
            "native validation failed; summary retained at {evidence_directory}: {}",
            validation_failures.join("; ")
        )))
    }
}

fn configured_repetitions() -> io::Result<usize> {
    let value = env::var("TQ_NATIVE_VALIDATION_REPETITIONS")
        .unwrap_or_else(|_| DEFAULT_REPETITIONS.to_string());
    let repetitions = value.parse::<usize>().map_err(|error| {
        io::Error::other(format!("invalid repetition count {value:?}: {error}"))
    })?;
    if repetitions < MIN_REPETITIONS {
        return Err(io::Error::other(format!(
            "native validation requires at least {MIN_REPETITIONS} repetitions, got {repetitions}"
        )));
    }
    Ok(repetitions)
}

fn rss_comparison_failures(records: &[Record]) -> Vec<String> {
    let mut counts = BTreeMap::new();
    for record in records {
        if record
            .rss_comparison
            .as_ref()
            .is_some_and(|comparison| !comparison.within_page_aware_tolerance)
        {
            *counts.entry(record.case.clone()).or_insert(0_usize) += 1;
        }
    }
    counts
        .into_iter()
        .map(|(case, count)| {
            format!("{case}: {count} native/time RSS comparisons outside tolerance")
        })
        .collect()
}

fn cpu_comparison_failures(records: &[Record]) -> Vec<String> {
    let mut counts = BTreeMap::new();
    for record in records {
        let Some(comparison) = record.cpu_comparison.as_ref() else {
            continue;
        };
        if !comparison.user.within_tolerance {
            *counts
                .entry(format!("{} user CPU", record.case))
                .or_insert(0_usize) += 1;
        }
        if !comparison.system.within_tolerance {
            *counts
                .entry(format!("{} system CPU", record.case))
                .or_insert(0_usize) += 1;
        }
    }
    counts
        .into_iter()
        .map(|(metric, count)| {
            format!("{metric}: {count} native/time CPU comparisons outside tolerance")
        })
        .collect()
}

fn allocation_delta_checks(summaries: &[CaseSummary], page_size: u64) -> Vec<AllocationDeltaCheck> {
    let Some(noop) = summaries.iter().find(|summary| summary.case == "noop") else {
        return Vec::new();
    };
    let noop_rss = noop.native_peak_rss_median_bytes;
    let runtime_noise_slack = (noop_rss.saturating_mul(ALLOCATION_NOOP_NOISE_NUMERATOR)
        / ALLOCATION_NOOP_NOISE_DENOMINATOR)
        .max(page_size.saturating_mul(8));
    ["burst-4m", "burst-16m", "threads-4m-x2", "threads-2m-x4"]
        .iter()
        .filter_map(|name| {
            let summary = summaries.iter().find(|summary| summary.case == *name)?;
            let page_rounded_bytes = summary.page_rounded_bytes?;
            let minimum_delta_bytes = page_rounded_bytes.saturating_mul(ALLOCATION_DELTA_NUMERATOR)
                / ALLOCATION_DELTA_DENOMINATOR;
            let observed_delta_bytes = summary
                .native_peak_rss_median_bytes
                .saturating_sub(noop_rss);
            Some(AllocationDeltaCheck {
                case: (*name).to_owned(),
                native_peak_rss_median_bytes: summary.native_peak_rss_median_bytes,
                noop_median_rss_bytes: noop_rss,
                observed_delta_bytes,
                page_rounded_bytes,
                minimum_delta_bytes,
                runtime_noise_slack_bytes: runtime_noise_slack,
                within_tolerance: observed_delta_bytes.saturating_add(runtime_noise_slack)
                    >= minimum_delta_bytes,
            })
        })
        .collect()
}

fn short_burst_check(records: &[Record]) -> ShortBurstCheck {
    let mut cases = records
        .iter()
        .filter(|record| {
            record.case.starts_with("burst-")
                && record.touched_bytes.unwrap_or(0) > MEANINGFUL_BURST_BYTES
                && record.native.wall_time_micros < SHORT_BURST_LIMIT_MICROS
        })
        .map(|record| record.case.clone())
        .collect::<Vec<_>>();
    cases.sort();
    cases.dedup();
    ShortBurstCheck {
        bytes_threshold: MEANINGFUL_BURST_BYTES,
        duration_limit_micros: SHORT_BURST_LIMIT_MICROS,
        observed: !cases.is_empty(),
        status: if cases.is_empty() {
            "unverified"
        } else {
            "verified"
        },
        cases,
    }
}

fn validation_cases(page_size: u64) -> io::Result<Vec<Case>> {
    let mut cases = vec![Case {
        name: "noop".to_owned(),
        args: vec!["noop".to_owned()],
        touched_bytes: None,
        page_rounded_bytes: None,
        expected_duration_micros: None,
        rss_limit_bytes: None,
    }];

    // The two smallest cases have no deliberate dwell and exercise the old
    // 25 ms sampler gap. The larger cases make page rounding and peak deltas
    // visible without turning this control suite into a campaign.
    for (label, bytes) in [
        ("short-burst-page", page_size),
        ("short-burst-64k", 64 * 1024),
        ("burst-4m", 4 * 1024 * 1024),
        ("burst-16m", 16 * 1024 * 1024),
    ] {
        let page_rounded_bytes = page_round_up(bytes, page_size)?;
        cases.push(Case {
            name: label.to_owned(),
            args: vec!["allocate-burst".to_owned(), bytes.to_string()],
            touched_bytes: Some(bytes),
            page_rounded_bytes: Some(page_rounded_bytes),
            expected_duration_micros: None,
            rss_limit_bytes: None,
        });
    }

    for (label, bytes, threads) in [
        ("threads-4m-x2", 4 * 1024 * 1024_u64, 2_u64),
        ("threads-2m-x4", 2 * 1024 * 1024_u64, 4_u64),
    ] {
        let total = bytes
            .checked_mul(threads)
            .ok_or_else(|| io::Error::other("thread allocation size overflow"))?;
        let page_rounded_bytes = page_round_up(bytes, page_size)?
            .checked_mul(threads)
            .ok_or_else(|| io::Error::other("thread page allocation size overflow"))?;
        cases.push(Case {
            name: label.to_owned(),
            args: vec![
                "allocate-threads".to_owned(),
                bytes.to_string(),
                threads.to_string(),
            ],
            touched_bytes: Some(total),
            page_rounded_bytes: Some(page_rounded_bytes),
            expected_duration_micros: None,
            rss_limit_bytes: None,
        });
    }

    for millis in [20_u64, 100, 250] {
        cases.push(Case {
            name: format!("known-duration-{millis}ms"),
            args: vec!["sleep".to_owned(), millis.to_string()],
            touched_bytes: None,
            page_rounded_bytes: None,
            expected_duration_micros: Some(u128::from(millis) * 1_000),
            rss_limit_bytes: None,
        });
    }

    // Run a fresh no-op after every allocation family. Its summary is compared
    // with the first no-op family to expose cross-child accounting leakage.
    cases.push(Case {
        name: "noop-after".to_owned(),
        args: vec!["noop".to_owned()],
        touched_bytes: None,
        page_rounded_bytes: None,
        expected_duration_micros: None,
        rss_limit_bytes: None,
    });

    // Event-stream callers enable the diagnostic sampler to enforce an RSS
    // limit. Keep equivalent controls distinct so sampler overhead is visible
    // and cannot be mistaken for the default no-sampler calibration.
    cases.push(Case {
        name: "sampler-noop".to_owned(),
        args: vec!["noop".to_owned()],
        touched_bytes: None,
        page_rounded_bytes: None,
        expected_duration_micros: None,
        rss_limit_bytes: Some(SAMPLER_RSS_LIMIT_BYTES),
    });
    for millis in [20_u64, 100, 250] {
        cases.push(Case {
            name: format!("sampler-known-duration-{millis}ms"),
            args: vec!["sleep".to_owned(), millis.to_string()],
            touched_bytes: None,
            page_rounded_bytes: None,
            expected_duration_micros: Some(u128::from(millis) * 1_000),
            rss_limit_bytes: Some(SAMPLER_RSS_LIMIT_BYTES),
        });
    }
    Ok(cases)
}

fn page_round_up(bytes: u64, page_size: u64) -> io::Result<u64> {
    if page_size == 0 {
        return Err(io::Error::other("page size was zero"));
    }
    let pages = bytes
        .checked_add(page_size - 1)
        .ok_or_else(|| io::Error::other("page rounding overflow"))?
        / page_size;
    pages
        .checked_mul(page_size)
        .ok_or_else(|| io::Error::other("page-rounded allocation overflow"))
}

fn run_native(args: &[String], rss_limit_bytes: Option<u64>) -> io::Result<MeasuredOutcome> {
    let invocation = BenchmarkInvocation {
        executable: PathBuf::from(PROBE),
        args: args.to_vec(),
        stdin: Vec::new(),
        current_dir: None,
        timeout: TIMEOUT,
        output_limit: OUTPUT_LIMIT,
        rss_limit: rss_limit_bytes,
        retain_output: false,
        cancellation: None,
    };
    let outcome = match rss_limit_bytes {
        Some(_) => measure_process(&invocation),
        None => measure_process_uninstrumented(&invocation),
    };
    outcome.map_err(|error| io::Error::other(error.to_string()))
}

fn run_independent_time(args: &[String]) -> io::Result<(Output, TimeObservation)> {
    let started = Instant::now();
    let output = Command::new("/usr/bin/time")
        .arg(time_flag())
        .arg(PROBE)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    let status = output.status;
    let (parsed_user_cpu_micros, parsed_system_cpu_micros) = parse_time_cpu(&output.stderr)
        .map_or((None, None), |(user, system)| (Some(user), Some(system)));
    let observation = TimeObservation {
        mode: time_mode(),
        success: status.success(),
        exit_code: status.code(),
        signal: exit_signal(status),
        wrapper_wall_micros: started.elapsed().as_micros(),
        parsed_wall_micros: parse_time_elapsed(&output.stderr),
        parsed_user_cpu_micros,
        parsed_system_cpu_micros,
        parsed_peak_rss_bytes: parse_time_rss(&output.stderr),
        stdout_file: String::new(),
        stderr_file: String::new(),
    };
    Ok((output, observation))
}

fn save_time_output(
    directory: &Path,
    case: &str,
    repetition: usize,
    output: &Output,
) -> io::Result<(String, String)> {
    let stem = format!("{case}-{repetition}");
    let stdout_name = format!("{stem}.time.stdout");
    let stderr_name = format!("{stem}.time.stderr");
    fs::write(directory.join(&stdout_name), &output.stdout)?;
    fs::write(directory.join(&stderr_name), &output.stderr)?;
    Ok((stdout_name, stderr_name))
}

// Keep the complete per-case reduction together so each retained field has
// the same selected records and cannot accidentally mix sampler modes.
#[allow(clippy::too_many_lines)]
fn summarize(records: &[Record], page_size: u64) -> Vec<CaseSummary> {
    let mut names = Vec::new();
    for record in records {
        if !names.iter().any(|name| name == &record.case) {
            names.push(record.case.clone());
        }
    }
    names
        .into_iter()
        .filter_map(|case| {
            let selected = records
                .iter()
                .filter(|record| record.case == case)
                .collect::<Vec<_>>();
            let mut walls = selected
                .iter()
                .map(|record| record.native.wall_time_micros)
                .collect::<Vec<_>>();
            let mut native_rss = selected
                .iter()
                .filter_map(|record| record.native.peak_rss_bytes)
                .collect::<Vec<_>>();
            if walls.is_empty() || native_rss.is_empty() {
                return None;
            }
            let wall_median = median(&mut walls);
            let wall_mad = median_absolute_deviation(
                &selected
                    .iter()
                    .map(|record| record.native.wall_time_micros)
                    .collect::<Vec<_>>(),
            );
            let native_rss_mad = median_absolute_deviation_u64(&native_rss);
            let native_median = median(&mut native_rss);
            let mut native_user_cpu = selected
                .iter()
                .filter_map(|record| record.native.user_cpu_micros)
                .collect::<Vec<_>>();
            let native_user_cpu_mad =
                (!native_user_cpu.is_empty()).then(|| median_absolute_deviation(&native_user_cpu));
            let native_user_cpu_median =
                (!native_user_cpu.is_empty()).then(|| median(&mut native_user_cpu));
            let mut native_system_cpu = selected
                .iter()
                .filter_map(|record| record.native.system_cpu_micros)
                .collect::<Vec<_>>();
            let native_system_cpu_mad = (!native_system_cpu.is_empty())
                .then(|| median_absolute_deviation(&native_system_cpu));
            let native_system_cpu_median =
                (!native_system_cpu.is_empty()).then(|| median(&mut native_system_cpu));
            let expected_duration = selected[0].expected_duration_micros;
            let duration_error_bound = expected_duration.map(|expected| {
                selected
                    .iter()
                    .map(|record| record.native.wall_time_micros.abs_diff(expected))
                    .max()
                    .unwrap_or(0)
            });
            let duration_excess_median = expected_duration.map(|expected| {
                let mut excesses = selected
                    .iter()
                    .map(|record| record.native.wall_time_micros.saturating_sub(expected))
                    .collect::<Vec<_>>();
                median(&mut excesses)
            });
            let mut time_rss = selected
                .iter()
                .filter_map(|record| record.independent_time.parsed_peak_rss_bytes)
                .collect::<Vec<_>>();
            let time_median = (!time_rss.is_empty()).then(|| median(&mut time_rss));
            let mut time_wall = selected
                .iter()
                .filter_map(|record| record.independent_time.parsed_wall_micros)
                .collect::<Vec<_>>();
            let time_wall_median = (!time_wall.is_empty()).then(|| median(&mut time_wall));
            let mut time_user_cpu = selected
                .iter()
                .filter_map(|record| record.independent_time.parsed_user_cpu_micros)
                .collect::<Vec<_>>();
            let time_user_cpu_median =
                (!time_user_cpu.is_empty()).then(|| median(&mut time_user_cpu));
            let mut time_system_cpu = selected
                .iter()
                .filter_map(|record| record.independent_time.parsed_system_cpu_micros)
                .collect::<Vec<_>>();
            let time_system_cpu_median =
                (!time_system_cpu.is_empty()).then(|| median(&mut time_system_cpu));
            let comparisons = selected
                .iter()
                .filter_map(|record| record.rss_comparison.as_ref())
                .collect::<Vec<_>>();
            let tolerance = comparisons.iter().map(|value| value.tolerance_bytes).max();
            let within = (!comparisons.is_empty()).then(|| {
                comparisons
                    .iter()
                    .filter(|value| value.within_page_aware_tolerance)
                    .count()
            });
            let cpu_comparisons = selected
                .iter()
                .filter_map(|record| record.cpu_comparison.as_ref())
                .collect::<Vec<_>>();
            let cpu_within = (!cpu_comparisons.is_empty()).then(|| {
                cpu_comparisons
                    .iter()
                    .filter(|comparison| {
                        comparison.user.within_tolerance && comparison.system.within_tolerance
                    })
                    .count()
            });
            let first = selected[0];
            Some(CaseSummary {
                case,
                repetitions: selected.len(),
                touched_bytes: first.touched_bytes,
                page_rounded_bytes: first.page_rounded_bytes,
                expected_duration_micros: first.expected_duration_micros,
                rss_limit_bytes: first.rss_limit_bytes,
                native_wall_median_micros: wall_median,
                native_wall_mad_micros: wall_mad,
                native_duration_error_bound_micros: duration_error_bound,
                native_duration_excess_median_micros: duration_excess_median,
                native_user_cpu_median_micros: native_user_cpu_median,
                native_user_cpu_mad_micros: native_user_cpu_mad,
                native_system_cpu_median_micros: native_system_cpu_median,
                native_system_cpu_mad_micros: native_system_cpu_mad,
                native_peak_rss_median_bytes: native_median,
                native_peak_rss_mad_bytes: native_rss_mad,
                time_wall_median_micros: time_wall_median,
                time_user_cpu_median_micros: time_user_cpu_median,
                time_system_cpu_median_micros: time_system_cpu_median,
                time_peak_rss_median_bytes: time_median,
                rss_tolerance_bytes: tolerance.or_else(|| Some(page_size.saturating_mul(4))),
                rss_comparisons_within_tolerance: within,
                cpu_comparisons_within_tolerance: cpu_within,
            })
        })
        .collect()
}

fn noop_isolation(summaries: &[CaseSummary], page_size: u64) -> Option<NoopIsolation> {
    let before = summaries.iter().find(|summary| summary.case == "noop")?;
    let after = summaries
        .iter()
        .find(|summary| summary.case == "noop-after")?;
    let absolute_difference_bytes = before
        .native_peak_rss_median_bytes
        .abs_diff(after.native_peak_rss_median_bytes);
    let tolerance_bytes = (before
        .native_peak_rss_median_bytes
        .max(after.native_peak_rss_median_bytes)
        / 4)
    .max(page_size.saturating_mul(8));
    Some(NoopIsolation {
        before_median_rss_bytes: before.native_peak_rss_median_bytes,
        after_median_rss_bytes: after.native_peak_rss_median_bytes,
        absolute_difference_bytes,
        tolerance_bytes,
        within_tolerance: absolute_difference_bytes <= tolerance_bytes,
    })
}

fn rss_comparison(native_bytes: u64, time_bytes: u64, page_size: u64) -> RssComparison {
    let absolute_difference_bytes = native_bytes.abs_diff(time_bytes);
    let relative_tolerance = native_bytes.max(time_bytes) / 4;
    let page_tolerance = page_size.saturating_mul(4);
    let tolerance_bytes = relative_tolerance.max(page_tolerance);
    RssComparison {
        native_bytes,
        time_bytes,
        absolute_difference_bytes,
        tolerance_bytes,
        within_page_aware_tolerance: absolute_difference_bytes <= tolerance_bytes,
    }
}

fn cpu_comparison(native_micros: u128, time_micros: u128) -> CpuComparison {
    let absolute_difference_micros = native_micros.abs_diff(time_micros);
    let relative_tolerance = native_micros.max(time_micros) / 4;
    let tolerance_micros = TIME_CPU_TOLERANCE_MICROS.max(relative_tolerance);
    CpuComparison {
        native_micros,
        time_micros,
        absolute_difference_micros,
        tolerance_micros,
        within_tolerance: absolute_difference_micros <= tolerance_micros,
    }
}

fn sampler_distortion_checks(summaries: &[CaseSummary]) -> Vec<SamplerDistortion> {
    [
        ("noop", "sampler-noop"),
        ("known-duration-20ms", "sampler-known-duration-20ms"),
        ("known-duration-100ms", "sampler-known-duration-100ms"),
        ("known-duration-250ms", "sampler-known-duration-250ms"),
    ]
    .into_iter()
    .filter_map(|(normal_name, sampler_name)| {
        let normal = summaries
            .iter()
            .find(|summary| summary.case == normal_name)?;
        let sampler = summaries
            .iter()
            .find(|summary| summary.case == sampler_name)?;
        Some(SamplerDistortion {
            normal_case: normal_name.to_owned(),
            sampler_case: sampler_name.to_owned(),
            normal_wall_median_micros: Some(normal.native_wall_median_micros),
            normal_wall_mad_micros: normal.native_wall_mad_micros,
            sampler_wall_median_micros: Some(sampler.native_wall_median_micros),
            wall_median_delta_micros: Some(
                normal
                    .native_wall_median_micros
                    .abs_diff(sampler.native_wall_median_micros),
            ),
            wall_delta_within_normal_mad: Some(
                normal
                    .native_wall_median_micros
                    .abs_diff(sampler.native_wall_median_micros)
                    <= normal.native_wall_mad_micros,
            ),
            normal_user_cpu_median_micros: normal.native_user_cpu_median_micros,
            normal_user_cpu_mad_micros: normal.native_user_cpu_mad_micros,
            sampler_user_cpu_median_micros: sampler.native_user_cpu_median_micros,
            user_cpu_median_delta_micros: normal
                .native_user_cpu_median_micros
                .zip(sampler.native_user_cpu_median_micros)
                .map(|(normal, sampler)| normal.abs_diff(sampler)),
            user_cpu_delta_within_normal_mad: normal
                .native_user_cpu_median_micros
                .zip(normal.native_user_cpu_mad_micros)
                .zip(sampler.native_user_cpu_median_micros)
                .map(|((normal, mad), sampler)| normal.abs_diff(sampler) <= mad),
            normal_system_cpu_median_micros: normal.native_system_cpu_median_micros,
            normal_system_cpu_mad_micros: normal.native_system_cpu_mad_micros,
            sampler_system_cpu_median_micros: sampler.native_system_cpu_median_micros,
            system_cpu_median_delta_micros: normal
                .native_system_cpu_median_micros
                .zip(sampler.native_system_cpu_median_micros)
                .map(|(normal, sampler)| normal.abs_diff(sampler)),
            system_cpu_delta_within_normal_mad: normal
                .native_system_cpu_median_micros
                .zip(normal.native_system_cpu_mad_micros)
                .zip(sampler.native_system_cpu_median_micros)
                .map(|((normal, mad), sampler)| normal.abs_diff(sampler) <= mad),
            normal_peak_rss_median_bytes: Some(normal.native_peak_rss_median_bytes),
            normal_peak_rss_mad_bytes: normal.native_peak_rss_mad_bytes,
            sampler_peak_rss_median_bytes: Some(sampler.native_peak_rss_median_bytes),
            peak_rss_median_delta_bytes: Some(
                normal
                    .native_peak_rss_median_bytes
                    .abs_diff(sampler.native_peak_rss_median_bytes),
            ),
            peak_rss_delta_within_normal_mad: Some(
                normal
                    .native_peak_rss_median_bytes
                    .abs_diff(sampler.native_peak_rss_median_bytes)
                    <= normal.native_peak_rss_mad_bytes,
            ),
        })
    })
    .collect()
}

fn make_calibration_linkage(
    metadata: &Metadata,
    records: &[Record],
    summaries: &[CaseSummary],
    rss_limit_bytes: Option<u64>,
    calibration_verified: bool,
) -> CalibrationLinkage {
    let measurement_protocol = records
        .iter()
        .find(|record| record.rss_limit_bytes == rss_limit_bytes)
        .map(|record| record.native.measurement_protocol.clone());
    let conservative_bound = summaries
        .iter()
        .filter(|summary| summary.rss_limit_bytes == rss_limit_bytes)
        .filter_map(|summary| summary.native_duration_error_bound_micros)
        .max();
    CalibrationLinkage {
        measurement_protocol,
        rss_limit_bytes,
        machine_identity: metadata.environment.machine_identity.clone(),
        probe_sha256: metadata.probe_sha256.clone(),
        runner_build_profile: metadata.build_profile,
        collector_source_sha256: metadata.collector_source_sha256.clone(),
        time_mode: metadata.time_mode,
        conservative_observed_duration_error_bound_micros: conservative_bound,
        status: if !calibration_verified {
            "unverified-validation-failures"
        } else if conservative_bound.is_some() {
            "verified"
        } else {
            "unverified-missing-no-sampler-duration-controls"
        },
    }
}

fn worker_isolation_passes(
    evidence: &WorkerIsolationEvidence,
    metadata: &Metadata,
    repetitions: usize,
    page_size: u64,
) -> bool {
    const EXPECTED_ALLOCATIONS: [u64; 3] = [0, 32 * 1024 * 1024, 128 * 1024 * 1024];
    const MIN_PREPARED_STDIN_BYTES: u64 = 1 << 20;

    if evidence.collector_source_sha256 != metadata.collector_source_sha256
        || page_size == 0
        || evidence.residual_floor_bytes == 0
        || evidence.prepared_stdin_bytes < MIN_PREPARED_STDIN_BYTES
        || evidence.prepared_stdin_repetitions < repetitions
        || !evidence.prepared_stdin_verified
        || evidence.high_low_request_repetitions < repetitions
        || !evidence.high_low_request_verified
        || evidence.independent_time_repetitions < repetitions
        || !evidence.independent_time_verified
    {
        return false;
    }

    if evidence.coordinator_controls.len() != EXPECTED_ALLOCATIONS.len() {
        return false;
    }
    let Some(zero_control) = evidence
        .coordinator_controls
        .iter()
        .find(|control| control.coordinator_allocation_bytes == 0)
    else {
        return false;
    };
    if evidence.residual_floor_bytes != zero_control.native_rss_median_bytes {
        return false;
    }
    EXPECTED_ALLOCATIONS.iter().all(|expected| {
        evidence
            .coordinator_controls
            .iter()
            .find(|control| control.coordinator_allocation_bytes == *expected)
            .is_some_and(|control| {
                control.repetitions >= repetitions
                    && control.native_rss_median_bytes > 0
                    && control
                        .independent_time_rss_median_bytes
                        .is_some_and(|rss| {
                            rss > 0
                                && rss_comparison_tolerance(
                                    control.native_rss_median_bytes,
                                    rss,
                                    page_size,
                                )
                                .is_some_and(|bound| {
                                    control.native_rss_median_bytes.abs_diff(rss) <= bound
                                })
                        })
                    && control.rss_comparisons_within_tolerance == control.repetitions
                    && control.cpu_comparisons_within_tolerance == control.repetitions
                    && control.tolerance_bytes >= page_size
                    && noop_tolerance(
                        zero_control.native_rss_median_bytes,
                        control.native_rss_median_bytes,
                        page_size,
                    )
                    .is_some_and(|bound| control.tolerance_bytes <= bound)
                    && control.max_delta_from_zero_bytes <= control.tolerance_bytes
                    && control
                        .native_rss_median_bytes
                        .abs_diff(zero_control.native_rss_median_bytes)
                        <= control.max_delta_from_zero_bytes
            })
    })
}

fn noop_tolerance(zero_rss: u64, control_rss: u64, page_size: u64) -> Option<u64> {
    zero_rss
        .max(control_rss)
        .checked_div(4)
        .zip(page_size.checked_mul(8))
        .map(|(relative, page)| relative.max(page))
}

fn rss_comparison_tolerance(native_rss: u64, independent_rss: u64, page_size: u64) -> Option<u64> {
    native_rss
        .max(independent_rss)
        .checked_div(4)
        .zip(page_size.checked_mul(4))
        .map(|(relative, page)| relative.max(page))
}

fn median<T: Ord + Copy>(values: &mut [T]) -> T {
    values.sort_unstable();
    values[values.len() / 2]
}

fn median_absolute_deviation(values: &[u128]) -> u128 {
    let median_value = median(&mut values.to_vec());
    let mut deviations = values
        .iter()
        .map(|value| value.abs_diff(median_value))
        .collect::<Vec<_>>();
    median(&mut deviations)
}

fn median_absolute_deviation_u64(values: &[u64]) -> u64 {
    let median_value = median(&mut values.to_vec());
    let mut deviations = values
        .iter()
        .map(|value| value.abs_diff(median_value))
        .collect::<Vec<_>>();
    median(&mut deviations)
}

fn write_json(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, value).map_err(io::Error::other)
}

fn fail_after_recording(
    errors_file: &mut File,
    case: &Case,
    repetition: usize,
    stage: &str,
    error: impl Into<String>,
) -> io::Error {
    let record = ErrorRecord {
        case: case.name.clone(),
        args: case.args.clone(),
        repetition,
        stage: stage.to_owned(),
        error: error.into(),
    };
    if let Err(write_error) = serde_json::to_writer(&mut *errors_file, &record)
        .map_err(io::Error::other)
        .and_then(|()| errors_file.write_all(b"\n"))
        .and_then(|()| errors_file.flush())
    {
        return io::Error::other(format!(
            "{stage} failed and error artifact could not be written: {write_error}"
        ));
    }
    io::Error::other(record.error)
}

fn file_sha256(path: &Path) -> io::Result<String> {
    Ok(hex_digest(&Sha256::digest(fs::read(path)?)))
}

fn hex_digest(digest: &[u8]) -> String {
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(hex, "{byte:02x}").expect("writing to String cannot fail");
    }
    hex
}

const fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

fn native_page_size() -> io::Result<u64> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("sysctl");
    #[cfg(target_os = "macos")]
    command.args(["-n", "hw.pagesize"]);
    #[cfg(target_os = "linux")]
    let mut command = Command::new("getconf");
    #[cfg(target_os = "linux")]
    command.arg("PAGESIZE");
    let output = command.output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "page-size command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .map_err(|error| io::Error::other(format!("invalid native page size: {error}")))
}

fn command_version(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    };
    Some(String::from_utf8_lossy(text).trim().to_owned())
}

fn unix_micros() -> Option<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_micros())
}

#[cfg(target_os = "macos")]
const fn expected_provenance() -> RssProvenance {
    RssProvenance::DarwinWait4
}

#[cfg(target_os = "linux")]
const fn expected_provenance() -> RssProvenance {
    RssProvenance::LinuxWait4
}

#[cfg(target_os = "macos")]
const fn time_mode() -> &'static str {
    "bsd-time-l"
}

#[cfg(target_os = "linux")]
const fn time_mode() -> &'static str {
    "gnu-time-v"
}

#[cfg(target_os = "macos")]
const fn time_flag() -> &'static str {
    "-l"
}

#[cfg(target_os = "linux")]
const fn time_flag() -> &'static str {
    "-v"
}

#[cfg(target_os = "macos")]
fn parse_time_rss(stderr: &[u8]) -> Option<u64> {
    String::from_utf8_lossy(stderr).lines().find_map(|line| {
        line.trim()
            .strip_suffix("maximum resident set size")
            .and_then(|value| value.trim().parse::<u64>().ok())
    })
}

#[cfg(target_os = "linux")]
fn parse_time_rss(stderr: &[u8]) -> Option<u64> {
    const LABEL: &str = "Maximum resident set size (kbytes):";
    String::from_utf8_lossy(stderr).lines().find_map(|line| {
        line.trim()
            .strip_prefix(LABEL)
            .and_then(|value| value.trim().parse::<u64>().ok())
            .and_then(|value| value.checked_mul(1024))
    })
}

#[cfg(target_os = "macos")]
fn parse_time_elapsed(stderr: &[u8]) -> Option<u128> {
    String::from_utf8_lossy(stderr).lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        let seconds = fields.next()?;
        (fields.next() == Some("real")).then(|| parse_decimal_seconds(seconds))?
    })
}

#[cfg(target_os = "linux")]
fn parse_time_elapsed(stderr: &[u8]) -> Option<u128> {
    const LABEL: &str = "Elapsed (wall clock) time (h:mm:ss or m:ss):";
    String::from_utf8_lossy(stderr).lines().find_map(|line| {
        let value = line.trim().strip_prefix(LABEL)?.trim();
        let fields = value.split(':').collect::<Vec<_>>();
        let (hours, minutes, seconds) = match fields.as_slice() {
            [minutes, seconds] => (0_u128, minutes.parse::<u128>().ok()?, *seconds),
            [hours, minutes, seconds] => (
                hours.parse::<u128>().ok()?,
                minutes.parse::<u128>().ok()?,
                *seconds,
            ),
            _ => return None,
        };
        let total_minutes = hours.checked_mul(60)?.checked_add(minutes)?;
        total_minutes
            .checked_mul(60_000_000)?
            .checked_add(parse_decimal_seconds(seconds)?)
    })
}

#[cfg(target_os = "macos")]
fn parse_time_cpu(stderr: &[u8]) -> Option<(u128, u128)> {
    String::from_utf8_lossy(stderr).lines().find_map(|line| {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 6 || fields[1] != "real" || fields[3] != "user" || fields[5] != "sys" {
            return None;
        }
        Some((
            parse_decimal_seconds(fields[2])?,
            parse_decimal_seconds(fields[4])?,
        ))
    })
}

#[cfg(target_os = "linux")]
fn parse_time_cpu(stderr: &[u8]) -> Option<(u128, u128)> {
    let user = parse_gnu_time_field(stderr, "User time (seconds):")?;
    let system = parse_gnu_time_field(stderr, "System time (seconds):")?;
    Some((user, system))
}

#[cfg(target_os = "linux")]
fn parse_gnu_time_field(stderr: &[u8], label: &str) -> Option<u128> {
    String::from_utf8_lossy(stderr).lines().find_map(|line| {
        line.trim()
            .strip_prefix(label)
            .and_then(|value| parse_decimal_seconds(value.trim()))
    })
}

fn parse_decimal_seconds(value: &str) -> Option<u128> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    let whole = whole.parse::<u128>().ok()?;
    let mut micros = 0_u128;
    let mut digits = 0;
    for digit in fraction.chars().take(6) {
        micros = micros
            .checked_mul(10)?
            .checked_add(u128::from(digit.to_digit(10)?))?;
        digits += 1;
    }
    for _ in digits..6 {
        micros = micros.checked_mul(10)?;
    }
    whole.checked_mul(1_000_000)?.checked_add(micros)
}

#[cfg(unix)]
fn exit_signal(status: ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt as _;
    status.signal()
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "macos")]
    #[test]
    fn parses_bsd_time_l_units() {
        let stderr = b"       2097152  maximum resident set size\n  0.02 real         0.00 user         0.00 sys\n";
        assert_eq!(super::parse_time_rss(stderr), Some(2_097_152));
        assert_eq!(super::parse_time_elapsed(stderr), Some(20_000));
        assert_eq!(super::parse_time_cpu(stderr), Some((0, 0)));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_gnu_time_v_units() {
        let stderr = b"Maximum resident set size (kbytes): 2048\nElapsed (wall clock) time (h:mm:ss or m:ss): 0:01.25\nUser time (seconds): 0.01\nSystem time (seconds): 0.002\n";
        assert_eq!(super::parse_time_rss(stderr), Some(2_097_152));
        assert_eq!(super::parse_time_elapsed(stderr), Some(1_250_000));
        assert_eq!(super::parse_time_cpu(stderr), Some((10_000, 2_000)));
    }

    #[test]
    fn keeps_rss_tolerance_page_aware() {
        let comparison = super::rss_comparison(10_000, 10_100, 4_096);
        assert_eq!(comparison.tolerance_bytes, 16_384);
        assert!(comparison.within_page_aware_tolerance);
    }
}
