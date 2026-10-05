//! Windows target measurement, hosted only by the isolated worker.

use std::{
    fs::{File, OpenOptions},
    io,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use super::{
    BenchmarkInvocation, MeasureError, MeasuredOutcome, MeasuredStatus, RssProvenance,
    capture_windows::{CaptureSink, CaptureWorker, FIRST_UNSET, StreamCounts},
    native_process_windows::{EXIT_POLL, NativeChild, RSS_SAMPLE_INTERVAL},
    pipe_windows::OutputPipe,
    report::MeasurementProtocol,
    worker::{PreparedPaths, WorkerControl},
};

/// Target cleanup is driven to completion here, even after a caller deadline.
/// The coordinator retains this worker in its deferred lifecycle executor.
#[allow(
    clippy::too_many_lines,
    reason = "keep exact-child accounting and capture cleanup ordered"
)]
pub(super) async fn measure_target(
    invocation: &BenchmarkInvocation,
    paths: &PreparedPaths,
    control: &WorkerControl,
    sample_tree: bool,
) -> Result<MeasuredOutcome, MeasureError> {
    if control.cancelled()? {
        return Err(MeasureError::Cancelled);
    }
    let stdout_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&paths.stdout)?;
    let stderr_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&paths.stderr)?;
    measure_target_with_sinks(
        invocation,
        paths,
        control,
        sample_tree,
        CaptureSink::File(stdout_file),
        CaptureSink::File(stderr_file),
    )
    .await
}

