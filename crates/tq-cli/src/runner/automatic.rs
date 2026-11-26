//! Execute automatic event, subtree, and hybrid plans.

use super::events::buffered_input;
use super::input::{
    LimitedReader, decode_options, open_path, proxyable_format_error, read_limited,
    selected_input_format, validate_proxy_event_source,
};
use super::output::ResultOutput;
use super::reports::{ReportExecution, write_report};
use super::test_file::RunTestCaptureHandle;
use super::transcode::resource_outcome;
use super::{
    ROOT_INDEX_BTREE_NODE_BYTES, RunError, cancellation, flush_vm_effects,
    is_recoverable_document_runtime_error, merge_observations, report_document_runtime_error,
    vm_limits,
};
use crate::runtime_spool::{
    REPLAY_BUFFER_BYTES, RuntimeRecordId, RuntimeSpool, RuntimeSpoolConfig, RuntimeSpoolError,
    RuntimeSpoolItem, RuntimeSpoolReplayError,
};
use crate::{ExitStatus, RunOptions};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tq_core::{
    Analysis, Compiled, HybridBlocking, HybridPreparation, PathComponent, Plan, PlanKind,
    StableSortPipeline, StableSortPipelineObservations, Value, Vm, VmError, VmLimits,
    VmObservations, parallel_worker_count,
};
use tq_formats::{
    FormatError, InputFormat, InputRepresentation, NativeFormat, ParallelJsonOptions,
    SelectedRootSink, SelectedStreamObservations, SelectionFallback, SelectionReplacement,
    StreamOptions, StreamRecord, StreamSelection, probe_reader,
    stream_json_selected_roots_with_control,
};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct RetentionObservations {
    pub(super) bytes_high_water: usize,
    pub(super) depth_high_water: usize,
    pub(super) decoder_depth_high_water: usize,
    pub(super) completed_subtrees: u64,
    pub(super) retained_results_high_water: usize,
    pub(super) retained_bytes_high_water: usize,
    pub(super) sort_runs: usize,
    pub(super) in_flight_batches_high_water: usize,
    pub(super) in_flight_bytes_high_water: usize,
    pub(super) decode_batches: usize,
    pub(super) decode_in_flight_batches_high_water: usize,
    pub(super) decode_in_flight_bytes_high_water: usize,
    pub(super) decode_reordered_batches_high_water: usize,
    /// Fixed private-spool buffers, reported separately from the dynamic
    /// preparation budget because they are owned by the I/O layer.
    pub(super) runtime_spool_fixed_io_bytes_high_water: usize,
    pub(super) root_staging_encoded_bytes_high_water: usize,
    pub(super) root_staging_memory_bytes_high_water: usize,
    pub(super) root_staging_index_bytes_high_water: usize,
    pub(super) root_staging_spool_bytes_written_high_water: u64,
}

#[allow(
    clippy::too_many_arguments,
    reason = "the automatic route keeps input, output, diagnostics, analysis, and test capture explicit"
)]
pub(super) fn run_automatic_filter<R: Read, W: Write, E: Write, M>(
    options: &RunOptions,
    plan: &Plan<Compiled, M>,
    variables: &BTreeMap<Arc<str>, Value>,
    analysis: &Analysis,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
    capture: Option<RunTestCaptureHandle>,
) -> Result<ExitStatus, RunError> {
    let mut executor = AutomaticExecutor {
        plan,
        prefix: plan
            .automatic_prefix()
            .expect("automatic plan has a proven prefix")
            .to_vec(),
        projection: plan.automatic_projection().map(<[PathComponent]>::to_vec),
        capture_paths: plan.automatic_capture_paths(),
        variables,
        output: ResultOutput::with_capture(stdout, options, capture),
        stderr,
        trace_remaining: options.trace_limit,
        observations: VmObservations::default(),
        retention: RetentionObservations::default(),
        current: None,
        current_item: None,
        deferred_item: None,
        collected: None,
        hybrid_suffix: None,
        last: None,
        results: 0,
        current_retained_results: 0,
        current_retained_bytes: 0,
        root_spool: runtime_spool_for(options),
        root_probe: PrefixProbe::new(
            plan.automatic_prefix()
                .expect("automatic plan has a proven prefix"),
        ),
        root_fallback: None,
        root_active: false,
        root_object_order: Vec::new(),
        root_object_latest: HashMap::new(),
        root_object_seen: HashSet::new(),
        root_runtime_error: None,
        root_fatal: None,
        root_index_bytes: 0,
        root_index_limit: options.limits.preparation_memory_bytes,
        root_step_start: None,
    };
    let files = if options.files.is_empty() {
        vec![Path::new("-").to_owned()]
    } else {
        options.files.clone()
    };
    for path in files {
        if cancellation().is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(RunError::Interrupted);
        }
        if path == Path::new("-") {
            automatic_reader(
                options,
                selected_input_format(options, &path),
                &mut *stdin,
                "<stdin>",
                &mut executor,
            )?;
        } else {
            let identity = path.display().to_string();
            automatic_reader(
                options,
                selected_input_format(options, &path),
                open_path(&path)?,
                &identity,
                &mut executor,
            )?;
        }
    }
    executor.output.finish()?;
    if let Some(error) = executor.root_runtime_error.take() {
        return Err(RunError::ReportedRuntime(error));
    }
    if let Some(path) = &options.report_file {
        write_report(
            path,
            &[executor.observations],
            executor.results,
            executor.output.written(),
            options,
            analysis.selected_plan,
            ReportExecution {
                analysis,
                retention: executor.retention,
                resource_outcome: "success",
            },
        )?;
    }
    Ok(executor
        .output
        .exit_status(options.exit_status, executor.last.as_ref()))
}

#[allow(
    clippy::too_many_arguments,
    reason = "the hybrid route keeps input, output, diagnostics, analysis, and test capture explicit"
)]
pub(super) fn run_hybrid_filter<R: Read, W: Write, E: Write>(
    options: &RunOptions,
    plan: &Plan<Compiled, HybridBlocking>,
    variables: &BTreeMap<Arc<str>, Value>,
    analysis: &Analysis,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
    capture: Option<RunTestCaptureHandle>,
) -> Result<ExitStatus, RunError> {
    let mut executor = AutomaticExecutor {
        plan,
        prefix: plan
            .automatic_prefix()
            .expect("hybrid plan has a proven producer prefix")
            .to_vec(),
        projection: plan.automatic_projection().map(<[PathComponent]>::to_vec),
        capture_paths: plan.automatic_capture_paths(),
        variables,
        output: ResultOutput::with_capture(stdout, options, capture),
        stderr,
        trace_remaining: options.trace_limit,
        observations: VmObservations::default(),
        retention: RetentionObservations::default(),
        current: None,
        current_item: None,
        deferred_item: None,
        collected: Some(HybridCollection::new(plan, options)),
        hybrid_suffix: Some(plan),
        last: None,
        results: 0,
        current_retained_results: 0,
        current_retained_bytes: 0,
        root_spool: runtime_spool_for(options),
        root_probe: PrefixProbe::new(
            plan.automatic_prefix()
                .expect("hybrid plan has a proven producer prefix"),
        ),
        root_fallback: None,
        root_active: false,
        root_object_order: Vec::new(),
        root_object_latest: HashMap::new(),
        root_object_seen: HashSet::new(),
        root_runtime_error: None,
        root_fatal: None,
        root_index_bytes: 0,
        root_index_limit: options.limits.preparation_memory_bytes,
        root_step_start: None,
    };
    let files = if options.files.is_empty() {
        vec![Path::new("-").to_owned()]
    } else {
        options.files.clone()
    };
    let execution = (|| {
        for path in files {
            if cancellation().is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                return Err(RunError::Interrupted);
            }
            if path == Path::new("-") {
                automatic_reader(
                    options,
                    selected_input_format(options, &path),
                    &mut *stdin,
                    "<stdin>",
                    &mut executor,
                )?;
            } else {
                let identity = path.display().to_string();
                automatic_reader(
                    options,
                    selected_input_format(options, &path),
                    open_path(&path)?,
                    &identity,
                    &mut executor,
                )?;
            }
        }
        executor.output.finish()?;
        if let Some(error) = executor.root_runtime_error.take() {
            return Err(RunError::ReportedRuntime(error));
        }
        Ok(executor
            .output
            .exit_status(options.exit_status, executor.last.as_ref()))
    })();
    if let Some(path) = &options.report_file {
        write_report(
            path,
            &[executor.observations],
            executor.results,
            executor.output.written(),
            options,
            PlanKind::HybridBlocking,
            ReportExecution {
                analysis,
                retention: executor.retention,
                resource_outcome: execution
                    .as_ref()
                    .map_or_else(|error| resource_outcome(error), |_| "success"),
            },
        )?;
    }
    execution
}

