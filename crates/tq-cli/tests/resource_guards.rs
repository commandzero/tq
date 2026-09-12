//! CLI-level early-consumer guards keep large lazy branches bounded.

use std::{
    io::Write,
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

fn run_with_deadline(args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn resource guard process");
    let mut stdin = child.stdin.take().expect("resource guard stdin");
    stdin.write_all(input).expect("write resource guard input");
    drop(stdin);
    wait_with_deadline(child, Duration::from_secs(2))
}

fn wait_with_deadline(mut child: Child, timeout: Duration) -> Output {
    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait().expect("poll resource guard").is_some() {
            return child
                .wait_with_output()
                .expect("collect resource guard output");
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("lazy consumer exceeded deadline");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn first_assignment_stops_before_an_unbounded_rhs_tail() {
    let output = run_with_deadline(
        &[
            "--input-format",
            "json",
            "--output-format",
            "json",
            "--compact-output",
            "--max-vm-steps",
            "1000",
            "first(.a = (1, range(1000000000)))",
        ],
        b"{}\n",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"{\"a\":1}\n");
}

#[test]
fn first_walk_stops_before_an_unbounded_recursive_tail() {
    let output = run_with_deadline(
        &[
            "--input-format",
            "json",
            "--output-format",
            "json",
            "--compact-output",
            "--max-vm-steps",
            "1000",
            "first(walk((., range(1000000000))))",
        ],
        b"null\n",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"null\n");
}
