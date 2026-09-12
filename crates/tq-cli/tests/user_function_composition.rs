//! Public process tests for composed constructors and slices.

use std::{
    io::Write,
    process::{Command, Stdio},
};

use tempfile::tempdir;

fn run(query: &str, input: &[u8]) -> std::process::Output {
    let home = tempdir().expect("controlled home creates");
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-o", "toon", "--unframed", query])
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