fn automatic_reader<R: Read, W: Write, E: Write, M>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    executor: &mut AutomaticExecutor<'_, W, E, M>,
) -> Result<(), RunError> {
    if options.proxy_on_error {
        let bytes = read_limited(reader, options.limits.input_bytes, identity)?;
        match validate_proxy_event_source(&bytes, identity, options, format) {
            Ok(()) => {
                return automatic_reader_inner(
                    options,
                    format,
                    bytes.as_slice(),
                    identity,
                    executor,
                );
            }
            Err(error) if proxyable_format_error(&error) => {
                return executor.output.proxy(&bytes);
            }
            Err(error) => return Err(error.into()),
        }
    }
    automatic_reader_inner(options, format, reader, identity, executor)
}

fn automatic_reader_inner<R: Read, W: Write, E: Write, M>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    executor: &mut AutomaticExecutor<'_, W, E, M>,
) -> Result<(), RunError> {
    if format == InputFormat::Auto {
        let reader = LimitedReader::new(reader, options.limits.input_bytes, identity);
        let (report, replay) = probe_reader(reader, options.limits.lookahead_bytes)?;
        return automatic_committed_reader(
            options,
            report.selected,
            replay,
            identity,
            executor,
            true,
        );
    }
    automatic_committed_reader(options, format, reader, identity, executor, false)
}

fn automatic_committed_reader<R: Read, W: Write, E: Write, M>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    executor: &mut AutomaticExecutor<'_, W, E, M>,
    source_limit_already_applied: bool,
) -> Result<(), RunError> {
    if format == InputFormat::Json {
        return automatic_json_root_reader(
            options,
            reader,
            identity,
            executor,
            source_limit_already_applied,
        );
    }
    let input = NativeFormat::from_input(format)
        .expect("input is committed")
        .select_input(decode_options(options, format), InputRepresentation::Events)?
        .open(reader, identity);
    let parallel = (executor.hybrid_suffix.is_some() && parallel_worker_count() > 1).then_some(
        ParallelJsonOptions {
            batch_values: options.limits.decode_batch_values,
            batch_bytes: options.limits.decode_batch_bytes,
            in_flight_batches: options.limits.decode_in_flight_batches,
            in_flight_bytes: options.limits.decode_in_flight_bytes,
        },
    );
    let decoded = input.consume_selected(
        executor.stream_selection(),
        parallel,
        cancellation(),
        |observation| {
            match observation {
                tq_formats::SelectedInputObservation::Record(record) => executor.accept(record),
                tq_formats::SelectedInputObservation::DocumentEnd => executor.finish_source(),
            }
            .map(|()| std::ops::ControlFlow::Continue(()))
        },
    );
    let observations = match decoded {
        Ok(observations) => observations,
        Err(tq_formats::InputDeliveryError::Consumer(error)) => return Err(error),
        Err(tq_formats::InputDeliveryError::Input(error)) => {
            if cancellation().is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                return Err(RunError::Interrupted);
            }
            return Err(error.into());
        }
    };
    executor.retention.decode_batches = executor
        .retention
        .decode_batches
        .saturating_add(observations.batches);
    executor.retention.decoder_depth_high_water = executor
        .retention
        .decoder_depth_high_water
        .max(observations.depth_high_water);
    executor.retention.decode_in_flight_batches_high_water = executor
        .retention
        .decode_in_flight_batches_high_water
        .max(observations.in_flight_batches_high_water);
    executor.retention.decode_in_flight_bytes_high_water = executor
        .retention
        .decode_in_flight_bytes_high_water
        .max(observations.in_flight_bytes_high_water);
    executor.retention.decode_reordered_batches_high_water = executor
        .retention
        .decode_reordered_batches_high_water
        .max(observations.reordered_batches_high_water);
    Ok(())
}

fn automatic_json_root_reader<R: Read, W: Write, E: Write, M>(
    options: &RunOptions,
    reader: R,
    identity: &str,
    executor: &mut AutomaticExecutor<'_, W, E, M>,
    source_limit_already_applied: bool,
) -> Result<(), RunError> {
    let mut observations = SelectedStreamObservations::default();
    let stream_options = StreamOptions {
        maximum_depth: options.limits.depth,
        maximum_token_bytes: options.limits.token_bytes,
        errors_as_values: false,
    };
    let selection = executor.stream_selection();
    let decoded = if source_limit_already_applied {
        stream_json_selected_roots_with_control(
            buffered_input(reader),
            stream_options,
            selection,
            cancellation(),
            &mut observations,
            executor,
        )
    } else {
        stream_json_selected_roots_with_control(
            buffered_input(LimitedReader::new(
                reader,
                options.limits.input_bytes,
                identity,
            )),
            stream_options,
            selection,
            cancellation(),
            &mut observations,
            executor,
        )
    };
    executor.retention.decoder_depth_high_water = executor
        .retention
        .decoder_depth_high_water
        .max(observations.depth_high_water);
    if let Some(error) = executor.root_fatal.take() {
        return Err(error);
    }
    match decoded {
        Ok(_) => Ok(()),
        Err(FormatError::Io(error))
            if error.to_string().contains("input resource limit exceeded") =>
        {
            Err(RunError::ResourceSource {
                identity: identity.to_owned(),
                resource: "input-bytes",
            })
        }
        Err(error) => Err(error.into()),
    }
}

enum HybridCollection {
    Collect(Vec<Value>),
    StableSort {
        pipeline: Option<StableSortPipeline>,
        config: (usize, usize, usize, usize),
    },
}

impl HybridCollection {
    fn new(plan: &Plan<Compiled, HybridBlocking>, options: &RunOptions) -> Self {
        if plan.hybrid_preparation() == HybridPreparation::StableSortRuns {
            let in_flight_bytes = options.limits.hybrid_in_flight_bytes.max(1);
            let in_flight_batches = options.limits.hybrid_in_flight_batches.max(1);
            let config = (
                options.limits.hybrid_batch_values.max(1),
                (in_flight_bytes / in_flight_batches).max(1),
                in_flight_batches,
                in_flight_bytes,
            );
            Self::StableSort {
                pipeline: Some(hybrid_sort_pipeline(config)),
                config,
            }
        } else {
            Self::Collect(Vec::new())
        }
    }

    fn push(&mut self, value: Value) -> Result<(), RunError> {
        match self {
            Self::Collect(values) => {
                values.push(value);
                Ok(())
            }
            Self::StableSort { pipeline, .. } => {
                let bytes = estimate_value_bytes(&value).saturating_add(32);
                pipeline
                    .as_mut()
                    .expect("active hybrid sort pipeline")
                    .push(value, bytes)
                    .map_err(hybrid_pipeline_error)
            }
        }
    }

    fn finish_document(
        &mut self,
    ) -> Result<(Vec<Value>, StableSortPipelineObservations), RunError> {
        match self {
            Self::Collect(values) => Ok((
                std::mem::take(values),
                StableSortPipelineObservations::default(),
            )),
            Self::StableSort { pipeline, config } => {
                let active = pipeline.take().expect("active hybrid sort pipeline");
                let (values, observations) = active.finish().map_err(hybrid_pipeline_error)?;
                *pipeline = Some(hybrid_sort_pipeline(*config));
                Ok((values, observations))
            }
        }
    }

    fn discard_document(&mut self) {
        match self {
            Self::Collect(values) => values.clear(),
            Self::StableSort { pipeline, config } => {
                *pipeline = Some(hybrid_sort_pipeline(*config));
            }
        }
    }
}

fn hybrid_sort_pipeline(config: (usize, usize, usize, usize)) -> StableSortPipeline {
    let pipeline = StableSortPipeline::new(config.0, config.1, config.2, config.3);
    if let Some(flag) = cancellation() {
        pipeline.with_cancellation(flag)
    } else {
        pipeline
    }
}

