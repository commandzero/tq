//! Selection-aware JSON input regressions.

use std::io::Cursor;

use tq_core::{JsonEvent, JsonInput, JsonInputError, JsonInputOptions, JsonLimit, PathComponent};

fn selected_events(input: &[u8]) -> (Vec<JsonEvent>, Result<bool, JsonInputError>) {
    let mut reader = JsonInput::new(Cursor::new(input), JsonInputOptions::default());
    let mut events = Vec::new();
    let mut checkpoint = || Ok(());
    let mut retain = |path: &[PathComponent]| {
        path.is_empty() || path.starts_with(&[PathComponent::Key("features".into())])
    };
    let result = reader.next_events_selected(
        &mut |event| {
            events.push(event);
            Ok(())
        },
        &mut checkpoint,
        &mut retain,
    );
    (events, result)
}

fn selected_result(
    token: &str,
    options: JsonInputOptions,
    retain_discarded: bool,
) -> Result<bool, JsonInputError> {
    let input = format!(r#"{{"features":[0],"discarded":{token}}}"#).into_bytes();
    let mut reader = JsonInput::new(Cursor::new(input), options);
    let mut checkpoint = || Ok(());
    let mut retain = |path: &[PathComponent]| {
        retain_discarded
            || path.is_empty()
            || path.starts_with(&[PathComponent::Key("features".into())])
    };
    reader.next_events_selected(&mut |_| Ok(()), &mut checkpoint, &mut retain)
}

fn assert_retained_and_skipped_equivalent(token: &str, options: JsonInputOptions) {
    let retained = selected_result(token, options, true);
    let skipped = selected_result(token, options, false);
    assert_eq!(
        format!("{retained:?}"),
        format!("{skipped:?}"),
        "retained/skipped result diverged for {token}"
    );
}

#[test]
fn selected_events_skip_unselected_values_but_retain_selected_lexemes() {
    let (events, result) = selected_events(
        br#"{"features":[{"id":1.00,"value":NaN}],"discarded":{"huge":"not emitted"}}"#,
    );
    assert!(result.is_ok(), "selected parse failed: {result:?}");
    assert!(events.iter().any(|event| matches!(
        event,
        JsonEvent::Key { value, .. } if value.as_ref() == "features"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        JsonEvent::Scalar { value, .. } if value.to_string() == "1.00"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        JsonEvent::Key { value, .. } if value.as_ref() == "discarded"
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, JsonEvent::Skipped { .. }))
    );
}

#[test]
fn selected_events_validate_discarded_malformed_subtrees() {
    let (events, result) = selected_events(br#"{"features":[{"id":1}],"discarded":{"bad":[1 2]}}"#);
    assert!(matches!(result, Err(JsonInputError::Syntax { .. })));
    assert!(events.iter().any(|event| matches!(
        event,
        JsonEvent::Scalar { value, .. } if value.to_string() == "1"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        JsonEvent::Key { value, .. } if value.as_ref() == "bad"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        JsonEvent::Scalar { value, .. } if value.to_string() == "2"
    )));
}

#[test]
fn selected_events_validate_discarded_utf8_and_numeric_limits() {
    let (_, repaired) = selected_events(b"{\"features\":[1],\"discarded\":\"\xed\xa0\x80\"}");
    assert!(
        repaired.is_ok(),
        "discarded UTF-8 repair failed: {repaired:?}"
    );

    let (_, escaped_surrogate) = selected_events(br#"{"features":[1],"discarded":"\ud800"}"#);
    assert!(matches!(
        escaped_surrogate,
        Err(JsonInputError::Syntax { .. })
    ));

    let (_, oversized_exponent) = selected_events(br#"{"features":[1],"discarded":1e2000000001}"#);
    assert!(oversized_exponent.is_err());
}

#[test]
fn skipped_numeric_validation_matches_retained_numeric_validation() {
    for token in [
        "null",
        "true",
        "false",
        "NaN",
        "+Infinity",
        "-sNaN",
        "+.1",
        "01.200",
        "1.",
        "1.e2",
        "NaNfoo",
        "1true",
        "0x10",
    ] {
        assert_retained_and_skipped_equivalent(token, JsonInputOptions::default());
    }

    assert_retained_and_skipped_equivalent(
        "12345",
        JsonInputOptions {
            maximum_depth: 256,
            maximum_token_bytes: 4,
        },
    );
    assert_retained_and_skipped_equivalent(&"1".repeat(4097), JsonInputOptions::default());
    assert_retained_and_skipped_equivalent("1e2000000001", JsonInputOptions::default());
    let long_zero = "0".repeat(9000);
    let token_limit = JsonInputOptions {
        maximum_depth: 256,
        maximum_token_bytes: 8192,
    };
    let retained = selected_result(&long_zero, token_limit, true);
    let skipped = selected_result(&long_zero, token_limit, false);
    assert!(matches!(
        retained,
        Err(JsonInputError::Limit {
            limit: JsonLimit::TokenBytes,
            ..
        })
    ));
    assert!(matches!(
        skipped,
        Err(JsonInputError::Limit {
            limit: JsonLimit::TokenBytes,
            ..
        })
    ));
}
