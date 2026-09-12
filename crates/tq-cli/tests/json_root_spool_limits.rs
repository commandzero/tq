//! Automatic JSON root staging honors the disk-spool budget independently of memory.

use std::{ffi::OsString, fmt::Write as _, fs, io::Cursor};

use tempfile::NamedTempFile;
use tq_cli::{ExitStatus, parse_args, run_with_io};
use tq_core::{PlanKind, ResolveOptions, analyze, parse, resolve};

#[derive(Debug, Eq, PartialEq)]
struct Observation {
    status: Result<ExitStatus, ExitStatus>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn execute<I, S>(arguments: I, input: &[u8]) -> Observation
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let command = parse_args(arguments).expect("spool-limit arguments parse");
    let mut input = Cursor::new(input.to_vec());
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status =
        run_with_io(command, &mut input, &mut stdout, &mut stderr).map_err(|error| error.status());
    Observation {
        status,
        stdout,
        stderr,
    }
}

fn assert_subtree_proof(query: &str) {
    let analyzed = analyze(
        resolve(
            parse(query).expect("query parses"),
            &ResolveOptions::default(),
        )
        .expect("query resolves"),
    );
    assert_eq!(analyzed.analysis().selected_plan, PlanKind::Subtree);
    assert!(analyzed.analysis().stream_proof.is_some());
}

fn many_items(count: usize) -> String {
    let mut input = String::from(r#"{"features":["#);
    for index in 0..count {
        if index != 0 {
            input.push(',');
        }
        write!(input, r#"{{"id":{index}}}"#).expect("input string write");
    }
    input.push_str("]}");
    input
}

#[test]
fn zero_disk_budget_accepts_a_memory_fit_root() {
    let query = ".features[].id";
    assert_subtree_proof(query);
    let observation = execute(
        [
            "--input-format",
            "json",
            "--output-format",
            "jsonl",
            "--prepare-memory-bytes",
            "4096",
            "--max-spool-bytes",
            "0",
            query,
        ],
        br#"{"features":[{"id":1}]}"#,
    );

    assert_eq!(observation.status, Ok(ExitStatus::Success));
    assert_eq!(observation.stdout, b"1\n");
}

#[test]
fn zero_disk_budget_rejects_a_forced_spill_without_publishing_the_root() {
    let query = ".features[].id";
    assert_subtree_proof(query);
    let observation = execute(
        vec![
            "--input-format".to_owned(),
            "json".to_owned(),
            "--output-format".to_owned(),
            "jsonl".to_owned(),
            "--prepare-memory-bytes".to_owned(),
            "2048".to_owned(),
            "--max-spool-bytes".to_owned(),
            "0".to_owned(),
            query.to_owned(),
        ],
        many_items(1_000).as_bytes(),
    );

    assert_eq!(observation.status, Err(ExitStatus::Resource));
    assert_eq!(observation.stdout, [] as [u8; 0]);
}

#[test]
fn large_first_scalar_spills_before_replaying_a_small_tail() {
    let query = ".features[].id";
    assert_subtree_proof(query);
    let first = "x".repeat(800);
    let mut input = String::from(r#"{"features":["#);
    write!(input, r#"{{"id":"{first}"}}"#).expect("first input string write");
    let mut expected = String::new();
    writeln!(expected, "\"{first}\"").expect("first expected string write");
    for index in 0..40 {
        write!(input, r#",{{"id":{index}}}"#).expect("tail input string write");
        writeln!(expected, "{index}").expect("tail expected string write");
    }
    input.push_str("]}");

    let report_file = NamedTempFile::new().expect("create retention report path");
    let observation = execute(
        vec![
            "--input-format".to_owned(),
            "json".to_owned(),
            "--output-format".to_owned(),
            "jsonl".to_owned(),
            "--prepare-memory-bytes".to_owned(),
            "2048".to_owned(),
            "--max-spool-bytes".to_owned(),
            "1048576".to_owned(),
            "--report-file".to_owned(),
            report_file.path().display().to_string(),
            query.to_owned(),
        ],
        input.as_bytes(),
    );

    assert_eq!(observation.status, Ok(ExitStatus::Success));
    assert_eq!(observation.stdout, expected.as_bytes());
    let report = serde_json::from_slice::<serde_json::Value>(
        &fs::read(report_file.path()).expect("read retention report"),
    )
    .expect("parse retention report");
    assert!(
        report["execution"]["retention_high_water"]["root_staging"]["spool_bytes_written"]
            .as_u64()
            .is_some_and(|bytes| bytes > 0),
        "large first scalar should force root staging to disk before replay"
    );
}

#[test]
fn zero_disk_budget_allows_multiple_memory_fit_roots() {
    let query = ".features[].id";
    assert_subtree_proof(query);
    let observation = execute(
        [
            "--input-format",
            "json",
            "--output-format",
            "jsonl",
            "--prepare-memory-bytes",
            "4096",
            "--max-spool-bytes",
            "0",
            query,
        ],
        br#"{"features":[{"id":1}]} {"features":[{"id":2}]} {"features":[{"id":3}]}"#,
    );

    assert_eq!(observation.status, Ok(ExitStatus::Success));
    assert_eq!(observation.stdout, b"1\n2\n3\n");
}
