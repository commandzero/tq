//! Black-box lifecycle coverage for overlapping worker measurements.

#![cfg(any(target_os = "macos", target_os = "linux"))]

use std::{
    path::PathBuf,
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

use tq_test_support::benchmark::{
    BenchmarkInvocation, MeasuredStatus, measure_process_worker_uninstrumented,
};

const OVERLAP_SCRIPT: &str = r#"
set -eu
own_marker=$1
other_marker=$2
printf '%s\n' "$$" > "$own_marker"
attempt=0
while [ ! -f "$other_marker" ]; do
    attempt=$((attempt + 1))
    if [ "$attempt" -ge 500 ]; then
        exit 75
    fi
    sleep 0.01
done
"#;

#[test]
fn concurrent_worker_measurements_both_reap_healthy_targets() {
    let directory = tempfile::tempdir().expect("temporary overlap directory");
    let first_marker = directory.path().join("first.started");
    let second_marker = directory.path().join("second.started");
    let start = Arc::new(Barrier::new(2));
    let handles = [
        (first_marker.clone(), second_marker.clone()),
        (second_marker, first_marker),
    ]
    .into_iter()
    .map(|(own_marker, other_marker)| {
        let start = Arc::clone(&start);
        thread::spawn(move || {
            start.wait();
            measure_process_worker_uninstrumented(&BenchmarkInvocation {
                cancellation: None,
                executable: PathBuf::from("/bin/sh"),
                args: vec![
                    "-c".to_owned(),
                    OVERLAP_SCRIPT.to_owned(),
                    "benchmark-reaper-overlap".to_owned(),
                    own_marker.display().to_string(),
                    other_marker.display().to_string(),
                ],
                stdin: Vec::new(),
                current_dir: None,
                timeout: Duration::from_secs(5),
                output_limit: 1024,
                rss_limit: None,
                retain_output: false,
            })
        })
    })
    .collect::<Vec<_>>();

    for handle in handles {
        let outcome = handle
            .join()
            .expect("concurrent worker measurement thread")
            .expect("healthy worker measurement");
        assert_eq!(outcome.status, MeasuredStatus::Exited);
        assert_eq!(outcome.exit_code, Some(0));
        assert!(outcome.peak_rss_bytes.is_some_and(|rss| rss > 0));
    }
}
