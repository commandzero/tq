//! Shared structured-result output dispatch.

use std::{borrow::Borrow, io::Write};

use thiserror::Error;
use tq_core::Value;
use tq_toon::{SequenceError, WriterConfig, write_sequence, write_unframed};

use crate::OutputFormat;

/// TOON result framing choice.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ToonFraming {
    /// RS-prefix/LF-suffix TOON Text Sequence, including for one result.
    #[default]
    Sequence,
    /// Exactly one standalone TOON document.
    Unframed,
}

/// Structured output controls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputOptions {
    /// Selected structured syntax.
    pub format: OutputFormat,
    /// Pretty JSON rather than compact JSON.
    pub pretty_json: bool,
    /// TOON framing mode.
    pub toon_framing: ToonFraming,
    /// Canonical TOON options.
    pub toon: WriterConfig,
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self {
            format: OutputFormat::Toon,
            pretty_json: false,
            toon_framing: ToonFraming::Sequence,
            toon: WriterConfig::default(),
        }
    }
}

/// Structured output failure.
#[derive(Debug, Error)]
pub enum OutputError {
    /// TOON framing/cardinality or output error.
    #[error(transparent)]
    Toon(#[from] SequenceError),
    /// JSON serialization or output error.
    #[error("JSON output failed: {0}")]
    Json(#[from] serde_json::Error),
    /// Direct output I/O error.
    #[error("structured output failed: {0}")]
    Io(#[from] std::io::Error),
}

impl OutputError {
    /// Whether output stopped because the downstream reader closed its pipe.
    #[must_use]
    pub fn is_broken_pipe(&self) -> bool {
        match self {
            Self::Toon(SequenceError::Io(error)) | Self::Io(error) => {
                error.kind() == std::io::ErrorKind::BrokenPipe
            }
            Self::Json(error) => error.io_error_kind() == Some(std::io::ErrorKind::BrokenPipe),
            Self::Toon(SequenceError::Cardinality(_)) => false,
        }
    }
}

/// Writes ordered results in the selected structured format.
///
/// JSON emits one jq-compatible JSON text plus LF per result. TOON defaults to
/// explicit text-sequence framing and supports exactly-one unframed output.
///
/// # Errors
///
/// Returns serialization, framing/cardinality, or output I/O failures.
pub fn write_results<W, I, V>(
    mut writer: W,
    values: I,
    options: OutputOptions,
) -> Result<(), OutputError>
where
    W: Write,
    I: IntoIterator<Item = V>,
    V: Borrow<Value>,
{
    match options.format {
        OutputFormat::Toon => match options.toon_framing {
            ToonFraming::Sequence => write_sequence(writer, values, options.toon)?,
            ToonFraming::Unframed => write_unframed(writer, values, options.toon)?,
        },
        OutputFormat::Json => {
            for value in values {
                if options.pretty_json {
                    serde_json::to_writer_pretty(&mut writer, value.borrow())?;
                } else {
                    serde_json::to_writer(&mut writer, value.borrow())?;
                }
                writer.write_all(b"\n")?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tq_core::Value;

    use super::{OutputOptions, ToonFraming, write_results};
    use crate::OutputFormat;

    #[test]
    fn json_preserves_exact_literals_and_toon_defaults_to_sequence() {
        let value: Value = serde_json::from_str(r#"{"n":9007199254740993}"#).unwrap();
        let mut json = Vec::new();
        write_results(
            &mut json,
            [&value],
            OutputOptions {
                format: OutputFormat::Json,
                ..OutputOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            json,
            br#"{"n":9007199254740993}
"#
        );

        let mut toon = Vec::new();
        write_results(&mut toon, [&value], OutputOptions::default()).unwrap();
        assert_eq!(toon, b"\x1en: 9007199254740993\n");
    }

    #[test]
    fn unframed_rejects_zero_and_multiple_results_without_output() {
        let values = [Value::Null, Value::Bool(true)];
        for selection in [Vec::<&Value>::new(), values.iter().collect()] {
            let mut output = Vec::new();
            assert!(
                write_results(
                    &mut output,
                    selection,
                    OutputOptions {
                        toon_framing: ToonFraming::Unframed,
                        ..OutputOptions::default()
                    }
                )
                .is_err()
            );
            assert!(output.is_empty());
        }
    }
}
