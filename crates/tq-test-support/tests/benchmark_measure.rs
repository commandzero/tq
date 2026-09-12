//! Deterministic benchmark process and statistics tests.

use std::{path::PathBuf, time::Duration};

use tq_test_support::benchmark::{
    BenchmarkInvocation, BenchmarkSample, MeasuredStatus, RssProvenance, measure_process,
    measure_process_uninstrumented, summarize_samples,
};

fn invocation(args: &[&str], output_limit: u64, timeout: Duration) -> BenchmarkInvocation {
    BenchmarkInvocation {
        cancellation: None,
        executable: PathBuf::from(env!("CARGO_BIN_EXE_tq-bench-helper")),
        args: args.iter().map(|value| (*value).to_owned()).collect(),
        stdin: Vec::new(),
        current_dir: None,
        timeout,
        output_limit,
        rss_limit: None,
        retain_output: false,
    }
}

#[test]
fn sleeper_timeout_is_preserved_as_a_row_outcome() {
    let outcome = measure_process(&invocation(
        &["sleep", "500"],
        1024,
        Duration::from_millis(30),
    ))
    .expect("sleeper measurement");
    assert_eq!(outcome.status, MeasuredStatus::Timeout);
    assert!(outcome.wall_time_micros < 1_000_000);
    assert!(outcome.peak_rss_bytes.expect("timeout RSS") > 0);
    assert!(matches!(
        outcome.rss_provenance,
        RssProvenance::LinuxWait4 | RssProvenance::DarwinWait4
    ));
}

#[test]
fn output_limit_and_first_result_are_measured_without_reframing() {
    let limited = measure_process(&invocation(
        &["output", "1048576"],
        1024,
        Duration::from_secs(2),
    ))
    .expect("output measurement");
    assert_eq!(limited.status, MeasuredStatus::OutputLimit);
    assert!(limited.output_bytes > 1024);
    assert!(
        limited.first_result_micros.is_some(),
        "non-empty diagnostic output must retain a first-result timestamp"
    );
    assert!(limited.peak_rss_bytes.expect("output-limit RSS") > 0);
    let stdout_path = limited.stdout_path.as_deref().expect("saved stdout");
    assert!(stdout_path.exists());
    assert!(
        std::fs::metadata(stdout_path)
            .expect("stdout metadata")
            .len()
            > 1024
    );
    let stderr_path = limited.stderr_path.as_deref().expect("saved stderr");
    assert!(stderr_path.exists());
    std::fs::remove_file(stdout_path).expect("remove saved stdout");
    std::fs::remove_file(stderr_path).expect("remove saved stderr");

    let first = measure_process(&invocation(&["first", "100"], 1024, Duration::from_secs(2)))
        .expect("first-result measurement");
    assert_eq!(first.status, MeasuredStatus::Exited);
    assert_eq!(first.output_bytes, b"first\nlast\n".len() as u64);
    assert!(first.stdout_path.is_none());
    assert!(first.stderr_path.is_none());
    assert!(first.first_result_micros.expect("first result") < first.wall_time_micros);
}

#[test]
fn cpu_and_memory_metrics_include_authoritative_rss_provenance() {
    let outcome = measure_process(&invocation(
        &["memory", "8388608"],
        1024,
        Duration::from_secs(2),
    ))
    .expect("memory measurement");
    assert_eq!(outcome.status, MeasuredStatus::Exited);
    assert!(outcome.user_cpu_micros.is_some());
    assert!(outcome.system_cpu_micros.is_some());
    assert!(outcome.peak_rss_bytes.expect("authoritative RSS") > 0);
    assert!(matches!(
        outcome.rss_provenance,
        RssProvenance::LinuxWait4 | RssProvenance::DarwinWait4
    ));
}

