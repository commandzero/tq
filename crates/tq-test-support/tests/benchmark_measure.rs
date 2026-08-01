//! Deterministic benchmark process and statistics tests.

use std::{path::PathBuf, time::Duration};

use tq_test_support::benchmark::{
    BenchmarkInvocation, BenchmarkSample, MeasuredStatus, measure_process, summarize_samples,
};

fn invocation(args: &[&str], output_limit: u64, timeout: Duration) -> BenchmarkInvocation {
    BenchmarkInvocation {
        executable: PathBuf::from(env!("CARGO_BIN_EXE_tq-bench-helper")),
        args: args.iter().map(|value| (*value).to_owned()).collect(),
        stdin: Vec::new(),
        current_dir: None,
        timeout,
        output_limit,
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
    assert_eq!(limited.stdout.len(), 1024);

    let first = measure_process(&invocation(&["first", "100"], 1024, Duration::from_secs(2)))
        .expect("first-result measurement");
    assert_eq!(first.status, MeasuredStatus::Exited);
    assert_eq!(first.stdout, b"first\nlast\n");
    assert!(first.first_result_micros.expect("first result") < first.wall_time_micros);
}

#[test]
fn cpu_and_memory_metrics_are_values_or_explicitly_unavailable() {
    let outcome = measure_process(&invocation(
        &["memory", "8388608"],
        1024,
        Duration::from_secs(2),
    ))
    .expect("memory measurement");
    assert_eq!(outcome.status, MeasuredStatus::Exited);
    assert!(outcome.user_cpu_micros.is_some());
    assert!(outcome.system_cpu_micros.is_some());
    #[cfg(unix)]
    assert!(
        outcome.peak_rss_bytes.is_some(),
        "Unix process-group RSS sampling should be available"
    );
    if let Some(rss) = outcome.peak_rss_bytes {
        assert!(rss >= 8 * 1024 * 1024);
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
