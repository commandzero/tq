//! Slurp proxying preserves the complete ordered source set.

use std::{fs, path::Path};

use tempfile::tempdir;
use tq_cli::{ExitStatus, parse_args, run_with_io};

fn run_slurp(files: &[&Path]) -> (ExitStatus, Vec<u8>, Vec<u8>) {
    let mut arguments = vec![
        "-x".to_owned(),
        "-s".to_owned(),
        "-ijson".to_owned(),
        "-ojsonl".to_owned(),
        ".".to_owned(),
    ];
    arguments.extend(files.iter().map(|path| path.display().to_string()));
    let command = parse_args(arguments).expect("slurp proxy arguments parse");
    let mut input = &[][..];
    let mut output = Vec::new();
    let mut error = Vec::new();
    let status =
        run_with_io(command, &mut input, &mut output, &mut error).expect("slurp proxy execution");
    (status, output, error)
}

#[test]
fn rejected_slurp_source_proxies_all_sources_in_both_orders() {
    let directory = tempdir().expect("temporary source directory");
    let valid = directory.path().join("valid.json");
    let rejected = directory.path().join("rejected.json");
    fs::write(&valid, b"{\"id\":1}\n").expect("valid source write");
    fs::write(&rejected, b"not-json\n").expect("rejected source write");

    for (files, expected) in [
        (
            vec![valid.as_path(), rejected.as_path()],
            b"{\"id\":1}\nnot-json\n".as_slice(),
        ),
        (
            vec![rejected.as_path(), valid.as_path()],
            b"not-json\n{\"id\":1}\n".as_slice(),
        ),
    ] {
        let (status, output, error) = run_slurp(&files);
        assert_eq!(status, ExitStatus::Success);
        assert_eq!(output, expected);
        assert_eq!(error, [] as [u8; 0]);
    }
}

#[test]
fn valid_slurp_sources_still_form_one_array() {
    let directory = tempdir().expect("temporary source directory");
    let first = directory.path().join("first.json");
    let second = directory.path().join("second.json");
    fs::write(&first, b"{\"id\":1}\n").expect("first source write");
    fs::write(&second, b"{\"id\":2}\n").expect("second source write");

    let (status, output, error) = run_slurp(&[first.as_path(), second.as_path()]);
    assert_eq!(status, ExitStatus::Success);
    assert_eq!(output, b"[{\"id\":1},{\"id\":2}]\n");
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn slurp_proxy_does_not_turn_resource_failures_into_success() {
    let command = parse_args([
        "-x",
        "-s",
        "-ijson",
        "-ojsonl",
        "--max-input-bytes",
        "3",
        ".",
    ])
    .expect("resource-limited slurp arguments parse");
    let mut input = b"null\n".as_slice();
    let mut output = Vec::new();
    let error = run_with_io(command, &mut input, &mut output, &mut Vec::new())
        .expect_err("source limit remains fatal under proxy");
    assert_eq!(error.status(), ExitStatus::Resource);
    assert_eq!(output, [] as [u8; 0]);
}

#[test]
fn slurp_proxy_keeps_null_and_raw_input_modes_separate() {
    let command = parse_args(["-x", "-s", "-n", "-ijson", "-ojsonl", "."])
        .expect("null-input slurp arguments parse");
    let mut input = &[][..];
    let mut output = Vec::new();
    let status = run_with_io(command, &mut input, &mut output, &mut Vec::new())
        .expect("null-input slurp execution");
    assert_eq!(status, ExitStatus::Success);
    assert_eq!(output, b"null\n");

    let command =
        parse_args(["-x", "-s", "-R", "-ojsonl", "."]).expect("raw-input slurp arguments parse");
    let mut input = b"a\nb\n".as_slice();
    let mut output = Vec::new();
    let status = run_with_io(command, &mut input, &mut output, &mut Vec::new())
        .expect("raw-input slurp execution");
    assert_eq!(status, ExitStatus::Success);
    assert_eq!(output, b"\"a\\nb\\n\"\n");
}
