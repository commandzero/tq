//! Automatic JSON root retention must prune only fields proven unobservable.

use std::{ffi::OsString, fs, io::Cursor};

use tempfile::NamedTempFile;
use tq_cli::{ExitStatus, parse_args, run_with_io};
use tq_core::{PlanKind, ResolveOptions, analyze, parse, resolve};

#[derive(Debug, Eq, PartialEq)]
struct Observation {
    status: Result<ExitStatus, ExitStatus>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

const WIDE_INPUT: &[u8] = br#"{"features":[
  {"id":11,"properties":{"mag":3,"label":"alpha","nested":{"left":1,"right":2}},"unused":{"padding":"abcdefghijklmnopqrstuvwxyz0123456789","more":[1,2,3]}},
  {"id":22,"properties":{"mag":1,"label":"beta","nested":{"left":3,"right":4}},"unused":{"padding":"ABCDEFGHIJKLMNOPQRSTUVWXYZ9876543210","more":[4,5,6]}},
  {"id":33,"properties":{"mag":4,"label":"gamma","nested":{"left":5,"right":6}},"unused":{"padding":"0123456789abcdefghijklmnopqrstuvwxyz","more":[7,8,9]}}
]}"#;

const FULL_RESULTS: &[u8] = br#"{"id":11,"properties":{"mag":3,"label":"alpha","nested":{"left":1,"right":2}},"unused":{"padding":"abcdefghijklmnopqrstuvwxyz0123456789","more":[1,2,3]}}
{"id":22,"properties":{"mag":1,"label":"beta","nested":{"left":3,"right":4}},"unused":{"padding":"ABCDEFGHIJKLMNOPQRSTUVWXYZ9876543210","more":[4,5,6]}}
{"id":33,"properties":{"mag":4,"label":"gamma","nested":{"left":5,"right":6}},"unused":{"padding":"0123456789abcdefghijklmnopqrstuvwxyz","more":[7,8,9]}}
"#;

const SELECTED_RESULTS: &[u8] = br#"{"id":11,"properties":{"mag":3,"label":"alpha","nested":{"left":1,"right":2}},"unused":{"padding":"abcdefghijklmnopqrstuvwxyz0123456789","more":[1,2,3]}}
{"id":33,"properties":{"mag":4,"label":"gamma","nested":{"left":5,"right":6}},"unused":{"padding":"0123456789abcdefghijklmnopqrstuvwxyz","more":[7,8,9]}}
"#;

fn execute<I, S>(arguments: I, input: &[u8]) -> Observation
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let command = parse_args(arguments).expect("footprint arguments parse");
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

fn execute_with_report(query: &str, input: &[u8]) -> (Observation, serde_json::Value) {
    let report_file = NamedTempFile::new().expect("create footprint report path");
    let observation = execute(
        [
            OsString::from("--input-format"),
            OsString::from("json"),
            OsString::from("--output-format"),
            OsString::from("jsonl"),
            OsString::from("--prepare-memory-bytes"),
            OsString::from("65536"),
            OsString::from("--max-spool-bytes"),
            OsString::from("1048576"),
            OsString::from("--report-file"),
            report_file.path().as_os_str().to_owned(),
            OsString::from(query),
        ],
        input,
    );
    let report = serde_json::from_slice::<serde_json::Value>(
        &fs::read(report_file.path()).expect("read footprint report"),
    )
    .expect("parse footprint report");
    (observation, report)
}

