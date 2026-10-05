//! Focused native Windows accounting and worker-lifecycle contracts.

#![cfg(windows)]

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tq_test_support::benchmark::{
    BenchmarkInvocation, MeasureError, MeasuredOutcome, MeasuredStatus, RssProvenance,
    measure_process, measure_process_uninstrumented, measure_process_worker,
    measure_process_worker_uninstrumented,
};

fn invocation(args: &[&str]) -> BenchmarkInvocation {
    BenchmarkInvocation {
        cancellation: None,
        executable: PathBuf::from(env!("CARGO_BIN_EXE_tq-bench-probe")),
        args: args.iter().map(|a| (*a).to_owned()).collect(),
        stdin: Vec::new(),
        current_dir: None,
        timeout: Duration::from_secs(3),
        output_limit: 1024 * 1024,
        rss_limit: None,
        retain_output: false,
    }
}
fn assert_resources(outcome: &MeasuredOutcome) {
    assert_eq!(outcome.rss_provenance, RssProvenance::WindowsPeakWorkingSet);
    assert!(outcome.peak_rss_bytes.expect("OS peak working set") > 0);
    assert!(outcome.user_cpu_micros.is_some());
    assert!(outcome.system_cpu_micros.is_some());
    assert!(outcome.measurement_protocol.worker.is_some());
    assert!(
        outcome
            .measurement_protocol
            .rss_scope
            .contains("excluding descendants")
    );
    assert!(
        outcome
            .measurement_protocol
            .validated_accuracy_micros
            .is_none()
    );
}
fn remove_captures(outcome: &MeasuredOutcome) {
    for path in [&outcome.stdout_path, &outcome.stderr_path]
        .into_iter()
        .flatten()
    {
        std::fs::remove_file(path).expect("remove retained capture");
    }
}
fn cancelled(error: &MeasureError) -> bool {
    match error {
        MeasureError::Cancelled => true,
        MeasureError::Collection { source, .. } => cancelled(source),
        _ => false,
    }
}

