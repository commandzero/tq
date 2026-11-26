//! Select input formats and check their support for streamed execution.

use super::input::{LimitedReader, format_from_path, open_path, proxyable_format_error};
use super::{JSON5_STREAM_UNSUPPORTED, RunError};
use crate::RunOptions;
use std::path::Path;
use tq_formats::{InputFormat, OutputFormat, ProbeReport, probe_reader};

#[allow(
    clippy::too_many_lines,
    reason = "bounded format detection and planning stay visibly ahead of decoder execution"
)]
pub(super) fn validate_stream_input_formats(options: &RunOptions) -> Result<(), RunError> {
    if !options.stream {
        return Ok(());
    }
    let json5_selected = options.input_format == InputFormat::Json5
        || (options.input_format == InputFormat::Auto
            && options.files.iter().any(|path| {
                path != Path::new("-") && format_from_path(path) == Some(InputFormat::Json5)
            }));
    if json5_selected {
        return Err(RunError::Unsupported(JSON5_STREAM_UNSUPPORTED.to_owned()));
    }
    Ok(())
}

pub(super) struct CommonInputFormat {
    pub(super) selected: Option<InputFormat>,
    pub(super) detections: Vec<DetectionObservation>,
}

#[derive(Clone, Debug)]
pub(super) struct DetectionObservation {
    pub(super) identity: String,
    pub(super) selected: InputFormat,
    pub(super) lookahead_bytes: usize,
    pub(super) commitment_bytes: usize,
    pub(super) rejections: Vec<(InputFormat, String)>,
}

impl DetectionObservation {
    pub(super) fn new(identity: impl Into<String>, report: &ProbeReport) -> Self {
        Self {
            identity: identity.into(),
            selected: report.selected,
            lookahead_bytes: report.lookahead_bytes,
            commitment_bytes: report.commitment_bytes,
            rejections: report.rejections.clone(),
        }
    }
}

pub(super) fn auto_file_common_format(options: &RunOptions) -> Result<CommonInputFormat, RunError> {
    let mut common = None;
    let mut detections = Vec::new();
    for path in options.files.iter().filter(|path| *path != Path::new("-")) {
        let identity = path.display().to_string();
        let selected = if let Some(format) = format_from_path(path) {
            format
        } else {
            let reader =
                LimitedReader::new(open_path(path)?, options.limits.input_bytes, &identity);
            let (report, _) = probe_reader(reader, options.limits.lookahead_bytes)?;
            detections.push(DetectionObservation::new(identity, &report));
            report.selected
        };
        if common.is_some_and(|format| format != selected) {
            return Ok(CommonInputFormat {
                selected: None,
                detections,
            });
        }
        common = Some(selected);
    }
    Ok(CommonInputFormat {
        selected: common,
        detections,
    })
}

pub(super) fn requires_staged_toon_events(options: &RunOptions, format: InputFormat) -> bool {
    !options.strict && matches!(format, InputFormat::Toon | InputFormat::ToonSequence)
}

pub(super) fn decoder_events_available(options: &RunOptions, format: InputFormat) -> bool {
    // Duplicate writes cannot be retracted by automatic event consumers.
    if requires_staged_toon_events(options, format) {
        return false;
    }
    match tq_formats::NativeFormat::from_input(format) {
        // Recoverable RS payloads need complete-document validation in normal
        // mode. Explicit --stream can publish partial events before recovery.
        Some(native) => {
            native.descriptor().events
                && !matches!(
                    native.descriptor().framing,
                    tq_formats::Framing::RecordSeparator
                )
        }
        None => false,
    }
}

pub(super) fn auto_file_events_available(options: &RunOptions) -> Result<bool, RunError> {
    let mut available = true;
    for path in options.files.iter().filter(|path| *path != Path::new("-")) {
        available &= file_events_available(options, path)?;
    }
    Ok(available)
}

fn file_events_available(options: &RunOptions, path: &Path) -> Result<bool, RunError> {
    if let Some(format) = format_from_path(path) {
        return Ok(decoder_events_available(options, format));
    }
    let identity = path.display().to_string();
    let reader = LimitedReader::new(open_path(path)?, options.limits.input_bytes, &identity);
    match probe_reader(reader, options.limits.lookahead_bytes) {
        Ok((probe, _)) => Ok(decoder_events_available(options, probe.selected)),
        Err(error) if options.proxy_on_error && proxyable_format_error(&error) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn json_sequence_input_requested(options: &RunOptions) -> bool {
    if !options.json_sequence {
        return false;
    }
    match options.input_format {
        InputFormat::Json | InputFormat::JsonSequence => true,
        InputFormat::Auto => {
            matches!(
                options.output_format,
                OutputFormat::Json | OutputFormat::JsonSequence | OutputFormat::JsonLines
            )
        }
        InputFormat::Toon
        | InputFormat::Yaml
        | InputFormat::Json5
        | InputFormat::ToonSequence
        | InputFormat::JsonLines
        | InputFormat::Csv
        | InputFormat::Tsv => false,
    }
}
