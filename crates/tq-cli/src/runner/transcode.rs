//! Execute a proven identity transcode and map decoder or publication failures.

use super::detection::DetectionObservation;
use super::input::{decode_options, open_path};
use super::output::{LimitedWriter, color_palette};
use super::reports::{TranscodeReportExecution, write_transcode_report};
use super::{RunError, cancellation};
use crate::{ColorMode, ExitStatus, RunOptions};
use std::io;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use tq_core::{
    Analysis, Compiled, Plan, SourceId, Transcode, TranscodeCommitment, TranscodeDuplicatePolicy,
    TranscodeInput, TranscodeLimits, TranscodeProof,
};
use tq_formats::{
    FormatError, InputFormat, InputRepresentation, NativeFormat, OutputFormat, ToonFraming,
};
use tq_toon::{
    ArrayPreparationConfig, DuplicateKeyPolicy, PreparationArena, PreparationLimits,
    PublicationBuffer, PublicationError, SpoolError, TranscodeConsumer, TranscodeError,
    WriterError,
};

pub(super) fn transcode_proof(
    options: &RunOptions,
    capabilities: tq_core::Capabilities,
    format: Option<InputFormat>,
) -> Result<TranscodeProof, &'static str> {
    if !capabilities.semantic_identity {
        return Err("query is not proven semantic identity");
    }
    if options.output_format != OutputFormat::Toon {
        return Err("selected output is not TOON");
    }
    if options.stream
        || options.slurp
        || options.raw_input
        || options.null_input
        || options.raw_output
        || options.join_output
        || options.raw_output0
        || options.proxy_on_error
    {
        return Err("selected CLI mode changes structured identity semantics");
    }
    if options.sort_keys {
        return Err("sorted-key output requires document execution");
    }
    let (input, duplicate_policy) = match format {
        Some(InputFormat::Json) => (TranscodeInput::Json, TranscodeDuplicatePolicy::Reject),
        Some(InputFormat::Toon) if options.strict => {
            (TranscodeInput::Toon, TranscodeDuplicatePolicy::Reject)
        }
        Some(InputFormat::Toon) => return Err("non-strict TOON requires document execution"),
        _ => return Err("selected input is not one common JSON or strict TOON syntax"),
    };
    Ok(TranscodeProof {
        input,
        duplicate_policy,
        late_errors: true,
        canonical_toon_writer: true,
        commitment: match options.framing {
            ToonFraming::Values => TranscodeCommitment::DirectValues,
            ToonFraming::Sequence => TranscodeCommitment::DirectSequence,
            ToonFraming::Unframed => TranscodeCommitment::AtomicUnframed,
        },
        limits: TranscodeLimits {
            maximum_memory_bytes: options.limits.preparation_memory_bytes as u64,
            maximum_spool_bytes: options.limits.spool_bytes,
            maximum_output_bytes: options.limits.output_bytes,
            maximum_depth: options.limits.depth,
            maximum_token_bytes: options.limits.token_bytes,
        },
    })
}

