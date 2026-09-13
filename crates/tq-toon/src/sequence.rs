//! TOON Text Sequence and exactly-one standalone framing.

use std::{borrow::Borrow, io::Write};

use thiserror::Error;
use tq_core::{Value, presentation::ColorPalette};

use crate::{WriterConfig, WriterError, write_value_colored};

/// Exactly-one output cardinality failure.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum CardinalityError {
    /// Evaluation produced no values.
    #[error("unframed TOON output requires one result, but evaluation produced none")]
    Zero,
    /// Evaluation produced more than one value.
    #[error("unframed TOON output requires one result, but evaluation produced multiple")]
    Multiple,
}

/// Structured output framing failure.
#[derive(Debug, Error)]
pub enum SequenceError {
    /// Output I/O failed.
    #[error("TOON sequence output I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// Exactly-one mode received the wrong number of results.
    #[error(transparent)]
    Cardinality(#[from] CardinalityError),
}

/// Writes zero or more RS-prefix/LF-suffix canonical TOON records.
///
/// # Errors
///
/// Returns the first output I/O failure.
pub fn write_sequence<W, I, V>(
    writer: W,
    values: I,
    config: WriterConfig,
) -> Result<(), SequenceError>
where
    W: Write,
    I: IntoIterator<Item = V>,
    V: Borrow<Value>,
{
    write_sequence_colored(writer, values, config, None)
}

/// Writes zero or more RS-prefix/LF-suffix canonical TOON records with an
/// optional semantic palette.
///
/// # Errors
///
/// Returns the first framing or output I/O error.
pub fn write_sequence_colored<W, I, V>(
    mut writer: W,
    values: I,
    config: WriterConfig,
    palette: Option<&ColorPalette>,
) -> Result<(), SequenceError>
where
    W: Write,
    I: IntoIterator<Item = V>,
    V: Borrow<Value>,
{
    for value in values {
        writer.write_all(b"\x1e")?;
        write_value_colored(&mut writer, value.borrow(), config, palette)
            .map_err(|WriterError::Io(error)| error)?;
        writer.write_all(b"\n")?;
    }
    Ok(())
}

/// Writes exactly one unframed canonical TOON document.
///
/// The value is encoded only after cardinality has been validated, so a zero
/// or multiple-result failure cannot leave a seemingly valid document behind.
///
/// # Errors
///
/// Returns a cardinality or output I/O failure.
pub fn write_unframed<W, I, V>(
    writer: W,
    values: I,
    config: WriterConfig,
) -> Result<(), SequenceError>
where
    W: Write,
    I: IntoIterator<Item = V>,
    V: Borrow<Value>,
{
    write_unframed_colored(writer, values, config, None)
}

/// Writes exactly one canonical TOON document with an optional semantic
/// palette after validating result cardinality.
///
/// # Errors
///
/// Returns a cardinality or output I/O failure.
pub fn write_unframed_colored<W, I, V>(
    mut writer: W,
    values: I,
    config: WriterConfig,
    palette: Option<&ColorPalette>,
) -> Result<(), SequenceError>
where
    W: Write,
    I: IntoIterator<Item = V>,
    V: Borrow<Value>,
{
    let mut values = values.into_iter();
    let first = values.next().ok_or(CardinalityError::Zero)?;
    if values.next().is_some() {
        return Err(CardinalityError::Multiple.into());
    }
    write_value_colored(&mut writer, first.borrow(), config, palette)
        .map_err(|WriterError::Io(error)| error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};

    use tq_core::{Value, presentation::ColorPalette};

    use super::{
        CardinalityError, SequenceError, write_sequence, write_sequence_colored, write_unframed,
    };
    use crate::WriterConfig;

    #[test]
    fn sequence_frames_zero_one_and_many_values() {
        let values = [Value::string("a"), Value::string("b")];
        let mut output = Vec::new();
        write_sequence(&mut output, values.iter(), WriterConfig::default()).unwrap();
        assert_eq!(output, b"\x1ea\n\x1eb\n");

        output.clear();
        write_sequence(
            &mut output,
            std::iter::empty::<&Value>(),
            WriterConfig::default(),
        )
        .unwrap();
        assert_eq!(output, [] as [u8; 0]);
    }

    #[test]
    fn unframed_validates_cardinality_before_writing() {
        let values = [Value::Bool(true), Value::Bool(false)];
        let mut output = Vec::new();
        let error =
            write_unframed(&mut output, values.iter(), WriterConfig::default()).unwrap_err();
        assert!(matches!(
            error,
            SequenceError::Cardinality(CardinalityError::Multiple)
        ));
        assert_eq!(output, [] as [u8; 0]);

        write_unframed(&mut output, [&values[0]], WriterConfig::default()).unwrap();
        assert_eq!(output, b"true");
    }

    #[test]
    fn multiline_record_is_independent_and_output_failure_is_reported() {
        let value: Value = serde_json::from_str(r#"{"outer":{"x":1}}"#).unwrap();
        let mut output = Vec::new();
        write_sequence(&mut output, [&value], WriterConfig::default()).unwrap();
        assert_eq!(output, b"\x1eouter:\n  x: 1\n");

        let mut failing = FailAfter {
            limit: 4,
            bytes: Vec::new(),
        };
        assert!(write_sequence(&mut failing, [&value], WriterConfig::default()).is_err());
        assert_ne!(failing.bytes, [] as [u8; 0]);
    }

    struct FailAfter {
        limit: usize,
        bytes: Vec<u8>,
    }

    impl Write for FailAfter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            if self.bytes.len() >= self.limit {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "injected"));
            }
            let count = buffer.len().min(self.limit - self.bytes.len());
            self.bytes.extend_from_slice(&buffer[..count]);
            Ok(count)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn strip_sgr(bytes: &[u8]) -> Vec<u8> {
        let mut output = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index..].starts_with(b"\x1b[") {
                index += 2;
                while index < bytes.len() && bytes[index] != b'm' {
                    index += 1;
                }
                assert!(index < bytes.len(), "unterminated SGR");
                index += 1;
            } else {
                output.push(bytes[index]);
                index += 1;
            }
        }
        output
    }

    #[test]
    fn colored_sequence_keeps_transport_boundaries_unstyled() {
        let values = [Value::string("a,b"), Value::string("c,d")];
        let palette = ColorPalette::from_jq_colors("10:11:12:13:14:15:16:17");
        let mut plain = Vec::new();
        let mut colored = Vec::new();
        write_sequence(&mut plain, values.iter(), WriterConfig::default()).unwrap();
        write_sequence_colored(
            &mut colored,
            values.iter(),
            WriterConfig::default(),
            Some(&palette),
        )
        .unwrap();

        assert_eq!(strip_sgr(&colored), plain);
        assert!(
            colored
                .windows(b"\x1b[0m\x0a".len())
                .any(|window| window == b"\x1b[0m\x0a")
        );
        assert!(
            colored
                .windows(b"\x0a\x1e".len())
                .any(|window| window == b"\x0a\x1e")
        );
        assert!(
            !colored
                .windows(b"\x1b[0m\x1e".len())
                .any(|window| window == b"\x1b[0m\x1e")
        );
    }
}
