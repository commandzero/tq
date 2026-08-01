//! Structured, raw, stderr, cardinality, and exit normalization tests.

use serde_json::json;
use tq_test_support::compatibility::{
    ErrorClass, NormalizationNote, ProcessOutcome, ProcessStatus, ToolKind, normalize_jq,
    normalize_raw, normalize_toon_sequence, normalize_yq,
};

fn outcome(stdout: &[u8]) -> ProcessOutcome {
    ProcessOutcome {
        status: ProcessStatus::Exited,
        exit_code: Some(0),
        signal: None,
        stdout: stdout.to_vec(),
        stderr: Vec::new(),
        wall_time_micros: 1,
        recorded_command: vec!["tool".to_owned()],
    }
}

#[test]
fn jq_distinguishes_zero_results_one_null_and_multiple_ordered_results() {
    assert!(normalize_jq(&outcome(b"")).unwrap().results.is_empty());
    assert_eq!(
        normalize_jq(&outcome(b"null\n")).unwrap().results,
        [json!(null)]
    );
    assert_eq!(
        normalize_jq(&outcome(b"1\nnull\n{\"z\":2,\"a\":3}\n"))
            .unwrap()
            .results,
        [json!(1), json!(null), json!({"z": 2, "a": 3})]
    );
}

#[test]
fn jq_accepts_multiline_structured_values_as_one_result() {
    let normalized = normalize_jq(&outcome(b"{\n  \"a\": [1, 2]\n}\n")).unwrap();
    assert_eq!(normalized.results, [json!({"a": [1, 2]})]);
}

#[test]
fn yq_preserves_document_sequence_and_records_yaml_boundary() {
    let normalized = normalize_yq(&outcome(b"a: 1\n---\na: 2\n")).unwrap();
    assert_eq!(normalized.results, [json!({"a": 1}), json!({"a": 2})]);
    assert!(
        normalized
            .notes
            .contains(&NormalizationNote::YamlPresentationNotRetained)
    );
}

#[test]
fn toon_text_sequence_preserves_zero_one_and_multiline_results() {
    assert!(
        normalize_toon_sequence(&outcome(b""))
            .unwrap()
            .results
            .is_empty()
    );
    let normalized =
        normalize_toon_sequence(&outcome(b"\x1ename: Alice\nage: 30\n\x1eitems[2]: 1,2\n"))
            .unwrap();
    assert_eq!(normalized.results.len(), 2);
    assert_eq!(normalized.results[0], json!({"name": "Alice", "age": 30}));
    assert_eq!(normalized.results[1], json!({"items": [1, 2]}));
}

#[test]
fn raw_bytes_stderr_and_exit_status_remain_independent() {
    let mut process = outcome(b"raw\0bytes\n");
    process.exit_code = Some(5);
    process.stderr = b"cannot index string with string".to_vec();
    let normalized = normalize_raw(ToolKind::Jq, &process);
    assert_eq!(normalized.raw_bytes, Some(b"raw\0bytes\n".to_vec()));
    assert_eq!(normalized.stderr, process.stderr);
    assert_eq!(normalized.exit_code, Some(5));
    assert_eq!(normalized.error_class, Some(ErrorClass::RuntimeTypePath));
    assert!(
        normalized
            .notes
            .contains(&NormalizationNote::StderrCaptured)
    );
}

#[test]
fn timeout_and_signal_are_stable_error_classes() {
    let mut process = outcome(b"");
    process.status = ProcessStatus::TimedOut;
    process.exit_code = None;
    assert_eq!(
        normalize_raw(ToolKind::Yq, &process).error_class,
        Some(ErrorClass::Timeout)
    );
    process.status = ProcessStatus::Signaled;
    process.signal = Some(9);
    assert_eq!(
        normalize_raw(ToolKind::Tq, &process).error_class,
        Some(ErrorClass::Signal)
    );
}
