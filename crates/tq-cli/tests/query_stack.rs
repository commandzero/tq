//! Bounded CLI main-stack regressions for literal comma trees.

use std::{
    io::Write,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const DOCUMENTED_ARITIES: &str = include_str!("fixtures/documented-arities.jq");

fn assert_query_output(query: &str, expected: &[u8]) {
    let home = tempfile::tempdir().expect("controlled home creates");
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-ijson", "-ojson", "-c", query])
        .env("HOME", home.path())
        .env_remove("JQ_LIBRARY_PATH")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn arity query");
    child
        .stdin
        .take()
        .expect("query stdin")
        .write_all(b"null\n")
        .expect("write query input");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if child.try_wait().expect("query status").is_some() {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("comma-tree query exceeded its 5-second deadline");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().expect("collect query output");
    assert!(
        output.status.success(),
        "status={}\nstdout={}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert_eq!(output.stdout, expected);
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[test]
fn documented_arities_fit_the_cli_main_stack() {
    assert_query_output(DOCUMENTED_ARITIES.trim(), b"[]\n");
}

#[test]
fn composed_documented_arities_fit_the_cli_main_stack() {
    assert_query_output(
        &format!(
            "def __manual_composition_witness: {}; __manual_composition_witness",
            DOCUMENTED_ARITIES.trim(),
        ),
        b"[]\n",
    );
}

fn assert_right_nested_commas(length: usize) {
    assert!(length <= tq_cli::ResourceLimits::default().depth);
    let mut query = length.to_string();
    for value in (1..length).rev() {
        query = format!("{value},({query})");
    }
    let values = (1..=length).collect::<Vec<_>>();
    let mut expected = serde_json::to_vec(&values).expect("expected array serializes");
    expected.push(b'\n');
    assert_query_output(&format!("[{query}]"), &expected);
}

#[test]
fn right_nested_commas_fit_the_cli_main_stack() {
    assert_right_nested_commas(32);
}

#[test]
fn right_nested_commas_44_fit_the_cli_main_stack() {
    assert_right_nested_commas(44);
}

#[test]
fn right_nested_commas_256_fit_the_cli_main_stack() {
    assert_right_nested_commas(256);
}
