//! Deterministic benchmark process and statistics tests.

use std::{path::PathBuf, time::Duration};

use tq_test_support::benchmark::{
    BenchmarkInvocation, BenchmarkSample, MeasuredStatus, RssProvenance, measure_process,
    summarize_samples,
};

fn invocation(args: &[&str], output_limit: u64, timeout: Duration) -> BenchmarkInvocation {
    BenchmarkInvocation {
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
        RssProvenance::GnuTimeV | RssProvenance::BsdTimeL
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
        RssProvenance::GnuTimeV | RssProvenance::BsdTimeL
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
fn summary_reports_median_dispersion_throughput_and_output() {
    let samples = [100_u128, 200, 300]
        .into_iter()
        .map(|wall_time_micros| BenchmarkSample {
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