fn hybrid_pipeline_error(resource: &'static str) -> RunError {
    if resource == "interrupted" {
        RunError::Interrupted
    } else {
        RunError::Resource(resource)
    }
}

fn runtime_spool_for(options: &RunOptions) -> RuntimeSpool {
    let config = RuntimeSpoolConfig {
        memory_threshold_bytes: options.limits.preparation_memory_bytes.max(1),
        // `--max-spool-bytes` bounds bytes written to the private spill file,
        // not logical records retained in memory.  Root staging and replay
        // enforce the per-root preparation budget separately, so the
        // cumulative logical counter must not make a zero-disk configuration
        // reject an otherwise in-memory root.
        maximum_total_bytes: u64::MAX,
        maximum_spool_bytes: options.limits.spool_bytes,
        maximum_item_bytes: u64::try_from(options.limits.preparation_memory_bytes.max(1))
            .unwrap_or(u64::MAX),
        maximum_depth: options.limits.depth,
        maximum_token_bytes: options.limits.token_bytes,
        maximum_decoded_bytes: options.limits.preparation_memory_bytes as u64,
        spool_directory: std::env::temp_dir(),
        allow_spool: options.capability_policy.filesystem,
    };
    let spool = RuntimeSpool::new(config);
    match cancellation() {
        Some(flag) => spool.with_cancellation(flag),
        None => spool,
    }
}

#[derive(Clone, Debug)]
enum PrefixWitness {
    Unknown,
    /// The decoder found a scalar or an incompatible container at this
    /// boundary.  The value is the actual bounded ancestor, not a padded
    /// array synthesized for a large static index.
    Mismatch {
        boundary: usize,
        value: Value,
    },
    /// The decoder found the right container shape, but the selected path was
    /// absent below it.  The value is the smallest typed ancestor needed by
    /// the VM access operations.
    Missing {
        boundary: usize,
        value: Value,
    },
}

struct PrefixProbe {
    prefix: Vec<PathComponent>,
    witness: PrefixWitness,
}

impl PrefixProbe {
    fn new(prefix: &[PathComponent]) -> Self {
        Self {
            prefix: prefix.to_vec(),
            witness: PrefixWitness::Unknown,
        }
    }

    fn reset(&mut self) {
        self.witness = PrefixWitness::Unknown;
    }

    fn observe(&mut self, path: &[PathComponent], replacement: &SelectionReplacement) {
        if path.len() > self.prefix.len() || !self.prefix.starts_with(path) {
            return;
        }
        let boundary = path.len();
        if boundary == self.prefix.len() {
            return;
        }
        let compatible = match replacement {
            SelectionReplacement::ArrayStart => {
                matches!(self.prefix[boundary], PathComponent::Index(_))
            }
            SelectionReplacement::ObjectStart => {
                matches!(self.prefix[boundary], PathComponent::Key(_))
            }
            SelectionReplacement::Scalar(_) | SelectionReplacement::Missing => false,
        };
        let value = match replacement {
            SelectionReplacement::ArrayStart => Value::array(Vec::new()),
            SelectionReplacement::ObjectStart => Value::object(tq_core::Object::new()),
            SelectionReplacement::Scalar(value) => value.clone(),
            SelectionReplacement::Missing => Value::Null,
        };
        self.witness = if compatible {
            PrefixWitness::Missing { boundary, value }
        } else {
            PrefixWitness::Mismatch { boundary, value }
        };
    }

    fn missing(&self) -> PrefixWitness {
        match &self.witness {
            PrefixWitness::Unknown => PrefixWitness::Missing {
                boundary: 0,
                value: Value::Null,
            },
            witness => witness.clone(),
        }
    }
}

enum ProjectionPosition {
    WholeItem,
    Fallback,
    Selected,
    Descendant(usize),
    Unrelated,
}

struct AutomaticExecutor<'a, W, E, M> {
    plan: &'a Plan<Compiled, M>,
    prefix: Vec<PathComponent>,
    projection: Option<Vec<PathComponent>>,
    capture_paths: Option<&'a [Vec<PathComponent>]>,
    variables: &'a BTreeMap<Arc<str>, Value>,
    output: ResultOutput<'a, W>,
    stderr: &'a mut E,
    trace_remaining: usize,
    observations: VmObservations,
    retention: RetentionObservations,
    current: Option<Capture>,
    current_item: Option<Vec<PathComponent>>,
    deferred_item: Option<Value>,
    collected: Option<HybridCollection>,
    hybrid_suffix: Option<&'a Plan<Compiled, HybridBlocking>>,
    last: Option<Value>,
    results: usize,
    current_retained_results: usize,
    current_retained_bytes: usize,
    root_spool: RuntimeSpool,
    root_probe: PrefixProbe,
    root_fallback: Option<SelectionFallback>,
    root_active: bool,
    root_object_order: Vec<Arc<str>>,
    root_object_latest: HashMap<Arc<str>, RuntimeRecordId>,
    root_object_seen: HashSet<Arc<str>>,
    root_runtime_error: Option<VmError>,
    root_fatal: Option<RunError>,
    root_index_bytes: usize,
    root_index_limit: usize,
    root_step_start: Option<u64>,
}