fn staged_encoded_bytes(report: &serde_json::Value) -> u64 {
    report["execution"]["retention_high_water"]["root_staging"]["encoded_bytes_high_water"]
        .as_u64()
        .expect("root staging encoded-byte observation")
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

#[test]
fn static_select_footprint_reduces_staged_bytes_without_changing_results() {
    let query = ".features[] | select(.properties.mag >= 2) | .id";
    assert_subtree_proof(query);
    let (pruned, pruned_report) = execute_with_report(query, WIDE_INPUT);
    let (control, control_report) =
        execute_with_report(".features[] | select(.properties.mag >= 2)", WIDE_INPUT);

    assert_eq!(pruned.status, Ok(ExitStatus::Success));
    assert_eq!(control.status, Ok(ExitStatus::Success));
    assert_eq!(pruned.stdout, b"11\n33\n");
    assert_eq!(control.stdout, SELECTED_RESULTS);
    assert!(
        staged_encoded_bytes(&pruned_report) < staged_encoded_bytes(&control_report),
        "static select footprint should retain fewer encoded bytes: pruned={} control={}",
        staged_encoded_bytes(&pruned_report),
        staged_encoded_bytes(&control_report)
    );
}

#[test]
fn static_test_footprint_reduces_staged_bytes_without_changing_results() {
    let query = r#".features[] | .id | test("^[a-z]+$")"#;
    assert_subtree_proof(query);
    let input = br#"{"features":[
  {"id":"alpha","properties":{"mag":3,"label":"keep"},"unused":{"padding":"abcdefghijklmnopqrstuvwxyz0123456789"}},
  {"id":"123","properties":{"mag":1,"label":"keep"},"unused":{"padding":"ABCDEFGHIJKLMNOPQRSTUVWXYZ9876543210"}}
]}"#;
    let full_results = br#"{"id":"alpha","properties":{"mag":3,"label":"keep"},"unused":{"padding":"abcdefghijklmnopqrstuvwxyz0123456789"}}
{"id":"123","properties":{"mag":1,"label":"keep"},"unused":{"padding":"ABCDEFGHIJKLMNOPQRSTUVWXYZ9876543210"}}
"#;
    let (pruned, pruned_report) = execute_with_report(query, input);
    let (control, control_report) = execute_with_report(".features[]", input);

    assert_eq!(pruned.status, Ok(ExitStatus::Success));
    assert_eq!(control.status, Ok(ExitStatus::Success));
    assert_eq!(pruned.stdout, b"true\nfalse\n");
    assert_eq!(control.stdout, full_results);
    assert!(
        staged_encoded_bytes(&pruned_report) < staged_encoded_bytes(&control_report),
        "static test footprint should retain fewer encoded bytes: pruned={} control={}",
        staged_encoded_bytes(&pruned_report),
        staged_encoded_bytes(&control_report)
    );
}

#[test]
fn observable_full_results_and_shape_operations_keep_all_fields() {
    let identity = execute(["-ijson", "-ojson", "-c", ".features[]"], WIDE_INPUT);
    assert_eq!(identity.status, Ok(ExitStatus::Success));
    assert_eq!(identity.stdout, FULL_RESULTS);

    let standalone_select = execute(
        [
            "-ijson",
            "-ojson",
            "-c",
            ".features[] | select(.properties.mag >= 2)",
        ],
        WIDE_INPUT,
    );
    assert_eq!(standalone_select.status, Ok(ExitStatus::Success));
    assert_eq!(standalone_select.stdout, SELECTED_RESULTS);

    let keys = execute(["-ijson", "-ojson", "-c", ".features[] | keys"], WIDE_INPUT);
    assert_eq!(keys.status, Ok(ExitStatus::Success));
    assert_eq!(
        keys.stdout,
        b"[\"id\",\"properties\",\"unused\"]\n[\"id\",\"properties\",\"unused\"]\n[\"id\",\"properties\",\"unused\"]\n"
    );
}

#[test]
fn observable_tojson_and_debug_keep_all_fields() {
    let tojson = execute(
        ["-ijson", "-ojson", "-c", ".features[] | tojson"],
        WIDE_INPUT,
    );
    assert_eq!(tojson.status, Ok(ExitStatus::Success));
    assert_eq!(
        tojson.stdout,
        b"\"{\\\"id\\\":11,\\\"properties\\\":{\\\"mag\\\":3,\\\"label\\\":\\\"alpha\\\",\\\"nested\\\":{\\\"left\\\":1,\\\"right\\\":2}},\\\"unused\\\":{\\\"padding\\\":\\\"abcdefghijklmnopqrstuvwxyz0123456789\\\",\\\"more\\\":[1,2,3]}}\"\n\"{\\\"id\\\":22,\\\"properties\\\":{\\\"mag\\\":1,\\\"label\\\":\\\"beta\\\",\\\"nested\\\":{\\\"left\\\":3,\\\"right\\\":4}},\\\"unused\\\":{\\\"padding\\\":\\\"ABCDEFGHIJKLMNOPQRSTUVWXYZ9876543210\\\",\\\"more\\\":[4,5,6]}}\"\n\"{\\\"id\\\":33,\\\"properties\\\":{\\\"mag\\\":4,\\\"label\\\":\\\"gamma\\\",\\\"nested\\\":{\\\"left\\\":5,\\\"right\\\":6}},\\\"unused\\\":{\\\"padding\\\":\\\"0123456789abcdefghijklmnopqrstuvwxyz\\\",\\\"more\\\":[7,8,9]}}\"\n"
    );

    let debug = execute(
        ["-ijson", "-ojson", "-c", ".features[] | debug"],
        WIDE_INPUT,
    );
    assert_eq!(debug.status, Ok(ExitStatus::Success));
    assert_eq!(debug.stdout, FULL_RESULTS);
    assert_eq!(
        debug.stderr,
        br#"["DEBUG:",{"id":11,"properties":{"mag":3,"label":"alpha","nested":{"left":1,"right":2}},"unused":{"padding":"abcdefghijklmnopqrstuvwxyz0123456789","more":[1,2,3]}}]
["DEBUG:",{"id":22,"properties":{"mag":1,"label":"beta","nested":{"left":3,"right":4}},"unused":{"padding":"ABCDEFGHIJKLMNOPQRSTUVWXYZ9876543210","more":[4,5,6]}}]
["DEBUG:",{"id":33,"properties":{"mag":4,"label":"gamma","nested":{"left":5,"right":6}},"unused":{"padding":"0123456789abcdefghijklmnopqrstuvwxyz","more":[7,8,9]}}]
"#
    );
}