#[allow(
    clippy::too_many_lines,
    reason = "keep target accounting before explicitly owned capture cleanup"
)]
pub(super) async fn measure_target_with_sinks(
    invocation: &BenchmarkInvocation,
    paths: &PreparedPaths,
    control: &WorkerControl,
    sample_tree: bool,
    stdout_sink: CaptureSink,
    stderr_sink: CaptureSink,
) -> Result<MeasuredOutcome, MeasureError> {
    if control.cancelled()? {
        return Err(MeasureError::Cancelled);
    }
    let stdin = File::open(&paths.stdin)?;
    let out = Arc::new(StreamCounts::default());
    let err = Arc::new(StreamCounts::default());
    let first = Arc::new(AtomicU64::new(FIRST_UNSET));
    let mut stdout_task = CaptureWorker::prepare(stdout_sink, Arc::clone(&out))?;
    let mut stderr_task = CaptureWorker::prepare(stderr_sink, Arc::clone(&err))?;
    let (stdout, stdout_child) = OutputPipe::prepare().await?.into_parts();
    let (stderr, stderr_child) = OutputPipe::prepare().await?.into_parts();
    let mut command = tokio::process::Command::new(&invocation.executable);
    command
        .args(&invocation.args)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::from(stdout_child))
        .stderr(Stdio::from(stderr_child));
    if let Some(directory) = &invocation.current_dir {
        command.current_dir(directory);
    }
    let prepared = NativeChild::prepare(command)?;
    let started = Instant::now();
    let mut owner = prepared.spawn()?;
    stdout_task.start(
        stdout,
        Some(Arc::clone(&first)),
        invocation.output_limit,
        started,
    );
    stderr_task.start(stderr, None, invocation.output_limit, started);
    let mut forced = None;
    let mut failure = None;
    let mut cancelled = false;
    let sampling_requested = sample_tree && invocation.rss_limit.is_some();
    let mut sampled_peak: Option<u64> = None;
    let mut next_sample = Instant::now();
    let status = loop {
        match owner.observe_exit() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                failure.get_or_insert(MeasureError::Io(error));
            }
        }
        match control.cancelled() {
            Ok(true) => {
                cancelled = true;
            }
            Ok(false) => {}
            Err(error) => {
                failure.get_or_insert(MeasureError::Io(error));
            }
        }
        if out.failed.load(Ordering::Acquire) || err.failed.load(Ordering::Acquire) {
            failure.get_or_insert(MeasureError::Io(io::Error::other("capture failed")));
        }
        if forced.is_none() && !cancelled && failure.is_none() {
            if out.limited.load(Ordering::Acquire) || err.limited.load(Ordering::Acquire) {
                forced = Some(MeasuredStatus::OutputLimit);
            } else if started.elapsed() >= invocation.timeout {
                forced = Some(MeasuredStatus::Timeout);
            } else if sampling_requested && Instant::now() >= next_sample {
                match owner.sampled_tree_working_set() {
                    Ok(0) => match owner.observe_exit() {
                        Ok(Some(status)) => break status,
                        Ok(None) => {
                            failure = Some(MeasureError::RssSamplerUnavailable(
                                "no positive job working set for a live target".to_owned(),
                            ));
                        }
                        Err(error) => {
                            failure = Some(MeasureError::Io(error));
                        }
                    },
                    Ok(bytes) => {
                        sampled_peak = Some(sampled_peak.map_or(bytes, |peak| peak.max(bytes)));
                        if invocation.rss_limit.is_some_and(|limit| bytes > limit) {
                            forced = Some(MeasuredStatus::RssLimit);
                        }
                    }
                    Err(error) => {
                        failure = Some(MeasureError::RssSamplerUnavailable(error.to_string()));
                    }
                }
                next_sample = Instant::now() + RSS_SAMPLE_INTERVAL;
            }
        }
        if (forced.is_some() || cancelled || failure.is_some())
            && let Err(error) = owner.terminate()
        {
            failure.get_or_insert(MeasureError::Io(error));
        }
        tokio::time::sleep(EXIT_POLL).await;
    };
    let wall_time_micros = started.elapsed().as_micros();
    let resources = owner.resources();
    // Collect before releasing the accounting handle, then kill remaining
    // descendants. Neither tree cleanup nor pipe draining extends wall time.
    loop {
        if let Err(error) = owner.terminate() {
            failure.get_or_insert(MeasureError::Io(error));
        }
        match owner.tree_is_empty() {
            Ok(true) => break,
            Ok(false) => {}
            Err(error) => {
                failure.get_or_insert(MeasureError::Io(error));
            }
        }
        tokio::time::sleep(EXIT_POLL).await;
    }
    control.mark_target_collected();
    let drain = forced.is_none() && !cancelled && failure.is_none();
    let (stdout_result, stderr_result) =
        tokio::join!(stdout_task.finish(drain), stderr_task.finish(drain));
    let result = (|| {
        let resources = resources?;
        stdout_result?;
        stderr_result?;
        if cancelled {
            return Err(MeasureError::Cancelled);
        }
        if let Some(error) = failure {
            return Err(error);
        }
        let output_bytes = out.bytes.load(Ordering::Acquire);
        let stderr_bytes = err.bytes.load(Ordering::Acquire);
        let first = first.load(Ordering::Acquire);
        let first_result_micros = if output_bytes == 0 {
            None
        } else if first == FIRST_UNSET {
            Some(wall_time_micros)
        } else {
            Some(u128::from(first).min(wall_time_micros))
        };
        let measured_status =
            if out.limited.load(Ordering::Acquire) || err.limited.load(Ordering::Acquire) {
                MeasuredStatus::OutputLimit
            } else if invocation
                .rss_limit
                .is_some_and(|limit| resources.peak_working_set > limit)
            {
                MeasuredStatus::RssLimit
            } else {
                forced.unwrap_or(MeasuredStatus::Exited)
            };
        Ok(MeasuredOutcome {
            status: measured_status, exit_code: status.code(), signal: None,
            wall_time_micros, first_result_micros,
            user_cpu_micros: Some(resources.user.as_micros()),
            system_cpu_micros: Some(resources.system.as_micros()),
            peak_rss_bytes: Some(resources.peak_working_set),
            rss_provenance: RssProvenance::WindowsPeakWorkingSet,
            measurement_protocol: MeasurementProtocol {
                timing_method: format!("windows-peak-working-set direct spawn to exact-child exit observation; retained-handle GetProcessTimes/GetProcessMemoryInfo; overlapped named-pipe capture with bounded owned file-writer queues; first stdout byte capture observation; no accuracy inferred from timer storage; {}", if sampling_requested { "target Job Object working set sampled every 25000 micros for live enforcement" } else { "OS peak working set checked at exit; no in-flight RSS enforcement" }),
                input_delivery: "prepared-seekable-stdin-file".to_owned(),
                rss_scope: "windows-exact-target-process-lifetime peak working set including threads, excluding descendants; optional sampled target Job Object working set".to_owned(),
                exit_poll_interval_micros: 100,
                rss_poll_interval_micros: sampling_requested.then_some(25_000),
                validated_accuracy_micros: None, worker: None, isolation_evidence: None,
            },
            process_group_peak_rss_bytes: sampled_peak,
            output_bytes, stderr_bytes, stdout_path: None, stderr_path: None,
            process_group_rss_observed: sampled_peak.is_some_and(|peak| peak > 0),
        })
    })();
    result.map_err(|source| MeasureError::Collection {
        source: Box::new(source),
        exit_code: status.code(),
        signal: None,
        wall_time_micros: Some(wall_time_micros),
        stdout_path: paths.stdout.clone(),
        stderr_path: paths.stderr.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmark::native_process_windows::{open_process, process_has_exited};
    use std::{
        fs,
        sync::{atomic::AtomicBool, mpsc},
        time::Duration,
    };

    struct ReleaseOnDrop(Option<mpsc::Sender<()>>);
    impl Drop for ReleaseOnDrop {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() {
                let _ = sender.send(());
            }
        }
    }

    #[test]
    fn blocked_capture_writer_does_not_stall_target_timeout_or_cancellation() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            for cancel in [false, true] {
                blocked_writer_case(cancel).await;
            }
        });
    }

    async fn blocked_writer_case(cancel: bool) {
        let directory = tempfile::tempdir().unwrap();
        let paths = PreparedPaths {
            stdin: directory.path().join("stdin"),
            stdout: directory.path().join("stdout"),
            stderr: directory.path().join("stderr"),
            reply: directory.path().join("reply"),
        };
        File::create(&paths.stdin).unwrap();
        let target_pid = directory.path().join("target-pid");
        let escaped_pid_path = target_pid.display().to_string().replace('\'', "''");
        let invocation = BenchmarkInvocation {
            cancellation: None,
            executable: "powershell.exe".into(),
            args: vec![
                "-NoProfile".into(),
                "-Command".into(),
                format!(
                    "Set-Content -LiteralPath '{escaped_pid_path}' -Value $PID; Write-Output blocked; Start-Sleep -Seconds 30"
                ),
            ],
            stdin: Vec::new(),
            current_dir: None,
            timeout: if cancel {
                Duration::from_secs(20)
            } else {
                Duration::from_secs(1)
            },
            output_limit: 1024,
            rss_limit: None,
            retain_output: true,
        };
        let entered = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::channel();
        let sink = CaptureSink::Blocked {
            file: File::create(&paths.stdout).unwrap(),
            entered: Arc::clone(&entered),
            release: receiver,
        };
        let control = WorkerControl::for_test(&paths.reply).unwrap();
        let mut measurement = std::pin::pin!(measure_target_with_sinks(
            &invocation,
            &paths,
            &control,
            false,
            sink,
            CaptureSink::File(File::create(&paths.stderr).unwrap()),
        ));
        // Declared after the future so panic unwinding releases the writer
        // before CaptureWorker's ownership-preserving destructor joins it.
        let mut release = ReleaseOnDrop(Some(sender));
        let deadline = Instant::now() + Duration::from_secs(5);
        while !entered.load(Ordering::Acquire) {
            tokio::select! {
                result = &mut measurement => panic!("measurement ended before writer blocked: {result:?}"),
                () = tokio::time::sleep(EXIT_POLL) => {}
            }
            assert!(
                Instant::now() < deadline,
                "capture writer was never entered"
            );
        }
        let pid = fs::read_to_string(&target_pid)
            .unwrap()
            .trim()
            .trim_start_matches('\u{feff}')
            .parse::<u32>()
            .unwrap();
        let target = open_process(pid).unwrap();
        if cancel {
            fs::write(paths.reply.with_extension("abort"), []).unwrap();
        }
        while !process_has_exited(&target).unwrap() {
            tokio::select! {
                result = &mut measurement => panic!("measurement escaped owned blocked writer: {result:?}"),
                () = tokio::time::sleep(EXIT_POLL) => {}
            }
            assert!(
                Instant::now() < deadline,
                "blocked writer stalled target termination"
            );
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut measurement)
                .await
                .is_err(),
            "blocked file writer lost lifecycle ownership"
        );
        release.0.take().unwrap().send(()).unwrap();
        assert_blocked_writer_outcome(cancel, measurement.await);
    }

    fn assert_blocked_writer_outcome(cancel: bool, result: Result<MeasuredOutcome, MeasureError>) {
        if cancel {
            let Err(MeasureError::Collection {
                source,
                exit_code,
                wall_time_micros,
                ..
            }) = result
            else {
                panic!("cancellation lost its exact-child diagnostics: {result:?}");
            };
            assert!(matches!(*source, MeasureError::Cancelled));
            assert!(exit_code.is_some());
            assert!(wall_time_micros.is_some());
        } else {
            let outcome = result.unwrap();
            assert_eq!(outcome.status, MeasuredStatus::Timeout);
            assert!(outcome.user_cpu_micros.is_some());
            assert!(outcome.system_cpu_micros.is_some());
            assert!(outcome.peak_rss_bytes.unwrap() > 0);
        }
    }
}
