//! Pull-based bytecode VM kernel with explicit bounded stacks and forks.

use std::{collections::BTreeMap, sync::Arc};

use thiserror::Error;

use crate::{
    Bytecode, Compiled, Diagnostic, DiagnosticClass, PathComponent, Program, Value,
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
}

impl VmError {
    /// Converts a VM failure to the shared diagnostic model.
    #[must_use]
    pub fn diagnostic(&self) -> Diagnostic {
        let class = match self {
            Self::Resource { .. } => DiagnosticClass::Resource,
            Self::Unsupported { .. } => DiagnosticClass::Unsupported,
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
}

impl Vm {
    /// Creates a VM for a validated compiled program and one document value.
    #[must_use]
    pub fn new(program: &Program<Compiled>, input: Value, limits: VmLimits) -> Self {
        Self::new_with_variables(program, input, limits, BTreeMap::new())
    }

    /// Creates a VM with immutable CLI/external variable values.
    #[must_use]
    pub fn new_with_variables(
        program: &Program<Compiled>,
        input: Value,
        limits: VmLimits,
        variables: BTreeMap<Arc<str>, Value>,
    ) -> Self {
        Self::from_bytecode(program.bytecode_arc(), input, limits, variables)
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
        }
    }

    /// Enables bounded instruction tracing. Zero disables tracing.
    #[must_use]
    pub fn with_trace_limit(mut self, limit: usize) -> Self {
        self.trace_limit = limit;
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
                    self.done = true;
                    return Err(VmError::Unsupported {
                        operation: format!("{operation:?}").into(),
                    });
                }
                _ => {}
            }
        }
        self.run_kernel()
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
        self.values.clear();
        self.calls.clear();
        self.paths.clear();
        self.forks.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::{Number, ResolveOptions, analyze, bytecode::Operation, parse, resolve};

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
    fn compiled_identity_and_literals_execute_through_pull_api() {
        for (query, input, expected) in [
            (".", Value::string("input"), Value::string("input")),
            ("42", Value::Null, number("42")),
        ] {
            let program =
                analyze(resolve(parse(query).unwrap(), &ResolveOptions::default()).unwrap())
                    .compile()
                    .unwrap();
            let mut vm = Vm::new(&program, input, VmLimits::default());
            assert_eq!(vm.next_result().unwrap(), Some(expected));
            assert_eq!(vm.next_result().unwrap(), None);
        }
    }
}