#[test]
fn user_select_override_and_binding_alias_keep_full_input_shape() {
    let override_query = "def select(f): .; .features[] | select(.properties.mag >= 2)";
    let override_observation = execute(["-ijson", "-ojson", "-c", override_query], WIDE_INPUT);
    assert_eq!(override_observation.status, Ok(ExitStatus::Success));
    assert_eq!(override_observation.stdout, FULL_RESULTS);

    let alias_query = ".features[] as $feature | select($feature.properties.mag >= 2) | $feature";
    let alias_observation = execute(["-ijson", "-ojson", "-c", alias_query], WIDE_INPUT);
    assert_eq!(alias_observation.status, Ok(ExitStatus::Success));
    assert_eq!(
        alias_observation.stdout,
        b"{\"id\":11,\"properties\":{\"mag\":3,\"label\":\"alpha\",\"nested\":{\"left\":1,\"right\":2}},\"unused\":{\"padding\":\"abcdefghijklmnopqrstuvwxyz0123456789\",\"more\":[1,2,3]}}\n{\"id\":33,\"properties\":{\"mag\":4,\"label\":\"gamma\",\"nested\":{\"left\":5,\"right\":6}},\"unused\":{\"padding\":\"0123456789abcdefghijklmnopqrstuvwxyz\",\"more\":[7,8,9]}}\n"
    );
}

#[test]
fn dynamic_index_keeps_the_selected_runtime_shape() {
    let query = ".features[] | .[if .properties.mag >= 2 then \"id\" else \"properties\" end]";
    let observation = execute(["-ijson", "-ojson", "-c", query], WIDE_INPUT);
    assert_eq!(observation.status, Ok(ExitStatus::Success));
    assert_eq!(
        observation.stdout,
        b"11\n{\"mag\":1,\"label\":\"beta\",\"nested\":{\"left\":3,\"right\":4}}\n33\n"
    );
}

#[test]
fn missing_null_scalar_and_duplicate_ancestors_keep_filter_semantics() {
    let query = ".features[] | select(.properties.mag >= 2) | .id";
    for (input, expected_status, expected_stdout, expected_stderr) in [
        (
            br#"{"features":[{"id":40}]}"#.as_slice(),
            Ok(ExitStatus::Success),
            b"".as_slice(),
            b"".as_slice(),
        ),
        (
            br#"{"features":[{"id":41,"properties":null}]}"#.as_slice(),
            Ok(ExitStatus::Success),
            b"".as_slice(),
            b"".as_slice(),
        ),
        (
            br#"{"features":[{"id":42,"properties":"wrong"}]}"#.as_slice(),
            Err(ExitStatus::Runtime),
            b"".as_slice(),
            b"tq: runtime error: field access cannot be applied to string\n".as_slice(),
        ),
        (
            br#"{"features":[{"id":43,"properties":{"mag":3},"properties":{"mag":1}}]}"#.as_slice(),
            Ok(ExitStatus::Success),
            b"".as_slice(),
            b"".as_slice(),
        ),
        (
            br#"{"features":[{"id":44,"properties":{"mag":1},"properties":{"mag":3}}]}"#.as_slice(),
            Ok(ExitStatus::Success),
            b"44\n".as_slice(),
            b"".as_slice(),
        ),
    ] {
        let observation = execute(["-ijson", "-ojson", "-c", query], input);
        assert_eq!(observation.status, expected_status);
        assert_eq!(observation.stdout, expected_stdout);
        assert_eq!(observation.stderr, expected_stderr);
    }

    let type_error = execute(
        [
            "-ijson",
            "-ojson",
            "-c",
            ".features[] | select(.properties.mag >= 2) | .id | test(\"^x+$\")",
        ],
        br#"{"features":[{"id":45,"properties":{"mag":3}}]}"#,
    );
    assert_eq!(type_error.status, Err(ExitStatus::Runtime));
    assert!(type_error.stdout.is_empty());
    assert_eq!(
        type_error.stderr,
        b"tq: runtime error: test cannot be applied to number\n"
    );
}
