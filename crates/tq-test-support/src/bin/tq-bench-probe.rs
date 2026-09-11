//! Small, deterministic subprocesses used by benchmark-harness tests.
//!
//! Behavior modes expose an exit status, output, allocation, or lifetime.
//! Measurement modes also exercise the shared collector for isolation tests.

use std::{
    env,
    io::{self, Write as _},
    process::{Command, ExitCode, Stdio},
    thread,
    time::Duration,
};

use tq_test_support::benchmark::run_allocation_probe;

const DEFAULT_BLOCKED_INPUT_MILLIS: u64 = 10_000;
const DEFAULT_DESCENDANT_MILLIS: u64 = 10_000;
const OUTPUT_CHUNK_BYTES: usize = 64 * 1024;

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let Some(mut mode) = arguments.next() else {
        print_error(usage());
        return ExitCode::from(64);
    };
    // Accept Cargo's conventional separator when the binary is invoked in a
    // command line copied from `cargo run --bin tq-bench-probe -- MODE`.
    if mode == "--" {
        let Some(next_mode) = arguments.next() else {
            print_error(usage());
            return ExitCode::from(64);
        };
        mode = next_mode;
    }

    match run_mode(&mode, &mut arguments) {
        Ok(status) => status,
        Err(error) => {
            print_error(error);
            ExitCode::from(64)
        }
    }
}

fn run_mode(mode: &str, arguments: &mut impl Iterator<Item = String>) -> Result<ExitCode, String> {
    match mode {
        "measure-worker-parent" => {
            let parent_bytes = parse_bytes(arguments.next(), "parent allocation bytes")?;
            let input_bytes = arguments
                .next()
                .map(|value| parse_bytes(Some(value), "stdin bytes"))
                .transpose()?
                .unwrap_or(0);
            ensure_no_arguments(arguments)?;
            measured_worker_parent(parent_bytes, input_bytes)
        }
        "-axo" => {
            if arguments.next().as_deref() != Some("pgid=,rss=") {
                return Err("empty inspection probe requires pgid=,rss=".to_owned());
            }
            ensure_no_arguments(arguments)?;
            thread::sleep(Duration::from_millis(20));
            Ok(ExitCode::SUCCESS)
        }
        "measure-noop"
        | "measure-limit"
        | "measure-uninstrumented-noop"
        | "measure-sleep-limit" => {
            ensure_no_arguments(arguments)?;
            measured_probe(mode)
        }
        "help" | "--help" => {
            ensure_no_arguments(arguments)?;
            println!("{}", usage());
            Ok(ExitCode::SUCCESS)
        }
        "noop" | "no-op" | "empty" => ensure_no_arguments(arguments).map(|()| ExitCode::SUCCESS),
        "allocate" | "allocate-burst" | "burst" | "memory" | "release" => {
            let bytes = parse_bytes(arguments.next(), "allocation bytes")?;
            let threads = parse_allocate_options(arguments)?;
            allocate_and_release(bytes, threads)
        }
        "allocate-threads" | "threads" => {
            let bytes = parse_bytes(arguments.next(), "allocation bytes")?;
            let threads = parse_count(arguments.next(), "allocation thread count")?;
            ensure_no_arguments(arguments)?;
            allocate_and_release(bytes, threads)
        }
        "waited-allocation-child" => {
            let bytes = parse_bytes(arguments.next(), "allocation bytes")?;
            ensure_no_arguments(arguments)?;
            waited_allocation_child(bytes)
        }
        "sleep" | "sleep-ms" | "duration" | "known-duration" | "descendant-child" => {
            let millis = parse_millis(arguments.next())?;
            ensure_no_arguments(arguments)?;
            thread::sleep(Duration::from_millis(millis));
            Ok(ExitCode::SUCCESS)
        }
        "args" | "literal-args" | "literal" => write_literal_arguments(arguments),
        "stdin-echo" | "echo-stdin" | "stdin" | "echo" => echo_stdin(arguments),
        "exit" => {
            let code = parse_exit_code(arguments.next())?;
            ensure_no_arguments(arguments)?;
            Ok(ExitCode::from(code))
        }
        "nonzero" | "nonzero-exit" | "exit-nonzero" | "fail" => {
            let code = parse_exit_code(arguments.next())?;
            if code == 0 {
                return Err("nonzero exit code must be between 1 and 255".to_owned());
            }
            ensure_no_arguments(arguments)?;
            Ok(ExitCode::from(code))
        }
        "blocked-input" | "block-input" | "block-stdin" | "blocked" | "hang" => {
            let millis = arguments
                .next()
                .map(|value| parse_number(Some(value), "blocked-input duration"))
                .transpose()?
                .unwrap_or(DEFAULT_BLOCKED_INPUT_MILLIS);
            ensure_no_arguments(arguments)?;
            thread::sleep(Duration::from_millis(millis));
            Ok(ExitCode::SUCCESS)
        }
        "flood-output" | "output-flood" | "flood" | "output" => {
            let bytes = parse_bytes(arguments.next(), "output bytes")?;
            ensure_no_arguments(arguments)?;
            flood_output(bytes)
        }
        "self-signal" | "signal" => {
            let signal = arguments.next();
            ensure_no_arguments(arguments)?;
            self_signal(signal)
        }
        "descendant" | "spawn-descendant" | "spawn-child" => {
            let millis = arguments
                .next()
                .map(|value| parse_number(Some(value), "descendant duration"))
                .transpose()?
                .unwrap_or(DEFAULT_DESCENDANT_MILLIS);
            ensure_no_arguments(arguments)?;
            spawn_descendant(millis)
        }
        _ => Err(format!("unknown probe mode {mode:?}\n\n{}", usage())),
    }
}

