//! Resolve the query and select an execution plan using the input formats.

use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    path::Path,
    sync::Arc,
};

use tq_core::{
    Analysis, AnalysisContext, Analyzed, AutomaticPlan, Compiled, PlanKind, Program, Query,
    ResolveOptions, Resolved, Value, analyze_with_context, parse_bytes, parse_with_startup,
    resolve,
};
use tq_formats::{InputFormat, probe_format, probe_reader};

use super::{
    RunError,
    automatic::{run_automatic_filter, run_hybrid_filter},
    detection::{
        DetectionObservation, auto_file_common_format, auto_file_events_available,
        decoder_events_available, json_sequence_input_requested, validate_stream_input_formats,
    },
    document::DocumentPlan,
    events::run_event_filter,
    input::{LimitedReader, proxyable_format_error, read_limited},
    query::{load_filter, module_roots, parse_external_arguments, validate_capability_policy},
    reports::write_explain,
    test_file::RunTestCaptureHandle,
    transcode::{run_transcode_filter, transcode_proof},
};
use crate::{ExecutionOverride, ExitStatus, RunOptions};

pub(super) fn run_filter<R: Read + Send, W: Write, E: Write>(
    options: &RunOptions,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
    capture: Option<RunTestCaptureHandle>,
) -> Result<ExitStatus, RunError> {
    validate_capability_policy(options)?;
    validate_stream_input_formats(options)?;
    let filter = ResolvedFilter::load(options)?;
    let automatic = filter.permits_automatic_events();
    let output = FilterOutput {
        stdout,
        stderr,
        capture,
    };
    if automatic && options.input_format == InputFormat::Auto {
        return filter.run_with_probed_input(stdin, output);
    }
    filter.run(
        stdin,
        output,
        InputPlan {
            events: automatic && decoder_events_available(options, options.input_format),
            transcode: explicit_transcode_format(options),
            detections: Vec::new(),
        },
    )
}

struct FilterOutput<'a, W, E> {
    stdout: &'a mut W,
    stderr: &'a mut E,
    capture: Option<RunTestCaptureHandle>,
}

#[derive(Default)]
struct InputPlan {
    events: bool,
    transcode: Option<InputFormat>,
    detections: Vec<DetectionObservation>,
}

struct ResolvedFilter<'a> {
    options: &'a RunOptions,
    query: Query<Resolved>,
    variables: BTreeMap<Arc<str>, Value>,
}

impl<'a> ResolvedFilter<'a> {
    fn load(options: &'a RunOptions) -> Result<Self, RunError> {
        let (query_name, query, startup) = load_filter(options)?;
        let variables = parse_external_arguments(options)?;
        let resolve_options = ResolveOptions {
            variables: variables
                .keys()
                .filter(|name| !name.starts_with("__tq_"))
                .cloned()
                .collect::<BTreeSet<_>>(),
            module_roots: module_roots(options, &query_name),
            module_limit: options.limits.depth,
            module_bytes: usize::try_from(options.limits.input_bytes)
                .unwrap_or(usize::MAX)
                .min(ResolveOptions::default().module_bytes),
        };
        let parsed = startup
            .map_or_else(
                || parse_bytes(&query_name, &query),
                |(startup_name, startup)| {
                    parse_with_startup(&query_name, &query, &startup_name, &startup)
                },
            )
            .map_err(RunError::Compile)?;
        let query = resolve(parsed, &resolve_options).map_err(RunError::Compile)?;
        Ok(Self {
            options,
            query,
            variables,
        })
    }

    fn permits_automatic_events(&self) -> bool {
        let capabilities = analyze_with_context(
            self.query.clone(),
            AnalysisContext {
                automatic_streaming: true,
                ..AnalysisContext::default()
            },
        )
        .capabilities();
        let options = self.options;
        !options.stream
            && !options.slurp
            && !options.raw_input
            && !options.null_input
            && !capabilities.whole_input
            && !json_sequence_input_requested(options)
    }

    fn run_with_probed_input<R: Read + Send, W: Write, E: Write>(
        self,
        stdin: &mut R,
        output: FilterOutput<'_, W, E>,
    ) -> Result<ExitStatus, RunError> {
        let options = self.options;
        let file_events = auto_file_events_available(options)?;
        let uses_stdin =
            options.files.is_empty() || options.files.iter().any(|path| path == Path::new("-"));
        if options.proxy_on_error {
            let input = InputPlan {
                events: file_events,
                ..InputPlan::default()
            };
            if uses_stdin {
                return self.run_proxy_input(stdin, output, input);
            }
            return self.run(stdin, output, input);
        }
        let common = auto_file_common_format(options)?;
        let input = InputPlan {
            events: file_events,
            transcode: common.selected,
            detections: common.detections,
        };
        if uses_stdin {
            return self.run_probed_stdin(stdin, output, input);
        }
        self.run(stdin, output, input)
    }

    fn run_proxy_input<R: Read + Send, W: Write, E: Write>(
        self,
        stdin: &mut R,
        output: FilterOutput<'_, W, E>,
        mut input: InputPlan,
    ) -> Result<ExitStatus, RunError> {
        let options = self.options;
        let bytes = read_limited(stdin, options.limits.input_bytes, "<stdin>")?;
        let stdin_events = match probe_format(&bytes, options.limits.lookahead_bytes) {
            Ok(probe) => decoder_events_available(options, probe.selected),
            Err(error) if proxyable_format_error(&error) => false,
            Err(error) => return Err(error.into()),
        };
        input.events &= stdin_events;
        self.run(&mut bytes.as_slice(), output, input)
    }

