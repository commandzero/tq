//! Advanced manual programs must retain process contracts across plan modes.

use std::io::Cursor;

use tq_cli::{Command, ExecutionOverride, ExitStatus, parse_args, run_with_io};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Contract {
    Result,
    Error,
}

struct Case {
    id: String,
    query: String,
    input: String,
    contract: Contract,
}

fn advanced_cases() -> Vec<Case> {
    include_str!("../../../tests/compatibility/cases/manual-advanced-features.jsonl")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let record: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|error| panic!("advanced catalog line is invalid JSON: {error}"));
            let id = record
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| panic!("advanced catalog record has no string id: {line}"));
            let input = record
                .get("fixture")
                .and_then(|fixture| fixture.get("inline"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| panic!("advanced catalog record {id} has no inline fixture"));
            let query = record
                .get("query")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| panic!("advanced catalog record {id} has no string query"));
            let contract = match record
                .get("expected")
                .and_then(|expected| expected.get("contract"))
                .and_then(serde_json::Value::as_str)
            {
                Some("result-sequence") => Contract::Result,
                Some("error") => Contract::Error,
                other => panic!("advanced catalog record {id} has unsupported contract {other:?}"),
            };
            Case {
                id: id.to_owned(),
                query: query.to_owned(),
                input: input.to_owned(),
                contract,
            }
        })
        .collect()
}

#[derive(Debug, Eq, PartialEq)]
struct Observation {
    status: Result<ExitStatus, ExitStatus>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn execute(case: &Case, execution_override: ExecutionOverride, explain: bool) -> Observation {
    let mut arguments = vec![
        "--input-format".to_owned(),
        "json".to_owned(),
        "--output-format".to_owned(),
        "json".to_owned(),
        "--compact-output".to_owned(),
    ];
    if explain {
        arguments.push("--explain-json".to_owned());
    }
    arguments.push(case.query.clone());
    let argument_refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let mut command =
        parse_args(argument_refs).unwrap_or_else(|error| panic!("{} parse_args: {error}", case.id));
    let Command::Run(options) = &mut command else {
        panic!("{} did not produce a run command", case.id);
    };
    options.execution_override = execution_override;
    let mut input = Cursor::new(case.input.as_bytes());
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

fn execute_stream(query: &str, input: &str, explain: bool) -> Observation {
    let mut arguments = vec![
        "--stream".to_owned(),
        "--input-format".to_owned(),
        "json".to_owned(),
        "--output-format".to_owned(),
        "json".to_owned(),
        "--compact-output".to_owned(),
    ];
    if explain {
        arguments.push("--explain-json".to_owned());
    }
    arguments.push(query.to_owned());
    let argument_refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let command = parse_args(argument_refs).expect("stream parser regression arguments parse");
    let mut input = Cursor::new(input.as_bytes());
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

fn execute_toon(
    query: &str,
    input: &str,
    execution_override: ExecutionOverride,
    explain: bool,
) -> Observation {
    let mut arguments = vec![
        "--input-format".to_owned(),
        "toon".to_owned(),
        "--output-format".to_owned(),
        "json".to_owned(),
        "--compact-output".to_owned(),
    ];
    if explain {
        arguments.push("--explain-json".to_owned());
    }
    arguments.push(query.to_owned());
    let argument_refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let mut command =
        parse_args(argument_refs).expect("TOON equivalence parser regression arguments parse");
    let Command::Run(options) = &mut command else {
        panic!("TOON equivalence did not produce a run command");
    };
    options.execution_override = execution_override;
    let mut input = Cursor::new(input.as_bytes());
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

fn selected_plan(observation: &Observation) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(&observation.stderr)
        .ok()?
        .get("execution")?
        .get("plan")?
        .as_str()
        .map(str::to_owned)
}

#[test]
fn advanced_manual_catalog_is_equivalent_in_automatic_and_document_modes() {
    let cases = advanced_cases();
    assert_eq!(
        cases.len(),
        43,
        "advanced catalog executable denominator changed"
    );

    let mut automatic_reports = 0;
    let mut automatic_optimized = 0;
    for case in &cases {
        let automatic = execute(case, ExecutionOverride::Automatic, false);
        let document = execute(case, ExecutionOverride::Document, false);
        assert_eq!(
            automatic, document,
            "execution mode diverged for {}",
            case.id
        );
        match case.contract {
            Contract::Result => assert_eq!(
                automatic.status,
                Ok(ExitStatus::Success),
                "{} expected a successful result contract",
                case.id
            ),
            Contract::Error => assert_ne!(
                automatic.status,
                Ok(ExitStatus::Success),
                "{} expected an error contract",
                case.id
            ),
        }

        let report = execute(case, ExecutionOverride::Automatic, true);
        if let Some(plan) = selected_plan(&report) {
            automatic_reports += 1;
            if !matches!(plan.as_str(), "document" | "blocking-document") {
                automatic_optimized += 1;
            }
        }
    }
    assert!(
        automatic_reports > 0,
        "advanced cases produced no execution reports"
    );
    eprintln!(
        "advanced automatic execution reports: {automatic_reports}; optimized: {automatic_optimized}; document fallback: {}",
        automatic_reports - automatic_optimized
    );
}

#[test]
fn eligible_stream_identity_reports_the_optimized_event_plan() {
    let observation = execute_stream(".", r#"{"a":1}"#, true);
    assert_eq!(observation.status, Ok(ExitStatus::Success));
    assert_eq!(selected_plan(&observation).as_deref(), Some("events"));
    assert_eq!(
        observation.stdout,
        br#"[["a"],1]
[["a"]]
"#
    );
}

#[test]
fn eligible_toon_execution_matches_for_automatic_and_document_modes() {
    let input = "items[2]{a}:\n  1\n  2\n";
    let automatic = execute_toon(
        ".items[] | .\"a\"",
        input,
        ExecutionOverride::Automatic,
        false,
    );
    let document = execute_toon(
        ".items[] | .\"a\"",
        input,
        ExecutionOverride::Document,
        false,
    );
    assert_eq!(automatic, document);
    assert_eq!(automatic.status, Ok(ExitStatus::Success));
    assert_eq!(automatic.stdout, b"1\n2\n");

    let report = execute_toon(
        ".items[] | .\"a\"",
        input,
        ExecutionOverride::Automatic,
        true,
    );
    let plan = selected_plan(&report).expect("automatic TOON execution emits a plan report");
    assert_ne!(plan, "document");
    assert_ne!(plan, "blocking-document");
}
