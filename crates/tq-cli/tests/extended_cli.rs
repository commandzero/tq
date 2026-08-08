//! Executable jq-shaped CLI baselines: argv, input sources, output bytes,
//! diagnostics, and exit status are all observable.

use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

use tempfile::tempdir;

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

#[test]
fn short_clusters_sort_ascii_indent_and_raw_zero_have_stable_bytes() {
    let output = tq(&["--output-format", "json", "-nacS", r#"{z:"µ",a:1}"#], b"");
    assert_eq!(output.code, 0);
    assert_eq!(
        output.stdout,
        br#"{"a":1,"z":"\u00b5"}
"#
    );
    assert!(output.stderr.is_empty());

    let output = tq(&["-n", "--raw-output0", r#""a","b""#], b"");
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"a\0b\0");
    assert!(output.stderr.is_empty());

    let output = tq(&["-n", "--raw-output0", r#""\u0000""#], b"");
    assert_eq!(output.code, 5);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("NUL"));
}

#[test]
fn indentation_and_monochrome_controls_have_stable_output_bytes() {
    let output = tq(
        &[
            "--output-format",
            "json",
            "-n",
            "--indent",
            "4",
            r"{a:{b:1}}",
        ],
        b"",
    );
    assert_eq!(output.code, 0);
    assert_eq!(
        output.stdout,
        b"{\n    \"a\": {\n        \"b\": 1\n    }\n}\n"
    );
    assert!(output.stderr.is_empty());

    let output = tq(
        &["--output-format", "json", "-n", "--tab", r"{a:{b:1}}"],
        b"",
    );
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"{\n\t\"a\": {\n\t\t\"b\": 1\n\t}\n}\n");
    assert!(output.stderr.is_empty());

    let output = tq(&["--output-format", "json", "-nCM", "1"], b"");
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"1\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn named_positional_and_file_arguments_are_bounded_and_ordered() {
    let directory = tempdir().unwrap();
    let raw = directory.path().join("raw.txt");
    let json = directory.path().join("values.json");
    fs::write(&raw, "hello\n").unwrap();
    fs::write(&json, "1\n2\n").unwrap();

    let output = tq(
        &[
            "--output-format",
            "json",
            "-nc",
            "--arg",
            "name",
            "Ada",
            "--rawfile",
            "raw",
            raw.to_str().unwrap(),
            "--slurpfile",
            "values",
            json.to_str().unwrap(),
            "[$name,$ARGS.named.name,$raw,$values]",
        ],
        b"",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[\"Ada\",\"Ada\",\"hello\\n\",[1,2]]\n");

    let output = tq(
        &[
            "--output-format",
            "json",
            "-nc",
            "--args",
            "$ARGS.positional",
            "x",
            "2",
        ],
        b"",
    );
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"[\"x\",\"2\"]\n");

    let output = tq(
        &[
            "--output-format",
            "json",
            "-nc",
            "--jsonargs",
            "$ARGS.positional",
            "1",
            "{\"x\":2}",
        ],
        b"",
    );
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"[1,{\"x\":2}]\n");
}

#[test]
fn yaml_multi_file_and_invalid_combinations_preserve_channels_and_status() {
    let directory = tempdir().unwrap();
    let first = directory.path().join("first.json");
    let second = directory.path().join("second.json");
    fs::write(&first, "{\"n\":1}").unwrap();
    fs::write(&second, "{\"n\":2}").unwrap();

    let output = tq(
        &[
            "--input-format",
            "json",
            "--output-format",
            "yaml",
            ".n",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"1\n---\n2\n");
    assert!(output.stderr.is_empty());

    let output = tq(&["--output-format", "toon", "-a", "."], b"null");
    assert_eq!(output.code, 2);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("incompatible"));
}

#[test]
fn generated_help_and_build_configuration_are_stdout_only() {
    for arguments in [&["--help"][..], &["--build-configuration"][..]] {
        let output = tq(arguments, b"");
        assert_eq!(output.code, 0);
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
    let help = tq(&["--help"], b"");
    let help = String::from_utf8(help.stdout).unwrap();
    for option in ["--raw-output0", "--sort-keys", "--slurpfile", "--jsonargs"] {
        assert!(help.contains(option), "missing {option}");
    }
}

#[test]
fn color_binary_encoding_and_argument_file_limits_are_classified() {
    let output = tq(&["--output-format", "json", "-nC", "1"], b"");
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"\x1b[36m1\x1b[0m\n");

    let output = tq(&["--output-format", "json", "-nbc", "1"], b"");
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"1\n");

    let output = tq(&["-R", "."], &[0xff]);
    assert_eq!(output.code, 5);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("UTF-8"));

    let output = tq(&["-n", "--argjson", "value", "1 2", "$value"], b"");
    assert_eq!(output.code, 5);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("exactly one"));

    let directory = tempdir().unwrap();
    let secret = directory.path().join("secret.txt");
    fs::write(&secret, "do-not-leak").unwrap();
    let output = tq(
        &[
            "-n",
            "--max-input-bytes",
            "2",
            "--rawfile",
            "value",
            secret.to_str().unwrap(),
            "$value",
        ],
        b"",
    );
    assert_eq!(output.code, 5);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("resource limit"));
    assert!(stderr.contains(secret.to_str().unwrap()));
    assert!(!stderr.contains("do-not-leak"));
}

#[cfg(unix)]
#[test]
fn ordered_files_preserve_a_complete_frame_before_a_closed_downstream_pipe() {
    use std::io::Read;

    let directory = tempdir().unwrap();
    let first = directory.path().join("first.json");
    let second = directory.path().join("second.json");
    fs::write(&first, "{\"n\":1}").unwrap();
    fs::write(&second, "{\"n\":2}").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args([
            "--input-format",
            "json",
            "--output-format",
            "json",
            "-c",
            ".n, range(0; 100000000)",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq");
    let mut stdout = child.stdout.take().unwrap();
    let mut complete_first_frame = [0_u8; 2];
    stdout.read_exact(&mut complete_first_frame).unwrap();
    assert_eq!(&complete_first_frame, b"1\n");
    drop(stdout);
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
}
