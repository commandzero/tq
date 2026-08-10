//! Pull-based bytecode VM kernel with explicit bounded stacks and forks.

use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
    thread::{self, JoinHandle},
};

use thiserror::Error;

use crate::{
    Bytecode, Compiled, Diagnostic, DiagnosticClass, Document, Events, PathComponent, Plan, Value,
    bytecode::Operation,
};

/// Explicit VM stack and step limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmLimits {
    /// Maximum value stack entries.
    pub value_stack: usize,
    /// Maximum call/catch frames.
    pub call_stack: usize,
    /// Maximum active path components.
    pub path_stack: usize,
    /// Maximum pending backtracking forks.
    pub fork_stack: usize,
    /// Maximum instructions across all pulled results.
    pub steps: u64,
}

impl Default for VmLimits {
    fn default() -> Self {
        Self {
            value_stack: 4096,
            call_stack: 1024,
            path_stack: 1024,
            fork_stack: 4096,
            steps: 10_000_000,
        }
    }
}

/// VM high-water and work observations.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VmObservations {
    /// Maximum value-stack entries.
    pub value_stack_high_water: usize,
    /// Maximum call/catch frames.
    pub call_stack_high_water: usize,
    /// Maximum path components.
    pub path_stack_high_water: usize,
    /// Maximum pending forks.
    pub fork_stack_high_water: usize,
    /// Instructions evaluated.
    pub steps: u64,
    /// Results returned to the caller.
    pub results: u64,
}

/// Deterministic VM failure.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum VmError {
    /// Runtime error instruction or unsupported value operation.
    #[error("runtime error: {message}")]
    Runtime {
        /// Stable human message.
        message: Arc<str>,
    },
    /// A configured managed-stack or work limit was exceeded.
    #[error("VM resource limit exceeded: {resource}")]
    Resource {
        /// Stable resource name.
        resource: &'static str,
    },
    /// Validated bytecode invariant was violated during execution.
    #[error("invalid executable state: {message}")]
    InvalidProgram {
        /// Stable invariant description.
        message: &'static str,
    },
    /// Tree instruction is not admitted by the current progressive wave.
    #[error("bytecode operation is not executable in this language wave: {operation}")]
    Unsupported {
        /// Operation name.
        operation: Arc<str>,
    },
    /// Execution was cancelled by the caller or an interrupt handler.
    #[error("execution interrupted")]
    Interrupted,
}

impl VmError {
    /// Converts a VM failure to the shared diagnostic model.
    #[must_use]
    pub fn diagnostic(&self) -> Diagnostic {
        let class = match self {
            Self::Resource { .. } => DiagnosticClass::Resource,
            Self::Unsupported { .. } => DiagnosticClass::Unsupported,
            Self::Interrupted => DiagnosticClass::Cancelled,
            Self::Runtime { .. } | Self::InvalidProgram { .. } => DiagnosticClass::Runtime,
        };
        Diagnostic::new("TQ-VM-001", class, self.to_string())
    }
}

#[derive(Clone, Debug)]
struct ForkState {
    pc: usize,
    values: Vec<Value>,
    calls: Vec<CatchFrame>,
    paths: Vec<PathComponent>,
}

#[derive(Clone, Copy, Debug)]
struct CatchFrame {
    target: usize,
    value_height: usize,
    fork_height: usize,
}

#[derive(Debug)]
enum TreeMessage {
    Result(Result<Value, VmError>, VmObservations),
    Done(VmObservations),
}

/// Pull-based VM. Dropping it releases all pending forks and shallow value
/// handles without evaluating remaining branches.
#[derive(Debug)]
pub struct Vm {
    bytecode: Arc<Bytecode>,
    input: Value,
    variables: BTreeMap<Arc<str>, Value>,
    pc: usize,
    values: Vec<Value>,
    calls: Vec<CatchFrame>,
    paths: Vec<PathComponent>,
    forks: Vec<ForkState>,
    limits: VmLimits,
    observations: VmObservations,
    trace: Vec<String>,
    trace_limit: usize,
    done: bool,
    tree_started: bool,
    tree_receiver: Option<Receiver<TreeMessage>>,
    tree_demand: Option<SyncSender<()>>,
    tree_worker: Option<JoinHandle<()>>,
    tree_stop: Arc<AtomicBool>,
    cancellation: Option<Arc<AtomicBool>>,
}

