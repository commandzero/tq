//! Process effects that do not belong in the structured output stream.

use std::process::{Command, Stdio};

struct Output {
    status: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run(query: &str, input: &[u8], extra: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(extra)
        .args(["-c", query])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq");
    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(input)
        .expect("write input");
    let output = child.wait_with_output().expect("wait for tq");
    Output {
        status: output.status.code().expect("normal exit"),
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

use std::io::Write;

#[test]
fn debug_and_stderr_keep_effects_off_structured_stdout() {
    let debug = run("debug", b"42\n", &[]);
    assert_eq!(debug.status, 0);
    assert_eq!(debug.stdout, b"42\n");
    assert_eq!(debug.stderr, b"[\"DEBUG:\",42]\n");

    let stderr = run("stderr", b"42\n", &[]);
    assert_eq!(stderr.status, 0);
    assert_eq!(stderr.stdout, b"42\n");
    assert_eq!(stderr.stderr, b"42");
}

#[test]
fn debug_message_generators_are_consumed_once_and_preserve_order() {
    let output = run("debug(1,2)", b"null\n", &[]);
    assert_eq!(output.status, 0);
    assert_eq!(output.stdout, b"null\n");
    assert_eq!(output.stderr, b"[\"DEBUG:\",1]\n[\"DEBUG:\",2]\n");
}

#[test]
fn halt_error_preserves_prior_results_and_uses_raw_stderr() {
    let output = run("1,halt_error(7)", b"null\n", &[]);
    assert_eq!(output.status, 7);
    assert_eq!(output.stdout, b"1\n");
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[test]
fn halt_error_status_conversion_matches_jq_for_edge_numbers() {
    for (query, status) in [
        ("halt_error(7)", 7),
        ("halt_error(-1)", 0),
        ("halt_error(256)", 0),
        ("halt_error(300)", 44),
        ("halt_error(3.5)", 3),
        ("halt_error(nan)", 0),
        ("halt_error(infinite)", 255),
    ] {
        let output = run(query, &[], &["-n"]);
        assert_eq!(output.status, status, "{query}");
        assert!(output.stdout.is_empty(), "{query}");
        assert!(output.stderr.is_empty(), "{query}");
    }
}

#[test]
fn halt_error_without_an_argument_terminates_with_the_input_payload() {
    let output = run("halt_error", b"\"payload\"\n", &[]);
    assert_eq!(output.status, 5);
    assert_eq!(output.stdout, [] as [u8; 0]);
    assert_eq!(output.stderr, b"payload");
}

#[test]
fn halt_error_payload_matches_jq_for_null_and_non_null_inputs() {
    for (input, stderr) in [
        (b"null\n".as_slice(), b"".as_slice()),
        (b"\"abc\"\n".as_slice(), b"abc".as_slice()),
        (b"3\n".as_slice(), b"3\n".as_slice()),
        (b"true\n".as_slice(), b"true\n".as_slice()),
        (br#"{"x":1}"#.as_slice(), b"{\"x\":1}\n".as_slice()),
    ] {
        let output = run("halt_error(7)", input, &[]);
        assert_eq!(output.status, 7);
        assert_eq!(output.stdout, [] as [u8; 0]);
        assert_eq!(output.stderr, stderr);
    }
}

#[test]
fn halt_is_not_catchable_and_does_not_evaluate_later_branches() {
    let output = run("try halt catch \"caught\"", b"null\n", &[]);
    assert_eq!(output.status, 0);
    assert_eq!(output.stdout, [] as [u8; 0]);
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[test]
fn effects_are_emitted_in_event_execution_too() {
    let output = run("debug", b"42\n", &["--stream"]);
    assert_eq!(output.status, 0);
    assert_eq!(output.stdout, b"[[],42]\n");
    assert_eq!(output.stderr, b"[\"DEBUG:\",[[],42]]\n");
}

#[test]
fn effect_sink_enforces_the_vm_output_envelope() {
    let output = run(
        "range(0;100) | debug | empty",
        &[],
        &["-n", "--max-output-bytes", "32"],
    );
    assert_eq!(output.status, 5);
    assert!(String::from_utf8_lossy(&output.stderr).contains("output-bytes"));
}

#[cfg(unix)]
#[test]
fn unbuffered_effects_arrive_before_remaining_input_reaches_eof() {
    use std::{io::Read, sync::mpsc, thread, time::Duration};

    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args([
            "-n",
            "--unbuffered",
            "-c",
            r#"debug("ready"), inputs | empty"#,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn live effect process");
    let stdin = child.stdin.take().expect("live stdin");
    let mut stderr = child.stderr.take().expect("live stderr");
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut byte = [0_u8; 1];
        let result = stderr.read(&mut byte).map(|count| (count, byte[0]));
        let _ = sender.send(result);
    });
    let debug_result = match receiver.recv_timeout(Duration::from_secs(2)) {
        Ok(result) => result.expect("read debug effect"),
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("debug effect before stdin EOF: {error}");
        }
    };
    assert_eq!(debug_result.0, 1);
    drop(stdin);
    let output = child
        .wait_with_output()
        .expect("collect live effect process");
    assert_eq!(output.status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn unbuffered_stdout_reaches_a_live_consumer_before_stdin_eof() {
    use std::{
        io::{BufRead, BufReader},
        sync::mpsc,
        thread,
        time::Duration,
    };

    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["--unbuffered", "-c", "."])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn live stdout process");
    let mut stdin = child.stdin.take().expect("live stdin");
    let stdout = child.stdout.take().expect("live stdout");
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut first = String::new();
        let first_result = reader.read_line(&mut first);
        let _ = sender.send((first_result, first));
        let mut second = String::new();
        let second_result = reader.read_line(&mut second);
        let _ = sender.send((second_result, second));
    });
    stdin.write_all(b"1\n").expect("write first live value");
    let (result, first) = match receiver.recv_timeout(Duration::from_secs(2)) {
        Ok(value) => value,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("first stdout result before stdin EOF: {error}");
        }
    };
    assert_eq!(result.expect("read first stdout result"), 2);
    assert_eq!(first, "1\n");

    stdin.write_all(b"2\n").expect("write second live value");
    let (result, second) = match receiver.recv_timeout(Duration::from_secs(2)) {
        Ok(value) => value,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("second stdout result before stdin EOF: {error}");
        }
    };
    assert_eq!(result.expect("read second stdout result"), 2);
    assert_eq!(second, "2\n");

    drop(stdin);
    let output = child
        .wait_with_output()
        .expect("collect live stdout process");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[cfg(unix)]
#[test]
fn unbuffered_empty_effect_does_not_wait_for_a_delivery_acknowledgement() {
    let output = run("\"\" | stderr", b"", &["-n", "--unbuffered"]);
    assert_eq!(output.status, 0);
    assert_eq!(output.stdout, b"\"\"\n");
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[cfg(unix)]
fn assert_exits_quickly(child: &mut std::process::Child) {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait().expect("poll child") {
            Some(_) => return,
            None if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            None => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("unbuffered child did not exit after its output failed");
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn unbuffered_stdout_failure_stops_before_an_open_input_read() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-n", "--unbuffered", "-c", "1, inputs"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn failing stdout process");
    let _stdin = child.stdin.take().expect("keep stdin open");
    drop(child.stdout.take());

    assert_exits_quickly(&mut child);
}

#[cfg(unix)]
#[test]
fn unbuffered_stderr_failure_stops_before_an_open_input_read() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-n", "--unbuffered", "-c", "debug, inputs"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn failing stderr process");
    let _stdin = child.stdin.take().expect("keep stdin open");
    drop(child.stderr.take());

    assert_exits_quickly(&mut child);
}
