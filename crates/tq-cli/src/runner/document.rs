//! Document-plan execution and coordination of input roots, results, and effects.

use std::{
    collections::BTreeMap,
    io::{Read, Write},
    sync::{
        Arc,
        mpsc::{Receiver, sync_channel},
    },
};

use tq_core::{
    Analysis, Compiled, Document, InputCursor, Plan, Value, Vm, VmError, VmObservations,
};
use tq_formats::{FormatError, InputFormat};

use self::inputs::{DocumentInputs, ProxyHandoff};
use super::{
    RunError,
    automatic::RetentionObservations,
    cancellation, flush_buffered_effects, flush_vm_effects,
    input::{StructuredInput, render_native_failure, render_recovery_warning},
    is_recoverable_document_runtime_error,
    output::ResultOutput,
    process_input_diagnostic, record_document_observations, report_document_runtime_error,
    reports::{ReportExecution, write_report},
    test_file::RunTestCaptureHandle,
    vm_limits,
};
use crate::{ExitStatus, RunOptions};

mod inputs;
mod live;

pub(super) struct DocumentPlan<'a> {
    pub(super) options: &'a RunOptions,
    pub(super) variables: &'a BTreeMap<Arc<str>, Value>,
    pub(super) plan: &'a Plan<Compiled, Document>,
    pub(super) analysis: &'a Analysis,
}

impl DocumentPlan<'_> {
    pub(super) fn run<R: Read + Send, W: Write, E: Write>(
        self,
        stdin: &mut R,
        stdout: &mut W,
        stderr: &mut E,
        capture: Option<RunTestCaptureHandle>,
    ) -> Result<ExitStatus, RunError> {
        let (proxy_sender, proxy_receiver) = sync_channel(0);
        let options = self.options;
        let whole_input = self.analysis.capabilities.whole_input;
        let mut execution = DocumentExecution {
            plan: self,
            output: DocumentOutput {
                results: ResultOutput::with_capture(stdout, options, capture),
                count: 0,
                last: None,
            },
            stderr,
            observations: Vec::new(),
            runtime_error: None,
            runtime_error_reported: false,
            proxy_receiver: Some(proxy_receiver),
        };
        let mut diagnostics = Vec::new();
        DocumentInputs {
            options,
            whole_input,
            proxy_sender,
        }
        .drive(stdin, &mut diagnostics, &mut |input, cursor| {
            execution.evaluate(input, cursor)
        })?;
        for diagnostic in diagnostics {
            writeln!(
                execution.stderr,
                "tq: {}",
                process_input_diagnostic(&diagnostic)
            )?;
        }
        execution.finish()
    }

    fn allows_recovery(&self, error: &VmError) -> bool {
        self.can_run_multiple_roots() && is_recoverable_document_runtime_error(error)
    }

    fn can_run_multiple_roots(&self) -> bool {
        !self.options.null_input && !self.options.slurp
    }

    fn uses_live_vm(&self) -> bool {
        let options = self.options;
        options.unbuffered || (self.analysis.capabilities.whole_input && proxy_whole_input(options))
    }
}

fn proxy_whole_input(options: &RunOptions) -> bool {
    options.proxy_on_error && !options.slurp && !options.raw_input && !options.stream
}

struct DocumentExecution<'a, 'out, W, E> {
    plan: DocumentPlan<'a>,
    output: DocumentOutput<'out, W>,
    stderr: &'out mut E,
    observations: Vec<VmObservations>,
    runtime_error: Option<VmError>,
    runtime_error_reported: bool,
    proxy_receiver: Option<Receiver<ProxyHandoff>>,
}

impl<W: Write, E: Write> DocumentExecution<'_, '_, W, E> {
    fn evaluate(
        &mut self,
        input: StructuredInput,
        cursor: Option<InputCursor>,
    ) -> Result<bool, RunError> {
        // Runtime status belongs to the current input root.
        self.runtime_error = None;
        match input {
            StructuredInput::Value(input) => return self.evaluate_value(input, cursor),
            StructuredInput::Document(document) => {
                let cursor = InputCursor::from_input_values(vec![document.clone()]);
                let _ = cursor.next_value()?;
                return self.evaluate_value(document.value, Some(cursor));
            }
            StructuredInput::Proxy(bytes) => self.output.results.proxy(&bytes)?,
            StructuredInput::Failure(failure) => {
                render_native_failure(self.stderr, &failure, self.plan.options.json_sequence)?;
            }
            StructuredInput::Warning { message, context } => {
                render_recovery_warning(self.stderr, &context, &message)?;
            }
        }
        Ok(true)
    }