impl<W: Write, E: Write, M> AutomaticExecutor<'_, W, E, M> {
    fn stream_selection(&self) -> StreamSelection {
        StreamSelection::new(self.prefix.clone(), self.projection.clone())
    }

    fn begin_root_stage(&mut self, index: u64) -> Result<(), RunError> {
        self.root_spool
            .begin_root()
            .map_err(|error| runtime_spool_run_error(&error))?;
        self.root_probe.reset();
        self.root_fallback = None;
        self.root_active = true;
        self.root_step_start = Some(self.observations.steps);
        self.root_object_order = Vec::new();
        self.root_object_latest = HashMap::new();
        self.root_object_seen = HashSet::new();
        self.root_index_bytes = 0;
        self.current = None;
        self.current_item = None;
        self.deferred_item = None;
        let _ = index;
        Ok(())
    }

    fn reset_root_items(&mut self) -> Result<(), RunError> {
        self.finish_pending_root_item()?;
        self.root_spool
            .replace_root()
            .map_err(|error| runtime_spool_run_error(&error))?;
        self.root_object_order = Vec::new();
        self.root_object_latest = HashMap::new();
        self.root_object_seen = HashSet::new();
        self.root_index_bytes = 0;
        Ok(())
    }

    fn finish_pending_root_item(&mut self) -> Result<(), RunError> {
        if self.projection.is_some() {
            self.complete_projection_item()
        } else {
            self.complete_capture()
        }
    }

    fn prune_root_value(&self, value: &mut Value) {
        if self.root_active
            && let Some(paths) = self.capture_paths
        {
            prune_object_fields_in_place(value, paths);
        }
    }

    fn prepare_root_storage(
        &mut self,
        additional_value_bytes: usize,
        additional_index_bytes: usize,
    ) -> Result<(), RunError> {
        let io_headroom = if self.root_spool.spooled() {
            REPLAY_BUFFER_BYTES.saturating_mul(2)
        } else {
            0
        };
        self.retention.runtime_spool_fixed_io_bytes_high_water = self
            .retention
            .runtime_spool_fixed_io_bytes_high_water
            .max(io_headroom);
        self.observe_root_staging();
        let retained_memory = self.root_spool.retained_memory_capacity_bytes();
        let fixed = self
            .root_index_bytes
            .saturating_add(self.current.as_ref().map_or(0, |capture| capture.bytes))
            .saturating_add(
                self.deferred_item
                    .as_ref()
                    .map_or(0, estimate_capture_value_bytes),
            )
            .saturating_add(retained_memory);
        let required = fixed
            .saturating_add(additional_value_bytes)
            .saturating_add(additional_index_bytes);
        if required > self.root_index_limit {
            if retained_memory != 0 {
                self.root_spool
                    .spill_to_disk()
                    .map_err(|error| runtime_spool_run_error(&error))?;
                self.retention.runtime_spool_fixed_io_bytes_high_water = self
                    .retention
                    .runtime_spool_fixed_io_bytes_high_water
                    .max(REPLAY_BUFFER_BYTES.saturating_mul(2));
            }
            let fixed_after_spill = fixed.saturating_sub(retained_memory.min(fixed));
            if fixed_after_spill
                .saturating_add(additional_value_bytes)
                .saturating_add(additional_index_bytes)
                > self.root_index_limit
            {
                return Err(RunError::Resource("runtime-staging-memory"));
            }
        }
        let reserved = self
            .root_index_limit
            .saturating_sub(
                self.root_index_bytes
                    .saturating_add(self.current.as_ref().map_or(0, |capture| capture.bytes))
                    .saturating_add(
                        self.deferred_item
                            .as_ref()
                            .map_or(0, estimate_capture_value_bytes),
                    )
                    .saturating_add(additional_value_bytes)
                    .saturating_add(additional_index_bytes),
            )
            .max(1);
        self.root_spool
            .set_memory_threshold(reserved)
            .map_err(|error| runtime_spool_run_error(&error))?;
        self.observe_root_staging();
        Ok(())
    }

    fn observe_root_staging(&mut self) {
        self.retention.root_staging_encoded_bytes_high_water = self
            .retention
            .root_staging_encoded_bytes_high_water
            .max(usize::try_from(self.root_spool.retained_bytes()).unwrap_or(usize::MAX));
        self.retention.root_staging_memory_bytes_high_water = self
            .retention
            .root_staging_memory_bytes_high_water
            .max(self.root_spool.retained_memory_capacity_bytes());
        self.retention.root_staging_index_bytes_high_water = self
            .retention
            .root_staging_index_bytes_high_water
            .max(self.root_index_bytes);
        self.retention.root_staging_spool_bytes_written_high_water = self
            .retention
            .root_staging_spool_bytes_written_high_water
            .max(self.root_spool.spool_bytes_written());
    }

    fn stage_root_value(
        &mut self,
        path: Option<&[PathComponent]>,
        value: Value,
        projected: bool,
    ) -> Result<(), RunError> {
        let item = if projected {
            RuntimeSpoolItem::Projected(value)
        } else {
            RuntimeSpoolItem::Item(value)
        };
        let value_bytes = match &item {
            RuntimeSpoolItem::Projected(value)
            | RuntimeSpoolItem::Item(value)
            | RuntimeSpoolItem::Base(value) => estimate_capture_value_bytes(value),
        };
        let Some(path) = path else {
            self.prepare_root_storage(value_bytes, 0)?;
            self.root_spool
                .push_item(item)
                .map_err(|error| runtime_spool_run_error(&error))?;
            self.observe_root_staging();
            return Ok(());
        };
        let Some(component) = path.get(self.prefix.len()) else {
            self.prepare_root_storage(value_bytes, 0)?;
            self.root_spool
                .push_item(item)
                .map_err(|error| runtime_spool_run_error(&error))?;
            self.observe_root_staging();
            return Ok(());
        };
        let (new_key, latest_missing, required_index_bytes) = match component {
            PathComponent::Key(key) => {
                let new_key = !self.root_object_seen.contains(key);
                let latest_missing = !self.root_object_latest.contains_key(key);
                let required = if new_key || latest_missing {
                    key.len()
                        .saturating_add(std::mem::size_of::<Arc<str>>())
                        .saturating_add(std::mem::size_of::<(Arc<str>, RuntimeRecordId)>())
                        .saturating_add(std::mem::size_of::<Arc<str>>())
                        .saturating_add(ROOT_INDEX_BTREE_NODE_BYTES.saturating_mul(2))
                } else {
                    0
                };
                (new_key, latest_missing, required)
            }
            PathComponent::Index(_) => (false, false, 0),
        };
        let next_index_bytes = self.root_index_bytes.saturating_add(required_index_bytes);
        if next_index_bytes > self.root_index_limit {
            return Err(RunError::Resource("runtime-staging-index"));
        }
        self.prepare_root_storage(value_bytes, required_index_bytes)?;
        if new_key {
            let PathComponent::Key(key) = component else {
                unreachable!("new object key requires a key path component")
            };
            self.root_object_order
                .try_reserve_exact(1)
                .map_err(|_| RunError::Resource("runtime-staging-index"))?;
            self.root_object_seen
                .try_reserve(1)
                .map_err(|_| RunError::Resource("runtime-staging-index"))?;
            self.root_object_order.push(Arc::clone(key));
            self.root_object_seen.insert(Arc::clone(key));
        }
        if latest_missing {
            self.root_object_latest
                .try_reserve(1)
                .map_err(|_| RunError::Resource("runtime-staging-index"))?;
        }
        self.root_index_bytes = next_index_bytes;
        let id = self
            .root_spool
            .push_item(item)
            .map_err(|error| runtime_spool_run_error(&error))?;
        self.observe_root_staging();
        let PathComponent::Key(key) = component else {
            return Ok(());
        };
        self.root_object_latest.insert(Arc::clone(key), id);
        Ok(())
    }

    fn stage_root_base(&mut self, value: Value) -> Result<(), RunError> {
        self.prepare_root_storage(estimate_capture_value_bytes(&value), 0)?;
        self.root_spool
            .push_item(RuntimeSpoolItem::Base(value))
            .map_err(|error| runtime_spool_run_error(&error))?;
        self.observe_root_staging();
        Ok(())
    }

    fn commit_root_stage(&mut self) -> Result<(), RunError> {
        self.finish_pending_root_item()?;
        let fallback = self.root_fallback.take();
        let witness = self.root_probe.missing();
        self.root_spool
            .finish_root()
            .map_err(|error| runtime_spool_run_error(&error))?;
        self.root_active = false;

        let recoverable_error = self.replay_root_stage(fallback, witness)?;

        if let Some(error) = recoverable_error {
            report_document_runtime_error(self.stderr, &error)?;
            self.discard_hybrid_document();
            self.root_runtime_error = Some(error);
        } else if let Some(plan) = self.hybrid_suffix {
            match self.finish_hybrid_document(plan) {
                Ok(()) => self.root_runtime_error = None,
                Err(RunError::Runtime(error)) if is_recoverable_document_runtime_error(&error) => {
                    report_document_runtime_error(self.stderr, &error)?;
                    self.discard_hybrid_document();
                    self.root_runtime_error = Some(error);
                }
                Err(RunError::ReportedRuntime(error)) => {
                    report_document_runtime_error(self.stderr, &error)?;
                    self.discard_hybrid_document();
                    self.root_runtime_error = Some(error);
                }
                Err(error) => return Err(error),
            }
        } else {
            self.root_runtime_error = None;
        }
        self.root_step_start = None;
        Ok(())
    }

    fn replay_root_stage(
        &mut self,
        fallback: Option<SelectionFallback>,
        witness: PrefixWitness,
    ) -> Result<Option<VmError>, RunError> {
        let mut spool = std::mem::replace(
            &mut self.root_spool,
            RuntimeSpool::new(RuntimeSpoolConfig::default()),
        );
        let replay = if matches!(fallback, Some(SelectionFallback::Missing { .. })) {
            self.evaluate_prefix_tail(witness)
                .map_err(|error| match error {
                    RunError::Runtime(error) if is_recoverable_document_runtime_error(&error) => {
                        RunError::ReportedRuntime(error)
                    }
                    error => error,
                })
        } else if let Some(fallback) = fallback {
            match fallback {
                SelectionFallback::PresentScalar { value, .. } => {
                    self.run_staged_item(RuntimeSpoolItem::Base(value))
                }
                SelectionFallback::PresentEmptyArray { .. } => {
                    self.run_staged_item(RuntimeSpoolItem::Base(Value::array(Vec::new())))
                }
                SelectionFallback::PresentEmptyObject { .. } => self.run_staged_item(
                    RuntimeSpoolItem::Base(Value::object(tq_core::Object::new())),
                ),
                SelectionFallback::Missing { .. } => unreachable!("missing handled above"),
            }
        } else if !spool.is_empty() {
            let replay_index_bytes = self.root_index_bytes;
            let mut replay_memory = spool.retained_memory_capacity_bytes();
            let replay_available = self.root_index_limit.saturating_sub(replay_index_bytes);
            let replay_requirement =
                usize::try_from(spool.maximum_decoded_requirement()).unwrap_or(usize::MAX);
            // Keep encoded records in memory when a decoded value still has
            // room in the shared budget.  If the resident capacity plus the
            // largest decoded item consumes that room, spill before replay so
            // the decoded limit is not hidden by the local `spool` variable.
            if replay_memory.saturating_add(replay_requirement) > replay_available {
                spool
                    .spill_to_disk()
                    .map_err(|error| runtime_spool_run_error(&error))?;
                replay_memory = spool.retained_memory_capacity_bytes();
                self.retention.runtime_spool_fixed_io_bytes_high_water = self
                    .retention
                    .runtime_spool_fixed_io_bytes_high_water
                    .max(REPLAY_BUFFER_BYTES.saturating_mul(2));
            }
            let replay_decoded_limit = replay_available.saturating_sub(replay_memory);
            spool
                .set_replay_decoded_budget(replay_decoded_limit)
                .map_err(|error| runtime_spool_run_error(&error))?;
            let order = std::mem::take(&mut self.root_object_order);
            let latest = std::mem::take(&mut self.root_object_latest);
            self.root_object_seen.clear();
            self.root_index_bytes = 0;
            let mut run_item = |item: RuntimeSpoolItem| self.run_staged_item(item);
            if order.is_empty() {
                spool
                    .replay_items(&mut run_item)
                    .map_err(runtime_replay_error)
            } else {
                spool
                    .replay_records(
                        order
                            .into_iter()
                            .filter_map(|key| latest.get(&key).copied()),
                        &mut run_item,
                    )
                    .map_err(runtime_replay_error)
            }
        } else {
            Ok(())
        };
        self.root_spool = spool;
        self.observe_root_staging();
        match replay {
            Err(RunError::ReportedRuntime(error)) => Ok(Some(error)),
            Err(error) => Err(error),
            Ok(()) => Ok(None),
        }
    }

    fn run_staged_item(&mut self, item: RuntimeSpoolItem) -> Result<(), RunError> {
        let result = match item {
            RuntimeSpoolItem::Projected(value) => self.emit_direct(value),
            RuntimeSpoolItem::Item(value) => self.evaluate(value, false),
            RuntimeSpoolItem::Base(value) => self.evaluate(value, true),
        };
        match result {
            Ok(()) => Ok(()),
            Err(RunError::Runtime(error)) if is_recoverable_document_runtime_error(&error) => {
                Err(RunError::ReportedRuntime(error))
            }
            Err(RunError::ReportedRuntime(error)) => Err(RunError::ReportedRuntime(error)),
            Err(error) => Err(error),
        }
    }

    fn discard_hybrid_document(&mut self) {
        if let Some(collected) = self.collected.as_mut() {
            collected.discard_document();
        }
    }

    fn evaluate_prefix_tail(&mut self, witness: PrefixWitness) -> Result<(), RunError> {
        let (PrefixWitness::Mismatch { boundary, value }
        | PrefixWitness::Missing { boundary, value }) = witness
        else {
            return Ok(());
        };
        let mut value = Some(value);
        for component in boundary..self.prefix.len() {
            let Some(input) = value.take() else {
                return Ok(());
            };
            value = self.evaluate_prefix_access(component, input)?;
        }
        if let Some(value) = value {
            self.evaluate(value, true)?;
        }
        Ok(())
    }

    fn evaluate_prefix_access(
        &mut self,
        component: usize,
        input: Value,
    ) -> Result<Option<Value>, RunError> {
        let mut vm = Vm::new_automatic_prefix_access(
            self.plan,
            component,
            input,
            self.remaining_vm_limits(),
            self.variables.clone(),
        )
        .with_trace_limit(self.trace_remaining);
        if let Some(flag) = cancellation() {
            vm = vm.with_cancellation(flag);
        }
        let mut value = None;
        let evaluated = vm.for_each_result(|candidate| {
            value = Some(candidate);
            true
        });
        flush_vm_effects(&vm, self.stderr)?;
        merge_observations(&mut self.observations, vm.observations());
        evaluated.map_err(RunError::Runtime)?;
        if self.trace_remaining != 0 {
            for entry in vm.trace() {
                writeln!(self.stderr, "trace: {entry}")?;
            }
            self.trace_remaining = self.trace_remaining.saturating_sub(vm.trace().len());
        }
        Ok(value)
    }

    fn accept(&mut self, record: StreamRecord) -> Result<(), RunError> {
        if cancellation().is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(RunError::Interrupted);
        }
        let (path, value) = record.into_parts();
        let record_bytes = estimate_event_bytes(&path, value.as_ref());
        if path == self.prefix {
            if let Some(value) = value
                && !matches!(&value, Value::Array(values) if values.is_empty())
                && !matches!(&value, Value::Object(values) if values.is_empty())
            {
                self.observe_complete_value(record_bytes, 0)?;
                if self.root_active {
                    self.stage_root_base(value)?;
                } else {
                    self.evaluate(value, true)?;
                }
            }
            return Ok(());
        }
        if path.len() <= self.prefix.len() || !path.starts_with(&self.prefix) {
            return Ok(());
        }
        if self.projection.is_some() {
            return self.accept_projection(&path, value, record_bytes);
        }
        self.accept_unprojected(&path, value, record_bytes)
    }

    fn accept_unprojected(
        &mut self,
        path: &[PathComponent],
        value: Option<Value>,
        record_bytes: usize,
    ) -> Result<(), RunError> {
        let target_length = self.prefix.len().saturating_add(1);
        let target = path[..target_length].to_vec();
        let relative = &path[target_length..];

        if self
            .current
            .as_ref()
            .is_some_and(|capture| capture.path != target)
        {
            self.complete_capture()?;
        }

        if self.plan.scalar_events_only() {
            if relative.is_empty()
                && let Some(value) = value
            {
                if self.root_active {
                    self.stage_root_value(Some(&target), value, false)?;
                } else {
                    self.evaluate(value, false)?;
                }
            }
            return Ok(());
        }

        if relative.is_empty() {
            if let Some(value) = value {
                self.observe_complete_value(record_bytes, 0)?;
                self.retention.completed_subtrees =
                    self.retention.completed_subtrees.saturating_add(1);
                if self.root_active {
                    let mut value = value;
                    self.prune_root_value(&mut value);
                    self.stage_root_value(Some(&target), value, false)?;
                } else {
                    self.evaluate(value, false)?;
                }
            } else if self
                .current
                .as_ref()
                .is_some_and(|capture| capture.path == target)
            {
                let capture = self.current.take().expect("capture was checked");
                self.retention.completed_subtrees =
                    self.retention.completed_subtrees.saturating_add(1);
                let mut used_bytes = capture.bytes;
                let mut value = capture.root.into_value_bounded(
                    self.output.options.limits.preparation_memory_bytes,
                    &mut used_bytes,
                )?;
                if self.root_active {
                    self.prune_root_value(&mut value);
                    self.stage_root_value(Some(&target), value, false)?;
                } else {
                    self.evaluate(value, false)?;
                }
            }
            return Ok(());
        }
        let Some(value) = value else {
            return Ok(());
        };
        let record_cost = record_bytes.saturating_add(64);
        let value_cost = estimate_capture_value_bytes(&value);
        self.prepare_root_storage(
            record_cost
                .saturating_add(value_cost)
                .saturating_add(build_node_path_bytes(relative)),
            0,
        )?;
        let capture = self.current.get_or_insert_with(|| Capture {
            path: target,
            root: BuildNode::empty_for(&relative[0]),
            bytes: 0,
        });
        capture.bytes = capture.bytes.saturating_add(record_cost);
        if capture.bytes > self.output.options.limits.preparation_memory_bytes {
            return Err(RunError::Resource("subtree-bytes"));
        }
        let relative_depth = relative.len();
        if relative_depth > self.output.options.limits.depth {
            return Err(RunError::Resource("subtree-depth"));
        }
        capture.root.insert_bounded(
            relative,
            value,
            self.output.options.limits.preparation_memory_bytes,
            &mut capture.bytes,
        )?;
        let capture_bytes = capture.bytes;
        self.retention.bytes_high_water = self.retention.bytes_high_water.max(capture_bytes);
        self.retention.depth_high_water = self.retention.depth_high_water.max(relative_depth);
        Ok(())
    }

    fn finish_source(&mut self) -> Result<(), RunError> {
        if self.projection.is_some() {
            self.complete_projection_item()?;
        } else {
            self.complete_capture()?;
        }
        if let Some(plan) = self.hybrid_suffix {
            self.finish_hybrid_document(plan)?;
        }
        Ok(())
    }

    fn accept_projection(
        &mut self,
        path: &[PathComponent],
        value: Option<Value>,
        record_bytes: usize,
    ) -> Result<(), RunError> {
        let item_length = self.prefix.len().saturating_add(1);
        let item = path[..item_length].to_vec();
        if self
            .current_item
            .as_ref()
            .is_some_and(|current| *current != item)
        {
            self.complete_projection_item()?;
        }
        if self.current_item.is_none() {
            self.current_item = Some(item);
        }
        let Some(value) = value else {
            return Ok(());
        };
        let position = self.projection_position(&path[item_length..]);
        match position {
            ProjectionPosition::WholeItem => {
                self.prepare_projection_value(&value, record_bytes)?;
                self.deferred_item = Some(value);
                Ok(())
            }
            ProjectionPosition::Fallback => self.defer_projected_item(&path[item_length..], value),
            ProjectionPosition::Selected => {
                self.prepare_projection_value(&value, record_bytes)?;
                self.current = Some(Capture {
                    path: path.to_vec(),
                    root: BuildNode::Value(value),
                    bytes: record_bytes,
                });
                Ok(())
            }
            ProjectionPosition::Descendant(depth) => {
                self.capture_projection_child(path, item_length + depth, value, record_bytes)
            }
            ProjectionPosition::Unrelated => Ok(()),
        }
    }

    fn projection_position(&self, relative: &[PathComponent]) -> ProjectionPosition {
        if relative.is_empty() {
            return ProjectionPosition::WholeItem;
        }
        let projection = self
            .projection
            .as_deref()
            .expect("projection branch requires a projected path");
        if path_kind_mismatch(relative, projection) {
            return ProjectionPosition::Fallback;
        }
        if relative == projection {
            return ProjectionPosition::Selected;
        }
        if projection.starts_with(relative) {
            return ProjectionPosition::Fallback;
        }
        if relative.starts_with(projection) {
            return ProjectionPosition::Descendant(projection.len());
        }
        ProjectionPosition::Unrelated
    }

    fn prepare_projection_value(
        &mut self,
        value: &Value,
        record_bytes: usize,
    ) -> Result<(), RunError> {
        self.observe_complete_value(record_bytes, 0)?;
        if self.root_active {
            self.prepare_root_storage(estimate_capture_value_bytes(value), 0)?;
        }
        Ok(())
    }

    fn defer_projected_item(
        &mut self,
        path: &[PathComponent],
        value: Value,
    ) -> Result<(), RunError> {
        if self.root_active {
            self.prepare_root_storage(
                estimate_capture_value_bytes(&value).saturating_add(build_node_path_bytes(path)),
                0,
            )?;
        }
        self.deferred_item = Some(synthetic_item(
            path,
            value,
            self.output.options.limits.preparation_memory_bytes,
        )?);
        Ok(())
    }

    fn capture_projection_child(
        &mut self,
        path: &[PathComponent],
        projection_depth: usize,
        value: Value,
        record_bytes: usize,
    ) -> Result<(), RunError> {
        let captured_path = &path[projection_depth..];
        let record_cost = record_bytes.saturating_add(64);
        self.prepare_root_storage(
            record_cost
                .saturating_add(estimate_capture_value_bytes(&value))
                .saturating_add(build_node_path_bytes(captured_path)),
            0,
        )?;
        let capture = self.current.get_or_insert_with(|| Capture {
            path: path[..projection_depth].to_vec(),
            root: BuildNode::empty_for(&captured_path[0]),
            bytes: 0,
        });
        capture.bytes = capture.bytes.saturating_add(record_cost);
        if capture.bytes > self.output.options.limits.preparation_memory_bytes {
            return Err(RunError::Resource("subtree-bytes"));
        }
        capture.root.insert_bounded(
            captured_path,
            value,
            self.output.options.limits.preparation_memory_bytes,
            &mut capture.bytes,
        )?;
        let capture_bytes = capture.bytes;
        self.retention.bytes_high_water = self.retention.bytes_high_water.max(capture_bytes);
        self.retention.depth_high_water = self.retention.depth_high_water.max(captured_path.len());
        Ok(())
    }

    fn complete_projection_item(&mut self) -> Result<(), RunError> {
        let Some(item_path) = self.current_item.take() else {
            return Ok(());
        };
        self.retention.completed_subtrees = self.retention.completed_subtrees.saturating_add(1);
        let deferred = self.deferred_item.take();
        if let Some(capture) = self.current.take() {
            let mut used_bytes = capture.bytes;
            let value = capture.root.into_value_bounded(
                self.output.options.limits.preparation_memory_bytes,
                &mut used_bytes,
            )?;
            if self.root_active {
                self.stage_root_value(Some(&item_path), value, true)?;
            } else {
                self.emit_direct(value)?;
            }
        } else if let Some(item) = deferred {
            if self.root_active {
                let mut item = item;
                self.prune_root_value(&mut item);
                self.stage_root_value(Some(&item_path), item, false)?;
            } else {
                self.evaluate(item, false)?;
            }
        } else {
            if self.root_active {
                self.stage_root_value(Some(&item_path), Value::Null, true)?;
            } else {
                self.emit_direct(Value::Null)?;
            }
        }
        Ok(())
    }

    fn emit_direct(&mut self, value: Value) -> Result<(), RunError> {
        if self.collected.is_some() {
            self.observe_retained_value(&value);
        }
        if let Some(collected) = self.collected.as_mut() {
            return collected.push(value);
        }
        self.output.emit(&value)?;
        self.last = Some(value);
        self.results = self.results.saturating_add(1);
        Ok(())
    }

    fn complete_capture(&mut self) -> Result<(), RunError> {
        let Some(capture) = self.current.take() else {
            return Ok(());
        };
        self.retention.completed_subtrees = self.retention.completed_subtrees.saturating_add(1);
        let mut used_bytes = capture.bytes;
        let value = capture.root.into_value_bounded(
            self.output.options.limits.preparation_memory_bytes,
            &mut used_bytes,
        )?;
        if self.root_active {
            let mut value = value;
            self.prune_root_value(&mut value);
            self.stage_root_value(Some(&capture.path), value, false)
        } else {
            self.evaluate(value, false)
        }
    }

    fn observe_complete_value(&mut self, bytes: usize, depth: usize) -> Result<(), RunError> {
        if !self.plan.scalar_events_only()
            && bytes > self.output.options.limits.preparation_memory_bytes
        {
            return Err(RunError::Resource("subtree-bytes"));
        }
        self.retention.bytes_high_water = self.retention.bytes_high_water.max(bytes);
        self.retention.depth_high_water = self.retention.depth_high_water.max(depth);
        Ok(())
    }

    fn evaluate(&mut self, input: Value, base: bool) -> Result<(), RunError> {
        let limits = self.remaining_vm_limits();
        let mut vm = if base {
            Vm::new_automatic_base(self.plan, input, limits, self.variables.clone())
        } else {
            Vm::new_automatic_item(self.plan, input, limits, self.variables.clone())
        }
        .with_trace_limit(self.trace_remaining);
        if let Some(flag) = cancellation() {
            vm = vm.with_cancellation(flag);
        }
        let mut output_error = None;
        let evaluated = vm.for_each_result(|value| {
            if self.collected.is_some() {
                self.observe_retained_value(&value);
            }
            if let Some(collected) = self.collected.as_mut() {
                if let Err(error) = collected.push(value) {
                    output_error = Some(error);
                    return false;
                }
                return true;
            }
            if let Err(error) = self.output.emit(&value) {
                output_error = Some(error);
                return false;
            }
            self.last = Some(value);
            self.results = self.results.saturating_add(1);
            true
        });
        flush_vm_effects(&vm, self.stderr)?;
        merge_observations(&mut self.observations, vm.observations());
        if let Some(error) = output_error {
            return Err(error);
        }
        evaluated.map_err(RunError::Runtime)?;
        if self.trace_remaining != 0 {
            for entry in vm.trace() {
                writeln!(self.stderr, "trace: {entry}")?;
            }
            self.trace_remaining = self.trace_remaining.saturating_sub(vm.trace().len());
        }
        Ok(())
    }

    fn finish_hybrid_document(
        &mut self,
        plan: &Plan<Compiled, HybridBlocking>,
    ) -> Result<(), RunError> {
        let (values, preparation) = self
            .collected
            .as_mut()
            .expect("hybrid execution collects producer results")
            .finish_document()?;
        self.retention.sort_runs = self.retention.sort_runs.saturating_add(preparation.batches);
        self.retention.in_flight_batches_high_water = self
            .retention
            .in_flight_batches_high_water
            .max(preparation.in_flight_batches);
        self.retention.in_flight_bytes_high_water = self
            .retention
            .in_flight_bytes_high_water
            .max(preparation.in_flight_bytes);
        let mut vm = Vm::new_hybrid_suffix(
            plan,
            Value::array(values),
            self.remaining_vm_limits(),
            self.variables.clone(),
        )
        .with_trace_limit(self.trace_remaining);
        if let Some(flag) = cancellation() {
            vm = vm.with_cancellation(flag);
        }
        let mut output_error = None;
        let evaluated = vm.for_each_result(|value| {
            if let Err(error) = self.output.emit(&value) {
                output_error = Some(error);
                return false;
            }
            self.last = Some(value);
            self.results = self.results.saturating_add(1);
            true
        });
        flush_vm_effects(&vm, self.stderr)?;
        merge_observations(&mut self.observations, vm.observations());
        self.current_retained_results = 0;
        self.current_retained_bytes = 0;
        if let Some(error) = output_error {
            return Err(error);
        }
        evaluated.map_err(RunError::Runtime)?;
        if self.trace_remaining != 0 {
            for entry in vm.trace() {
                writeln!(self.stderr, "trace: {entry}")?;
            }
            self.trace_remaining = self.trace_remaining.saturating_sub(vm.trace().len());
        }
        Ok(())
    }

    fn remaining_vm_limits(&self) -> VmLimits {
        let mut limits = vm_limits(self.output.options);
        let used_steps = self
            .root_step_start
            .map_or(self.observations.steps, |start| {
                self.observations.steps.saturating_sub(start)
            });
        limits.steps = limits.steps.saturating_sub(used_steps);
        limits
    }

    fn observe_retained_value(&mut self, value: &Value) {
        self.current_retained_results = self.current_retained_results.saturating_add(1);
        self.current_retained_bytes = self
            .current_retained_bytes
            .saturating_add(estimate_value_bytes(value).saturating_add(32));
        self.retention.retained_results_high_water = self
            .retention
            .retained_results_high_water
            .max(self.current_retained_results);
        self.retention.retained_bytes_high_water = self
            .retention
            .retained_bytes_high_water
            .max(self.current_retained_bytes);
    }
}