fn measured_probe(mode: &str) -> Result<ExitCode, String> {
    let invocation = tq_test_support::benchmark::BenchmarkInvocation {
        cancellation: None,
        executable: env::current_exe().map_err(|error| error.to_string())?,
        args: if mode == "measure-sleep-limit" {
            vec!["sleep".to_owned(), "500".to_owned()]
        } else {
            vec!["noop".to_owned()]
        },
        stdin: Vec::new(),
        current_dir: None,
        timeout: Duration::from_secs(2),
        output_limit: 1024,
        rss_limit: (mode != "measure-noop").then_some(if mode == "measure-limit" {
            u64::MAX
        } else {
            1024 * 1024 * 1024
        }),
        retain_output: false,
    };
    let outcome = if mode == "measure-uninstrumented-noop" {
        tq_test_support::benchmark::measure_process_uninstrumented(&invocation)
    } else {
        tq_test_support::benchmark::measure_process(&invocation)
    }
    .map_err(|error| error.to_string())?;
    if outcome.exit_code != Some(0) {
        return Err("measured child failed".to_owned());
    }
    Ok(ExitCode::SUCCESS)
}

fn measured_worker_parent(parent_bytes: usize, input_bytes: usize) -> Result<ExitCode, String> {
    use tq_test_support::benchmark::{BenchmarkInvocation, measure_process_worker};

    // Keep touched coordinator pages alive across the actual public measurement
    // call. Allocation before worker startup is the Linux regression trigger.
    let mut parent = vec![0_u8; parent_bytes];
    for page in parent.chunks_mut(4096) {
        page[0] = 1;
    }
    let invocation = BenchmarkInvocation {
        cancellation: None,
        executable: env::current_exe().map_err(|error| error.to_string())?,
        args: vec![
            if input_bytes == 0 {
                "noop"
            } else {
                "stdin-echo"
            }
            .to_owned(),
        ],
        stdin: vec![b'q'; input_bytes],
        current_dir: None,
        timeout: Duration::from_secs(10),
        output_limit: u64::try_from(input_bytes)
            .ok()
            .and_then(|bytes| bytes.checked_add(1024))
            .ok_or_else(|| "input size exceeds the output-limit range".to_string())?,
        rss_limit: None,
        retain_output: input_bytes > 0,
    };
    let outcome = measure_process_worker(&invocation).map_err(|error| error.to_string())?;
    std::hint::black_box(&parent);
    if outcome.exit_code != Some(0) {
        return Err(format!("worker target failed: {:?}", outcome.status));
    }
    if input_bytes > 0 {
        let path = outcome
            .stdout_path
            .as_ref()
            .ok_or("missing retained stdout")?;
        let actual = std::fs::read(path).map_err(|error| error.to_string())?;
        if actual != invocation.stdin {
            return Err("worker changed prepared stdin bytes".to_owned());
        }
        for path in [&outcome.stdout_path, &outcome.stderr_path]
            .into_iter()
            .flatten()
        {
            std::fs::remove_file(path).map_err(|error| error.to_string())?;
        }
    }
    println!(
        "{}",
        serde_json::to_string(&outcome).map_err(|error| error.to_string())?
    );
    Ok(ExitCode::SUCCESS)
}

