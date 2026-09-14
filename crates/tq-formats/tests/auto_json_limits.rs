//! Automatic JSON detection preserves explicit decoder limits.

use tq_formats::{DecodeOptions, FormatError, InputFormat, decode_bytes};

fn options(format: InputFormat) -> DecodeOptions {
    DecodeOptions {
        format,
        maximum_depth: 1,
        maximum_token_bytes: 3,
        ..DecodeOptions::default()
    }
}

#[test]
fn explicit_json_rejects_nested_values_at_the_configured_depth() {
    let result = decode_bytes(b"[[0]]", "explicit.json", options(InputFormat::Json));

    assert!(matches!(result, Err(FormatError::Resource("depth"))));
}

#[test]
fn auto_json_rejects_nested_values_at_the_configured_depth() {
    let result = decode_bytes(b"[[0]]", "auto.json", options(InputFormat::Auto));

    assert!(matches!(result, Err(FormatError::Resource("depth"))));
}

#[test]
fn explicit_json_rejects_tokens_at_the_configured_size() {
    let result = decode_bytes(b"\"long\"", "explicit.json", options(InputFormat::Json));

    assert!(matches!(result, Err(FormatError::Resource("token-bytes"))));
}

#[test]
fn auto_json_rejects_tokens_at_the_configured_size() {
    let result = decode_bytes(b"\"long\"", "auto.json", options(InputFormat::Auto));

    assert!(matches!(result, Err(FormatError::Resource("token-bytes"))));
}

#[test]
fn explicit_and_auto_json_accept_values_at_configured_limits() {
    for format in [InputFormat::Json, InputFormat::Auto] {
        for (input, expected) in [(b"[123]".as_slice(), "[123]"), (b"123".as_slice(), "123")] {
            let documents = decode_bytes(input, "at-limit.json", options(format)).unwrap();
            assert_eq!(documents.len(), 1);
            assert_eq!(documents[0].value.to_string(), expected);
        }
    }
}
