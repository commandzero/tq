//! Public process tests for composed constructors and slices.

use std::{
    io::Write,
    process::{Command, Stdio},
};

use tempfile::tempdir;

fn run(query: &str, input: &[u8]) -> std::process::Output {
    run_with_args(&["-o", "toon", "--unframed"], query, input)
}

fn run_with_args(args: &[&str], query: &str, input: &[u8]) -> std::process::Output {
    let home = tempdir().expect("controlled home creates");
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(args)
        .arg(query)
        .env("HOME", home.path())
        .env_remove("JQ_LIBRARY_PATH")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq");
    child
        .stdin
        .take()
        .expect("tq stdin")
        .write_all(input)
        .expect("write tq input");
    child.wait_with_output().expect("wait for tq")
}

#[test]
fn cli_executes_composed_object_and_slice_filters() {
    let object = run(
        "def f: {(.key): .value}; f",
        br#"{"key":"name","value":7}
"#,
    );
    assert!(
        object.status.success(),
        "{}",
        String::from_utf8_lossy(&object.stderr)
    );
    assert_eq!(object.stdout, b"name: 7");
    assert_eq!(object.stderr, [] as [u8; 0]);

    let slice = run("def f: .[0.5:2.5]; f", b"[0,1,2,3]\n");
    assert!(
        slice.status.success(),
        "{}",
        String::from_utf8_lossy(&slice.stderr)
    );
    assert_eq!(slice.stdout, b"[3]: 0,1,2");
    assert_eq!(slice.stderr, [] as [u8; 0]);
}

#[test]
fn cli_composes_uri_decode_through_default_toon_output() {
    let output = run_with_args(
        &[],
        "def decode: @urid; map(decode)",
        b"[\"a%2Fb\",\"x%20y\"]\n",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[2]: a/b,x y\n");
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[test]
fn cli_composes_uri_decode_with_explicit_json_and_compact_output() {
    let query = "def decode: @urid; map(decode)";
    let input = b"[\"a%2Fb\",\"x%20y\"]\n";

    let pretty = run_with_args(&["-i", "json", "-o", "json"], query, input);
    assert!(
        pretty.status.success(),
        "{}",
        String::from_utf8_lossy(&pretty.stderr)
    );
    assert_eq!(pretty.stdout, b"[\n  \"a/b\",\n  \"x y\"\n]\n");
    assert_eq!(pretty.stderr, [] as [u8; 0]);

    let compact = run_with_args(&["-i", "json", "-c"], query, input);
    assert!(
        compact.status.success(),
        "{}",
        String::from_utf8_lossy(&compact.stderr)
    );
    assert_eq!(compact.stdout, b"[\"a/b\",\"x y\"]\n");
    assert_eq!(compact.stderr, [] as [u8; 0]);
}

#[test]
fn cli_composed_uri_decode_reports_invalid_encoding_as_runtime_failure() {
    let output = run_with_args(
        &["-i", "json", "-c"],
        "def decode: @urid; map(decode)",
        b"[\"ok\",\"bad%ZZ\"]\n",
    );
    assert_eq!(output.status.code(), Some(5));
    assert_eq!(output.stdout, [] as [u8; 0]);
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("valid uri encoding"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