    fn run_probed_stdin<R: Read + Send, W: Write, E: Write>(
        self,
        stdin: &mut R,
        output: FilterOutput<'_, W, E>,
        mut input: InputPlan,
    ) -> Result<ExitStatus, RunError> {
        let options = self.options;
        let reader = LimitedReader::new(stdin, options.limits.input_bytes, "<stdin>");
        let (probe, mut replay) = probe_reader(reader, options.limits.lookahead_bytes)?;
        input.transcode = input
            .transcode
            .filter(|format| *format == probe.selected)
            .or_else(|| options.files.is_empty().then_some(probe.selected));
        input
            .detections
            .push(DetectionObservation::new("<stdin>", &probe));
        input.events = input.events && decoder_events_available(options, probe.selected);
        self.run(&mut replay, output, input)
    }

    fn run<R: Read + Send, W: Write, E: Write>(
        self,
        stdin: &mut R,
        output: FilterOutput<'_, W, E>,
        input: InputPlan,
    ) -> Result<ExitStatus, RunError> {
        let Self {
            options,
            query,
            variables,
        } = self;
        let analyzed = analyze_with_context(
            query,
            AnalysisContext {
                event_input: options.stream,
                whole_input: options.slurp,
                automatic_streaming: input.events,
            },
        );
        let analysis = select_plan(options, &analyzed, input.transcode);
        if let Some(explain) = options.explain {
            write_explain(explain, options, &analyzed, &analysis, output.stderr)?;
        }
        let program = analyzed.compile().map_err(RunError::Compile)?;
        CompiledFilter {
            options,
            variables,
            analysis,
            program,
        }
        .run(stdin, output, &input.detections)
    }
}

fn explicit_transcode_format(options: &RunOptions) -> Option<InputFormat> {
    match options.input_format {
        InputFormat::Json | InputFormat::Toon if !json_sequence_input_requested(options) => {
            Some(options.input_format)
        }
        _ => None,
    }
}

fn select_plan(
    options: &RunOptions,
    analyzed: &Query<Analyzed>,
    transcode_input: Option<InputFormat>,
) -> Analysis {
    let mut analysis = analyzed.analysis().clone();
    if options.execution_override == ExecutionOverride::Document {
        let reason = "forced document override for differential run";
        if !options.stream {
            analysis.selected_plan = PlanKind::Document;
            analysis.stream_rejection = Some(reason.to_owned());
        }
        analysis.transcode_rejection = Some(reason.to_owned());
        return analysis;
    }
    match transcode_proof(options, analyzed.capabilities(), transcode_input) {
        Ok(proof) => {
            analysis.selected_plan = PlanKind::Transcode;
            analysis.transcode_proof = Some(proof);
            analysis.transcode_rejection = None;
        }
        Err(reason) => analysis.transcode_rejection = Some(reason.to_owned()),
    }
    analysis
}

struct CompiledFilter<'a> {
    options: &'a RunOptions,
    variables: BTreeMap<Arc<str>, Value>,
    analysis: Analysis,
    program: Program<Compiled>,
}

impl CompiledFilter<'_> {
    fn run<R: Read + Send, W: Write, E: Write>(
        self,
        stdin: &mut R,
        output: FilterOutput<'_, W, E>,
        detections: &[DetectionObservation],
    ) -> Result<ExitStatus, RunError> {
        let Self {
            options,
            variables,
            analysis,
            program,
        } = self;
        let FilterOutput {
            stdout,
            stderr,
            capture,
        } = output;
        match analysis.selected_plan {
            PlanKind::Transcode => {
                let proof = analysis
                    .transcode_proof
                    .expect("selected transcode analysis carries a proof");
                let plan = program.transcode_plan(proof).map_err(RunError::Compile)?;
                run_transcode_filter(options, &plan, &analysis, detections, stdin, stdout)
            }
            PlanKind::Events if options.stream => {
                let plan = program.event_plan().map_err(RunError::Compile)?;
                run_event_filter(
                    options, &plan, &variables, &analysis, stdin, stdout, stderr, capture,
                )
            }
            PlanKind::Events | PlanKind::Subtree | PlanKind::HybridBlocking => {
                match program.automatic_plan().map_err(RunError::Compile)? {
                    AutomaticPlan::Events(plan) => run_automatic_filter(
                        options, &plan, &variables, &analysis, stdin, stdout, stderr, capture,
                    ),
                    AutomaticPlan::Subtree(plan) => run_automatic_filter(
                        options, &plan, &variables, &analysis, stdin, stdout, stderr, capture,
                    ),
                    AutomaticPlan::HybridBlocking(plan) => run_hybrid_filter(
                        options, &plan, &variables, &analysis, stdin, stdout, stderr, capture,
                    ),
                    _ => unreachable!("automatic selection returns an automatic typed plan"),
                }
            }
            _ => DocumentPlan {
                options,
                variables: &variables,
                plan: &program.document_plan(),
                analysis: &analysis,
            }
            .run(stdin, stdout, stderr, capture),
        }
    }
}
