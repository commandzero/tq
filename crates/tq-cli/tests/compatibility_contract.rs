//! The compatibility identity contract follows the CLI's generated help.

use std::process::Command;

use tq_test_support::compatibility::{
    ObservationState, ProcessStatus, ToolKind, ToolObservation, TqContract, encode_hex,
    tq_contract_matches,
};

fn current_help() -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .arg("--help")
        .output()
        .expect("run tq --help");
    assert!(output.status.success());
    assert_eq!(output.stderr, [] as [u8; 0]);
    output.stdout
}

fn observation(stdout: &[u8]) -> ToolObservation {
    ToolObservation {
        tool: ToolKind::Tq,
        input_format: None,
        state: ObservationState::Executed,
        results: Vec::new(),
        stdout_hex: None,
        raw_stdout_hex: Some(encode_hex(stdout)),
        stderr_hex: None,
        process_status: Some(ProcessStatus::Exited),
        exit_code: Some(0),
        error_class: None,
        wall_time_micros: None,
        note: None,
    }
}

fn without_once(bytes: &[u8], needle: &[u8]) -> Vec<u8> {
    without_once_after(bytes, &[], needle)
}

fn without_once_after(bytes: &[u8], anchor: &[u8], needle: &[u8]) -> Vec<u8> {
    let search_start = if anchor.is_empty() {
        0
    } else {
        bytes
            .windows(anchor.len())
            .position(|window| window == anchor)
            .expect("anchor in generated help")
            + anchor.len()
    };
    let start = search_start
        + bytes[search_start..]
            .windows(needle.len())
            .position(|window| window == needle)
            .expect("needle in generated help");
    let mut result = bytes.to_vec();
    result.drain(start..start + needle.len());
    result
}

#[test]
fn current_generated_help_satisfies_identity_contract() {
    assert!(tq_contract_matches(
        TqContract::Help,
        &observation(&current_help())
    ));
}

#[test]
fn help_contract_rejects_missing_documented_option() {
    let help = current_help();
    let incomplete = without_once(&help, b"--stream-errors");
    assert!(!tq_contract_matches(
        TqContract::Help,
        &observation(&incomplete)
    ));
}

#[test]
fn help_contract_rejects_missing_documented_format() {
    let help = current_help();
    let missing_input = without_once_after(
        &help,
        b"Formats: -i, --input-format auto|toon|yaml|json|json5|jsonl|toon-seq|json-seq|",
        b"csv|tsv",
    );
    assert!(!tq_contract_matches(
        TqContract::Help,
        &observation(&missing_input)
    ));

    let missing_output = without_once_after(
        &help,
        b"-o, --output-format toon|yaml|json|jsonl|toon-seq|json-seq|",
        b"csv|tsv",
    );
    assert!(!tq_contract_matches(
        TqContract::Help,
        &observation(&missing_output)
    ));
}

#[test]
fn help_contract_rejects_missing_generated_header() {
    let help = current_help();
    let incomplete = without_once(&help, b"tq - jq-compatible queries over ");
    assert!(!tq_contract_matches(
        TqContract::Help,
        &observation(&incomplete)
    ));
}

#[test]
fn help_contract_keeps_status_and_stderr_checks() {
    let help = current_help();
    let mut nonzero = observation(&help);
    nonzero.exit_code = Some(1);
    assert!(!tq_contract_matches(TqContract::Help, &nonzero));

    let mut failed = observation(&help);
    failed.stderr_hex = Some(encode_hex(b"diagnostic\n"));
    assert!(!tq_contract_matches(TqContract::Help, &failed));

    let mut timed_out = observation(&help);
    timed_out.process_status = Some(ProcessStatus::TimedOut);
    assert!(!tq_contract_matches(TqContract::Help, &timed_out));
}