    fn evaluate_value(
        &mut self,
        input: Value,
        cursor: Option<InputCursor>,
    ) -> Result<bool, RunError> {
        let options = self.plan.options;
        self.runtime_error_reported = false;
        let mut vm = Vm::new_with_variables(
            self.plan.plan,
            input,
            vm_limits(options),
            self.plan.variables.clone(),
        )
        .with_trace_limit(options.trace_limit);
        if let Some(cursor) = cursor {
            vm = vm.with_input_cursor(cursor);
        }
        let (error, observations) = if self.plan.uses_live_vm() {
            live::run(vm, &mut self.output, self.stderr, &mut self.proxy_receiver)?
        } else {
            self.run_buffered(vm)?
        };
        if let Some(error) = &error
            && self.plan.allows_recovery(error)
        {
            report_document_runtime_error(self.stderr, error)?;
            self.runtime_error_reported = true;
        }
        record_document_observations(options, &mut self.observations, observations);
        self.runtime_error = error;
        Ok(self.runtime_error.is_none() || self.runtime_error_reported)
    }

    fn run_buffered(&mut self, mut vm: Vm) -> Result<(Option<VmError>, VmObservations), RunError> {
        if let Some(flag) = cancellation() {
            vm = vm.with_cancellation(flag);
        }
        if self.plan.options.trace_limit == 0 && self.plan.can_run_multiple_roots() {
            return self.drain_buffered(&mut vm);
        }
        let error = loop {
            let next = vm.next_result();
            flush_vm_effects(&vm, self.stderr)?;
            match next {
                Ok(Some(value)) => self.output.emit(value)?,
                Ok(None) => break None,
                Err(error) => break Some(error),
            }
        };
        for entry in vm.trace() {
            writeln!(self.stderr, "trace: {entry}")?;
        }
        Ok((error, vm.observations()))
    }

    fn drain_buffered(
        &mut self,
        vm: &mut Vm,
    ) -> Result<(Option<VmError>, VmObservations), RunError> {
        let sink = vm.effect_sink();
        let mut output = Ok(());
        let runtime = vm.drain_results(|value| {
            output =
                flush_buffered_effects(&sink, self.stderr).and_then(|()| self.output.emit(value));
            output.is_ok()
        });
        output?;
        flush_buffered_effects(&sink, self.stderr)?;
        Ok((runtime.err(), vm.observations()))
    }

    fn finish(mut self) -> Result<ExitStatus, RunError> {
        let options = self.plan.options;
        if self.runtime_error.is_none() {
            self.output.results.finish()?;
        }
        if let Some(path) = &options.report_file {
            let analysis = self.plan.analysis;
            write_report(
                path,
                &self.observations,
                self.output.count,
                self.output.results.written(),
                options,
                analysis.selected_plan,
                ReportExecution {
                    analysis,
                    retention: RetentionObservations::default(),
                    resource_outcome: "success",
                },
            )?;
        }
        if let Some(error) = self.runtime_error.take() {
            return self.finish_error(error);
        }
        Ok(self
            .output
            .results
            .exit_status(options.exit_status, self.output.last.as_ref()))
    }

    fn finish_error(&self, error: VmError) -> Result<ExitStatus, RunError> {
        if self.runtime_error_reported {
            return Ok(ExitStatus::Runtime);
        }
        Err(match error {
            VmError::Input { message } => RunError::Input(FormatError::Parse {
                format: InputFormat::Auto,
                message: message.to_string(),
            }),
            VmError::Halt { status, stderr }
                if self.plan.options.null_input && stderr.as_ref() == b"null\n" =>
            {
                RunError::Runtime(VmError::Halt {
                    status,
                    stderr: Arc::from([] as [u8; 0]),
                })
            }
            error if self.plan.allows_recovery(&error) => RunError::ReportedRuntime(error),
            error => RunError::Runtime(error),
        })
    }
}

struct DocumentOutput<'a, W> {
    results: ResultOutput<'a, W>,
    count: usize,
    last: Option<Value>,
}

impl<W: Write> DocumentOutput<'_, W> {
    fn emit(&mut self, value: Value) -> Result<(), RunError> {
        self.results.emit(&value)?;
        self.last = Some(value);
        self.count = self.count.saturating_add(1);
        Ok(())
    }
}
