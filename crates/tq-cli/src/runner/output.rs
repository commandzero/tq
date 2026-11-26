//! Serialize and write complete result frames with output limits.

use super::RunError;
use super::test_file::RunTestCaptureHandle;
use crate::{ColorMode, ExitStatus, RunOptions};
use std::io;
use std::io::Write;
use std::sync::Arc;
use tq_core::Value;
use tq_formats::{
    JsonColorPalette, NativeFormat, NativeOutputSequence, OutputError, OutputFormat, OutputOptions,
    ToonFraming,
};

pub(super) struct ResultOutput<'a, W> {
    writer: &'a mut W,
    pub(super) options: &'a RunOptions,
    native: NativeOutputSequence,
    raw_palette: Option<JsonColorPalette>,
    capture: Option<RunTestCaptureHandle>,
    written: u64,
    emitted: u64,
    last_was_proxy: bool,
}

pub(super) fn color_palette(options: &RunOptions) -> JsonColorPalette {
    if options.capability_policy.environment {
        std::env::var_os("TQ_COLORS")
            .or_else(|| std::env::var_os("JQ_COLORS"))
            .and_then(|value| value.into_string().ok())
            .map_or_else(JsonColorPalette::default, |value| {
                JsonColorPalette::from_jq_colors(&value)
            })
    } else {
        JsonColorPalette::default()
    }
}

impl<'a, W: Write> ResultOutput<'a, W> {
    pub(super) fn with_capture(
        writer: &'a mut W,
        options: &'a RunOptions,
        capture: Option<RunTestCaptureHandle>,
    ) -> Self {
        let palette = color_palette(options);
        Self {
            writer,
            options,
            native: NativeOutputSequence::new(
                NativeFormat::from_output(options.output_format)
                    .select_output(
                        OutputOptions {
                            format: options.output_format,
                            strict_conversion: options.strict_conversion,
                            delimited_limits: tq_formats::DelimitedLimits {
                                row_bytes: options.limits.line_bytes,
                                field_bytes: options.limits.token_bytes,
                                fields: options.limits.fields,
                            },
                            pretty_json: options.pretty_json
                                && matches!(
                                    options.output_format,
                                    OutputFormat::Json | OutputFormat::JsonSequence
                                ),
                            json_indent: if matches!(
                                options.output_format,
                                OutputFormat::Json | OutputFormat::JsonSequence
                            ) {
                                options.json_indent
                            } else {
                                tq_formats::JsonIndent::default()
                            },
                            ascii_json: options.ascii_output,
                            color_json: false,
                            color_palette: palette.clone(),
                            yaml_document_start: false,
                            toon_framing: options.framing,
                            json_sequence: options.json_sequence,
                            toon: options.toon_writer,
                        }
                        .with_color(options.color == ColorMode::Always),
                    )
                    .expect("CLI output selection is validated"),
            ),
            raw_palette: (options.color == ColorMode::Always).then_some(palette),
            capture,
            written: 0,
            emitted: 0,
            last_was_proxy: false,
        }
    }

    fn record(&self, value: &Value) {
        let Some(capture) = &self.capture else {
            return;
        };
        if let Ok(mut capture) = capture.lock() {
            capture.record(value);
        }
    }

    pub(super) fn emit(&mut self, value: &Value) -> Result<(), RunError> {
        if self.emitted >= self.options.limits.results {
            return Err(RunError::Resource("result-count"));
        }
        if self.last_was_proxy && self.writes_unframed_toon() {
            return Err(multiple_unframed_values());
        }
        self.emitted = self.emitted.saturating_add(1);
        self.last_was_proxy = false;
        let original_value = value;
        let sorted;
        let value = if self.options.sort_keys {
            sorted = sort_value_keys(original_value);
            &sorted
        } else {
            original_value
        };
        if self.options.raw_output {
            self.emit_raw(value)?;
            self.record(original_value);
            return Ok(());
        }
        let mut writer = LimitedWriter::new(
            &mut *self.writer,
            &mut self.written,
            self.options.limits.output_bytes,
        );
        self.native.write_result(&mut writer, value)?;
        if self.options.unbuffered {
            self.writer.flush()?;
        }
        self.record(original_value);
        Ok(())
    }

    fn emit_raw(&mut self, value: &Value) -> Result<(), RunError> {
        let mut writer = LimitedWriter::new(
            &mut *self.writer,
            &mut self.written,
            self.options.limits.output_bytes,
        );
        write_raw(
            &mut writer,
            value,
            RawSeparator::from_options(self.options),
            self.raw_palette.as_ref(),
        )?;
        if self.options.unbuffered {
            writer.flush()?;
        }
        Ok(())
    }