impl Vm {
    /// Creates a VM for a mode-checked document plan and one document value.
    #[must_use]
    pub fn new(plan: &Plan<Compiled, Document>, input: Value, limits: VmLimits) -> Self {
        Self::new_with_variables(plan, input, limits, BTreeMap::new())
    }

    /// Creates a document VM with immutable CLI/external variable values.
    #[must_use]
    pub fn new_with_variables(
        plan: &Plan<Compiled, Document>,
        input: Value,
        limits: VmLimits,
        variables: BTreeMap<Arc<str>, Value>,
    ) -> Self {
        Self::from_bytecode(plan.program().bytecode_arc(), input, limits, variables)
    }

    /// Creates a VM for one value produced by a mode-checked event plan.
    #[must_use]
    pub fn new_events(plan: &Plan<Compiled, Events>, input: Value, limits: VmLimits) -> Self {
        Self::new_events_with_variables(plan, input, limits, BTreeMap::new())
    }

    /// Creates an event VM with immutable CLI/external variable values.
    #[must_use]
    pub fn new_events_with_variables(
        plan: &Plan<Compiled, Events>,
        input: Value,
        limits: VmLimits,
        variables: BTreeMap<Arc<str>, Value>,
    ) -> Self {
        Self::from_bytecode(plan.program().bytecode_arc(), input, limits, variables)
    }

    /// Creates a VM for one independently captured value in an automatic
    /// event or subtree plan.
    ///
    /// # Panics
    ///
    /// Panics if `plan` was not constructed by automatic plan selection.
    #[must_use]
    pub fn new_automatic_item<M>(
        plan: &Plan<Compiled, M>,
        input: Value,
        limits: VmLimits,
        variables: BTreeMap<Arc<str>, Value>,
    ) -> Self {
        Self::from_bytecode(
            plan.automatic_item_bytecode()
                .expect("automatic item VM requires an automatic plan"),
            input,
            limits,
            variables,
        )
    }

    /// Creates a VM that preserves iteration errors when the proven prefix
    /// resolves to a scalar rather than an array or object.
    ///
    /// # Panics
    ///
    /// Panics if `plan` was not constructed by automatic plan selection.
    #[must_use]
    pub fn new_automatic_base<M>(
        plan: &Plan<Compiled, M>,
        input: Value,
        limits: VmLimits,
        variables: BTreeMap<Arc<str>, Value>,
    ) -> Self {
        Self::from_bytecode(
            plan.automatic_base_bytecode()
                .expect("automatic base VM requires an automatic plan"),
            input,
            limits,
            variables,
        )
    }

    /// Creates a VM from an independently decoded and validated bytecode value.
    #[must_use]
    pub fn from_validated_bytecode(bytecode: Bytecode, input: Value, limits: VmLimits) -> Self {
        Self::from_bytecode(Arc::new(bytecode), input, limits, BTreeMap::new())
    }

    fn from_bytecode(
        bytecode: Arc<Bytecode>,
        input: Value,
        limits: VmLimits,
        variables: BTreeMap<Arc<str>, Value>,
    ) -> Self {
        Self {
            bytecode,
            input,
            variables,
            pc: 0,
            values: Vec::new(),
            calls: Vec::new(),
            paths: Vec::new(),
            forks: Vec::new(),
            limits,
            observations: VmObservations::default(),
            trace: Vec::new(),
            trace_limit: 0,
            done: false,
            tree_started: false,
            tree_receiver: None,
            tree_demand: None,
            tree_worker: None,
            tree_stop: Arc::new(AtomicBool::new(false)),
            cancellation: None,
        }
    }

    /// Enables bounded instruction tracing. Zero disables tracing.
    #[must_use]
    pub fn with_trace_limit(mut self, limit: usize) -> Self {
        self.trace_limit = limit;
        self
    }

    /// Installs a shared cooperative cancellation flag checked during managed
    /// evaluation and before each kernel instruction.
    #[must_use]
    pub fn with_cancellation(mut self, cancellation: Arc<AtomicBool>) -> Self {
        self.cancellation = Some(cancellation);
        self
    }