#[test]
fn rss_limit_stops_the_process_when_host_sampling_is_available() {
    let mut request = invocation(&["memory", "8388608"], 1024, Duration::from_secs(2));
    request.rss_limit = Some(1);
    let outcome = measure_process(&request).expect("RSS-limited measurement");
    #[cfg(target_os = "linux")]
    {
        assert!(outcome.peak_rss_bytes.is_some());
        assert_eq!(outcome.status, MeasuredStatus::RssLimit);
    }
    if outcome.peak_rss_bytes.is_some() {
        assert_eq!(outcome.status, MeasuredStatus::RssLimit);
    }
}

#[test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn uninstrumented_rss_limit_is_classified_at_native_exit_without_sampler() {
    let mut request = probe(&["allocate-burst", "67108864"]);
    request.rss_limit = Some(32 * 1024 * 1024);
    let outcome = measure_process_uninstrumented(&request).expect("uninstrumented measurement");
    assert_eq!(outcome.status, MeasuredStatus::RssLimit);
    assert!(outcome.peak_rss_bytes.unwrap() > request.rss_limit.unwrap());
    assert_eq!(outcome.process_group_peak_rss_bytes, None);
    assert!(!outcome.process_group_rss_observed);
    assert_eq!(outcome.measurement_protocol.rss_poll_interval_micros, None);
    assert!(
        outcome
            .measurement_protocol
            .timing_method
            .contains("no in-flight RSS enforcement")
    );
    remove_captures(&outcome);
}

#[test]
fn summary_reports_median_dispersion_throughput_and_output() {
    let samples = [100_u128, 200, 300]
        .into_iter()
        .map(|wall_time_micros| BenchmarkSample {
            measurement_protocol: None,
            wall_time_micros,
            user_cpu_micros: Some(wall_time_micros / 2),
            system_cpu_micros: Some(10),
            peak_rss_bytes: Some(1024),
            rss_provenance: None,
            process_group_peak_rss_bytes: None,
            first_result_micros: Some(20),
            output_bytes: 7,
        })
        .collect::<Vec<_>>();
    let summary = summarize_samples(&samples, 1024 * 1024, 10).expect("summary");
    assert!((summary.wall_time_micros.median - 200.0).abs() < f64::EPSILON);
    assert!((summary.wall_time_micros.median_absolute_deviation - 100.0).abs() < f64::EPSILON);
    assert_eq!(summary.output_bytes, 7);
    assert!((summary.physical_mib_per_second - 5_000.0).abs() < f64::EPSILON);
    assert!((summary.logical_records_per_second - 50_000.0).abs() < f64::EPSILON);
}

fn probe(args: &[&str]) -> BenchmarkInvocation {
    let mut request = invocation(args, 16 * 1024 * 1024, Duration::from_secs(5));
    request.executable = PathBuf::from(env!("CARGO_BIN_EXE_tq-bench-probe"));
    request
}

fn remove_captures(outcome: &tq_test_support::benchmark::MeasuredOutcome) {
    for path in [&outcome.stdout_path, &outcome.stderr_path]
        .into_iter()
        .flatten()
    {
        std::fs::remove_file(path).expect("remove retained measurement capture");
    }
}

#[test]
fn direct_measurement_preserves_literal_arguments_and_stdin_eof() {
    let mut request = probe(&["literal-args", "space value", "* ; $HOME", "quote'\""]);
    request.retain_output = true;
    let outcome = measure_process(&request).unwrap();
    assert_eq!(outcome.exit_code, Some(0));
    assert_eq!(
        std::fs::read(outcome.stdout_path.as_ref().unwrap()).unwrap(),
        b"space value\n* ; $HOME\nquote'\"\n"
    );
    assert!(!outcome.process_group_rss_observed);
    assert_eq!(outcome.process_group_peak_rss_bytes, None);
    assert_eq!(outcome.measurement_protocol.rss_poll_interval_micros, None);
    remove_captures(&outcome);

    request = probe(&["stdin-echo"]);
    request.stdin = b"line\n\0binary\xff".repeat(100_000);
    request.retain_output = true;
    let outcome = measure_process(&request).unwrap();
    assert_eq!(
        std::fs::read(outcome.stdout_path.as_ref().unwrap()).unwrap(),
        request.stdin
    );
    remove_captures(&outcome);
}

