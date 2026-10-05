//! Bounded CLI main-stack regressions for the exact 220-signature manual witnesses.

use std::{
    io::Write,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const DOCUMENTED_ARITIES: &str = include_str!("fixtures/documented-arities.jq");

fn assert_arity_query(query: &str) {
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
            panic!("220-signature arity query exceeded its 5-second deadline");
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
    assert_eq!(output.stdout, b"[]\n");
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[test]
fn documented_arities_fit_the_cli_main_stack() {
    assert_arity_query(DOCUMENTED_ARITIES.trim());
}

#[test]
fn composed_documented_arities_fit_the_cli_main_stack() {
    assert_arity_query(&format!(
        "def __manual_composition_witness: {}; __manual_composition_witness",
        DOCUMENTED_ARITIES.trim(),
    ));
}
