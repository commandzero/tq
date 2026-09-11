//! Controlled date, environment, clock, and input-source process contracts.

use std::{
    fs,
    io::Write,
    process::{Command, Output, Stdio},
};

use tempfile::tempdir;

fn run(args: &[&str], input: &[u8], environment: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tq"));
    command
        .args(args)
        .env_remove("HOME")
        .env_remove("JQ_LIBRARY_PATH");
    for (key, value) in environment {
        command.env(key, value);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq date/platform process");
    child
        .stdin
        .take()
        .expect("date/platform stdin")
        .write_all(input)
        .expect("write date/platform input");
    child
        .wait_with_output()
        .expect("collect date/platform output")
}

#[test]
fn controlled_date_environment_clock_and_source_metadata_match_contracts() {
    let environment = run(
        &[
            "-n",
            "--output-format",
            "json",
            "-c",
            r"[$ENV.TQ_DATE_PLATFORM_SENTINEL, (now | type)]",
        ],
        b"",
        &[("TQ_DATE_PLATFORM_SENTINEL", "controlled-environment")],
    );
    assert!(environment.status.success(), "{environment:?}");
    assert_eq!(
        environment.stdout,
        br#"["controlled-environment","number"]
"#
    );
    assert!(environment.stderr.is_empty());

    let date = run(
        &[
            "--output-format",
            "json",
            "-c",
            "fromdateiso8601 | todateiso8601",
        ],
        b"\"2020-01-02T03:04:05Z\"\n",
        &[],
    );
    assert!(date.status.success(), "{date:?}");
    assert_eq!(date.stdout, b"\"2020-01-02T03:04:05Z\"\n");
    assert!(date.stderr.is_empty());

    let directory = tempdir().expect("source metadata directory");
    let input = directory.path().join("source.json");
    fs::write(&input, b"null\n").expect("source metadata input");
    let input_name = input.to_string_lossy().into_owned();
    let source = run(
        &[
            "--output-format",
            "json",
            "-c",
            "[input_filename, input_line_number]",
            &input_name,
        ],
        b"",
        &[],
    );
    assert!(source.status.success(), "{source:?}");
    assert_eq!(
        source.stdout,
        format!("[{:?},1]\n", input.display()).as_bytes()
    );
    assert!(source.stderr.is_empty());
}