    /// Returns one result, zero-result completion, or a deterministic error.
    ///
    /// # Errors
    ///
    /// Returns a runtime/resource error. After an uncaught error the VM is
    /// complete and subsequent calls return `Ok(None)`.
    pub fn next_result(&mut self) -> Result<Option<Value>, VmError> {
        if self.done {
            return Ok(None);
        }
        if self.tree_receiver.is_some() {
            return self.pull_tree();
        }
        if !self.tree_started {
            self.tree_started = true;
            let root = self.bytecode.root() as usize;
            match &self
                .bytecode
                .instructions()
                .get(root)
                .ok_or(VmError::InvalidProgram {
                    message: "root instruction missing after validation",
                })?
                .operation
            {
                Operation::Identity => {
                    self.done = true;
                    self.observations.results += 1;
                    return Ok(Some(self.input.clone()));
                }
                Operation::Literal(constant) => {
                    let value = self
                        .bytecode
                        .constants()
                        .get(*constant as usize)
                        .ok_or(VmError::InvalidProgram {
                            message: "constant missing after validation",
                        })?
                        .clone();
                    self.done = true;
                    self.observations.results += 1;
                    return Ok(Some(value));
                }
                Operation::Variable(name) => {
                    let name = self.bytecode.string(*name).ok_or(VmError::InvalidProgram {
                        message: "variable name missing after validation",
                    })?;
                    let value = self.variables.get(name).ok_or_else(|| VmError::Runtime {
                        message: format!("variable ${name} has no runtime value").into(),
                    })?;
                    self.done = true;
                    self.observations.results += 1;
                    return Ok(Some(value.clone()));
                }
                Operation::Empty => {
                    self.done = true;
                    return Ok(None);
                }
                operation if !super::bytecode::kernel(operation) => {
                    self.start_tree()?;
                    return self.pull_tree();
                }
                _ => {}
            }
        }
        self.run_kernel()
    }

    /// Exhausts this VM synchronously, passing each result to `emit`.
    ///
    /// This avoids the demand-channel worker used by [`Self::next_result`] when
    /// a caller already knows it will consume every result. It is especially
    /// useful for event streams, where starting one worker thread per input
    /// record would dominate decoding and query execution. Returning `false`
    /// from `emit` stops the VM cleanly after the current result.
    ///
    /// # Errors
    ///
    /// Returns the same deterministic runtime or resource error as
    /// [`Self::next_result`].
    pub fn for_each_result(&mut self, mut emit: impl FnMut(Value) -> bool) -> Result<(), VmError> {
        let uses_tree = !self.tree_started
            && self.tree_receiver.is_none()
            && self
                .bytecode
                .instructions()
                .get(self.bytecode.root() as usize)
                .is_some_and(|instruction| !super::bytecode::kernel(&instruction.operation));
        if uses_tree {
            self.tree_started = true;
            self.done = true;
            let mut failure = None;
            let observations = crate::eval::evaluate_stream(
                &self.bytecode,
                &self.input,
                &self.variables,
                self.limits,
                self.cancellation.as_deref(),
                &self.tree_stop,
                |result, _| match result {
                    Ok(value) => emit(value),
                    Err(error) => {
                        failure = Some(error);
                        false
                    }
                },
            );
            self.observations = observations;
            return failure.map_or(Ok(()), Err);
        }

        while let Some(value) = self.next_result()? {
            if !emit(value) {
                self.done = true;
                self.forks.clear();
                self.stop_tree_worker();
                break;
            }
        }
        Ok(())
    }

    fn start_tree(&mut self) -> Result<(), VmError> {
        let bytecode = Arc::clone(&self.bytecode);
        let input = self.input.clone();
        let variables = self.variables.clone();
        let limits = self.limits;
        let cancellation = self.cancellation.clone();
        let stop = Arc::clone(&self.tree_stop);
        let (result_sender, result_receiver) = sync_channel(0);
        let (demand_sender, demand_receiver) = sync_channel(0);
        let worker = thread::Builder::new()
            .name("tq-pull-evaluator".to_owned())
            .spawn(move || {
                if demand_receiver.recv().is_err() {
                    return;
                }
                let observations = crate::eval::evaluate_stream(
                    &bytecode,
                    &input,
                    &variables,
                    limits,
                    cancellation.as_deref(),
                    &stop,
                    |result, observations| {
                        result_sender
                            .send(TreeMessage::Result(result, observations))
                            .is_ok()
                            && demand_receiver.recv().is_ok()
                    },
                );
                let _ = result_sender.send(TreeMessage::Done(observations));
            })
            .map_err(|error| VmError::Runtime {
                message: format!("could not start pull evaluator: {error}").into(),
            })?;
        self.tree_receiver = Some(result_receiver);
        self.tree_demand = Some(demand_sender);
        self.tree_worker = Some(worker);
        Ok(())
    }

