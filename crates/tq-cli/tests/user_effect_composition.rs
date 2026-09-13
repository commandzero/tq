//! Public CLI contracts for composed input, effects, regex callbacks, and termination.

use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use tempfile::tempdir;

fn run_json(query: &str, input: &[u8], extra: &[&str]) -> Output {
    let home = tempdir().expect("controlled home creates");
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-i", "json", "-c"])
        .args(extra)
        .arg(query)
        .env("HOME", home.path())
        .env_remove("JQ_LIBRARY_PATH")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq");
    let _ = child.stdin.take().expect("tq stdin").write_all(input);
    wait_output_with_deadline(child, Duration::from_secs(2))
}

fn wait_output_with_deadline(mut child: Child, timeout: Duration) -> Output {
    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait().expect("poll tq").is_some() {
            return child.wait_with_output().expect("collect tq output");
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("composed process did not terminate before the deadline");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_status_with_deadline(child: &mut Child, timeout: Duration) -> std::process::ExitStatus {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().expect("poll tq") {
            return status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("composed early consumer did not terminate before the deadline");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn composed_filter_parameter_consumes_each_shared_input_once() {
    let output = run_json(
        "def apply(f): [f, input, inputs]; apply(. + 10)",
        b"1\n2\n3\n",
        &[],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[11,2,3]\n");
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[test]
fn composed_effect_callbacks_preserve_stdout_and_stderr_order() {
    let output = run_json(
        "def emit(f): (f | debug), (f | stderr); emit(.)",
        b"\"x\"\n",
        &[],
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"\"x\"\n\"x\"\n");
    assert_eq!(output.stderr, b"[\"DEBUG:\",\"x\"]\nx");
}

#[test]
fn scalar_effect_order_distinguishes_filter_and_value_parameters() {
    for (query, stdout, stderr) in [
        (
            r#"pow((debug("left")|2);(debug("right")|3))"#,
            "8\n",
            "[\"DEBUG:\",\"right\"]\n[\"DEBUG:\",\"left\"]\n",
        ),
        (
            r#"pow((debug("left")|2);(debug("right")|empty))"#,
            "",
            "[\"DEBUG:\",\"right\"]\n",
        ),
        (
            r#"def p(a;b):pow(a;b); p((debug("left")|2);(debug("right")|3))"#,
            "8\n",
            "[\"DEBUG:\",\"right\"]\n[\"DEBUG:\",\"left\"]\n",
        ),
        (
            r#"def p($a;$b):pow($a;$b); p((debug("left")|2);(debug("right")|3))"#,
            "8\n",
            "[\"DEBUG:\",\"left\"]\n[\"DEBUG:\",\"right\"]\n",
        ),
    ] {
        let output = run_json(query, b"null\n", &[]);
        assert!(output.status.success(), "{query}");
        assert_eq!(output.stdout, stdout.as_bytes(), "{query}");
        assert_eq!(output.stderr, stderr.as_bytes(), "{query}");
    }
}

#[test]
fn composed_effect_identity_distinguishes_debug_from_source_metadata() {
    for (query, expected) in [
        (r"def d: debug(empty); . as $p | d | ($p = 9)", "9\n"),
        (
            r#"def line: input_line_number; . as $p | line | try ($p = 9) catch "caught""#,
            "\"caught\"\n",
        ),
    ] {
        let output = run_json(query, b"1\n", &[]);
        assert!(output.status.success(), "{query}");
        assert_eq!(output.stdout, expected.as_bytes(), "{query}");
        assert!(output.stderr.is_empty(), "{query}");
    }
}

#[test]
fn composed_regex_replacement_keeps_value_scope_and_all_results() {
    let output = run_json(
        r#"def rewrite($suffix): [sub("p"; ("a" + $suffix, "b" + $suffix))]; rewrite("!")"#,
        b"\"p\"\n",
        &[],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[\"a!\",\"b!\"]\n");
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[test]
fn composed_halt_is_not_catchable_and_abandons_later_results() {
    let output = run_json(
        r#"def stop: halt; "before", (try stop catch "caught"), "after""#,
        b"null\n",
        &[],
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"\"before\"\n");
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[test]
fn composed_halt_error_preserves_status_payload_and_prior_output() {
    for (query, status) in [
        (
            r#"def stop: halt_error; "before", ("failure" | try stop catch "caught"), "after""#,
            5,
        ),
        (
            r#"def stop(code): halt_error(code); "before", ("failure" | try stop(7) catch "caught"), "after""#,
            7,
        ),
    ] {
        let output = run_json(query, b"null\n", &[]);
        assert_eq!(output.status.code(), Some(status), "{query}");
        assert_eq!(output.stdout, b"\"before\"\n", "{query}");
        assert_eq!(output.stderr, b"failure", "{query}");
    }
}

#[test]
fn first_composed_branch_skips_late_effects() {
    let output = run_json(
        r#"def later: debug("late") | empty; first(("first", later))"#,
        b"null\n",
        &[],
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"\"first\"\n");
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[cfg(unix)]
#[test]
fn live_first_composed_filter_emits_before_input_eof() {
    let home = tempdir().expect("controlled home creates");
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-n", "-i", "json", "--unbuffered", "-c"])
        .arg("def choose(f): f; first(choose(inputs))")
        .env("HOME", home.path())
        .env_remove("JQ_LIBRARY_PATH")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn live composed process");
    let mut stdin = child.stdin.take().expect("live stdin");
    let stdout = child.stdout.take().expect("live stdout");
    let (sender, receiver) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let result = reader.read_line(&mut line);
        let _ = sender.send((result, line));
    });
    stdin.write_all(b"1\n").expect("write first live value");
    let (result, line) = match receiver.recv_timeout(Duration::from_secs(2)) {
        Ok(value) => value,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("first composed result did not arrive before stdin EOF: {error}");
        }
    };
    if result.as_ref().ok() != Some(&2) || line != "1\n" {
        let _ = child.kill();
        let _ = child.wait();
        panic!("unexpected live composed output: read={result:?}, line={line:?}");
    }

    // Keep stdin open while waiting: this proves `first` completed without EOF.
    let status = wait_status_with_deadline(&mut child, Duration::from_secs(2));
    drop(stdin);
    let mut stderr = Vec::new();
    child
        .stderr
        .take()
        .expect("live stderr")
        .read_to_end(&mut stderr)
        .expect("read live stderr");
    assert!(status.success());
    assert_eq!(stderr, [] as [u8; 0]);
}