fn runtime_spool_run_error(error: &RuntimeSpoolError) -> RunError {
    match error {
        RuntimeSpoolError::Cancelled => RunError::Interrupted,
        RuntimeSpoolError::Io(_) => RunError::Resource("runtime-staging-io"),
        RuntimeSpoolError::TotalLimit
        | RuntimeSpoolError::SpoolLimit
        | RuntimeSpoolError::ItemLimit
        | RuntimeSpoolError::TokenLimit
        | RuntimeSpoolError::DepthLimit
        | RuntimeSpoolError::DecodedLimit
        | RuntimeSpoolError::SpoolDisabled => RunError::Resource("runtime-staging"),
        RuntimeSpoolError::Number(_) => RunError::Resource("runtime-staging-number"),
        RuntimeSpoolError::Decode(_)
        | RuntimeSpoolError::Utf8
        | RuntimeSpoolError::State(_)
        | RuntimeSpoolError::GenerationLimit
        | RuntimeSpoolError::StaleRecord
        | RuntimeSpoolError::RecordBounds => RunError::Unsupported(
            "automatic runtime staging produced an invalid retained record".to_owned(),
        ),
    }
}

fn runtime_replay_error(error: RuntimeSpoolReplayError<RunError>) -> RunError {
    match error {
        RuntimeSpoolReplayError::Storage(error) => runtime_spool_run_error(&error),
        RuntimeSpoolReplayError::Consumer(error) => error,
    }
}

