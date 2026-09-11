//! Automatic JSON root staging and input-boundary resource contracts.

use std::{ffi::OsString, fmt::Write as FmtWrite, fs, io::Cursor};

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
    let command = parse_args(arguments).expect("resource arguments parse");
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

fn vm_step_arguments(budget: u64) -> Vec<String> {
    vec![
        "--input-format".to_owned(),
        "json".to_owned(),
        "--output-format".to_owned(),
        "jsonl".to_owned(),
        "--max-vm-steps".to_owned(),
        budget.to_string(),
        ".features[] | .id + 1".to_owned(),
    ]
}

fn vm_step_observation(budget: u64, input: &[u8]) -> Observation {
    execute(vm_step_arguments(budget), input)
}

#[test]
fn automatic_subtree_vm_steps_are_per_document_root() {
    let analyzed = analyze(
        resolve(
            parse(".features[] | .id + 1").expect("query parses"),
            &ResolveOptions::default(),
        )
        .expect("query resolves"),
    );
    assert_eq!(analyzed.analysis().selected_plan, PlanKind::Subtree);
    assert!(analyzed.analysis().stream_proof.is_some());

    let root = br#"{"features":[{"id":1}]}"#;
    let mut high = 1_u64;
    loop {
        let observation = vm_step_observation(high, root);
        if observation.status == Ok(ExitStatus::Success) {
            break;
        }
        assert_eq!(observation.status, Err(ExitStatus::Resource));
        high = high.saturating_mul(2);
        assert!(
            high <= 4096,
            "small root needs an unexpectedly large budget"
        );
    }

    let mut low = 0_u64;
    while high.saturating_sub(low) > 1 {
        let middle = low + high.saturating_sub(low) / 2;
        let observation = vm_step_observation(middle, root);
        match observation.status {
            Ok(ExitStatus::Success) => high = middle,
            Err(ExitStatus::Resource) => low = middle,
            other => panic!("unexpected one-root budget status: {other:?}"),
        }
    }
    let one = vm_step_observation(high, root);
    assert_eq!(one.status, Ok(ExitStatus::Success));
    assert_eq!(one.stdout, b"2\n");

    let two_roots = br#"{"features":[{"id":1}]} {"features":[{"id":1}]}"#;
    let two = vm_step_observation(high, two_roots);
    assert_eq!(two.status, Ok(ExitStatus::Success));
    assert_eq!(two.stdout, b"2\n2\n");
}

#[test]
fn automatic_root_stage_handles_small_preparation_memory() {
    let count = 1_000_usize;
    let mut input = String::from(r#"{"features":["#);
    let mut expected = String::new();
    for index in 0..count {
        if index != 0 {
            input.push(',');
        }
        write!(input, r#"{{"id":{index}}}"#).expect("input string write");
        writeln!(expected, "{index}").expect("expected string write");
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
            ".features[] | .id".to_owned(),
        ],
        input.as_bytes(),
    );
    assert_eq!(observation.status, Ok(ExitStatus::Success));
    assert_eq!(observation.stdout, expected.as_bytes());

    let report = serde_json::from_slice::<serde_json::Value>(
        &fs::read(report_file.path()).expect("read retention report"),
    )
    .expect("parse retention report");
    assert_eq!(report["execution"]["plan"], "subtree");
    assert!(
        report["execution"]["retention_high_water"]["runtime_spool_fixed_io_bytes"]
            .as_u64()
            .is_some_and(|bytes| bytes > 0),
        "small preparation memory should report fixed I/O headroom after root staging spills"
    );
}

#[test]
fn automatic_object_index_has_a_bounded_capacity_budget() {
    let make_input = |count: usize| {
        let mut input = String::from(r#"{"features":{"#);
        let mut expected = String::new();
        for index in 0..count {
            if index != 0 {
                input.push(',');
            }
            write!(input, r#""k{index}":{{"id":{index}}}"#).expect("input string write");
            writeln!(expected, "{index}").expect("expected string write");
        }
        input.push_str("}}");
        (input, expected)
    };

    let (small_input, small_expected) = make_input(64);
    let small = execute(
        vec![
            "--input-format".to_owned(),
            "json".to_owned(),
            "--output-format".to_owned(),
            "jsonl".to_owned(),
            "--prepare-memory-bytes".to_owned(),
            "131072".to_owned(),
            "--max-spool-bytes".to_owned(),
            "1048576".to_owned(),
            ".features[].id".to_owned(),
        ],
        small_input.as_bytes(),
    );
    assert_eq!(small.status, Ok(ExitStatus::Success));
    assert_eq!(small.stdout, small_expected.as_bytes());

    let (large_input, _) = make_input(5_000);
    let large = execute(
        vec![
            "--input-format".to_owned(),
            "json".to_owned(),
            "--output-format".to_owned(),
            "jsonl".to_owned(),
            "--prepare-memory-bytes".to_owned(),
            "1048576".to_owned(),
            "--max-spool-bytes".to_owned(),
            "16777216".to_owned(),
            ".features[].id".to_owned(),
        ],
        large_input.as_bytes(),
    );
    assert_eq!(large.status, Err(ExitStatus::Resource));
    assert!(large.stdout.is_empty());
}

#[test]
fn json_lines_rejects_adjacent_roots_on_one_physical_line() {
    let analyzed = analyze(
        resolve(
            parse(".features[].id").expect("query parses"),
            &ResolveOptions::default(),
        )
        .expect("query resolves"),
    );
    assert_eq!(analyzed.analysis().selected_plan, PlanKind::Subtree);
    assert!(analyzed.analysis().stream_proof.is_some());

    let invalid = execute(
        [
            "--input-format",
            "jsonl",
            "--output-format",
            "jsonl",
            ".features[].id",
        ],
        br#"{"features":[{"id":1}]} {"features":[{"id":2}]}
"#,
    );
    assert_eq!(invalid.status, Err(ExitStatus::Input));
    assert!(invalid.stdout.is_empty());

    let valid = execute(
        [
            "--input-format",
            "jsonl",
            "--output-format",
            "jsonl",
            ".features[].id",
        ],
        br#"{"features":[{"id":1}]}
{"features":[{"id":2}]}
"#,
    );
    assert_eq!(valid.status, Ok(ExitStatus::Success));
    assert_eq!(valid.stdout, b"1\n2\n");

    let whitespace_json = execute(
        [
            "--input-format",
            "json",
            "--output-format",
            "jsonl",
            ".features[].id",
        ],
        br#"{"features":[{"id":1}]} {"features":[{"id":2}]}
"#,
    );
    assert_eq!(whitespace_json.status, Ok(ExitStatus::Success));
    assert_eq!(whitespace_json.stdout, b"1\n2\n");
}