    fn pull_tree(&mut self) -> Result<Option<Value>, VmError> {
        let demand = self.tree_demand.as_ref().ok_or(VmError::InvalidProgram {
            message: "tree demand channel missing while evaluator is active",
        })?;
        if demand.send(()).is_err() {
            self.stop_tree_worker();
            self.done = true;
            return Err(VmError::InvalidProgram {
                message: "pull evaluator stopped before accepting demand",
            });
        }
        let message = self
            .tree_receiver
            .as_ref()
            .ok_or(VmError::InvalidProgram {
                message: "tree result channel missing while evaluator is active",
            })?
            .recv();
        match message {
            Ok(TreeMessage::Result(result, observations)) => {
                self.observations = observations;
                match result {
                    Ok(value) => Ok(Some(value)),
                    Err(error) => {
                        self.stop_tree_worker();
                        self.done = true;
                        Err(error)
                    }
                }
            }
            Ok(TreeMessage::Done(observations)) => {
                self.observations = observations;
                self.stop_tree_worker();
                self.done = true;
                Ok(None)
            }
            Err(_) => {
                self.stop_tree_worker();
                self.done = true;
                Err(VmError::InvalidProgram {
                    message: "pull evaluator terminated without completion",
                })
            }
        }
    }

    fn stop_tree_worker(&mut self) {
        self.tree_stop.store(true, Ordering::Relaxed);
        self.tree_demand.take();
        self.tree_receiver.take();
        if let Some(worker) = self.tree_worker.take() {
            let _ = worker.join();
        }
    }

    /// Current high-water/work observations.
    #[must_use]
    pub const fn observations(&self) -> VmObservations {
        self.observations
    }

    /// Bounded trace entries.
    #[must_use]
    pub fn trace(&self) -> &[String] {
        &self.trace
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the VM dispatch loop keeps state transitions and cleanup auditable"
    )]
    fn run_kernel(&mut self) -> Result<Option<Value>, VmError> {
        loop {
            if self
                .cancellation
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
            {
                self.done = true;
                return Err(VmError::Interrupted);
            }
            if self.observations.steps >= self.limits.steps {
                self.done = true;
                return Err(VmError::Resource {
                    resource: "vm-steps",
                });
            }
            let operation = self
                .bytecode
                .instructions()
                .get(self.pc)
                .ok_or(VmError::InvalidProgram {
                    message: "program counter left validated bytecode",
                })?
                .operation
                .clone();
            self.record_trace(&operation);
            self.observations.steps += 1;
            match operation {
                Operation::LoadInput => {
                    self.push_value(self.input.clone())?;
                    self.pc += 1;
                }
                Operation::LoadConstant(constant) => {
                    let value = self
                        .bytecode
                        .constants()
                        .get(constant as usize)
                        .ok_or(VmError::InvalidProgram {
                            message: "constant left validated pool",
                        })?
                        .clone();
                    self.push_value(value)?;
                    self.pc += 1;
                }
                Operation::Duplicate => {
                    let value = self.values.last().ok_or(VmError::InvalidProgram {
                        message: "duplicate on empty value stack",
                    })?;
                    self.push_value(value.clone())?;
                    self.pc += 1;
                }
                Operation::Pop => {
                    self.pop_value()?;
                    self.pc += 1;
                }
                Operation::Jump(target) => self.pc = target as usize,
                Operation::Branch { truthy, falsey } => {
                    let condition = self.pop_value()?;
                    self.pc = if condition.is_truthy() {
                        truthy as usize
                    } else {
                        falsey as usize
                    };
                }
                Operation::Fork(target) => {
                    if self.forks.len() >= self.limits.fork_stack {
                        return Err(self.resource("fork-stack"));
                    }
                    self.forks.push(ForkState {
                        pc: target as usize,
                        values: self.values.clone(),
                        calls: self.calls.clone(),
                        paths: self.paths.clone(),
                    });
                    self.observations.fork_stack_high_water = self
                        .observations
                        .fork_stack_high_water
                        .max(self.forks.len());
                    self.pc += 1;
                }
                Operation::Backtrack => {
                    if !self.restore_fork() {
                        self.done = true;
                        return Ok(None);
                    }
                }
                Operation::Return => {
                    let result = self.pop_value()?;
                    if !self.restore_fork() {
                        self.done = true;
                    }
                    self.observations.results += 1;
                    return Ok(Some(result));
                }
                Operation::Raise(_) => {
                    if let Some(catch) = self.calls.pop() {
                        self.values.truncate(catch.value_height);
                        self.forks.truncate(catch.fork_height);
                        self.push_value(Value::string("runtime error"))?;
                        self.pc = catch.target;
                    } else {
                        self.done = true;
                        self.forks.clear();
                        return Err(VmError::Runtime {
                            message: "explicit bytecode error".into(),
                        });
                    }
                }
                Operation::Catch(target) => {
                    if self.calls.len() >= self.limits.call_stack {
                        return Err(self.resource("call-stack"));
                    }
                    self.calls.push(CatchFrame {
                        target: target as usize,
                        value_height: self.values.len(),
                        fork_height: self.forks.len(),
                    });
                    self.observations.call_stack_high_water = self
                        .observations
                        .call_stack_high_water
                        .max(self.calls.len());
                    self.pc += 1;
                }
                Operation::EndCatch => {
                    self.calls.pop().ok_or(VmError::InvalidProgram {
                        message: "end-catch without catch frame",
                    })?;
                    self.pc += 1;
                }
                _ => {
                    self.done = true;
                    return Err(VmError::Unsupported {
                        operation: format!("{operation:?}").into(),
                    });
                }
            }
        }
    }

    fn push_value(&mut self, value: Value) -> Result<(), VmError> {
        if self.values.len() >= self.limits.value_stack {
            return Err(self.resource("value-stack"));
        }
        self.values.push(value);
        self.observations.value_stack_high_water = self
            .observations
            .value_stack_high_water
            .max(self.values.len());
        Ok(())
    }

    fn pop_value(&mut self) -> Result<Value, VmError> {
        self.values.pop().ok_or(VmError::InvalidProgram {
            message: "value stack underflow after validation",
        })
    }

    fn restore_fork(&mut self) -> bool {
        let Some(fork) = self.forks.pop() else {
            return false;
        };
        self.pc = fork.pc;
        self.values = fork.values;
        self.calls = fork.calls;
        self.paths = fork.paths;
        true
    }

    fn record_trace(&mut self, operation: &Operation) {
        if self.trace.len() < self.trace_limit {
            self.trace.push(format!(
                "pc={} op={operation:?} values={} calls={} paths={} forks={}",
                self.pc,
                self.values.len(),
                self.calls.len(),
                self.paths.len(),
                self.forks.len()
            ));
        }
    }

    fn resource(&mut self, resource: &'static str) -> VmError {
        self.done = true;
        self.forks.clear();
        VmError::Resource { resource }
    }

    #[cfg(test)]
    fn from_kernel(bytecode: Bytecode, input: Value, limits: VmLimits) -> Self {
        Self::from_validated_bytecode(bytecode, input, limits)
    }
}