impl<W: Write, E: Write, M> SelectedRootSink for AutomaticExecutor<'_, W, E, M> {
    type Error = String;

    fn begin_root(&mut self, index: u64) -> Result<(), Self::Error> {
        self.begin_root_stage(index).map_err(|error| {
            self.root_fatal = Some(error);
            "automatic root staging could not begin".to_owned()
        })
    }

    fn fallback(&mut self, fallback: SelectionFallback) -> Result<(), Self::Error> {
        self.finish_pending_root_item().map_err(|error| {
            self.root_fatal = Some(error);
            "automatic root staging could not finish a pending item".to_owned()
        })?;
        self.root_fallback = Some(fallback);
        Ok(())
    }

    fn replace_selected_prefix(
        &mut self,
        path: &[PathComponent],
        replacement: SelectionReplacement,
    ) -> Result<(), Self::Error> {
        self.reset_root_items().map_err(|error| {
            self.root_fatal = Some(error);
            "automatic root staging could not replace a selected prefix".to_owned()
        })?;
        self.root_probe.observe(path, &replacement);
        self.root_fallback = None;
        Ok(())
    }

    fn replace_item_path(
        &mut self,
        path: &[PathComponent],
        _replacement: SelectionReplacement,
    ) -> Result<(), Self::Error> {
        let item_length = self.prefix.len() + 1;
        let target = &path[..item_length];
        let pending = self.current_item.as_deref().or_else(|| {
            self.current
                .as_ref()
                .map(|capture| &capture.path[..item_length])
        });
        if path.len() == item_length || pending.is_some_and(|pending| pending != target) {
            self.finish_pending_root_item().map_err(|error| {
                self.root_fatal = Some(error);
                "automatic root staging could not replace an item".to_owned()
            })?;
        }
        if path.len() == item_length {
            if let Some(PathComponent::Key(key)) = path.get(self.prefix.len()) {
                self.root_object_latest.remove(key);
            }
        } else {
            if self
                .current
                .as_ref()
                .is_some_and(|capture| capture.path.starts_with(path))
            {
                self.current = None;
            } else if let Some(capture) = self.current.as_mut()
                && path.starts_with(&capture.path)
            {
                capture.root.clear_path(&path[capture.path.len()..]);
            }
            if self
                .projection
                .as_ref()
                .is_some_and(|projection| projection.starts_with(&path[item_length..]))
            {
                self.deferred_item = None;
            }
        }
        Ok(())
    }

    fn record(&mut self, record: StreamRecord) -> Result<(), Self::Error> {
        self.accept(record).map_err(|error| {
            self.root_fatal = Some(error);
            "automatic root staging rejected a selected record".to_owned()
        })
    }

    fn finish_root(&mut self) -> Result<(), Self::Error> {
        self.commit_root_stage().map_err(|error| {
            self.root_fatal = Some(error);
            "automatic root staging could not commit the validated root".to_owned()
        })
    }

    fn abort_root(&mut self) {
        self.current = None;
        self.current_item = None;
        self.deferred_item = None;
        let _ = self.root_spool.abort_root();
        self.root_active = false;
        self.root_step_start = None;
        self.root_fallback = None;
        self.root_object_order.clear();
        self.root_object_latest.clear();
        self.root_object_seen.clear();
        self.root_index_bytes = 0;
        self.root_probe.reset();
    }
}

