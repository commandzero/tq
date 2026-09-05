//! Fused structural selection behind committed native input.

use std::{
    io::{BufReader, Read},
    ops::ControlFlow,
    sync::{Arc, atomic::AtomicBool},
};

use crate::{
    DecodeOptions, FormatError, InputDeliveryError, InputFormat, ParallelJsonObservations,
    ParallelJsonOptions, SelectedStreamObservations, StreamOptions, StreamRecord, StreamSelection,
};

/// Selected decoder observations retain Document completion boundaries.
#[derive(Debug)]
pub enum SelectedInputObservation {
    /// A path/value record from fused structural selection.
    Record(StreamRecord),
    /// The selected decoder completed one Document.
    DocumentEnd,
}

struct Publication<F, E> {
    consume: F,
    stopped: Option<Result<(), E>>,
}

impl<F, E> Publication<F, E>
where
    F: FnMut(SelectedInputObservation) -> Result<ControlFlow<()>, E>,
{
    fn publish(&mut self, observation: SelectedInputObservation) -> Result<(), String> {
        match (self.consume)(observation) {
            Ok(ControlFlow::Continue(())) => Ok(()),
            Ok(ControlFlow::Break(())) => {
                self.stopped = Some(Ok(()));
                Err("selected input consumer stopped".to_owned())
            }
            Err(error) => {
                self.stopped = Some(Err(error));
                Err("selected input consumer failed".to_owned())
            }
        }
    }
}

pub(crate) fn consume<R: Read, E>(
    reader: R,
    identity: &str,
    options: DecodeOptions,
    selection: StreamSelection,
    parallel: Option<ParallelJsonOptions>,
    cancellation: Option<Arc<AtomicBool>>,
    consume: impl FnMut(SelectedInputObservation) -> Result<ControlFlow<()>, E>,
) -> Result<ParallelJsonObservations, InputDeliveryError<E>> {
    let mut publication = Publication {
        consume,
        stopped: None,
    };
    let mut decoder = SelectedDecoder {
        options,
        selection,
        parallel,
        cancellation,
        observations: ParallelJsonObservations::default(),
    };
    let result = decoder.decode(reader, identity, &mut publication);
    if let Some(stopped) = publication.stopped {
        return stopped
            .map(|()| decoder.observations)
            .map_err(InputDeliveryError::Consumer);
    }
    result
        .map(|()| decoder.observations)
        .map_err(InputDeliveryError::Input)
}

struct SelectedDecoder {
    options: DecodeOptions,
    selection: StreamSelection,
    parallel: Option<ParallelJsonOptions>,
    cancellation: Option<Arc<AtomicBool>>,
    observations: ParallelJsonObservations,
}

impl SelectedDecoder {
    fn decode<R: Read, F, E>(
        &mut self,
        reader: R,
        identity: &str,
        publication: &mut Publication<F, E>,
    ) -> Result<(), FormatError>
    where
        F: FnMut(SelectedInputObservation) -> Result<ControlFlow<()>, E>,
    {
        match self.options.format {
            InputFormat::Json => self.json(
                BufReader::with_capacity(64 * 1024, reader),
                true,
                publication,
            )?,
            InputFormat::JsonLines => {
                let mut lines = crate::adapters::JsonLinesDocumentSource::new(
                    BufReader::new(reader),
                    identity,
                    self.options,
                );
                while let Some((bytes, line)) = lines.next_record()? {
                    self.json(bytes.as_slice(), false, publication)
                        .map_err(|error| line_error(error, identity, line))?;
                    publication
                        .publish(SelectedInputObservation::DocumentEnd)
                        .map_err(consumer_error)?;
                }
                return Ok(());
            }
            InputFormat::Toon => {
                let mut observed = SelectedStreamObservations::default();
                let result = crate::stream_toon_selected_records_with_control(
                    BufReader::new(reader),
                    self.options.toon,
                    self.stream_options(),
                    self.selection.clone(),
                    self.cancellation.clone(),
                    &mut observed,
                    |record| publication.publish(SelectedInputObservation::Record(record)),
                );
                self.observations.depth_high_water = observed.depth_high_water;
                result?;
            }
            _ => {
                return Err(FormatError::Parse {
                    format: self.options.format,
                    message: "selected structural decoding is unavailable for this format"
                        .to_owned(),
                });
            }
        }
        publication
            .publish(SelectedInputObservation::DocumentEnd)
            .map_err(consumer_error)
    }

    fn stream_options(&self) -> StreamOptions {
        StreamOptions {
            maximum_depth: self.options.maximum_depth,
            maximum_token_bytes: self.options.maximum_token_bytes,
            errors_as_values: false,
        }
    }

    fn json<R: std::io::BufRead, F, E>(
        &mut self,
        reader: R,
        allow_parallel: bool,
        publication: &mut Publication<F, E>,
    ) -> Result<(), FormatError>
    where
        F: FnMut(SelectedInputObservation) -> Result<ControlFlow<()>, E>,
    {
        let mut emit = |record| publication.publish(SelectedInputObservation::Record(record));
        if allow_parallel && let Some(parallel) = self.parallel {
            self.observations = crate::stream_json_selected_records_parallel(
                reader,
                self.stream_options(),
                self.selection.clone(),
                parallel,
                self.cancellation.clone(),
                &mut emit,
            )?;
            return Ok(());
        }
        let mut observed = SelectedStreamObservations::default();
        let result = crate::stream_json_selected_records_with_control(
            reader,
            self.stream_options(),
            self.selection.clone(),
            self.cancellation.clone(),
            &mut observed,
            &mut emit,
        );
        self.observations.depth_high_water = self
            .observations
            .depth_high_water
            .max(observed.depth_high_water);
        result
    }
}

fn consumer_error(message: String) -> FormatError {
    FormatError::Parse {
        format: InputFormat::Auto,
        message,
    }
}

fn line_error(error: FormatError, identity: &str, line: u64) -> FormatError {
    match error {
        FormatError::Parse { message, .. } => FormatError::Parse {
            format: InputFormat::JsonLines,
            message: format!("{identity}:{line}: {message}"),
        },
        FormatError::Resource(resource) => FormatError::ResourceLine {
            identity: identity.to_owned(),
            line,
            resource,
        },
        error => error,
    }
}