#[allow(
    clippy::too_many_lines,
    reason = "typed proof construction, commitment dispatch, and reporting stay in one lifecycle"
)]
pub(super) fn run_transcode_filter<R: Read, W: Write>(
    options: &RunOptions,
    plan: &Plan<Compiled, Transcode>,
    analysis: &Analysis,
    detections: &[DetectionObservation],
    stdin: &mut R,
    stdout: &mut W,
) -> Result<ExitStatus, RunError> {
    let proof = *plan
        .transcode_proof()
        .expect("typed transcode plan carries its proof");
    let format = match proof.input {
        TranscodeInput::Json => InputFormat::Json,
        TranscodeInput::Toon => InputFormat::Toon,
    };
    let duplicate_keys = match proof.duplicate_policy {
        TranscodeDuplicatePolicy::Reject => DuplicateKeyPolicy::Reject,
    };
    let commitment = match proof.commitment {
        TranscodeCommitment::DirectValues => tq_toon::TranscodeCommitment::DirectValues,
        TranscodeCommitment::DirectSequence => tq_toon::TranscodeCommitment::DirectSequence,
        TranscodeCommitment::AtomicUnframed => tq_toon::TranscodeCommitment::AtomicUnframed,
    };
    let preparation = ArrayPreparationConfig {
        memory_threshold_bytes: options.limits.preparation_memory_bytes,
        maximum_spool_bytes: options.limits.spool_bytes,
        ..ArrayPreparationConfig::default()
    };
    let arena = PreparationArena::new(PreparationLimits {
        memory_bytes: options.limits.preparation_memory_bytes,
        spool_bytes: options.limits.spool_bytes,
        output_bytes: options.limits.output_bytes,
        nesting: options.limits.depth,
    });
    let mut output_bytes = 0_u64;
    let palette = (options.color == ColorMode::Always).then(|| color_palette(options));

    let (execution, documents, last_truthy) = match proof.commitment {
        TranscodeCommitment::DirectSequence | TranscodeCommitment::DirectValues => {
            let writer = LimitedWriter::new(stdout, &mut output_bytes, options.limits.output_bytes);
            let mut consumer = TranscodeConsumer::new(
                writer,
                options.toon_writer,
                preparation,
                arena.clone(),
                duplicate_keys,
                commitment,
            )
            .with_document_limit(options.limits.results);
            if let Some(palette) = &palette {
                consumer = consumer.with_palette(palette.clone());
            }
            if let Some(flag) = cancellation() {
                consumer = consumer.with_cancellation(flag);
            }
            let result = transcode_sources(options, format, stdin, &mut consumer);
            (result, consumer.documents(), consumer.last_truthy())
        }
        TranscodeCommitment::AtomicUnframed => {
            let publication = BufWriter::with_capacity(
                64 * 1024,
                PublicationBuffer::new(preparation.clone(), arena.clone()),
            );
            let mut consumer = TranscodeConsumer::new(
                publication,
                options.toon_writer,
                preparation,
                arena.clone(),
                duplicate_keys,
                commitment,
            )
            .with_document_limit(options.limits.results);
            if let Some(palette) = &palette {
                consumer = consumer.with_palette(palette.clone());
            }
            if let Some(flag) = cancellation() {
                consumer = consumer.with_cancellation(flag);
            }
            let decode = transcode_sources(options, format, stdin, &mut consumer);
            let documents = consumer.documents();
            let last_truthy = consumer.last_truthy();
            let publication = consumer.into_inner();
            let result = decode.and_then(|()| {
                let mut publication = publication
                    .into_inner()
                    .map_err(|error| map_publication_buffer_error(error.into_error()))?;
                let mut writer =
                    LimitedWriter::new(stdout, &mut output_bytes, options.limits.output_bytes);
                publication
                    .publish_single_colored(&mut writer, documents, palette.as_ref())
                    .map_err(map_publication_error)
            });
            (result, documents, last_truthy)
        }
    };

    if let Some(path) = &options.report_file {
        write_transcode_report(
            path,
            options,
            analysis,
            detections,
            TranscodeReportExecution {
                documents,
                output_bytes,
                observations: arena.observations(),
                resource_outcome: execution
                    .as_ref()
                    .map_or_else(|error| resource_outcome(error), |()| "success"),
            },
        )?;
    }
    execution?;
    Ok(if options.exit_status {
        match last_truthy {
            None => ExitStatus::NoResult,
            Some(false) => ExitStatus::FalseOrNull,
            Some(true) => ExitStatus::Success,
        }
    } else {
        ExitStatus::Success
    })
}

pub(super) fn resource_outcome(error: &RunError) -> &'static str {
    match error.status() {
        ExitStatus::Resource => "resource-limit",
        ExitStatus::Interrupted => "interrupted",
        ExitStatus::Input => "input-error",
        ExitStatus::Runtime | ExitStatus::Unsupported | ExitStatus::Halt(_) => "output-error",
        ExitStatus::Usage
        | ExitStatus::Compile
        | ExitStatus::NoResult
        | ExitStatus::FalseOrNull => "error",
        ExitStatus::Success => "success",
    }
}

fn transcode_sources<R: Read, W: Write>(
    options: &RunOptions,
    format: InputFormat,
    stdin: &mut R,
    consumer: &mut TranscodeConsumer<W>,
) -> Result<(), RunError> {
    let files = if options.files.is_empty() {
        vec![Path::new("-").to_owned()]
    } else {
        options.files.clone()
    };
    for (index, path) in files.into_iter().enumerate() {
        let source = SourceId::new(u32::try_from(index + 1).unwrap_or(u32::MAX));
        if path == Path::new("-") {
            transcode_reader(options, format, &mut *stdin, "<stdin>", source, consumer)?;
        } else {
            let identity = path.display().to_string();
            transcode_reader(
                options,
                format,
                open_path(&path)?,
                &identity,
                source,
                consumer,
            )?;
        }
    }
    Ok(())
}