struct Capture {
    path: Vec<PathComponent>,
    root: BuildNode,
    bytes: usize,
}

enum BuildNode {
    Missing,
    Value(Value),
    Array(Vec<BuildNode>),
    Object(Vec<(Arc<str>, BuildNode)>),
}

impl BuildNode {
    fn clear_path(&mut self, path: &[PathComponent]) {
        let Some((component, tail)) = path.split_first() else {
            *self = Self::Missing;
            return;
        };
        match (self, component) {
            (Self::Array(values), PathComponent::Index(index)) => {
                if let Some(child) = values.get_mut(*index) {
                    child.clear_path(tail);
                }
            }
            (Self::Object(values), PathComponent::Key(key)) => {
                if let Some((_, child)) = values.iter_mut().find(|(candidate, _)| candidate == key)
                {
                    child.clear_path(tail);
                }
            }
            _ => {}
        }
    }

    fn empty_for(component: &PathComponent) -> Self {
        match component {
            PathComponent::Index(_) => Self::Array(Vec::new()),
            PathComponent::Key(_) => Self::Object(Vec::new()),
        }
    }

    fn insert_bounded(
        &mut self,
        path: &[PathComponent],
        value: Value,
        maximum_bytes: usize,
        used_bytes: &mut usize,
    ) -> Result<(), RunError> {
        let Some((component, tail)) = path.split_first() else {
            *self = Self::Value(value);
            return Ok(());
        };
        match component {
            PathComponent::Index(index) => {
                let Self::Array(values) = self else {
                    return Err(invalid_automatic_event());
                };
                if values.len() <= *index {
                    let additional = index.saturating_add(1).saturating_sub(values.len());
                    let bytes =
                        additional.saturating_mul(std::mem::size_of::<Self>().saturating_add(16));
                    charge_build_node_bytes(used_bytes, maximum_bytes, bytes)?;
                    values
                        .try_reserve_exact(additional)
                        .map_err(|_| RunError::Resource("subtree-memory"))?;
                    values.resize_with(index.saturating_add(1), || Self::Missing);
                }
                if !tail.is_empty() && matches!(values[*index], Self::Missing) {
                    values[*index] = Self::empty_for(&tail[0]);
                }
                values[*index].insert_bounded(tail, value, maximum_bytes, used_bytes)
            }
            PathComponent::Key(key) => {
                let Self::Object(values) = self else {
                    return Err(invalid_automatic_event());
                };
                let index = values.iter().position(|(candidate, _)| candidate == key);
                let child = if let Some(index) = index {
                    &mut values[index].1
                } else {
                    charge_build_node_bytes(
                        used_bytes,
                        maximum_bytes,
                        std::mem::size_of::<(Arc<str>, Self)>().saturating_add(256),
                    )?;
                    values
                        .try_reserve_exact(1)
                        .map_err(|_| RunError::Resource("subtree-memory"))?;
                    values.push((Arc::clone(key), Self::Missing));
                    &mut values.last_mut().expect("entry was pushed").1
                };
                if !tail.is_empty() && matches!(child, Self::Missing) {
                    *child = Self::empty_for(&tail[0]);
                }
                child.insert_bounded(tail, value, maximum_bytes, used_bytes)
            }
        }
    }

