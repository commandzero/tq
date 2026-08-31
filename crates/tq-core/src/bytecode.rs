//! Immutable source-mapped bytecode, compiler, validation, and disassembly.

use std::collections::VecDeque;
use std::{fmt, sync::Arc};

use thiserror::Error;

use crate::{
    BuiltinRegistry, Diagnostic, DiagnosticClass, ModuleInfo, Span, Value,
    ast::{
        Access, AssignmentOperator, BinaryOperator, CallTarget, Expr, ExprKind,
        InterpolationSegment, ObjectKey, ParameterKind, UnaryOperator,
    },
};

/// Immutable validated tq bytecode.
#[derive(Clone, Debug)]
pub struct Bytecode {
    instructions: Arc<[Instruction]>,
    constants: Arc<[Value]>,
    strings: Arc<[Arc<str>]>,
    functions: Arc<[UserFunction]>,
    modules: Arc<[ModuleInfo]>,
    root: u32,
    managed_tree_execution: bool,
}

/// Stable bytecode validation or decoding failure.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum BytecodeError {
    /// Unknown encoded opcode.
    #[error("unknown opcode {opcode} at word {word}")]
    UnknownOpcode {
        /// Numeric opcode.
        opcode: u8,
        /// Encoded word offset.
        word: usize,
    },
    /// Encoded instruction ends before all operands are present.
    #[error("truncated instruction at word {word}")]
    Truncated {
        /// Encoded word offset.
        word: usize,
    },
    /// Operand count does not match the opcode.
    #[error("opcode at instruction {instruction} has invalid operand count")]
    OperandCount {
        /// Instruction index.
        instruction: usize,
    },
    /// Instruction target is outside the bytecode.
    #[error("invalid target {target} at instruction {instruction}")]
    Target {
        /// Instruction index.
        instruction: usize,
        /// Invalid target.
        target: u32,
    },
    /// Constant reference is outside the pool.
    #[error("invalid constant {constant} at instruction {instruction}")]
    Constant {
        /// Instruction index.
        instruction: usize,
        /// Invalid pool index.
        constant: u32,
    },
    /// String/function reference is outside the pool.
    #[error("invalid string {string} at instruction {instruction}")]
    String {
        /// Instruction index.
        instruction: usize,
        /// Invalid pool index.
        string: u32,
    },
    /// Root instruction is invalid.
    #[error("invalid root instruction {root}")]
    Root {
        /// Invalid root index.
        root: u32,
    },
    /// Static value-stack simulation underflowed.
    #[error("value stack underflow at instruction {instruction}")]
    StackUnderflow {
        /// Instruction index.
        instruction: usize,
    },
    /// Control-flow paths reach one instruction at incompatible stack heights.
    #[error("incompatible value stack heights at instruction {instruction}")]
    StackMismatch {
        /// Instruction index.
        instruction: usize,
    },
    /// Built-in call arity is invalid.
    #[error("invalid call arity {arity} for {name} at instruction {instruction}")]
    CallArity {
        /// Instruction index.
        instruction: usize,
        /// Built-in name.
        name: Arc<str>,
        /// Encoded argument count.
        arity: usize,
    },
    /// User-filter symbol is outside the function table.
    #[error("invalid user filter symbol {symbol} at instruction {instruction}")]
    Function {
        /// Instruction index.
        instruction: usize,
        /// Invalid symbol.
        symbol: u32,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct Instruction {
    pub(crate) operation: Operation,
    pub(crate) span: Span,
}

#[derive(Clone, Debug)]
#[allow(
    dead_code,
    reason = "progressive language waves activate validated tree operations in sequence"
)]
pub(crate) enum Operation {
    LoadInput,
    LoadConstant(u32),
    Duplicate,
    Pop,
    Jump(u32),
    Branch {
        truthy: u32,
        falsey: u32,
    },
    Fork(u32),
    Backtrack,
    Return,
    Raise(u32),
    Catch(u32),
    EndCatch,
    Identity,
    Literal(u32),
    Variable(u32),
    Empty,
    RecursiveDescent,
    Interpolation(Vec<InterpolationOperand>),
    AccessField {
        base: u32,
        key: u32,
    },
    AccessIndex {
        base: u32,
        index: u32,
    },
    Slice {
        base: u32,
        start: Option<u32>,
        end: Option<u32>,
    },
    Iterate(u32),
    Optional(u32),
    Pipe {
        left: u32,
        right: u32,
    },
    Comma {
        left: u32,
        right: u32,
    },
    Array(u32),
    Object(Vec<ObjectOperand>),
    Unary {
        operator: UnaryOperator,
        child: u32,
    },
    Binary {
        operator: BinaryOperator,
        left: u32,
        right: u32,
    },
    Conditional {
        branches: Vec<(u32, u32)>,
        alternative: u32,
    },
    Bind {
        value: u32,
        name: u32,
        body: u32,
    },
    Reduce {
        generator: u32,
        name: u32,
        initial: u32,
        update: u32,
    },
    Foreach {
        generator: u32,
        name: u32,
        initial: u32,
        update: u32,
        extract: u32,
    },
    Call {
        name: u32,
        arguments: Vec<u32>,
    },
    UserCall {
        symbol: u32,
        arguments: Vec<u32>,
    },
    ParameterCall {
        function: u32,
        parameter: u32,
    },
    TryCatch {
        expression: u32,
        catch: Option<u32>,
    },
    Assignment {
        operator: AssignmentOperator,
        path: u32,
        value: u32,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct UserFunction {
    pub(crate) name: u32,
    pub(crate) parameters: Vec<UserParameter>,
    pub(crate) body: u32,
    pub(crate) span: Span,
}

#[derive(Clone, Debug)]
pub(crate) struct UserParameter {
    pub(crate) kind: ParameterKind,
    pub(crate) runtime_name: Option<u32>,
}

#[derive(Clone, Debug)]
pub(crate) enum InterpolationOperand {
    Literal(u32),
    Expression(u32),
}

#[derive(Clone, Debug)]
pub(crate) struct ObjectOperand {
    pub(crate) key: KeyOperand,
    pub(crate) value: u32,
}

#[derive(Clone, Debug)]
pub(crate) enum KeyOperand {
    Static(u32),
    Computed(u32),
}

impl Bytecode {
    pub(crate) fn string(&self, index: u32) -> Option<&Arc<str>> {
        self.strings.get(index as usize)
    }
    /// Decodes and validates a kernel bytecode word stream. This format is
    /// intended for fuzzing, persistence tests, and untrusted program rejection.
    ///
    /// # Errors
    ///
    /// Returns a structural, target, stack, or operand diagnostic.
    pub fn decode(words: &[u32]) -> Result<Self, BytecodeError> {
        let source = crate::SourceId::new(0);
        let mut instructions = Vec::new();
        let mut word = 0;
        while word < words.len() {
            let header = words[word];
            let opcode = (header >> 24) as u8;
            let operand_count = usize::try_from(header & 0x00ff_ffff).unwrap_or(usize::MAX);
            let end = word.saturating_add(1).saturating_add(operand_count);
            if end > words.len() {
                return Err(BytecodeError::Truncated { word });
            }
            let operands = &words[word + 1..end];
            let operation = decode_kernel(opcode, operands, instructions.len(), word)?;
            instructions.push(Instruction {
                operation,
                span: Span::new(source, word as u64, end as u64),
            });
            word = end;
        }
        let bytecode = Self {
            root: 0,
            instructions: instructions.into(),
            constants: Arc::from([]),
            strings: Arc::from([]),
            functions: Arc::from([]),
            modules: Arc::from([]),
            managed_tree_execution: false,
        };
        bytecode.validate()?;
        Ok(bytecode)
    }

    /// Re-encodes kernel instructions. Tree-evaluation instructions remain an
    /// internal compiler representation and are shown through disassembly.
    #[must_use]
    pub fn encode_kernel(&self) -> Option<Vec<u32>> {
        let mut words = Vec::new();
        for instruction in &*self.instructions {
            let (opcode, operands) = encode_kernel(&instruction.operation)?;
            let operand_count = u32::try_from(operands.len()).ok()?;
            if operand_count > 0x00ff_ffff {
                return None;
            }
            words.push((u32::from(opcode) << 24) | operand_count);
            words.extend(operands);
        }
        Some(words)
    }

    /// Validates targets, pools, root, and kernel stack effects.
    ///
    /// # Errors
    ///
    /// Returns the first deterministic validation failure.
    pub fn validate(&self) -> Result<(), BytecodeError> {
        if self.instructions.is_empty() || self.root as usize >= self.instructions.len() {
            return Err(BytecodeError::Root { root: self.root });
        }
        for (index, instruction) in self.instructions.iter().enumerate() {
            validate_instruction(self, index, &instruction.operation)?;
        }
        for (symbol, function) in self.functions.iter().enumerate() {
            if function.name as usize >= self.strings.len() {
                return Err(BytecodeError::String {
                    instruction: symbol,
                    string: function.name,
                });
            }
            if function.body as usize >= self.instructions.len() {
                return Err(BytecodeError::Target {
                    instruction: symbol,
                    target: function.body,
                });
            }
            for parameter in &function.parameters {
                if let Some(name) = parameter.runtime_name
                    && name as usize >= self.strings.len()
                {
                    return Err(BytecodeError::String {
                        instruction: symbol,
                        string: name,
                    });
                }
            }
        }
        validate_kernel_stack(self)?;
        Ok(())
    }

    /// Stable source-annotated bytecode disassembly.
    #[must_use]
    pub fn disassemble(&self) -> String {
        let mut output = format!(
            "root={} instructions={} constants={} strings={} functions={} modules={}\n",
            self.root,
            self.instructions.len(),
            self.constants.len(),
            self.strings.len(),
            self.functions.len(),
            self.modules.len(),
        );
        for (symbol, function) in self.functions.iter().enumerate() {
            use std::fmt::Write as _;
            writeln!(
                output,
                "function[{symbol}] body={} params={} span={}..{}",
                function.body,
                function.parameters.len(),
                function.span.start,
                function.span.end
            )
            .expect("writing to String cannot fail");
        }
        for (offset, instruction) in self.instructions.iter().enumerate() {
            use std::fmt::Write as _;
            writeln!(
                output,
                "{offset:04} {:<40} stack={:+} span={}..{}",
                DisplayOperation(&instruction.operation),
                stack_effect(&instruction.operation),
                instruction.span.start,
                instruction.span.end
            )
            .expect("writing to String cannot fail");
        }
        output
    }

    pub(crate) fn compile(ast: &Expr, modules: &[ModuleInfo]) -> Result<Self, Box<Diagnostic>> {
        let mut compiler = Compiler::default();
        let root = compiler.expression(ast)?;
        let functions = compiler
            .functions
            .into_iter()
            .enumerate()
            .map(|(symbol, function)| {
                function.ok_or_else(|| {
                    Box::new(Diagnostic::new(
                        "TQ-BYTECODE-FUNCTION-001",
                        DiagnosticClass::Compile,
                        format!("user filter symbol {symbol} was not compiled"),
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut bytecode = Self {
            instructions: compiler.instructions.into(),
            constants: compiler.constants.into(),
            strings: compiler.strings.into(),
            functions: functions.into(),
            modules: modules.to_vec().into(),
            root,
            managed_tree_execution: false,
        };
        bytecode.validate().map_err(|error| {
            Box::new(
                Diagnostic::new(
                    "TQ-BYTECODE-VALIDATE-001",
                    DiagnosticClass::Compile,
                    error.to_string(),
                )
                .at(ast.span, "compiled bytecode failed mandatory validation"),
            )
        })?;
        bytecode.managed_tree_execution = crate::eval::managed_execution(&bytecode);
        Ok(bytecode)
    }

    pub(crate) fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    pub(crate) fn constants(&self) -> &[Value] {
        &self.constants
    }

    pub(crate) fn functions(&self) -> &[UserFunction] {
        &self.functions
    }

    pub(crate) fn modules(&self) -> &[ModuleInfo] {
        &self.modules
    }

    pub(crate) const fn root(&self) -> u32 {
        self.root
    }

    pub(crate) const fn managed_tree_execution(&self) -> bool {
        self.managed_tree_execution
    }

    #[cfg(test)]
    pub(crate) fn kernel(instructions: Vec<Operation>, constants: Vec<Value>) -> Self {
        let source = crate::SourceId::new(0);
        let instructions = instructions
            .into_iter()
            .enumerate()
            .map(|(index, operation)| Instruction {
                operation,
                span: Span::new(source, index as u64, index as u64 + 1),
            })
            .collect::<Vec<_>>();
        let has_raise = instructions
            .iter()
            .any(|instruction| matches!(instruction.operation, Operation::Raise(_)));
        Self {
            instructions: instructions.into(),
            constants: constants.into(),
            strings: if has_raise {
                Arc::from([Arc::from("runtime error")])
            } else {
                Arc::from([])
            },
            functions: Arc::from([]),
            modules: Arc::from([]),
            root: 0,
            managed_tree_execution: false,
        }
    }
}

#[derive(Default)]
struct Compiler {
    instructions: Vec<Instruction>,
    constants: Vec<Value>,
    strings: Vec<Arc<str>>,
    functions: Vec<Option<UserFunction>>,
}

impl Compiler {
    #[allow(
        clippy::too_many_lines,
        reason = "bytecode lowering exhaustively maps the source-spanned HIR"
    )]
    fn expression(&mut self, expr: &Expr) -> Result<u32, Box<Diagnostic>> {
        let operation = match &expr.kind {
            ExprKind::Identity => Operation::Identity,
            ExprKind::Literal(value) => {
                let constant = self.constant(value.clone())?;
                Operation::Literal(constant)
            }
            ExprKind::Variable(name) => Operation::Variable(self.string(name)),
            ExprKind::Empty => Operation::Empty,
            ExprKind::RecursiveDescent => Operation::RecursiveDescent,
            ExprKind::Interpolation(segments) => Operation::Interpolation(
                segments
                    .iter()
                    .map(|segment| match segment {
                        InterpolationSegment::Literal { value, .. } => {
                            Ok(InterpolationOperand::Literal(self.string(value)))
                        }
                        InterpolationSegment::Expression(expression) => Ok(
                            InterpolationOperand::Expression(self.expression(expression)?),
                        ),
                    })
                    .collect::<Result<_, Box<Diagnostic>>>()?,
            ),
            ExprKind::Access { base, access } => {
                let base = self.expression(base)?;
                match access {
                    Access::Field(key) => Operation::AccessField {
                        base,
                        key: self.string(key),
                    },
                    Access::Index(index) => Operation::AccessIndex {
                        base,
                        index: self.expression(index)?,
                    },
                    Access::Slice { start, end } => Operation::Slice {
                        base,
                        start: start
                            .as_deref()
                            .map(|value| self.expression(value))
                            .transpose()?,
                        end: end
                            .as_deref()
                            .map(|value| self.expression(value))
                            .transpose()?,
                    },
                    Access::Iterate => Operation::Iterate(base),
                }
            }
            ExprKind::Optional(child) => Operation::Optional(self.expression(child)?),
            ExprKind::Pipe(left, right) => Operation::Pipe {
                left: self.expression(left)?,
                right: self.expression(right)?,
            },
            ExprKind::Comma(left, right) => Operation::Comma {
                left: self.expression(left)?,
                right: self.expression(right)?,
            },
            ExprKind::Array(child) => Operation::Array(self.expression(child)?),
            ExprKind::Object(entries) => {
                let mut operands = Vec::with_capacity(entries.len());
                for entry in entries {
                    let key = match &entry.key {
                        ObjectKey::Static(key) => KeyOperand::Static(self.string(key)),
                        ObjectKey::Computed(key) => KeyOperand::Computed(self.expression(key)?),
                    };
                    operands.push(ObjectOperand {
                        key,
                        value: self.expression(&entry.value)?,
                    });
                }
                Operation::Object(operands)
            }
            ExprKind::Unary {
                operator,
                expression,
            } => Operation::Unary {
                operator: *operator,
                child: self.expression(expression)?,
            },
            ExprKind::Binary {
                operator,
                left,
                right,
            } => Operation::Binary {
                operator: *operator,
                left: self.expression(left)?,
                right: self.expression(right)?,
            },
            ExprKind::Conditional {
                branches,
                alternative,
            } => Operation::Conditional {
                branches: branches
                    .iter()
                    .map(|(condition, body)| {
                        Ok((self.expression(condition)?, self.expression(body)?))
                    })
                    .collect::<Result<_, Box<Diagnostic>>>()?,
                alternative: self.expression(alternative)?,
            },
            ExprKind::Bind { value, name, body } => Operation::Bind {
                value: self.expression(value)?,
                name: self.string(name),
                body: self.expression(body)?,
            },
            ExprKind::Reduce {
                generator,
                name,
                initial,
                update,
            } => Operation::Reduce {
                generator: self.expression(generator)?,
                name: self.string(name),
                initial: self.expression(initial)?,
                update: self.expression(update)?,
            },
            ExprKind::Foreach {
                generator,
                name,
                initial,
                update,
                extract,
            } => Operation::Foreach {
                generator: self.expression(generator)?,
                name: self.string(name),
                initial: self.expression(initial)?,
                update: self.expression(update)?,
                extract: self.expression(extract)?,
            },
            ExprKind::Define { definition, body } => {
                let symbol = definition.symbol.ok_or_else(|| {
                    Box::new(
                        Diagnostic::new(
                            "TQ-BYTECODE-FUNCTION-001",
                            DiagnosticClass::Compile,
                            "definition has no resolved symbol",
                        )
                        .at(definition.span, "resolve definitions before compilation"),
                    )
                })?;
                let symbol_index = usize::try_from(symbol).unwrap_or(usize::MAX);
                self.functions.resize(symbol_index.saturating_add(1), None);
                let function_body = self.expression(&definition.body)?;
                let parameters = definition
                    .parameters
                    .iter()
                    .map(|parameter| UserParameter {
                        kind: parameter.kind,
                        runtime_name: parameter
                            .runtime_name
                            .as_ref()
                            .map(|name| self.string(name)),
                    })
                    .collect();
                self.functions[symbol_index] = Some(UserFunction {
                    name: self.string(&definition.name),
                    parameters,
                    body: function_body,
                    span: definition.span,
                });
                return self.expression(body);
            }
            ExprKind::Call {
                name,
                arguments,
                target,
            } => {
                let arguments = arguments
                    .iter()
                    .map(|argument| self.expression(argument))
                    .collect::<Result<Vec<_>, _>>()?;
                match target {
                    Some(CallTarget::Builtin) => Operation::Call {
                        name: self.string(name),
                        arguments,
                    },
                    Some(CallTarget::User(symbol)) => Operation::UserCall {
                        symbol: *symbol,
                        arguments,
                    },
                    Some(CallTarget::Parameter { function, index }) => {
                        if !arguments.is_empty() {
                            return Err(Box::new(
                                Diagnostic::new(
                                    "TQ-BYTECODE-PARAMETER-001",
                                    DiagnosticClass::Compile,
                                    "filter parameter calls cannot carry arguments",
                                )
                                .at(expr.span, "invalid resolved filter parameter call"),
                            ));
                        }
                        Operation::ParameterCall {
                            function: *function,
                            parameter: *index,
                        }
                    }
                    None => {
                        return Err(Box::new(
                            Diagnostic::new(
                                "TQ-BYTECODE-CALL-001",
                                DiagnosticClass::Compile,
                                "call has no resolved target",
                            )
                            .at(expr.span, "resolve calls before compilation"),
                        ));
                    }
                }
            }
            ExprKind::Include { .. } | ExprKind::Import { .. } | ExprKind::Module { .. } => {
                return Err(Box::new(
                    Diagnostic::new(
                        "TQ-BYTECODE-MODULE-001",
                        DiagnosticClass::Compile,
                        "module directive remained after resolution",
                    )
                    .at(expr.span, "expand modules before compilation"),
                ));
            }
            ExprKind::TryCatch { expression, catch } => Operation::TryCatch {
                expression: self.expression(expression)?,
                catch: catch
                    .as_deref()
                    .map(|catch| self.expression(catch))
                    .transpose()?,
            },
            ExprKind::Assignment {
                operator,
                path,
                value,
            } => Operation::Assignment {
                operator: *operator,
                path: self.expression(path)?,
                value: self.expression(value)?,
            },
        };
        let index = u32::try_from(self.instructions.len()).map_err(|_| {
            Box::new(
                Diagnostic::new(
                    "TQ-RESOURCE-BYTECODE-001",
                    DiagnosticClass::Resource,
                    "query contains too many bytecode instructions",
                )
                .at(expr.span, "instruction limit exceeded"),
            )
        })?;
        self.instructions.push(Instruction {
            operation,
            span: expr.span,
        });
        Ok(index)
    }

    fn constant(&mut self, value: Value) -> Result<u32, Box<Diagnostic>> {
        let index = u32::try_from(self.constants.len()).map_err(|_| {
            Box::new(Diagnostic::new(
                "TQ-RESOURCE-CONSTANTS-001",
                DiagnosticClass::Resource,
                "query contains too many constants",
            ))
        })?;
        self.constants.push(value);
        Ok(index)
    }

    fn string(&mut self, value: &Arc<str>) -> u32 {
        if let Some(index) = self.strings.iter().position(|item| item == value) {
            return u32::try_from(index).unwrap_or(u32::MAX);
        }
        let index = u32::try_from(self.strings.len()).unwrap_or(u32::MAX);
        self.strings.push(value.clone());
        index
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "validation exhaustively checks each bytecode operation and operand kind"
)]
fn validate_instruction(
    bytecode: &Bytecode,
    index: usize,
    operation: &Operation,
) -> Result<(), BytecodeError> {
    let target = |target: u32| {
        if target as usize >= bytecode.instructions.len() {
            Err(BytecodeError::Target {
                instruction: index,
                target,
            })
        } else {
            Ok(())
        }
    };
    let constant = |constant: u32| {
        if constant as usize >= bytecode.constants.len() {
            Err(BytecodeError::Constant {
                instruction: index,
                constant,
            })
        } else {
            Ok(())
        }
    };
    let string = |string: u32| {
        if string as usize >= bytecode.strings.len() {
            Err(BytecodeError::String {
                instruction: index,
                string,
            })
        } else {
            Ok(())
        }
    };
    match operation {
        Operation::LoadConstant(value) | Operation::Literal(value) => constant(*value)?,
        Operation::Jump(value) | Operation::Fork(value) | Operation::Catch(value) => {
            target(*value)?;
        }
        Operation::Branch { truthy, falsey } => {
            target(*truthy)?;
            target(*falsey)?;
        }
        Operation::Raise(message) | Operation::Variable(message) => string(*message)?,
        Operation::Interpolation(segments) => {
            for segment in segments {
                match segment {
                    InterpolationOperand::Literal(value) => string(*value)?,
                    InterpolationOperand::Expression(expression) => target(*expression)?,
                }
            }
        }
        Operation::AccessField { base, key } => {
            target(*base)?;
            string(*key)?;
        }
        Operation::AccessIndex { base, index } => {
            target(*base)?;
            target(*index)?;
        }
        Operation::Slice { base, start, end } => {
            target(*base)?;
            if let Some(start) = start {
                target(*start)?;
            }
            if let Some(end) = end {
                target(*end)?;
            }
        }
        Operation::Iterate(child) | Operation::Optional(child) | Operation::Array(child) => {
            target(*child)?;
        }
        Operation::Pipe { left, right }
        | Operation::Comma { left, right }
        | Operation::Binary { left, right, .. }
        | Operation::Assignment {
            path: left,
            value: right,
            ..
        } => {
            target(*left)?;
            target(*right)?;
        }
        Operation::Object(entries) => {
            for entry in entries {
                match entry.key {
                    KeyOperand::Static(key) => string(key)?,
                    KeyOperand::Computed(key) => target(key)?,
                }
                target(entry.value)?;
            }
        }
        Operation::Unary { child, .. } => target(*child)?,
        Operation::Conditional {
            branches,
            alternative,
        } => {
            for (condition, body) in branches {
                target(*condition)?;
                target(*body)?;
            }
            target(*alternative)?;
        }
        Operation::Bind { value, name, body } => {
            target(*value)?;
            string(*name)?;
            target(*body)?;
        }
        Operation::Reduce {
            generator,
            name,
            initial,
            update,
        } => {
            target(*generator)?;
            string(*name)?;
            target(*initial)?;
            target(*update)?;
        }
        Operation::Foreach {
            generator,
            name,
            initial,
            update,
            extract,
        } => {
            target(*generator)?;
            string(*name)?;
            target(*initial)?;
            target(*update)?;
            target(*extract)?;
        }
        Operation::Call { name, arguments } => {
            string(*name)?;
            let name_value = &bytecode.strings[*name as usize];
            if let Some(builtin) = BuiltinRegistry.get(name_value) {
                if !(builtin.minimum_arity..=builtin.maximum_arity).contains(&arguments.len()) {
                    return Err(BytecodeError::CallArity {
                        instruction: index,
                        name: Arc::clone(name_value),
                        arity: arguments.len(),
                    });
                }
            }
            for argument in arguments {
                target(*argument)?;
            }
        }
        Operation::UserCall { symbol, arguments } => {
            let Some(function) = bytecode.functions.get(*symbol as usize) else {
                return Err(BytecodeError::Function {
                    instruction: index,
                    symbol: *symbol,
                });
            };
            if function.parameters.len() != arguments.len() {
                return Err(BytecodeError::CallArity {
                    instruction: index,
                    name: Arc::clone(&bytecode.strings[function.name as usize]),
                    arity: arguments.len(),
                });
            }
            for argument in arguments {
                target(*argument)?;
            }
        }
        Operation::ParameterCall {
            function,
            parameter,
        } => {
            let Some(function_value) = bytecode.functions.get(*function as usize) else {
                return Err(BytecodeError::Function {
                    instruction: index,
                    symbol: *function,
                });
            };
            let Some(parameter_value) = function_value.parameters.get(*parameter as usize) else {
                return Err(BytecodeError::CallArity {
                    instruction: index,
                    name: Arc::clone(&bytecode.strings[function_value.name as usize]),
                    arity: *parameter as usize,
                });
            };
            if parameter_value.kind != ParameterKind::Filter {
                return Err(BytecodeError::CallArity {
                    instruction: index,
                    name: Arc::clone(&bytecode.strings[function_value.name as usize]),
                    arity: *parameter as usize,
                });
            }
        }
        Operation::TryCatch { expression, catch } => {
            target(*expression)?;
            if let Some(catch) = catch {
                target(*catch)?;
            }
        }
        Operation::LoadInput
        | Operation::Duplicate
        | Operation::Pop
        | Operation::Backtrack
        | Operation::Return
        | Operation::EndCatch
        | Operation::Identity
        | Operation::Empty
        | Operation::RecursiveDescent => {}
    }
    Ok(())
}

fn validate_kernel_stack(bytecode: &Bytecode) -> Result<(), BytecodeError> {
    if !bytecode
        .instructions
        .iter()
        .all(|instruction| kernel(&instruction.operation))
    {
        return Ok(());
    }
    let mut heights = vec![None; bytecode.instructions.len()];
    let mut queue = VecDeque::from([(0_usize, 0_i64)]);
    while let Some((index, height)) = queue.pop_front() {
        if let Some(existing) = heights[index] {
            if existing != height {
                return Err(BytecodeError::StackMismatch { instruction: index });
            }
            continue;
        }
        heights[index] = Some(height);
        let operation = &bytecode.instructions[index].operation;
        let next_height = height + i64::from(stack_effect(operation));
        if next_height < 0 {
            return Err(BytecodeError::StackUnderflow { instruction: index });
        }
        let next = index + 1;
        let mut push = |target: usize, target_height: i64| {
            if target < bytecode.instructions.len() {
                queue.push_back((target, target_height));
            }
        };
        match operation {
            Operation::Jump(target) => push(*target as usize, next_height),
            Operation::Branch { truthy, falsey } => {
                push(*truthy as usize, next_height);
                push(*falsey as usize, next_height);
            }
            Operation::Fork(target) => {
                push(next, next_height);
                push(*target as usize, next_height);
            }
            Operation::Catch(target) => {
                push(next, next_height);
                push(*target as usize, next_height + 1);
            }
            Operation::Return | Operation::Backtrack | Operation::Raise(_) => {}
            _ => push(next, next_height),
        }
    }
    Ok(())
}

pub(crate) const fn kernel(operation: &Operation) -> bool {
    matches!(
        operation,
        Operation::LoadInput
            | Operation::LoadConstant(_)
            | Operation::Duplicate
            | Operation::Pop
            | Operation::Jump(_)
            | Operation::Branch { .. }
            | Operation::Fork(_)
            | Operation::Backtrack
            | Operation::Return
            | Operation::Raise(_)
            | Operation::Catch(_)
            | Operation::EndCatch
    )
}

const fn stack_effect(operation: &Operation) -> i8 {
    match operation {
        Operation::LoadInput | Operation::LoadConstant(_) | Operation::Duplicate => 1,
        Operation::Pop | Operation::Branch { .. } | Operation::Return => -1,
        _ => 0,
    }
}

fn decode_kernel(
    opcode: u8,
    operands: &[u32],
    instruction: usize,
    word: usize,
) -> Result<Operation, BytecodeError> {
    let expected = |count: usize| {
        if operands.len() == count {
            Ok(())
        } else {
            Err(BytecodeError::OperandCount { instruction })
        }
    };
    Ok(match opcode {
        0 => {
            expected(0)?;
            Operation::LoadInput
        }
        1 => {
            expected(1)?;
            Operation::LoadConstant(operands[0])
        }
        2 => {
            expected(0)?;
            Operation::Duplicate
        }
        3 => {
            expected(0)?;
            Operation::Pop
        }
        4 => {
            expected(1)?;
            Operation::Jump(operands[0])
        }
        5 => {
            expected(2)?;
            Operation::Branch {
                truthy: operands[0],
                falsey: operands[1],
            }
        }
        6 => {
            expected(1)?;
            Operation::Fork(operands[0])
        }
        7 => {
            expected(0)?;
            Operation::Backtrack
        }
        8 => {
            expected(0)?;
            Operation::Return
        }
        9 => {
            expected(1)?;
            Operation::Raise(operands[0])
        }
        10 => {
            expected(1)?;
            Operation::Catch(operands[0])
        }
        11 => {
            expected(0)?;
            Operation::EndCatch
        }
        _ => return Err(BytecodeError::UnknownOpcode { opcode, word }),
    })
}

fn encode_kernel(operation: &Operation) -> Option<(u8, Vec<u32>)> {
    Some(match operation {
        Operation::LoadInput => (0, Vec::new()),
        Operation::LoadConstant(value) => (1, vec![*value]),
        Operation::Duplicate => (2, Vec::new()),
        Operation::Pop => (3, Vec::new()),
        Operation::Jump(target) => (4, vec![*target]),
        Operation::Branch { truthy, falsey } => (5, vec![*truthy, *falsey]),
        Operation::Fork(target) => (6, vec![*target]),
        Operation::Backtrack => (7, Vec::new()),
        Operation::Return => (8, Vec::new()),
        Operation::Raise(message) => (9, vec![*message]),
        Operation::Catch(target) => (10, vec![*target]),
        Operation::EndCatch => (11, Vec::new()),
        _ => return None,
    })
}

struct DisplayOperation<'a>(&'a Operation);

impl fmt::Display for DisplayOperation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Operation::LoadInput => formatter.write_str("load-input"),
            Operation::LoadConstant(value) => write!(formatter, "load-constant {value}"),
            Operation::Duplicate => formatter.write_str("duplicate"),
            Operation::Pop => formatter.write_str("pop"),
            Operation::Jump(target) => write!(formatter, "jump {target}"),
            Operation::Branch { truthy, falsey } => {
                write!(formatter, "branch {truthy} {falsey}")
            }
            Operation::Fork(target) => write!(formatter, "fork {target}"),
            Operation::Backtrack => formatter.write_str("backtrack"),
            Operation::Return => formatter.write_str("return"),
            Operation::Raise(message) => write!(formatter, "raise {message}"),
            Operation::Catch(target) => write!(formatter, "catch {target}"),
            Operation::EndCatch => formatter.write_str("end-catch"),
            other => write!(formatter, "{other:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{ResolveOptions, Span, analyze, parse, resolve};

    use super::{Bytecode, BytecodeError, Instruction, Operation};

    #[test]
    fn compiler_validates_all_mvp_node_families_and_disassembles_spans() {
        let query = "(.a, [1, .[]], {x: .b}) | if . then try length catch 0 else empty end";
        let analyzed = analyze(resolve(parse(query).unwrap(), &ResolveOptions::default()).unwrap());
        let program = analyzed.compile().unwrap();
        let disassembly = program.disassemble();
        assert!(disassembly.contains("instructions="));
        assert!(disassembly.contains("span="));
        assert!(disassembly.contains("Conditional"));

        let folds = analyze(
            resolve(
                parse("reduce (1,2) as $x (0; . + $x), foreach (1,2) as $x (0; . + $x; .)")
                    .unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .disassemble();
        assert!(folds.contains("Reduce"));
        assert!(folds.contains("Foreach"));

        let recursive_interpolation = analyze(
            resolve(
                parse("\"value=\\(.. | scalars)\"").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .disassemble();
        assert!(recursive_interpolation.contains("RecursiveDescent"));
        assert!(recursive_interpolation.contains("Interpolation"));
    }

    #[test]
    fn decoder_rejects_unknown_truncated_targets_and_stack_underflow() {
        assert!(matches!(
            Bytecode::decode(&[255 << 24]),
            Err(BytecodeError::UnknownOpcode { .. })
        ));
        assert!(matches!(
            Bytecode::decode(&[(4 << 24) | 1]),
            Err(BytecodeError::Truncated { .. })
        ));
        assert!(matches!(
            Bytecode::decode(&[(4 << 24) | 1, 9]),
            Err(BytecodeError::Target { .. })
        ));
        assert!(matches!(
            Bytecode::decode(&[3 << 24]),
            Err(BytecodeError::StackUnderflow { .. })
        ));

        let source = crate::SourceId::new(0);
        let invalid_call = Bytecode {
            instructions: Arc::from([Instruction {
                operation: Operation::Call {
                    name: 0,
                    arguments: Vec::new(),
                },
                span: Span::new(source, 0, 1),
            }]),
            constants: Arc::from([]),
            strings: Arc::from([Arc::from("range")]),
            functions: Arc::from([]),
            modules: Arc::from([]),
            root: 0,
            managed_tree_execution: false,
        };
        assert!(matches!(
            invalid_call.validate(),
            Err(BytecodeError::CallArity { .. })
        ));
    }

    #[test]
    fn arbitrary_encoded_words_are_rejected_or_valid_without_panicking() {
        let mut state = 0x1234_5678_u32;
        for length in 0..64 {
            let mut words = Vec::new();
            for _ in 0..length {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                words.push(state);
            }
            if let Ok(bytecode) = Bytecode::decode(&words) {
                bytecode.validate().unwrap();
            }
        }
    }
}