fn transcode_reader<R: Read, W: Write>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    source: SourceId,
    consumer: &mut TranscodeConsumer<W>,
) -> Result<(), RunError> {
    NativeFormat::from_input(format)
        .expect("typed transcode proof has a committed format")
        .select_input(decode_options(options, format), InputRepresentation::Events)?
        .open(reader, identity)
        .consume_codec_events(source, consumer)
        .map_err(|error| match error {
            tq_formats::InputDeliveryError::Input(error) => error.into(),
            tq_formats::InputDeliveryError::Consumer(tq_formats::CodecConsumerError::Event(
                error,
            )) => map_transcode_error(error),
            tq_formats::InputDeliveryError::Consumer(tq_formats::CodecConsumerError::Text(
                message,
            )) => map_transcode_message(format, message),
        })
}

fn map_transcode_message(format: InputFormat, message: String) -> RunError {
    if message.contains("prepared output exceeds configured byte limit") {
        RunError::Resource("output-bytes")
    } else if message.contains("resource limit")
        || message.contains("limit exceeded")
        || message.contains("preparation")
        || message.contains("spool")
    {
        RunError::Resource("transcode-preparation")
    } else if message.contains("interrupted") || message.contains("cancelled") {
        RunError::Interrupted
    } else {
        RunError::Input(FormatError::Parse { format, message })
    }
}

fn transcode_error_resource(resource: &'static str) -> &'static str {
    match resource {
        "depth" => "depth",
        "field count" => "fields",
        "retained bytes" => "preparation-memory",
        _ => "transcode-preparation",
    }
}

pub(super) fn map_transcode_error(error: TranscodeError) -> RunError {
    match error {
        TranscodeError::ResultLimit => RunError::Resource("result-count"),
        TranscodeError::Cancelled | TranscodeError::Spool(SpoolError::Cancelled) => {
            RunError::Interrupted
        }
        TranscodeError::Writer(WriterError::Schema { resource, .. })
        | TranscodeError::Spool(SpoolError::Writer(WriterError::Schema { resource, .. })) => {
            RunError::Resource(transcode_error_resource(resource))
        }
        TranscodeError::Spool(
            SpoolError::Io(error) | SpoolError::Writer(WriterError::Io(error)),
        )
        | TranscodeError::Writer(WriterError::Io(error))
        | TranscodeError::Io(error) => {
            if let Some(mapped) = map_wrapped_spool_error(&error) {
                return mapped;
            }
            let message = error.to_string();
            if message.contains("output resource limit exceeded") {
                RunError::Resource("output-bytes")
            } else if message.contains("spool")
                || message.contains("resource limit")
                || message.contains("preparation")
            {
                RunError::Resource("transcode-preparation")
            } else {
                RunError::Io(error)
            }
        }
        TranscodeError::Spool(_) => RunError::Resource("transcode-preparation"),
        TranscodeError::Duplicate(key) => RunError::Input(FormatError::Parse {
            format: InputFormat::Toon,
            message: format!("duplicate object key '{key}'"),
        }),
        TranscodeError::Structure(message) => RunError::Input(FormatError::Parse {
            format: InputFormat::Auto,
            message: message.to_owned(),
        }),
    }
}

fn map_wrapped_spool_error(error: &io::Error) -> Option<RunError> {
    let source = error.get_ref()?.downcast_ref::<SpoolError>()?;
    Some(match source {
        SpoolError::OutputLimit => RunError::Resource("output-bytes"),
        SpoolError::Cancelled => RunError::Interrupted,
        SpoolError::MemoryLimit
        | SpoolError::Disabled
        | SpoolError::Limit
        | SpoolError::NestingLimit => RunError::Resource("transcode-preparation"),
        SpoolError::Writer(WriterError::Schema { resource, .. }) => {
            RunError::Resource(transcode_error_resource(resource))
        }
        SpoolError::Writer(WriterError::Io(_)) | SpoolError::Io(_) | SpoolError::Decode(_) => {
            return None;
        }
    })
}

fn map_publication_error(error: PublicationError) -> RunError {
    match error {
        PublicationError::Cardinality(_) => RunError::Cardinality(
            "unframed TOON requires exactly one result; omit --unframed for zero or multiple results",
        ),
        PublicationError::Spool(error) => map_transcode_error(TranscodeError::Spool(error)),
        PublicationError::Io(error) => RunError::Io(error),
    }
}

fn map_publication_buffer_error(error: io::Error) -> RunError {
    if error.to_string().contains("spool") || error.to_string().contains("resource limit") {
        RunError::Resource("transcode-preparation")
    } else {
        RunError::Io(error)
    }
}