    fn into_value_bounded(
        self,
        maximum_bytes: usize,
        used_bytes: &mut usize,
    ) -> Result<Value, RunError> {
        match self {
            Self::Missing => Err(invalid_automatic_event()),
            Self::Value(value) => Ok(value),
            Self::Array(values) => {
                let bytes = values
                    .len()
                    .saturating_mul(std::mem::size_of::<Value>().saturating_add(16));
                charge_build_node_bytes(used_bytes, maximum_bytes, bytes)?;
                let mut output = Vec::new();
                output
                    .try_reserve_exact(values.len())
                    .map_err(|_| RunError::Resource("subtree-memory"))?;
                for value in values {
                    output.push(value.into_value_bounded(maximum_bytes, used_bytes)?);
                }
                Ok(Value::array(output))
            }
            Self::Object(values) => {
                let bytes = values
                    .len()
                    .saturating_mul(std::mem::size_of::<(Arc<str>, Value)>().saturating_add(256));
                charge_build_node_bytes(used_bytes, maximum_bytes, bytes)?;
                let mut output = tq_core::Object::new();
                output
                    .try_reserve(values.len())
                    .map_err(|_| RunError::Resource("subtree-memory"))?;
                for (key, value) in values {
                    output.insert(key, value.into_value_bounded(maximum_bytes, used_bytes)?);
                }
                Ok(Value::object(output))
            }
        }
    }
}

fn charge_build_node_bytes(
    used_bytes: &mut usize,
    maximum_bytes: usize,
    additional_bytes: usize,
) -> Result<(), RunError> {
    let Some(next) = used_bytes.checked_add(additional_bytes) else {
        return Err(RunError::Resource("subtree-memory"));
    };
    if next > maximum_bytes {
        return Err(RunError::Resource("subtree-memory"));
    }
    *used_bytes = next;
    Ok(())
}

fn build_node_path_bytes(path: &[PathComponent]) -> usize {
    path.iter().fold(0_usize, |total, component| {
        total.saturating_add(match component {
            PathComponent::Index(_) => std::mem::size_of::<BuildNode>().saturating_add(16),
            PathComponent::Key(_) => {
                std::mem::size_of::<(Arc<str>, BuildNode)>().saturating_add(256)
            }
        })
    })
}

fn path_kind_mismatch(actual: &[PathComponent], expected: &[PathComponent]) -> bool {
    for (actual, expected) in actual.iter().zip(expected) {
        if actual == expected {
            continue;
        }
        return matches!(actual, PathComponent::Key(_))
            != matches!(expected, PathComponent::Key(_));
    }
    false
}

fn synthetic_item(
    relative: &[PathComponent],
    value: Value,
    maximum_bytes: usize,
) -> Result<Value, RunError> {
    let Some(first) = relative.first() else {
        return Ok(value);
    };
    let mut root = BuildNode::empty_for(first);
    let mut used_bytes = 0;
    root.insert_bounded(relative, value, maximum_bytes, &mut used_bytes)?;
    root.into_value_bounded(maximum_bytes, &mut used_bytes)
}

fn estimate_event_bytes(path: &[PathComponent], value: Option<&Value>) -> usize {
    let path_bytes = path.iter().fold(16_usize, |total, component| {
        total.saturating_add(match component {
            PathComponent::Key(key) => key.len().saturating_add(16),
            PathComponent::Index(_) => 16,
        })
    });
    path_bytes.saturating_add(value.map_or(0, estimate_value_bytes))
}

fn estimate_value_bytes(value: &Value) -> usize {
    match value {
        Value::Null => 4,
        Value::Bool(_) => 5,
        Value::Number(number) => number.to_string().len(),
        Value::String(value) => value.len().saturating_add(16),
        Value::Array(values) => values.iter().fold(24_usize, |total, value| {
            total.saturating_add(estimate_value_bytes(value))
        }),
        Value::Object(values) => values.iter().fold(32_usize, |total, (key, value)| {
            total
                .saturating_add(key.len())
                .saturating_add(estimate_value_bytes(value))
        }),
    }
}

pub(super) fn estimate_capture_value_bytes(value: &Value) -> usize {
    estimate_value_bytes(value).saturating_add(
        value_node_count(value).saturating_mul(std::mem::size_of::<Value>().saturating_add(16)),
    )
}

/// Retains only the statically proven object fields in an already materialized
/// item.  The proof is deliberately conservative, so a shared object is left
/// untouched rather than allocating a reduced clone beside its source.
pub(super) fn prune_object_fields_in_place(value: &mut Value, paths: &[Vec<PathComponent>]) {
    let Some(anchor) = paths.first().map(Vec::as_slice) else {
        return;
    };
    prune_object_fields_at_depth(value, paths, 0, anchor);
}

fn prune_object_fields_at_depth(
    value: &mut Value,
    paths: &[Vec<PathComponent>],
    depth: usize,
    anchor: &[PathComponent],
) {
    let Value::Object(values) = value else {
        return;
    };
    let Some(values) = Arc::get_mut(values) else {
        return;
    };
    values.retain(|key, child| {
        let Some(child_anchor) = paths
            .iter()
            .find(|path| path_matches_branch(path, anchor, depth, key))
            .map(Vec::as_slice)
        else {
            return false;
        };
        let has_nested_path = paths.iter().any(|path| {
            path_matches_branch(path, anchor, depth, key) && path.len() > depth.saturating_add(1)
        });
        let keeps_whole_child = paths.iter().any(|path| {
            path_matches_branch(path, anchor, depth, key) && path.len() == depth.saturating_add(1)
        });
        if has_nested_path && !keeps_whole_child {
            prune_object_fields_at_depth(child, paths, depth.saturating_add(1), child_anchor);
        }
        true
    });
}

fn path_matches_branch(
    path: &[PathComponent],
    anchor: &[PathComponent],
    depth: usize,
    key: &Arc<str>,
) -> bool {
    path.get(..depth) == anchor.get(..depth)
        && matches!(
            path.get(depth),
            Some(PathComponent::Key(candidate)) if candidate == key
        )
}

fn value_node_count(value: &Value) -> usize {
    1_usize.saturating_add(match value {
        Value::Array(values) => values.iter().fold(0_usize, |total, value| {
            total.saturating_add(value_node_count(value))
        }),
        Value::Object(values) => values.values().fold(0_usize, |total, value| {
            total.saturating_add(value_node_count(value))
        }),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => 0,
    })
}

fn invalid_automatic_event() -> RunError {
    RunError::Unsupported("automatic decoder produced an invalid path/value event".to_owned())
}