impl Drop for Vm {
    fn drop(&mut self) {
        self.stop_tree_worker();
        self.values.clear();
        self.calls.clear();
        self.paths.clear();
        self.forks.clear();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use crate::{
        AnalysisContext, Number, ResolveOptions, analyze, analyze_with_context,
        bytecode::Operation, parse, resolve,
    };

    use super::{Bytecode, Value, Vm, VmError, VmLimits};

    fn number(value: &str) -> Value {
        Value::Number(Number::parse(value).unwrap())
    }

    #[test]
    fn load_duplicate_pop_branch_jump_and_return_execute() {
        let bytecode = Bytecode::kernel(
            vec![
                Operation::LoadInput,
                Operation::Duplicate,
                Operation::Branch {
                    truthy: 3,
                    falsey: 6,
                },
                Operation::Pop,
                Operation::LoadInput,
                Operation::Jump(8),
                Operation::Pop,
                Operation::LoadConstant(0),
                Operation::Return,
            ],
            vec![number("0")],
        );
        bytecode.validate().unwrap();
        let mut vm = Vm::from_kernel(bytecode, Value::Bool(true), VmLimits::default());
        assert_eq!(vm.next_result().unwrap(), Some(Value::Bool(true)));
        assert_eq!(vm.next_result().unwrap(), None);
    }

    #[test]
    fn forks_backtrack_in_order_and_can_emit_zero_results() {
        let bytecode = Bytecode::kernel(
            vec![
                Operation::Fork(4),
                Operation::LoadConstant(0),
                Operation::Return,
                Operation::Backtrack,
                Operation::LoadConstant(1),
                Operation::Return,
            ],
            vec![number("1"), number("2")],
        );
        bytecode.validate().unwrap();
        let mut vm = Vm::from_kernel(bytecode, Value::Null, VmLimits::default());
        assert_eq!(vm.next_result().unwrap(), Some(number("1")));
        assert_eq!(vm.next_result().unwrap(), Some(number("2")));
        assert_eq!(vm.next_result().unwrap(), None);

        let empty = Bytecode::kernel(vec![Operation::Backtrack], Vec::new());
        empty.validate().unwrap();
        assert_eq!(
            Vm::from_kernel(empty, Value::Null, VmLimits::default())
                .next_result()
                .unwrap(),
            None
        );
    }

    #[test]
    fn errors_unwind_deterministically_and_limits_report_high_water() {
        let error = Bytecode::kernel(vec![Operation::Raise(0)], Vec::new());
        error.validate().unwrap();
        let mut vm = Vm::from_kernel(error, Value::Null, VmLimits::default());
        assert!(matches!(vm.next_result(), Err(VmError::Runtime { .. })));
        assert_eq!(vm.next_result().unwrap(), None);

        let caught = Bytecode::kernel(
            vec![
                Operation::Catch(4),
                Operation::Raise(0),
                Operation::EndCatch,
                Operation::Backtrack,
                Operation::Return,
            ],
            Vec::new(),
        );
        caught.validate().unwrap();
        let mut vm = Vm::from_kernel(caught, Value::Null, VmLimits::default());
        assert_eq!(
            vm.next_result().unwrap(),
            Some(Value::string("runtime error"))
        );
        assert_eq!(vm.observations().call_stack_high_water, 1);

        let limited = Bytecode::kernel(
            vec![
                Operation::LoadInput,
                Operation::Duplicate,
                Operation::Return,
            ],
            Vec::new(),
        );
        limited.validate().unwrap();
        let mut vm = Vm::from_kernel(
            limited,
            Value::Null,
            VmLimits {
                value_stack: 1,
                ..VmLimits::default()
            },
        );
        assert!(matches!(vm.next_result(), Err(VmError::Resource { .. })));
        assert_eq!(vm.observations().value_stack_high_water, 1);
    }

    #[test]
    fn caller_can_drop_before_exhausting_pending_forks() {
        let bytecode = Bytecode::kernel(
            vec![
                Operation::Fork(3),
                Operation::LoadInput,
                Operation::Return,
                Operation::LoadInput,
                Operation::Return,
            ],
            Vec::new(),
        );
        bytecode.validate().unwrap();
        let mut vm = Vm::from_kernel(bytecode, Value::string("shared"), VmLimits::default());
        assert!(vm.next_result().unwrap().is_some());
        drop(vm);
    }

    #[test]
    fn tree_evaluation_waits_for_demand_before_entering_the_next_branch() {
        let plan = analyze(resolve(parse("1, 2").unwrap(), &ResolveOptions::default()).unwrap())
            .compile()
            .unwrap()
            .document_plan();
        let cancellation = Arc::new(AtomicBool::new(false));
        let mut vm = Vm::new(&plan, Value::Null, VmLimits::default())
            .with_cancellation(Arc::clone(&cancellation));

        assert_eq!(vm.next_result().unwrap(), Some(number("1")));
        cancellation.store(true, Ordering::Relaxed);
        assert!(matches!(vm.next_result(), Err(VmError::Interrupted)));
    }

    #[test]
    fn exhaustive_consumers_evaluate_tree_without_a_worker_thread() {
        let plan = analyze_with_context(
            resolve(
                parse("select(length == 2)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
            AnalysisContext {
                event_input: true,
                whole_input: false,
                automatic_streaming: false,
            },
        )
        .compile()
        .unwrap()
        .event_plan()
        .unwrap();
        let mut vm = Vm::new_events(
            &plan,
            Value::array([number("1"), number("2")]),
            VmLimits::default(),
        );
        let mut results = Vec::new();

        vm.for_each_result(|value| {
            results.push(value);
            true
        })
        .unwrap();

        assert_eq!(results, [Value::array([number("1"), number("2")])]);
        assert!(vm.tree_worker.is_none());
        assert!(vm.done);
    }

    #[test]
    fn explicit_event_plan_executes_an_event_compatible_fold_end_to_end() {
        let plan = analyze_with_context(
            resolve(
                parse("reduce .[] as $part (null; $part)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
            AnalysisContext {
                event_input: true,
                whole_input: false,
                automatic_streaming: false,
            },
        )
        .compile()
        .unwrap()
        .event_plan()
        .unwrap();
        let event = Value::array([Value::array([Value::string("magnitude")]), number("7")]);
        let mut vm = Vm::new_events(&plan, event, VmLimits::default());

        assert_eq!(vm.next_result().unwrap(), Some(number("7")));
        assert_eq!(vm.next_result().unwrap(), None);
        assert_eq!(vm.observations().value_stack_high_water, 1);
    }

    #[test]
    fn generator_language_uses_bounded_explicit_forks_and_continuations() {
        let plan =
            analyze(resolve(parse(".[], empty").unwrap(), &ResolveOptions::default()).unwrap())
                .compile()
                .unwrap()
                .document_plan();
        let mut vm = Vm::new(
            &plan,
            Value::array([number("1"), number("2"), number("3")]),
            VmLimits::default(),
        );

        assert_eq!(vm.next_result().unwrap(), Some(number("1")));
        assert_eq!(vm.next_result().unwrap(), Some(number("2")));
        assert_eq!(vm.next_result().unwrap(), Some(number("3")));
        assert_eq!(vm.next_result().unwrap(), None);
        assert!(vm.observations().fork_stack_high_water >= 3);
        assert!(vm.observations().call_stack_high_water >= 2);

        let limited = analyze(resolve(parse(".[]").unwrap(), &ResolveOptions::default()).unwrap())
            .compile()
            .unwrap()
            .document_plan();
        let mut vm = Vm::new(
            &limited,
            Value::array([number("1"), number("2")]),
            VmLimits {
                fork_stack: 1,
                ..VmLimits::default()
            },
        );
        assert!(matches!(vm.next_result(), Err(VmError::Resource { .. })));

        let update =
            analyze(resolve(parse(".a.b = 2").unwrap(), &ResolveOptions::default()).unwrap())
                .compile()
                .unwrap()
                .document_plan();
        let mut vm = Vm::new(
            &update,
            Value::object(crate::Object::new()),
            VmLimits::default(),
        );
        assert!(vm.next_result().unwrap().is_some());
        assert!(vm.observations().path_stack_high_water >= 2);

        let caught = analyze(
            resolve(
                parse("try error(\"managed\") catch .").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut vm = Vm::new(&caught, Value::Null, VmLimits::default());
        assert_eq!(vm.next_result().unwrap(), Some(Value::string("managed")));
        assert!(vm.observations().call_stack_high_water >= 2);

        let optional =
            analyze(resolve(parse(".[]?").unwrap(), &ResolveOptions::default()).unwrap())
                .compile()
                .unwrap()
                .document_plan();
        let mut vm = Vm::new(&optional, Value::Bool(false), VmLimits::default());
        assert_eq!(vm.next_result().unwrap(), None);
    }

    #[test]
    fn reduce_preserves_jq_order_last_update_and_initializer_multiplicity() {
        let plan = analyze(
            resolve(
                parse("reduce (1,2) as $x ((0,10); . + $x, 100)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut vm = Vm::new(&plan, Value::Null, VmLimits::default());
        assert_eq!(vm.next_result().unwrap(), Some(number("100")));
        assert_eq!(vm.next_result().unwrap(), Some(number("100")));
        assert_eq!(vm.next_result().unwrap(), None);
        assert!(vm.observations().value_stack_high_water > 0);

        let empty_update = analyze(
            resolve(
                parse("reduce (1,2) as $x (0; if $x == 1 then empty else . + $x end)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut vm = Vm::new(&empty_update, Value::Null, VmLimits::default());
        assert_eq!(vm.next_result().unwrap(), Some(number("2")));
        assert_eq!(vm.next_result().unwrap(), None);
    }

    #[test]
    fn foreach_emits_update_and_extraction_multiplicity_in_jq_order() {
        let plan = analyze(
            resolve(
                parse("foreach (1,2) as $x (0; 100, . + $x; [$x,.], . * 10)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut vm = Vm::new(&plan, Value::Null, VmLimits::default());
        let mut output = Vec::new();
        while let Some(value) = vm.next_result().unwrap() {
            output.push(value);
        }
        assert_eq!(
            output,
            [
                Value::array([number("1"), number("100")]),
                number("1000"),
                Value::array([number("1"), number("1")]),
                number("10"),
                Value::array([number("2"), number("100")]),
                number("1000"),
                Value::array([number("2"), number("3")]),
                number("30"),
            ]
        );
    }

    #[test]
    fn foreach_retains_partial_output_then_unwinds_on_a_later_error() {
        let plan = analyze(
            resolve(
                parse("foreach (1,2,error(\"boom\")) as $x (0; . + $x; .)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut vm = Vm::new(&plan, Value::Null, VmLimits::default());
        assert_eq!(vm.next_result().unwrap(), Some(number("1")));
        assert_eq!(vm.next_result().unwrap(), Some(number("3")));
        assert!(matches!(vm.next_result(), Err(VmError::Runtime { .. })));
        assert_eq!(vm.next_result().unwrap(), None);
    }

    #[test]
    fn fold_frames_are_bounded_and_preserve_structural_sharing() {
        let plan = analyze(
            resolve(
                parse("reduce empty as $x (.; .)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let input = Value::array([Value::string("shared")]);
        let mut vm = Vm::new(&plan, input.clone(), VmLimits::default());
        let result = vm.next_result().unwrap().unwrap();
        assert!(result.shares_node_with(&input));

        let mut limited = Vm::new(
            &plan,
            input,
            VmLimits {
                value_stack: 0,
                ..VmLimits::default()
            },
        );
        assert!(matches!(
            limited.next_result(),
            Err(VmError::Resource {
                resource: "value-stack"
            })
        ));
    }

    #[test]
    fn folds_obey_work_cancellation_and_hostile_depth_limits() {
        let work_plan = analyze(
            resolve(
                parse("reduce range(0;1000) as $x (0; . + $x)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut limited = Vm::new(
            &work_plan,
            Value::Null,
            VmLimits {
                steps: 10,
                ..VmLimits::default()
            },
        );
        assert!(matches!(
            limited.next_result(),
            Err(VmError::Resource {
                resource: "vm-steps"
            })
        ));

        let cancellation = Arc::new(AtomicBool::new(false));
        let foreach_plan = analyze(
            resolve(
                parse("foreach range(0;1000) as $x (0; . + $x; .)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut cancelled = Vm::new(&foreach_plan, Value::Null, VmLimits::default())
            .with_cancellation(Arc::clone(&cancellation));
        assert_eq!(cancelled.next_result().unwrap(), Some(number("0")));
        cancellation.store(true, Ordering::Relaxed);
        assert!(matches!(cancelled.next_result(), Err(VmError::Interrupted)));

        let mut hostile = ".".to_owned();
        for _ in 0..64 {
            hostile = format!("reduce empty as $x ({hostile}; .)");
        }
        let hostile_plan =
            analyze(resolve(parse(&hostile).unwrap(), &ResolveOptions::default()).unwrap())
                .compile()
                .unwrap()
                .document_plan();
        let mut depth_limited = Vm::new(
            &hostile_plan,
            Value::Null,
            VmLimits {
                call_stack: 16,
                ..VmLimits::default()
            },
        );
        assert!(matches!(
            depth_limited.next_result(),
            Err(VmError::Resource {
                resource: "call-stack"
            })
        ));
    }

    #[test]
    fn compiled_identity_and_literals_execute_through_pull_api() {
        for (query, input, expected) in [
            (".", Value::string("input"), Value::string("input")),
            ("42", Value::Null, number("42")),
        ] {
            let plan = analyze(resolve(parse(query).unwrap(), &ResolveOptions::default()).unwrap())
                .compile()
                .unwrap()
                .document_plan();
            let mut vm = Vm::new(&plan, input, VmLimits::default());
            assert_eq!(vm.next_result().unwrap(), Some(expected));
            assert_eq!(vm.next_result().unwrap(), None);
        }
    }

    #[test]
    fn cancellation_interrupts_kernel_and_tree_evaluation() {
        let cancellation = Arc::new(AtomicBool::new(true));
        let kernel = Bytecode::kernel(vec![Operation::LoadInput, Operation::Return], Vec::new());
        kernel.validate().unwrap();
        let mut vm = Vm::from_kernel(kernel, Value::Null, VmLimits::default())
            .with_cancellation(Arc::clone(&cancellation));
        assert!(matches!(vm.next_result(), Err(VmError::Interrupted)));

        let plan = analyze(resolve(parse("map(.)").unwrap(), &ResolveOptions::default()).unwrap())
            .compile()
            .unwrap()
            .document_plan();
        let mut vm = Vm::new(&plan, Value::array([Value::Null]), VmLimits::default())
            .with_cancellation(cancellation);
        assert!(matches!(vm.next_result(), Err(VmError::Interrupted)));
    }
}