#[test]
fn blocked_stdin_and_nonzero_exit_complete_with_native_usage() {
    let mut request = probe(&["blocked-input", "10000"]);
    request.stdin = vec![b'x'; 8 * 1024 * 1024];
    request.timeout = Duration::from_millis(50);
    let started = std::time::Instant::now();
    let outcome = measure_process(&request).unwrap();
    assert_eq!(outcome.status, MeasuredStatus::Timeout);
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(outcome.peak_rss_bytes.unwrap() > 0);
    remove_captures(&outcome);

    let outcome = measure_process(&probe(&["nonzero", "23"])).unwrap();
    assert_eq!(outcome.status, MeasuredStatus::Exited);
    assert_eq!(outcome.exit_code, Some(23));
    assert!(outcome.peak_rss_bytes.unwrap() > 0);
    remove_captures(&outcome);
}

#[test]
fn independent_released_allocations_do_not_accumulate_across_children() {
    let low = measure_process(&probe(&["noop"])).unwrap();
    let high = measure_process(&probe(&["allocate-burst", "67108864"])).unwrap();
    let after = measure_process(&probe(&["noop"])).unwrap();
    // RSS includes runtime pages, not just the requested allocation. The broad
    // margin tests exact-child ownership, not allocator-specific byte equality.
    assert!(high.peak_rss_bytes.unwrap() > low.peak_rss_bytes.unwrap() + 32 * 1024 * 1024);
    assert!(after.peak_rss_bytes.unwrap() + 32 * 1024 * 1024 < high.peak_rss_bytes.unwrap());
    let concurrent =
        std::thread::spawn(|| measure_process(&probe(&["allocate-burst", "67108864"])).unwrap());
    let small = measure_process(&probe(&["noop"])).unwrap();
    let large = concurrent.join().unwrap();
    assert!(small.peak_rss_bytes.unwrap() + 32 * 1024 * 1024 < large.peak_rss_bytes.unwrap());
}

#[test]
fn deadline_exit_races_resolve_to_one_terminal_outcome() {
    for _ in 0..40 {
        let mut request = probe(&["sleep", "5"]);
        request.timeout = Duration::from_millis(5);
        let result =
            measure_process(&request).expect("deadline race must not double-reap or mis-signal");
        assert!(matches!(
            result.status,
            MeasuredStatus::Exited | MeasuredStatus::Timeout
        ));
        remove_captures(&result);
    }
}

#[test]
fn spawn_failure_is_an_infrastructure_error() {
    let directory = tempfile::tempdir().unwrap();
    let mut request = probe(&["noop"]);
    request.executable = directory.path().join("absent-executable");
    assert!(matches!(
        measure_process(&request),
        Err(tq_test_support::benchmark::MeasureError::Io(_))
    ));
}

#[test]
fn cancellation_terminates_and_retains_diagnostics_without_a_valid_sample() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use tq_test_support::benchmark::MeasureError;
    let mut request = probe(&["sleep", "10000"]);
    let cancelled = Arc::new(AtomicBool::new(true));
    request.cancellation = Some(Arc::clone(&cancelled));
    assert!(matches!(
        measure_process(&request),
        Err(MeasureError::Cancelled)
    ));
    cancelled.store(false, Ordering::Release);
    let trigger = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        cancelled.store(true, Ordering::Release);
    });
    let started = std::time::Instant::now();
    let error = measure_process(&request).unwrap_err();
    trigger.join().unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    let MeasureError::Collection {
        source,
        stdout_path,
        stderr_path,
        wall_time_micros,
        ..
    } = error
    else {
        panic!("expected retained cancelled outcome")
    };
    assert!(matches!(*source, MeasureError::Cancelled));
    assert!(wall_time_micros.is_some());
    assert!(stdout_path.is_file());
    assert!(stderr_path.is_file());
    std::fs::remove_file(stdout_path).unwrap();
    std::fs::remove_file(stderr_path).unwrap();
}
