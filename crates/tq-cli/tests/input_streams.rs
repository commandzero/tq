//! Process-level jq input cursor and JSON decoder contracts.

use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use tempfile::tempdir;
use tq_formats::InputFormat;

struct Outcome {
    code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn tq(arguments: &[&str], stdin: &[u8]) -> Outcome {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin)
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait for tq");
    Outcome {
        code: output.status.code().expect("ordinary exit"),
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

fn tq_fragments(arguments: &[&str], fragments: &[&[u8]]) -> Outcome {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq");
    let mut stdin = child.stdin.take().expect("stdin");
    for fragment in fragments {
        stdin.write_all(fragment).expect("write stdin fragment");
        stdin.flush().expect("flush stdin fragment");
    }
    drop(stdin);
    let output = child.wait_with_output().expect("wait for tq");
    Outcome {
        code: output.status.code().expect("ordinary exit"),
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

#[test]
fn input_and_inputs_share_the_ordered_cursor() {
    let output = tq(&["-ijson", "-ojsonl", "[., input, inputs]"], b"1 2 3\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[1,2,3]\n");
}

#[test]
fn managed_input_sources_apply_depth_limits_before_materialization() {
    let document = tq(
        &["-ijson", "-ojsonl", "--max-depth", "1", "[., inputs]"],
        b"0\n[[1]]\n",
    );
    assert_eq!(document.code, 5);
    assert!(document.stdout.is_empty());
    assert!(String::from_utf8_lossy(&document.stderr).contains("depth"));

    let sequence = tq(
        &["--seq", "-c", "--max-depth", "1", "."],
        b"\x1e0\n\x1e[[1]]\n",
    );
    assert_eq!(sequence.code, 5);
    assert_eq!(sequence.stdout, b"\x1e0\n");
    assert!(String::from_utf8_lossy(&sequence.stderr).contains("depth"));
}

#[test]
fn managed_input_sources_apply_token_limits_before_materialization() {
    let document = tq(
        &["-ijson", "-ojsonl", "--max-token-bytes", "1", "[., inputs]"],
        b"0\n123\n",
    );
    assert_eq!(document.code, 5);
    assert!(document.stdout.is_empty());
    assert!(String::from_utf8_lossy(&document.stderr).contains("token-bytes"));

    let sequence = tq(
        &["--seq", "-c", "--max-token-bytes", "1", "."],
        b"\x1e0\n\x1e123\n",
    );
    assert_eq!(sequence.code, 5);
    assert_eq!(sequence.stdout, b"\x1e0\n");
    assert!(String::from_utf8_lossy(&sequence.stderr).contains("token-bytes"));
}

#[test]
fn null_input_suppresses_only_the_initial_pull() {
    let output = tq(&["-n", "-ijson", "-ojsonl", "[., inputs]"], b"1 2\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[null,1,2]\n");
}

#[test]
fn input_line_number_tracks_null_and_physical_values() {
    let output = tq(
        &[
            "--allow-platform",
            "-n",
            "-ijson",
            "-ojsonl",
            "[input_line_number, input, input_line_number, input]",
        ],
        b"\n\n1 2\n",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[0,1,3,2]\n");
}

#[test]
fn input_line_number_matches_multiline_and_adjacent_json_values() {
    let multiline = tq(
        &[
            "--allow-platform",
            "-ijson",
            "-ojsonl",
            "[input_line_number, ., inputs]",
        ],
        b"[\n1\n]\n",
    );
    assert_eq!(
        multiline.code,
        0,
        "{}",
        String::from_utf8_lossy(&multiline.stderr)
    );
    assert_eq!(multiline.stdout, b"[3,[1]]\n");

    let adjacent = tq(
        &[
            "--allow-platform",
            "-ijson",
            "-ojsonl",
            "[input_line_number, ., inputs]",
        ],
        b"1 2\n3\n",
    );
    assert_eq!(
        adjacent.code,
        0,
        "{}",
        String::from_utf8_lossy(&adjacent.stderr)
    );
    assert_eq!(adjacent.stdout, b"[1,1,2,3]\n");
}

#[test]
fn input_eof_is_an_error() {
    let output = tq(&["-n", "-ijson", "-ojsonl", "input"], b"");
    assert_ne!(output.code, 0);
    assert!(String::from_utf8_lossy(&output.stderr).contains("break"));
    assert!(output.stdout.is_empty());
}

#[cfg(unix)]
#[test]
fn finite_input_consumer_exits_before_stdin_eof() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-n", "-ijson", "-ojsonl", "input"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"1\n")
        .expect("write one value without closing stdin");

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(status) = child.try_wait().expect("poll tq") {
            assert!(status.success(), "tq exited with {status}");
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("finite input consumer waited for stdin EOF");
        }
        thread::sleep(Duration::from_millis(10));
    }

    let output = child.wait_with_output().expect("collect tq output");
    assert_eq!(output.stdout, b"1\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn null_input_metadata_and_empty_input_eof_match_jq() {
    let output = tq(
        &[
            "--allow-platform",
            "-n",
            "-ijson",
            "-ojsonl",
            "[input_line_number, try input catch .]",
        ],
        b"",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[0,\"break\"]\n");
}

#[test]
fn raw_input_null_mode_uses_the_shared_cursor() {
    let output = tq(&["-Rn", "-ojsonl", "[inputs]"], b"a\nb\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[\"a\",\"b\"]\n");

    let output = tq(&["-Rn", "-ojsonl", "[input,inputs]"], b"a\nb\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[\"a\",\"b\"]\n");

    let output = tq(&["-R", "-ojsonl", "[input,inputs]"], b"a\nb\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[\"b\"]\n");
}

#[test]
fn slurp_null_mode_exposes_one_shared_array_input() {
    let output = tq(&["-sn", "-ijson", "-ojsonl", "[input,inputs]"], b"1 2\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[[1,2]]\n");
}

#[test]
fn remaining_files_share_order_and_source_metadata() {
    let directory = tempdir().expect("temporary directory");
    let first = directory.path().join("first.json");
    let second = directory.path().join("second.json");
    fs::write(&first, "[\n1\n]\n").expect("first input");
    fs::write(&second, "2\n").expect("second input");
    let output = tq(
        &[
            "--allow-platform",
            "-ojsonl",
            "[input_filename, input_line_number, input, input_filename, input_line_number]",
            first.to_str().expect("first path"),
            second.to_str().expect("second path"),
        ],
        b"",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let expected = format!("[\"{}\",3,2,\"{}\",1]\n", first.display(), second.display());
    assert_eq!(output.stdout, expected.as_bytes());
}

#[test]
fn json_whitespace_streams_work_with_strict_and_automatic_detection() {
    let input = b"{\"a\":1} \n\t {\"a\":2}\n";
    let strict = ["-ijson", "-ojsonl", ".a"];
    let automatic = ["-ojsonl", ".a"];
    for arguments in [&strict[..], &automatic[..]] {
        let output = tq(arguments, input);
        assert_eq!(
            output.code,
            0,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"1\n2\n");
    }
}

#[test]
fn automatic_detection_accepts_mixed_scalar_json_whitespace_streams() {
    let output = tq(&["-c", "."], b"1 true [2]\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"1\ntrue\n[2]\n");
}

#[test]
fn automatic_toon_headers_keep_their_priority() {
    let output = tq(&["-c", "."], b"items[2]:\n  - x: 1\n  - x: 2\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"{\"items\":[{\"x\":1},{\"x\":2}]}\n");
}

#[test]
fn automatic_detection_preserves_fragmented_root_toon_arrays_and_json_arrays() {
    let cases = [
        ("[2]: 1,2\n", InputFormat::Toon, b"[1,2]\n".as_slice()),
        (
            "[2]{a}:\n  1\n  2\n",
            InputFormat::Toon,
            b"[{\"a\":1},{\"a\":2}]\n".as_slice(),
        ),
        ("[1,2]\n", InputFormat::Json, b"[1,2]\n".as_slice()),
    ];
    for (input, explicit_format, expected) in cases {
        let (first, rest) = input.as_bytes().split_at(1);
        let automatic = tq_fragments(&["-c", "."], &[first, rest]);
        assert_eq!(
            automatic.code,
            0,
            "{}",
            String::from_utf8_lossy(&automatic.stderr)
        );
        assert_eq!(automatic.stdout, expected);

        let format = match explicit_format {
            InputFormat::Json => "json",
            InputFormat::Toon => "toon",
            _ => unreachable!(),
        };
        let explicit = tq(&["-i", format, "-c", "."], input.as_bytes());
        assert_eq!(
            explicit.code,
            0,
            "{}",
            String::from_utf8_lossy(&explicit.stderr)
        );
        assert_eq!(explicit.stdout, expected);
    }
}

#[test]
fn cursor_metadata_survives_automatic_document_evaluation() {
    let output = tq(
        &[
            "--allow-platform",
            "-i",
            "json",
            "-ojsonl",
            "input_line_number",
        ],
        b"1\n2\n",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"1\n2\n");
}

#[test]
fn strict_json_stream_errors_after_preserving_prior_output() {
    let output = tq(&["-ijson", "-ojsonl", ".a"], b"{\"a\":1} junk\n{\"a\":2}\n");
    assert_ne!(output.code, 0);
    assert_eq!(output.stdout, b"1\n");
    assert!(String::from_utf8_lossy(&output.stderr).contains("input"));
}

#[test]
fn stream_errors_enable_streaming_and_preserve_error_event_order() {
    let output = tq(
        &["--stream-errors", "-ijson", "-ojson", "-c", "."],
        b"[1, bad, 2]\n",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        b"[[0],1]\n[\"Invalid numeric literal at line 1, column 8\",[1]]\n"
    );
}

#[test]
fn stream_reduce_consumes_event_values_through_inputs() {
    let output = tq(
        &[
            "-n",
            "--stream",
            "--input-format",
            "json",
            "--output-format",
            "json",
            "--compact-output",
            "reduce inputs as $event (0; . + 1)",
        ],
        b"[1,2]\n",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"3\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn json_sequence_uses_rs_framing_only_for_json_output() {
    let output = tq(&["--seq", "-c", "."], b"\x1e1\n\x1e{\"a\":2}\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"\x1e1\n\x1e{\"a\":2}\n");
}

#[test]
fn json_sequence_explicit_json_output_keeps_rs_prefixes() {
    let output = tq(&["--seq", "-o", "json", "."], b"\x1e1\n\x1e{\"a\":2}\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"\x1e1\n\x1e{\n  \"a\": 2\n}\n");
}

#[test]
fn json_sequence_input_is_independent_of_toon_output_selection() {
    let input = b"\x1e1\n\x1e2\n";
    for arguments in [
        &["--seq", "-i", "json", "."][..],
        &["--seq", "-i", "json", "-o", "toon", "."][..],
    ] {
        let output = tq(arguments, input);
        assert_eq!(
            output.code,
            0,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"\x1e1\n\x1e2\n");
    }
}

#[test]
fn json_sequence_recovers_at_the_next_record_separator() {
    let output = tq(&["--seq", "-c", "."], b"\x1e1\n\x1ebad\n\x1e2\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"\x1e1\n\x1e2\n");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Invalid numeric literal at line 3, column 0 (need RS to resync)")
    );
}

#[test]
fn json_sequence_matches_jq_structured_recovery_diagnostics() {
    for (input, expected) in [
        (
            b"\x1e{\"a\":}\n\x1e2\n".as_slice(),
            "Unmatched '}' at line 1, column 7 (need RS to resync)",
        ),
        (
            b"\x1e[1,]\n\x1e2\n".as_slice(),
            "Expected another array element at line 1, column 5 (need RS to resync)",
        ),
        (
            b"\x1e{\"a\" 1}\n\x1e2\n".as_slice(),
            "Expected separator between values at line 1, column 8 (need RS to resync)",
        ),
    ] {
        let output = tq(&["--seq", "-c", "."], input);
        assert_eq!(
            output.code,
            0,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"\x1e2\n");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!("tq: ignoring parse error: {expected}\n")
        );
    }
}

#[test]
fn json_sequence_ignores_prefix_noise_and_reports_truncated_numbers() {
    let prefixed = tq(&["--seq", "-c", "."], b"noise\x1e1\n\x1e2\n");
    assert_eq!(
        prefixed.code,
        0,
        "{}",
        String::from_utf8_lossy(&prefixed.stderr)
    );
    assert_eq!(prefixed.stdout, b"\x1e1\n\x1e2\n");

    let truncated = tq(&["--seq", "-c", "."], b"\x1e1");
    assert_eq!(truncated.code, 0);
    assert!(truncated.stdout.is_empty());
    assert!(String::from_utf8_lossy(&truncated.stderr).contains("truncated"));

    let empty = tq(&["--seq", "-c", "."], b"");
    assert_eq!(empty.code, 0);
    assert!(empty.stdout.is_empty());
    assert!(empty.stderr.is_empty());
}

#[test]
fn json_sequence_tracks_physical_line_at_record_end() {
    let output = tq(
        &[
            "--allow-platform",
            "--seq",
            "-i",
            "json",
            "-o",
            "jsonl",
            "-c",
            "input_line_number",
        ],
        b"\x1e[\n1\n]\n\x1e3\n",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"3\n4\n");
}

#[test]
fn json_sequence_slurp_recovers_and_preserves_prior_values() {
    let output = tq(
        &["--seq", "-s", "-i", "json", "-o", "jsonl", "-c", "."],
        b"\x1e1\n\x1ebad\n\x1e2\n",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[1,2]\n");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Invalid numeric literal at line 3, column 0 (need RS to resync)")
    );
}

#[cfg(unix)]
#[test]
fn json_sequence_cursor_returns_before_stdin_eof() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["--seq", "-n", "-i", "json", "-o", "jsonl", "input"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"\x1e1\n")
        .expect("write one sequence record without closing stdin");

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(status) = child.try_wait().expect("poll tq") {
            assert!(status.success(), "tq exited with {status}");
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("JSON sequence cursor waited for stdin EOF");
        }
        thread::sleep(Duration::from_millis(10));
    }

    let output = child.wait_with_output().expect("collect tq output");
    assert_eq!(output.stdout, b"1\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn json_sequence_allows_multiple_values_inside_one_rs_record() {
    let output = tq(&["--seq", "-ijson", "-c", "."], b"\x1e1\n2\n\x1e3\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"\x1e1\n\x1e2\n\x1e3\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn json_sequence_whitespace_terminates_a_numeric_record() {
    let output = tq(&["--seq", "-ijson", "-c", "."], b"\x1e1 \x1e4\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"\x1e1\n\x1e4\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn json_sequence_rejects_unterminated_numeric_after_leading_whitespace() {
    let output = tq(&["--seq", "-ijson", "-c", "."], b"\x1e 123\x1e4\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"\x1e4\n");
    assert!(String::from_utf8_lossy(&output.stderr).contains("truncated"));
}

#[test]
fn json_sequence_rejects_non_json_whitespace_and_malformed_scalar_boundaries() {
    for input in [
        b"\x1etrue\x0bfalse\n\x1e4\n".as_slice(),
        b"\x1e1\x0c2\n\x1e4\n".as_slice(),
        b"\x1etruefalse\n\x1e4\n".as_slice(),
    ] {
        let output = tq(&["--seq", "-ijson", "-o", "jsonl", "-c", "."], input);
        assert_eq!(
            output.code,
            0,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"4\n");
        assert!(String::from_utf8_lossy(&output.stderr).contains("at line"));
    }
    let output = tq(
        &["--seq", "-ijson", "-o", "jsonl", "-c", "."],
        b"\x1e1\n2\n\x1e4\n",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"1\n2\n4\n");
    assert!(output.stderr.is_empty());
}

#[cfg(unix)]
#[test]
fn json_sequence_cursor_emits_complete_object_before_stdin_eof() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["--seq", "-n", "-i", "json", "-o", "jsonl", "input"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"\x1e{\"a\":1}")
        .expect("write complete object without closing stdin");

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(status) = child.try_wait().expect("poll tq") {
            assert!(status.success(), "tq exited with {status}");
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("complete JSON sequence object waited for stdin EOF");
        }
        thread::sleep(Duration::from_millis(10));
    }

    let output = child.wait_with_output().expect("collect tq output");
    assert_eq!(output.stdout, b"{\"a\":1}\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn seq_without_json_output_keeps_native_toon_sequence() {
    let output = tq(&["--seq", "."], b"a: 1\n");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"\x1ea: 1\n");
}
