//! Native sequence behavior across independent Result writes.

use tq_core::Value;
use tq_formats::{NativeFormat, NativeOutputSequence, OutputOptions, ToonFraming};

#[test]
fn strict_conversion_rejects_yaml_numeric_loss_before_publication() {
    let selection = NativeFormat::Yaml
        .select_output(OutputOptions {
            strict_conversion: true,
            ..OutputOptions::default()
        })
        .unwrap();
    let mut output = NativeOutputSequence::new(selection);
    let mut bytes = Vec::new();
    output.write_result(&mut bytes, &Value::Null).unwrap();
    let value = serde_json::from_str::<Value>("18446744073709551617").unwrap();
    assert!(
        output
            .write_result(&mut bytes, &value)
            .unwrap_err()
            .to_string()
            .contains("strict conversion")
    );
    assert_eq!(bytes, b"null\n");
}

#[test]
fn json_sequence_output_frames_each_result_and_keeps_exact_numbers() {
    let selection = NativeFormat::JsonSequence
        .select_output(OutputOptions::default())
        .unwrap();
    let mut sequence = NativeOutputSequence::new(selection);
    let mut bytes = Vec::new();
    sequence
        .write_result(
            &mut bytes,
            &serde_json::from_str::<Value>("9007199254740993").unwrap(),
        )
        .unwrap();
    assert_eq!(bytes, b"\x1e9007199254740993\n");
    sequence.write_result(&mut bytes, &Value::Null).unwrap();
    sequence.finish(&mut bytes).unwrap();
    assert_eq!(bytes, b"\x1e9007199254740993\n\x1enull\n");
    let mut empty = NativeOutputSequence::new(selection);
    let mut bytes = Vec::new();
    empty.finish(&mut bytes).unwrap();
    assert!(bytes.is_empty());
}

#[test]
fn native_output_retains_yaml_separators_until_completion() {
    let selection = NativeFormat::Yaml
        .select_output(OutputOptions::default())
        .unwrap();
    let mut sequence = NativeOutputSequence::new(selection);
    let mut bytes = Vec::new();
    sequence
        .write_result(&mut bytes, &Value::Bool(true))
        .unwrap();
    sequence.write_result(&mut bytes, &Value::Null).unwrap();
    sequence.finish(&mut bytes).unwrap();
    assert_eq!(bytes, b"true\n---\nnull\n");
    assert!(sequence.write_result(&mut bytes, &Value::Null).is_err());
    assert_eq!(bytes, b"true\n---\nnull\n");
}

#[test]
fn native_output_unframed_validates_cardinality_before_publication() {
    let selection = NativeFormat::Toon
        .select_output(OutputOptions {
            toon_framing: ToonFraming::Unframed,
            ..OutputOptions::default()
        })
        .unwrap();
    let mut sequence = NativeOutputSequence::new(selection);
    let mut bytes = Vec::new();
    sequence
        .write_result(&mut bytes, &Value::Bool(true))
        .unwrap();
    assert!(bytes.is_empty());
    assert!(sequence.write_result(&mut bytes, &Value::Null).is_err());
    assert!(sequence.finish(&mut bytes).is_err());
    assert!(bytes.is_empty());
    let mut one = NativeOutputSequence::new(selection);
    one.write_result(&mut bytes, &Value::Bool(true)).unwrap();
    one.finish(&mut bytes).unwrap();
    assert_eq!(bytes, b"true");
}

#[test]
fn native_output_io_failure_is_terminal_and_keeps_partial_bytes() {
    use std::io::{self, Write};
    struct FailAfterTwo(Vec<u8>);
    impl Write for FailAfterTwo {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.0.len() == 2 {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            let count = bytes.len().min(2 - self.0.len());
            self.0.extend_from_slice(&bytes[..count]);
            Ok(count)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let selection = NativeFormat::Json
        .select_output(OutputOptions::default())
        .unwrap();
    let mut sequence = NativeOutputSequence::new(selection);
    let mut writer = FailAfterTwo(Vec::new());
    assert!(
        sequence
            .write_result(&mut writer, &Value::Bool(true))
            .unwrap_err()
            .is_broken_pipe()
    );
    assert_eq!(writer.0, b"tr");
    let mut recovered = Vec::new();
    assert!(sequence.write_result(&mut recovered, &Value::Null).is_err());
    assert!(sequence.finish(&mut recovered).is_err());
    assert!(recovered.is_empty());
}

#[test]
fn selected_output_rejects_incompatible_controls_before_publication() {
    assert!(
        NativeFormat::Yaml
            .select_output(OutputOptions {
                ascii_json: true,
                ..OutputOptions::default()
            })
            .is_err()
    );
    assert!(
        NativeFormat::JsonLines
            .select_output(OutputOptions {
                pretty_json: true,
                ..OutputOptions::default()
            })
            .is_err()
    );
}