    pub(super) fn proxy(&mut self, bytes: &[u8]) -> Result<(), RunError> {
        let proxy_would_append = self.emitted != 0 || self.last_was_proxy;
        if proxy_would_append && self.writes_unframed_toon() {
            return Err(multiple_unframed_values());
        }
        let mut writer = LimitedWriter::new(
            &mut *self.writer,
            &mut self.written,
            self.options.limits.output_bytes,
        );
        writer.write_all(bytes).map_err(OutputError::Io)?;
        if self.options.unbuffered {
            writer.flush().map_err(OutputError::Io)?;
        }
        self.last_was_proxy = true;
        Ok(())
    }

    pub(super) fn finish(&mut self) -> Result<(), RunError> {
        let only_proxy = self.last_was_proxy && self.emitted == 0;
        if !self.options.raw_output && !only_proxy {
            let mut writer = LimitedWriter::new(
                &mut *self.writer,
                &mut self.written,
                self.options.limits.output_bytes,
            );
            self.native.finish(&mut writer)?;
        }
        if self.options.unbuffered {
            self.writer.flush()?;
        }
        Ok(())
    }

    pub(super) const fn written(&self) -> u64 {
        self.written
    }

    fn writes_unframed_toon(&self) -> bool {
        self.options.output_format == OutputFormat::Toon
            && self.options.framing == ToonFraming::Unframed
    }

    pub(super) fn exit_status(&self, requested: bool, last: Option<&Value>) -> ExitStatus {
        if self.last_was_proxy {
            ExitStatus::Success
        } else {
            exit_status(requested, last)
        }
    }
}

fn multiple_unframed_values() -> RunError {
    OutputError::Toon(tq_toon::SequenceError::Cardinality(
        tq_toon::CardinalityError::Multiple,
    ))
    .into()
}

pub(super) struct LimitedWriter<'a, W> {
    writer: &'a mut W,
    written: &'a mut u64,
    limit: u64,
}

impl<'a, W> LimitedWriter<'a, W> {
    pub(super) fn new(writer: &'a mut W, written: &'a mut u64, limit: u64) -> Self {
        Self {
            writer,
            written,
            limit,
        }
    }
}

impl<W: Write> Write for LimitedWriter<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let length = u64::try_from(buffer.len()).unwrap_or(u64::MAX);
        if (*self.written).saturating_add(length) > self.limit {
            return Err(io::Error::other(
                "output resource limit exceeded: output-bytes",
            ));
        }
        let count = self.writer.write(buffer)?;
        *self.written = (*self.written).saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum RawSeparator {
    Nul,
    Newline,
    None,
}

impl RawSeparator {
    fn from_options(options: &RunOptions) -> Self {
        if options.raw_output0 {
            Self::Nul
        } else if options.join_output {
            Self::None
        } else {
            Self::Newline
        }
    }
}

fn write_raw(
    mut output: impl Write,
    value: &Value,
    separator: RawSeparator,
    palette: Option<&JsonColorPalette>,
) -> Result<(), RunError> {
    match value {
        Value::String(value) => {
            if separator == RawSeparator::Nul && value.contains('\0') {
                return Err(RunError::RawOutput(
                    "cannot emit a string containing NUL with --raw-output0",
                ));
            }
            output.write_all(value.as_bytes())?;
        }
        _ => tq_formats::write_raw_json_value(&mut output, value, palette)?,
    }
    match separator {
        RawSeparator::Nul => output.write_all(b"\0")?,
        RawSeparator::Newline => output.write_all(b"\n")?,
        RawSeparator::None => {}
    }
    Ok(())
}

fn sort_value_keys(value: &Value) -> Value {
    match value {
        Value::Array(values) => {
            Value::array(values.iter().map(sort_value_keys).collect::<Vec<_>>())
        }
        Value::Object(values) => {
            let mut entries = values.iter().collect::<Vec<_>>();
            entries.sort_unstable_by_key(|(key, _)| *key);
            Value::object(
                entries
                    .into_iter()
                    .map(|(key, value)| (Arc::clone(key), sort_value_keys(value)))
                    .collect::<tq_core::Object>(),
            )
        }
        _ => value.clone(),
    }
}
fn exit_status(enabled: bool, last: Option<&Value>) -> ExitStatus {
    if enabled {
        match last {
            None => ExitStatus::NoResult,
            Some(Value::Null | Value::Bool(false)) => ExitStatus::FalseOrNull,
            Some(_) => ExitStatus::Success,
        }
    } else {
        ExitStatus::Success
    }
}