// Keep this suite in one test so its explicit concurrent-child case cannot
// collide with other cases at the bounded worker-admission capacity.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "exercise serial native lifecycle contracts without interleaving the global deferred-owner registry"
)]
fn native_windows_exact_handle_accounting_and_worker_cleanup() {
    let mut control_peak = 0;
    for measure in [
        measure_process,
        measure_process_uninstrumented,
        measure_process_worker,
        measure_process_worker_uninstrumented,
    ] {
        let outcome = measure(&invocation(&["noop"])).expect("short exact-child accounting");
        assert_resources(&outcome);
        assert_eq!(outcome.status, MeasuredStatus::Exited);
        assert_eq!(outcome.exit_code, Some(0));
        assert_eq!(outcome.first_result_micros, None);
        control_peak = control_peak.max(outcome.peak_rss_bytes.unwrap());
    }
    for bytes in [8 * 1024 * 1024, 32 * 1024 * 1024] {
        for threads in [1, 4] {
            let outcome = measure_process(&invocation(&[
                "allocate-threads",
                &bytes.to_string(),
                &threads.to_string(),
            ]))
            .expect("freed-before-exit allocation accounting");
            assert_resources(&outcome);
            assert!(outcome.peak_rss_bytes.unwrap().saturating_sub(control_peak) >= bytes / 2);
        }
    }
    let large = std::thread::spawn(|| {
        measure_process(&invocation(&["allocate-burst", "33554432"])).unwrap()
    });
    let small = measure_process(&invocation(&["noop"])).unwrap();
    let large = large.join().unwrap();
    assert_resources(&small);
    assert_resources(&large);
    assert!(large.peak_rss_bytes.unwrap() > small.peak_rss_bytes.unwrap() + 16 * 1024 * 1024);

    let waited = measure_process(&invocation(&["waited-allocation-child", "33554432"])).unwrap();
    assert_resources(&waited);
    assert!(waited.peak_rss_bytes.unwrap() < control_peak + 16 * 1024 * 1024);

    let directory = tempfile::tempdir().unwrap();
    let spaced = directory.path().join("native paths with spaces Ω");
    std::fs::create_dir(&spaced).unwrap();
    let executable = spaced.join("tq bench probe.exe");
    std::fs::copy(env!("CARGO_BIN_EXE_tq-bench-probe"), &executable).unwrap();
    let mut arguments = invocation(&[
        "literal-args",
        "space value",
        "quote\"value",
        "trail\\",
        "Ω",
    ]);
    arguments.executable = executable;
    arguments.current_dir = Some(spaced);
    arguments.retain_output = true;
    let outcome =
        measure_process(&arguments).expect("native executable paths and argument vectors");
    assert_resources(&outcome);
    assert_eq!(
        std::fs::read(outcome.stdout_path.as_ref().unwrap()).unwrap(),
        b"space value\nquote\"value\ntrail\\\n\xce\xa9\n"
    );
    assert!(outcome.first_result_micros.unwrap() <= outcome.wall_time_micros);
    remove_captures(&outcome);

    let mut input = invocation(&["stdin-echo"]);
    input.stdin = b"binary\0input\r\n\xff".to_vec();
    input.retain_output = true;
    let outcome = measure_process(&input).expect("seekable binary stdin delivery");
    assert_eq!(
        std::fs::read(outcome.stdout_path.as_ref().unwrap()).unwrap(),
        input.stdin
    );
    remove_captures(&outcome);

    let cpu = measure_process(&invocation(&["busy-duration", "200"])).expect("CPU accounting");
    assert_resources(&cpu);
    assert!(cpu.user_cpu_micros.unwrap() + cpu.system_cpu_micros.unwrap() > 0);

    let nonzero = measure_process(&invocation(&["exit", "17"])).expect("nonzero accounting");
    assert_resources(&nonzero);
    assert_eq!(nonzero.exit_code, Some(17));
    remove_captures(&nonzero);

    let mut timeout = invocation(&["sleep", "5000"]);
    timeout.timeout = Duration::from_millis(30);
    let started = Instant::now();
    let outcome = measure_process(&timeout).expect("forced-termination accounting");
    assert_resources(&outcome);
    assert_eq!(outcome.status, MeasuredStatus::Timeout);
    assert!(started.elapsed() < Duration::from_secs(3));
    remove_captures(&outcome);

    let mut flood = invocation(&["output", "1048576"]);
    flood.output_limit = 1024;
    let outcome = measure_process(&flood).expect("bounded output accounting");
    assert_resources(&outcome);
    assert_eq!(outcome.status, MeasuredStatus::OutputLimit);
    assert!(outcome.output_bytes > 1024);
    assert!(
        std::fs::metadata(outcome.stdout_path.as_ref().unwrap())
            .unwrap()
            .len()
            <= 1024
    );
    remove_captures(&outcome);

    let mut limited = invocation(&["sleep", "5000"]);
    limited.rss_limit = Some(1);
    let outcome = measure_process(&limited).expect("live Job Object memory enforcement");
    assert_resources(&outcome);
    assert_eq!(outcome.status, MeasuredStatus::RssLimit);
    assert!(outcome.process_group_peak_rss_bytes.unwrap() > 1);
    remove_captures(&outcome);

    let mut descendant = invocation(&["descendant", "5000"]);
    descendant.retain_output = true;
    let started = Instant::now();
    let outcome = measure_process(&descendant).expect("natural-exit descendant cleanup");
    assert_resources(&outcome);
    assert_eq!(outcome.exit_code, Some(0));
    assert!(started.elapsed() < Duration::from_secs(3));
    remove_captures(&outcome);

    let flag = Arc::new(AtomicBool::new(false));
    let mut request = invocation(&["sleep", "5000"]);
    request.cancellation = Some(Arc::clone(&flag));
    let trigger = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        flag.store(true, Ordering::Release);
    });
    let error = measure_process(&request).expect_err("cancellation is not a timeout row");
    trigger.join().unwrap();
    assert!(cancelled(&error), "{error:?}");
    if let MeasureError::Collection {
        stdout_path,
        stderr_path,
        ..
    } = error
    {
        std::fs::remove_file(stdout_path).unwrap();
        std::fs::remove_file(stderr_path).unwrap();
    }
    let outcome = measure_process(&invocation(&["noop"])).expect("cleanup releases admission");
    assert_resources(&outcome);
}