fn allocate_and_release(bytes: usize, threads: usize) -> Result<ExitCode, String> {
    run_allocation_probe(bytes, threads)
        .map(|()| ExitCode::SUCCESS)
        .map_err(|error| error.to_string())
}

fn waited_allocation_child(bytes: usize) -> Result<ExitCode, String> {
    let executable =
        env::current_exe().map_err(|error| format!("find allocation child executable: {error}"))?;
    let mut child = Command::new(executable)
        .args(["allocate-burst", &bytes.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("spawn waited allocation child: {error}"))?;
    let status = child
        .wait()
        .map_err(|error| format!("wait for allocation child: {error}"))?;
    if !status.success() {
        return Err(format!("allocation child exited with {status}"));
    }
    Ok(ExitCode::SUCCESS)
}

fn parse_allocate_options(arguments: &mut impl Iterator<Item = String>) -> Result<usize, String> {
    let mut threads = 1;
    while let Some(option) = arguments.next() {
        match option.as_str() {
            "--burst" | "burst" => {}
            "--threads" | "threads" => {
                threads = parse_count(arguments.next(), "allocation thread count")?;
            }
            value => {
                // Accept a bare second number as a compact spelling for the
                // per-thread allocation mode used by preflight experiments.
                if threads != 1 {
                    return Err(format!("unexpected allocation option {value:?}"));
                }
                threads = parse_count(Some(value.to_owned()), "allocation thread count")?;
            }
        }
    }
    Ok(threads)
}

fn write_literal_arguments(
    arguments: &mut impl Iterator<Item = String>,
) -> Result<ExitCode, String> {
    let mut stdout = io::stdout().lock();
    for argument in arguments {
        stdout
            .write_all(argument.as_bytes())
            .and_then(|()| stdout.write_all(b"\n"))
            .map_err(|error| format!("write literal arguments: {error}"))?;
    }
    stdout
        .flush()
        .map_err(|error| format!("flush literal arguments: {error}"))?;
    Ok(ExitCode::SUCCESS)
}

fn echo_stdin(arguments: &mut impl Iterator<Item = String>) -> Result<ExitCode, String> {
    ensure_no_arguments(arguments)?;
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    io::copy(&mut stdin, &mut stdout).map_err(|error| format!("echo stdin: {error}"))?;
    stdout
        .flush()
        .map_err(|error| format!("flush echoed stdin: {error}"))?;
    Ok(ExitCode::SUCCESS)
}

fn flood_output(bytes: usize) -> Result<ExitCode, String> {
    let chunk = vec![b'x'; OUTPUT_CHUNK_BYTES];
    let mut remaining = bytes;
    let mut stdout = io::stdout().lock();
    while remaining > 0 {
        let length = remaining.min(chunk.len());
        stdout
            .write_all(&chunk[..length])
            .map_err(|error| format!("write flood output: {error}"))?;
        remaining -= length;
    }
    stdout
        .flush()
        .map_err(|error| format!("flush flood output: {error}"))?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(unix)]
fn self_signal(signal_name: Option<String>) -> Result<ExitCode, String> {
    use nix::{sys::signal::Signal, unistd::Pid};

    let signal_name = signal_name.unwrap_or_else(|| "term".to_owned());
    let signal_name = signal_name.to_ascii_lowercase();
    let signal = match signal_name.as_str() {
        "hup" | "sighup" | "1" => Signal::SIGHUP,
        "int" | "sigint" | "2" => Signal::SIGINT,
        "quit" | "sigquit" | "3" => Signal::SIGQUIT,
        "abrt" | "sigabrt" | "6" => Signal::SIGABRT,
        "term" | "sigterm" | "15" => Signal::SIGTERM,
        "usr1" | "sigusr1" | "10" => Signal::SIGUSR1,
        "usr2" | "sigusr2" | "12" => Signal::SIGUSR2,
        "kill" | "sigkill" | "9" => Signal::SIGKILL,
        other => return Err(format!("unsupported self-signal {other:?}")),
    };
    nix::sys::signal::kill(Pid::this(), signal)
        .map_err(|error| format!("send {signal:?} to self: {error}"))?;
    // A terminating signal should end the process before this point returns.
    // Keep a successful return available for unusual signal handlers without
    // introducing a busy loop into the probe.
    Ok(ExitCode::SUCCESS)
}

#[cfg(not(unix))]
fn self_signal(_signal_name: Option<String>) -> Result<ExitCode, String> {
    Err("self-signal is supported only on Unix".to_owned())
}

fn spawn_descendant(millis: u64) -> Result<ExitCode, String> {
    let executable =
        env::current_exe().map_err(|error| format!("find probe executable: {error}"))?;
    let child = Command::new(executable)
        .args(["descendant-child", &millis.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("spawn descendant: {error}"))?;

    // The PID is intentionally observable so a lifecycle test can verify
    // that killing/cleaning the measured process also handles its descendant.
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{}", child.id())
        .and_then(|()| stdout.flush())
        .map_err(|error| format!("write descendant pid: {error}"))?;
    Ok(ExitCode::SUCCESS)
}

fn parse_bytes(value: Option<String>, label: &str) -> Result<usize, String> {
    parse_number(value, label).and_then(|number| {
        usize::try_from(number).map_err(|_| format!("{label} does not fit in usize: {number}"))
    })
}

fn parse_count(value: Option<String>, label: &str) -> Result<usize, String> {
    let count = parse_bytes(value, label)?;
    if count == 0 {
        return Err(format!("{label} must be greater than zero"));
    }
    Ok(count)
}

fn parse_millis(value: Option<String>) -> Result<u64, String> {
    parse_number(value, "duration in milliseconds")
}

fn parse_exit_code(value: Option<String>) -> Result<u8, String> {
    let value = value.ok_or_else(|| "missing exit code".to_owned())?;
    value
        .parse::<u8>()
        .map_err(|_| format!("exit code must be an integer from 0 to 255: {value:?}"))
}

fn parse_number(value: Option<String>, label: &str) -> Result<u64, String> {
    let value = value.ok_or_else(|| format!("missing {label}"))?;
    value
        .parse::<u64>()
        .map_err(|_| format!("{label} must be a non-negative integer: {value:?}"))
}

fn ensure_no_arguments(arguments: &mut impl Iterator<Item = String>) -> Result<(), String> {
    if let Some(unexpected) = arguments.next() {
        return Err(format!("unexpected argument {unexpected:?}\n\n{}", usage()));
    }
    Ok(())
}

fn print_error(error: impl AsRef<str>) {
    eprintln!("tq-bench-probe: {}", error.as_ref());
}

fn usage() -> &'static str {
    "usage: tq-bench-probe MODE [ARGS]\n\n\
modes:\n\
  noop\n\
  allocate BYTES [--threads N] [--burst]\n\
  allocate-threads BYTES THREADS\n\
  waited-allocation-child BYTES\n\
  measure-noop | measure-limit | measure-uninstrumented-noop\n\
  measure-worker-parent PARENT_BYTES [STDIN_BYTES]\n\
  sleep MILLIS\n\
  literal-args ARG...\n\
  stdin-echo\n\
  exit CODE | nonzero CODE\n\
  blocked-input [MILLIS]\n\
  flood-output BYTES\n\
  self-signal [TERM|KILL|INT]\n\
  descendant [MILLIS]"
}
