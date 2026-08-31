//! Bounded evaluator for source-mapped expression bytecode.

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    io,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};

use indexmap::IndexMap;
use rayon::{
    iter::{
        IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
    },
    slice::ParallelSliceMut,
};

use crate::{
    Bytecode, Number, Object, Path, PathComponent, Value, VmError, VmLimits, VmObservations,
    ast::{AssignmentOperator, BinaryOperator, ParameterKind, UnaryOperator},
    bytecode::{InterpolationOperand, KeyOperand, Operation},
    stdlib,
};

pub(crate) const AMBIENT_ENVIRONMENT: &str = "__tq_ambient_environment";
pub(crate) const AMBIENT_PLATFORM: &str = "__tq_ambient_platform";
pub(crate) const INPUT_FILENAME: &str = "__tq_input_filename";
pub(crate) const INPUT_LINE_NUMBER: &str = "__tq_input_line_number";

type Environment = BTreeMap<Arc<str>, Value>;
type Outcomes = Vec<Result<Value, VmError>>;
type UserFrames = Arc<[UserFrame]>;

// Below this size, thread-pool startup and scheduling cost more than the work.
const PARALLEL_SORT_THRESHOLD: usize = 16 * 1024;
const PARALLEL_REDUCTION_THRESHOLD: usize = 64 * 1024;

#[derive(Clone)]
struct FilterArgument {
    node: u32,
    environment: Arc<Environment>,
    frames: UserFrames,
}

#[derive(Clone)]
struct UserFrame {
    symbol: u32,
    filters: Vec<Option<FilterArgument>>,
}

pub(crate) fn evaluate_stream(
    bytecode: &Bytecode,
    input: &Value,
    variables: &Environment,
    limits: VmLimits,
    cancellation: Option<&AtomicBool>,
    stop: &AtomicBool,
    mut emit: impl FnMut(Result<Value, VmError>, VmObservations) -> bool,
) -> VmObservations {
    if generator_subset(bytecode) {
        return evaluate_generator_stream(
            bytecode,
            input,
            variables,
            limits,
            cancellation,
            stop,
            emit,
        );
    }
    let evaluator = Evaluator {
        bytecode,
        limits,
        observations: Cell::new(VmObservations::default()),
        cancellation,
        stop,
    };
    evaluator.emit_node(bytecode.root(), input, variables, 0, &mut |result| {
        let mut observations = evaluator.observations.get();
        if result.is_ok() {
            observations.results += 1;
            evaluator.observations.set(observations);
        }
        emit(result, observations)
    });
    evaluator.observations.get()
}

#[derive(Clone)]
enum GeneratorContinuation {
    AccessField(Arc<str>),
    Iterate,
    Pipe {
        node: u32,
        environment: Arc<Environment>,
    },
    BinaryLeft {
        operator: BinaryOperator,
        right: u32,
        input: Value,
        environment: Arc<Environment>,
    },
    BinaryRight {
        operator: BinaryOperator,
        left: Value,
    },
    Unary(UnaryOperator),
    ArrayItem(Rc<RefCell<Vec<Value>>>),
    AlternativeItem(Rc<Cell<bool>>),
    Bind {
        name: Arc<str>,
        body: u32,
        input: Value,
        environment: Arc<Environment>,
    },
    Conditional {
        branches: Arc<[(u32, u32)]>,
        next: usize,
        alternative: u32,
        input: Value,
        environment: Arc<Environment>,
    },
    AccessIndex {
        node: u32,
        environment: Arc<Environment>,
    },
    ApplyIndex(Value),
    OptionalBoundary,
    Catch {
        node: Option<u32>,
        environment: Arc<Environment>,
    },
    Interpolate {
        segments: Arc<[InterpolationOperand]>,
        next: usize,
        slot: usize,
        pieces: Vec<Option<Arc<str>>>,
        input: Value,
        environment: Arc<Environment>,
    },
    UserArgument {
        symbol: u32,
        arguments: Arc<[u32]>,
        next: usize,
        input: Value,
        caller_environment: Arc<Environment>,
        caller_frames: UserFrames,
        filters: Vec<Option<FilterArgument>>,
        bindings: Environment,
    },
    ReturnUser {
        environment: Arc<Environment>,
        frames: UserFrames,
    },
    Raise,
}

#[derive(Clone)]
struct GeneratorWork {
    node: u32,
    input: Value,
    environment: Arc<Environment>,
    frames: UserFrames,
    continuations: Vec<GeneratorContinuation>,
}

enum GeneratorTask {
    Eval(GeneratorWork),
    Iterate {
        values: Arc<[Value]>,
        next: usize,
        environment: Arc<Environment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    FinishArray {
        values: Rc<RefCell<Vec<Value>>>,
        environment: Arc<Environment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    FinishAlternative {
        matched: Rc<Cell<bool>>,
        right: u32,
        input: Value,
        environment: Arc<Environment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    Traverse {
        cursor: TraversalCursor,
        environment: Arc<Environment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
}

enum InterpolationWork {
    Expand {
        next: usize,
        pieces: Vec<Option<Arc<str>>>,
    },
    Error(VmError),
}

#[derive(Clone)]
struct TraversalCursor {
    frames: Vec<TraversalFrame>,
}

#[derive(Clone)]
struct TraversalFrame {
    value: Value,
    emitted: bool,
    next_child: usize,
}

impl TraversalCursor {
    fn new(value: Value) -> Self {
        Self {
            frames: vec![TraversalFrame {
                value,
                emitted: false,
                next_child: 0,
            }],
        }
    }

    fn next(&mut self, depth_limit: usize) -> Result<Option<Value>, VmError> {
        loop {
            let Some(frame) = self.frames.last_mut() else {
                return Ok(None);
            };
            if !frame.emitted {
                frame.emitted = true;
                return Ok(Some(frame.value.clone()));
            }
            let child = match &frame.value {
                Value::Array(values) => values.get(frame.next_child).cloned(),
                Value::Object(values) => values
                    .get_index(frame.next_child)
                    .map(|(_, value)| value.clone()),
                Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
            };
            if let Some(child) = child {
                frame.next_child = frame.next_child.saturating_add(1);
                if self.frames.len() >= depth_limit {
                    return Err(resource("path-stack"));
                }
                self.frames.push(TraversalFrame {
                    value: child,
                    emitted: false,
                    next_child: 0,
                });
            } else {
                self.frames.pop();
            }
        }
    }

    fn depth(&self) -> usize {
        self.frames.len()
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the generator admission walk exhaustively covers bytecode operations"
)]
fn generator_subset(bytecode: &Bytecode) -> bool {
    let mut pending = vec![bytecode.root()];
    let mut seen = vec![false; bytecode.instructions().len()];
    while let Some(node) = pending.pop() {
        let Some(instruction) = bytecode.instructions().get(node as usize) else {
            return false;
        };
        if std::mem::replace(&mut seen[node as usize], true) {
            continue;
        }
        match &instruction.operation {
            Operation::Identity
            | Operation::Literal(_)
            | Operation::Variable(_)
            | Operation::Empty
            | Operation::RecursiveDescent
            | Operation::ParameterCall { .. } => {}
            Operation::AccessField { base, .. }
            | Operation::Iterate(base)
            | Operation::Optional(base)
            | Operation::Array(base)
            | Operation::Unary { child: base, .. } => pending.push(*base),
            Operation::AccessIndex { base, index } => {
                pending.push(*index);
                pending.push(*base);
            }
            Operation::Pipe { left, right }
            | Operation::Comma { left, right }
            | Operation::Binary { left, right, .. } => {
                pending.push(*right);
                pending.push(*left);
            }
            Operation::Conditional {
                branches,
                alternative,
            } => {
                pending.push(*alternative);
                for (condition, body) in branches {
                    pending.push(*body);
                    pending.push(*condition);
                }
            }
            Operation::Bind { value, body, .. } => {
                pending.push(*body);
                pending.push(*value);
            }
            Operation::UserCall { symbol, arguments } => {
                let Some(function) = bytecode.functions().get(*symbol as usize) else {
                    return false;
                };
                pending.push(function.body);
                pending.extend(arguments.iter().copied());
            }
            Operation::TryCatch { expression, catch } => {
                if let Some(catch) = catch {
                    pending.push(*catch);
                }
                pending.push(*expression);
            }
            Operation::Interpolation(segments) => {
                pending.extend(segments.iter().filter_map(|segment| match segment {
                    InterpolationOperand::Literal(_) => None,
                    InterpolationOperand::Expression(expression) => Some(*expression),
                }));
            }
            Operation::Call { name, arguments }
                if bytecode.string(*name).is_some_and(|name| {
                    (name.as_ref() == "empty" && arguments.is_empty())
                        || (name.as_ref() == "error" && arguments.len() <= 1)
                        || (arguments.is_empty()
                            && matches!(
                                name.as_ref(),
                                "arrays"
                                    | "add"
                                    | "booleans"
                                    | "keys"
                                    | "keys_unsorted"
                                    | "length"
                                    | "iterables"
                                    | "max"
                                    | "min"
                                    | "modulemeta"
                                    | "nulls"
                                    | "numbers"
                                    | "objects"
                                    | "scalars"
                                    | "strings"
                                    | "tonumber"
                                    | "values"
                                    | "reverse"
                                    | "sort"
                                    | "tostring"
                                    | "type"
                                    | "unique"
                                    | "utf8bytelength"
                            ))
                }) =>
            {
                pending.extend(arguments.iter().copied());
            }
            _ => return false,
        }
    }
    true
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the explicit generator loop keeps fork and continuation state in one auditable place"
)]
fn evaluate_generator_stream(
    bytecode: &Bytecode,
    input: &Value,
    variables: &Environment,
    limits: VmLimits,
    cancellation: Option<&AtomicBool>,
    stop: &AtomicBool,
    mut emit: impl FnMut(Result<Value, VmError>, VmObservations) -> bool,
) -> VmObservations {
    let environment = Arc::new(variables.clone());
    let mut pending = vec![GeneratorTask::Eval(GeneratorWork {
        node: bytecode.root(),
        input: input.clone(),
        environment,
        frames: Arc::from([]),
        continuations: Vec::new(),
    })];
    let mut observations = VmObservations::default();

    while let Some(task) = pending.pop() {
        if stop.load(Ordering::Relaxed)
            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            let _ = emit(Err(VmError::Interrupted), observations);
            break;
        }
        if observations.steps >= limits.steps {
            let _ = emit(Err(resource("vm-steps")), observations);
            break;
        }
        observations.steps += 1;
        let (mut work, delivered) = match task {
            GeneratorTask::Eval(work) => (work, None),
            GeneratorTask::Iterate {
                values,
                next,
                environment,
                frames,
                continuations,
            } => {
                let Some(value) = values.get(next).cloned() else {
                    continue;
                };
                if next + 1 < values.len() {
                    pending.push(GeneratorTask::Iterate {
                        values,
                        next: next + 1,
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: continuations.clone(),
                    });
                }
                (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(Ok(value)),
                )
            }
            GeneratorTask::FinishArray {
                values,
                environment,
                frames,
                continuations,
            } => (
                GeneratorWork {
                    node: bytecode.root(),
                    input: Value::Null,
                    environment,
                    frames,
                    continuations,
                },
                Some(Ok(Value::array(values.take()))),
            ),
            GeneratorTask::FinishAlternative {
                matched,
                right,
                input,
                environment,
                frames,
                continuations,
            } => {
                if matched.get() {
                    continue;
                }
                (
                    GeneratorWork {
                        node: right,
                        input,
                        environment,
                        frames,
                        continuations,
                    },
                    None,
                )
            }
            GeneratorTask::Traverse {
                mut cursor,
                environment,
                frames,
                continuations,
            } => {
                if cursor.depth() > limits.path_stack {
                    let _ = emit(Err(resource("path-stack")), observations);
                    break;
                }
                observations.path_stack_high_water =
                    observations.path_stack_high_water.max(cursor.depth());
                match cursor.next(limits.path_stack) {
                    Ok(Some(value)) => {
                        if pending.len() >= limits.fork_stack {
                            let _ = emit(Err(resource("fork-stack")), observations);
                            break;
                        }
                        pending.push(GeneratorTask::Traverse {
                            cursor,
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: continuations.clone(),
                        });
                        (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Ok(value)),
                        )
                    }
                    Ok(None) => continue,
                    Err(error) => {
                        let _ = emit(Err(error), observations);
                        break;
                    }
                }
            }
        };
        observations.value_stack_high_water = observations.value_stack_high_water.max(1);
        observations.fork_stack_high_water = observations.fork_stack_high_water.max(pending.len());
        observations.call_stack_high_water = observations
            .call_stack_high_water
            .max(work.continuations.len().saturating_add(1));
        if work.continuations.len() >= limits.call_stack {
            let _ = emit(Err(resource("call-stack")), observations);
            break;
        }
        if pending.len() >= limits.fork_stack {
            let _ = emit(Err(resource("fork-stack")), observations);
            break;
        }

        let value = if let Some(result) = delivered {
            Some(result)
        } else {
            let Some(instruction) = bytecode.instructions().get(work.node as usize) else {
                let _ = emit(
                    Err(invalid("tree instruction missing after validation")),
                    observations,
                );
                break;
            };
            match &instruction.operation {
                Operation::Identity => Some(Ok(work.input)),
                Operation::Literal(index) => Some(
                    bytecode
                        .constants()
                        .get(*index as usize)
                        .cloned()
                        .ok_or_else(|| invalid("literal missing after validation")),
                ),
                Operation::Variable(index) => Some(
                    bytecode
                        .string(*index)
                        .ok_or_else(|| invalid("string missing after validation"))
                        .and_then(|name| {
                            work.environment
                                .get(name)
                                .cloned()
                                .ok_or_else(|| runtime(format!("variable ${name} has no value")))
                        }),
                ),
                Operation::Empty => None,
                Operation::RecursiveDescent => {
                    pending.push(GeneratorTask::Traverse {
                        cursor: TraversalCursor::new(work.input.clone()),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    None
                }
                Operation::Interpolation(segments) => {
                    let expression_count = segments
                        .iter()
                        .filter(|segment| matches!(segment, InterpolationOperand::Expression(_)))
                        .count();
                    if expression_count >= limits.call_stack {
                        Some(Err(resource("call-stack")))
                    } else {
                        let segments = Arc::from(segments.clone());
                        match interpolation_pieces(&segments, bytecode, limits.output_bytes) {
                            Ok(pieces) => schedule_interpolation(
                                &segments,
                                segments.len(),
                                pieces,
                                work.input.clone(),
                                Arc::clone(&work.environment),
                                Arc::clone(&work.frames),
                                work.continuations.clone(),
                                bytecode,
                                limits.output_bytes,
                                &mut pending,
                            ),
                            Err(error) => Some(Err(error)),
                        }
                    }
                }
                Operation::AccessField { base, key } => match bytecode.string(*key) {
                    Some(key) => {
                        work.continuations
                            .push(GeneratorContinuation::AccessField(Arc::clone(key)));
                        work.node = *base;
                        pending.push(GeneratorTask::Eval(work.clone()));
                        None
                    }
                    None => Some(Err(invalid("string missing after validation"))),
                },
                Operation::AccessIndex { base, index } => {
                    work.continuations.push(GeneratorContinuation::AccessIndex {
                        node: *index,
                        environment: Arc::clone(&work.environment),
                    });
                    work.node = *base;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Iterate(base) => {
                    work.continuations.push(GeneratorContinuation::Iterate);
                    work.node = *base;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Optional(child) => {
                    work.continuations
                        .push(GeneratorContinuation::OptionalBoundary);
                    work.node = *child;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Pipe { left, right } => {
                    work.continuations.push(GeneratorContinuation::Pipe {
                        node: *right,
                        environment: Arc::clone(&work.environment),
                    });
                    work.node = *left;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Comma { left, right } => {
                    if pending.len().saturating_add(2) > limits.fork_stack {
                        Some(Err(resource("fork-stack")))
                    } else {
                        let right_work = GeneratorWork {
                            node: *right,
                            input: work.input.clone(),
                            environment: Arc::clone(&work.environment),
                            frames: Arc::clone(&work.frames),
                            continuations: work.continuations.clone(),
                        };
                        work.node = *left;
                        pending.push(GeneratorTask::Eval(right_work));
                        pending.push(GeneratorTask::Eval(work.clone()));
                        observations.fork_stack_high_water =
                            observations.fork_stack_high_water.max(pending.len());
                        None
                    }
                }
                Operation::Array(child) => {
                    let values = Rc::new(RefCell::new(Vec::new()));
                    pending.push(GeneratorTask::FinishArray {
                        values: Rc::clone(&values),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    work.continuations
                        .push(GeneratorContinuation::ArrayItem(values));
                    work.node = *child;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Unary { operator, child } => {
                    work.continuations
                        .push(GeneratorContinuation::Unary(*operator));
                    work.node = *child;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Binary {
                    operator,
                    left,
                    right,
                } => {
                    if *operator == BinaryOperator::Alternative {
                        let matched = Rc::new(Cell::new(false));
                        pending.push(GeneratorTask::FinishAlternative {
                            matched: Rc::clone(&matched),
                            right: *right,
                            input: work.input.clone(),
                            environment: Arc::clone(&work.environment),
                            frames: Arc::clone(&work.frames),
                            continuations: work.continuations.clone(),
                        });
                        work.continuations
                            .push(GeneratorContinuation::AlternativeItem(matched));
                    } else {
                        work.continuations.push(GeneratorContinuation::BinaryLeft {
                            operator: *operator,
                            right: *right,
                            input: work.input.clone(),
                            environment: Arc::clone(&work.environment),
                        });
                    }
                    work.node = *left;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Bind { value, name, body } => match bytecode.string(*name) {
                    Some(name) => {
                        work.continuations.push(GeneratorContinuation::Bind {
                            name: Arc::clone(name),
                            body: *body,
                            input: work.input.clone(),
                            environment: Arc::clone(&work.environment),
                        });
                        work.node = *value;
                        pending.push(GeneratorTask::Eval(work.clone()));
                        None
                    }
                    None => Some(Err(invalid("string missing after validation"))),
                },
                Operation::Conditional {
                    branches,
                    alternative,
                } => {
                    if let Some((condition, _)) = branches.first() {
                        work.continuations.push(GeneratorContinuation::Conditional {
                            branches: Arc::from(branches.clone()),
                            next: 0,
                            alternative: *alternative,
                            input: work.input.clone(),
                            environment: Arc::clone(&work.environment),
                        });
                        work.node = *condition;
                    } else {
                        work.node = *alternative;
                    }
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::UserCall { symbol, arguments } => schedule_user_call(
                    *symbol,
                    Arc::from(arguments.clone()),
                    0,
                    work.input.clone(),
                    Arc::clone(&work.environment),
                    Arc::clone(&work.frames),
                    vec![None; arguments.len()],
                    work.environment.as_ref().clone(),
                    work.continuations.clone(),
                    bytecode,
                    limits.call_stack,
                    &mut pending,
                ),
                Operation::ParameterCall {
                    function,
                    parameter,
                } => {
                    let argument = work
                        .frames
                        .iter()
                        .rev()
                        .find(|frame| frame.symbol == *function)
                        .and_then(|frame| frame.filters.get(*parameter as usize))
                        .and_then(Clone::clone);
                    match argument {
                        Some(argument) => {
                            work.continuations.push(GeneratorContinuation::ReturnUser {
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                            });
                            work.node = argument.node;
                            work.environment = argument.environment;
                            work.frames = argument.frames;
                            pending.push(GeneratorTask::Eval(work.clone()));
                            None
                        }
                        None => Some(Err(invalid(
                            "filter parameter missing from active user frame",
                        ))),
                    }
                }
                Operation::TryCatch { expression, catch } => {
                    work.continuations.push(GeneratorContinuation::Catch {
                        node: *catch,
                        environment: Arc::clone(&work.environment),
                    });
                    work.node = *expression;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Call { name, arguments }
                    if arguments.is_empty()
                        && bytecode
                            .string(*name)
                            .is_some_and(|name| name.as_ref() == "empty") =>
                {
                    None
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "error") =>
                {
                    if let Some(argument) = arguments.first() {
                        work.continuations.push(GeneratorContinuation::Raise);
                        work.node = *argument;
                        pending.push(GeneratorTask::Eval(work.clone()));
                        None
                    } else {
                        Some(Err(VmError::Runtime {
                            message: work.input.to_string().into(),
                        }))
                    }
                }
                Operation::Call { name, arguments }
                    if arguments.is_empty()
                        && bytecode
                            .string(*name)
                            .is_some_and(|name| name.as_ref() == "modulemeta") =>
                {
                    Some(module_metadata(bytecode, &work.input))
                }
                Operation::Call { name, arguments } if arguments.is_empty() => bytecode
                    .string(*name)
                    .ok_or_else(|| invalid("string missing after validation"))
                    .and_then(|name| generator_builtin(name, &work.input, limits.output_bytes))
                    .transpose(),
                operation => Some(Err(VmError::Unsupported {
                    operation: format!("{operation:?}").into(),
                })),
            }
        };

        let Some(mut result) = value else {
            continue;
        };
        loop {
            if let Err(error) = result {
                let mut handled = false;
                while let Some(continuation) = work.continuations.pop() {
                    match continuation {
                        GeneratorContinuation::OptionalBoundary => {
                            handled = true;
                            break;
                        }
                        GeneratorContinuation::Catch { node, environment } => {
                            if let Some(node) = node {
                                pending.push(GeneratorTask::Eval(GeneratorWork {
                                    node,
                                    input: Value::string(catch_value(&error)),
                                    environment,
                                    frames: work.frames,
                                    continuations: work.continuations,
                                }));
                            }
                            handled = true;
                            break;
                        }
                        GeneratorContinuation::ReturnUser {
                            environment,
                            frames,
                        } => {
                            work.environment = environment;
                            work.frames = frames;
                        }
                        GeneratorContinuation::AccessField(_)
                        | GeneratorContinuation::Iterate
                        | GeneratorContinuation::Pipe { .. }
                        | GeneratorContinuation::BinaryLeft { .. }
                        | GeneratorContinuation::BinaryRight { .. }
                        | GeneratorContinuation::Unary(_)
                        | GeneratorContinuation::ArrayItem(_)
                        | GeneratorContinuation::AlternativeItem(_)
                        | GeneratorContinuation::Bind { .. }
                        | GeneratorContinuation::Conditional { .. }
                        | GeneratorContinuation::AccessIndex { .. }
                        | GeneratorContinuation::ApplyIndex(_)
                        | GeneratorContinuation::Interpolate { .. }
                        | GeneratorContinuation::UserArgument { .. }
                        | GeneratorContinuation::Raise => {}
                    }
                }
                if !handled {
                    let _ = emit(Err(error), observations);
                    return observations;
                }
                break;
            }
            let Some(continuation) = work.continuations.pop() else {
                observations.results += 1;
                if !emit(result, observations) {
                    return observations;
                }
                break;
            };
            let Ok(value) = result else {
                unreachable!("error handled before continuation dispatch")
            };
            match continuation {
                GeneratorContinuation::AccessField(key) => {
                    result = access_field(&value, &key);
                }
                GeneratorContinuation::ApplyIndex(base) => {
                    result = access_index(&base, &value);
                }
                GeneratorContinuation::AccessIndex { node, environment } => {
                    work.continuations
                        .push(GeneratorContinuation::ApplyIndex(value.clone()));
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node,
                        input: value,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::Pipe { node, environment } => {
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node,
                        input: value,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::BinaryLeft {
                    operator,
                    right,
                    input,
                    environment,
                } => {
                    if (operator == BinaryOperator::And && !value.is_truthy())
                        || (operator == BinaryOperator::Or && value.is_truthy())
                    {
                        result = Ok(Value::Bool(operator == BinaryOperator::Or));
                        continue;
                    }
                    work.continuations.push(GeneratorContinuation::BinaryRight {
                        operator,
                        left: value,
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: right,
                        input,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::BinaryRight { operator, left } => {
                    result = if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
                        Ok(Value::Bool(value.is_truthy()))
                    } else {
                        binary_value(operator, &left, &value)
                    };
                }
                GeneratorContinuation::Unary(operator) => {
                    result = unary(operator, &value);
                }
                GeneratorContinuation::ArrayItem(values) => {
                    values.borrow_mut().push(value);
                    break;
                }
                GeneratorContinuation::AlternativeItem(matched) => {
                    if value.is_truthy() {
                        matched.set(true);
                        result = Ok(value);
                    } else {
                        break;
                    }
                }
                GeneratorContinuation::Bind {
                    name,
                    body,
                    input,
                    environment,
                } => {
                    let mut nested = environment.as_ref().clone();
                    nested.insert(name, value);
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: body,
                        input,
                        environment: Arc::new(nested),
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::Conditional {
                    branches,
                    next,
                    alternative,
                    input,
                    environment,
                } => {
                    let node = if value.is_truthy() {
                        branches[next].1
                    } else if next + 1 < branches.len() {
                        let condition = branches[next + 1].0;
                        work.continuations.push(GeneratorContinuation::Conditional {
                            branches,
                            next: next + 1,
                            alternative,
                            input: input.clone(),
                            environment: Arc::clone(&environment),
                        });
                        condition
                    } else {
                        alternative
                    };
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node,
                        input,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::Iterate => {
                    let values: Arc<[Value]> = match value {
                        Value::Array(values) => values,
                        Value::Object(values) => {
                            values.values().cloned().collect::<Vec<_>>().into()
                        }
                        value => {
                            result = Err(type_error("iterate", &value));
                            continue;
                        }
                    };
                    if !values.is_empty() {
                        pending.push(GeneratorTask::Iterate {
                            values,
                            next: 0,
                            environment: Arc::clone(&work.environment),
                            frames: Arc::clone(&work.frames),
                            continuations: work.continuations.clone(),
                        });
                    }
                    observations.fork_stack_high_water =
                        observations.fork_stack_high_water.max(pending.len());
                    break;
                }
                GeneratorContinuation::OptionalBoundary | GeneratorContinuation::Catch { .. } => {
                    result = Ok(value);
                }
                GeneratorContinuation::Interpolate {
                    segments,
                    next,
                    slot,
                    mut pieces,
                    input,
                    environment,
                } => {
                    let piece = interpolation_remaining(&pieces, limits.output_bytes)
                        .and_then(|remaining| interpolation_value(&value, remaining));
                    let piece = match piece {
                        Ok(piece) => piece,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    pieces[slot] = Some(piece);
                    if let Some(next_result) = schedule_interpolation(
                        &segments,
                        next,
                        pieces,
                        input,
                        environment,
                        Arc::clone(&work.frames),
                        work.continuations.clone(),
                        bytecode,
                        limits.output_bytes,
                        &mut pending,
                    ) {
                        result = next_result;
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::UserArgument {
                    symbol,
                    arguments,
                    next,
                    input,
                    caller_environment,
                    caller_frames,
                    filters,
                    mut bindings,
                } => {
                    let Some(function) = bytecode.functions().get(symbol as usize) else {
                        result = Err(invalid("user function missing after validation"));
                        continue;
                    };
                    let Some(parameter) = function.parameters.get(next) else {
                        result = Err(invalid("user function parameter missing after validation"));
                        continue;
                    };
                    let Some(name) = parameter
                        .runtime_name
                        .and_then(|name| bytecode.string(name))
                    else {
                        result = Err(invalid("value parameter name missing after validation"));
                        continue;
                    };
                    bindings.insert(Arc::clone(name), value);
                    if let Some(next_result) = schedule_user_call(
                        symbol,
                        arguments,
                        next + 1,
                        input,
                        caller_environment,
                        caller_frames,
                        filters,
                        bindings,
                        work.continuations.clone(),
                        bytecode,
                        limits.call_stack,
                        &mut pending,
                    ) {
                        result = next_result;
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::ReturnUser {
                    environment,
                    frames,
                } => {
                    work.environment = environment;
                    work.frames = frames;
                    result = Ok(value);
                }
                GeneratorContinuation::Raise => {
                    let message = match value {
                        Value::String(value) => value,
                        value => value.to_string().into(),
                    };
                    result = Err(VmError::Runtime { message });
                }
            }
        }
    }
    observations
}

#[allow(
    clippy::too_many_arguments,
    reason = "user-call scheduling keeps every captured VM resource explicit"
)]
fn schedule_user_call(
    symbol: u32,
    arguments: Arc<[u32]>,
    mut next: usize,
    input: Value,
    caller_environment: Arc<Environment>,
    caller_frames: UserFrames,
    mut filters: Vec<Option<FilterArgument>>,
    bindings: Environment,
    mut continuations: Vec<GeneratorContinuation>,
    bytecode: &Bytecode,
    call_limit: usize,
    pending: &mut Vec<GeneratorTask>,
) -> Option<Result<Value, VmError>> {
    let Some(function) = bytecode.functions().get(symbol as usize) else {
        return Some(Err(invalid("user function missing after validation")));
    };
    if arguments.len() != function.parameters.len() {
        return Some(Err(invalid("user function arity changed after validation")));
    }
    while let Some(parameter) = function.parameters.get(next) {
        match parameter.kind {
            ParameterKind::Filter => {
                filters[next] = Some(FilterArgument {
                    node: arguments[next],
                    environment: Arc::clone(&caller_environment),
                    frames: Arc::clone(&caller_frames),
                });
                next += 1;
            }
            ParameterKind::Value => {
                continuations.push(GeneratorContinuation::UserArgument {
                    symbol,
                    arguments,
                    next,
                    input: input.clone(),
                    caller_environment: Arc::clone(&caller_environment),
                    caller_frames: Arc::clone(&caller_frames),
                    filters,
                    bindings,
                });
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: continuations
                        .last()
                        .and_then(|continuation| match continuation {
                            GeneratorContinuation::UserArgument {
                                arguments, next, ..
                            } => arguments.get(*next).copied(),
                            _ => None,
                        })
                        .expect("just-pushed user argument is structurally complete"),
                    input,
                    environment: caller_environment,
                    frames: caller_frames,
                    continuations,
                }));
                return None;
            }
        }
    }
    if caller_frames.len() >= call_limit {
        return Some(Err(resource("call-stack")));
    }
    continuations.push(GeneratorContinuation::ReturnUser {
        environment: caller_environment,
        frames: Arc::clone(&caller_frames),
    });
    let mut frames = caller_frames.to_vec();
    frames.push(UserFrame { symbol, filters });
    pending.push(GeneratorTask::Eval(GeneratorWork {
        node: function.body,
        input,
        environment: Arc::new(bindings),
        frames: frames.into(),
        continuations,
    }));
    None
}

#[allow(
    clippy::too_many_arguments,
    reason = "interpolation continuation state is explicit and independently bounded"
)]
fn schedule_interpolation(
    segments: &Arc<[InterpolationOperand]>,
    mut next: usize,
    mut pieces: Vec<Option<Arc<str>>>,
    input: Value,
    environment: Arc<Environment>,
    frames: UserFrames,
    mut continuations: Vec<GeneratorContinuation>,
    bytecode: &Bytecode,
    output_limit: usize,
    pending: &mut Vec<GeneratorTask>,
) -> Option<Result<Value, VmError>> {
    while next > 0 {
        next -= 1;
        match &segments[next] {
            InterpolationOperand::Literal(index) => {
                let Some(value) = bytecode.string(*index) else {
                    return Some(Err(invalid("string missing after validation")));
                };
                pieces[next] = Some(Arc::clone(value));
            }
            InterpolationOperand::Expression(node) => {
                let node = *node;
                continuations.push(GeneratorContinuation::Interpolate {
                    segments: Arc::clone(segments),
                    next,
                    slot: next,
                    pieces,
                    input: input.clone(),
                    environment: Arc::clone(&environment),
                });
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node,
                    input,
                    environment,
                    frames,
                    continuations,
                }));
                return None;
            }
        }
    }
    let capacity = match interpolation_capacity(&pieces, output_limit) {
        Ok(capacity) => capacity,
        Err(error) => return Some(Err(error)),
    };
    let mut output = String::with_capacity(capacity);
    for piece in pieces {
        let Some(piece) = piece else {
            return Some(Err(invalid(
                "interpolation segment missing after evaluation",
            )));
        };
        output.push_str(&piece);
    }
    Some(Ok(Value::string(output)))
}

fn interpolation_pieces(
    segments: &[InterpolationOperand],
    bytecode: &Bytecode,
    output_limit: usize,
) -> Result<Vec<Option<Arc<str>>>, VmError> {
    let mut pieces = vec![None; segments.len()];
    let mut literal_bytes = 0_usize;
    for (slot, segment) in segments.iter().enumerate() {
        let InterpolationOperand::Literal(index) = segment else {
            continue;
        };
        let value = bytecode
            .string(*index)
            .ok_or_else(|| invalid("string missing after validation"))?;
        literal_bytes = literal_bytes
            .checked_add(value.len())
            .filter(|bytes| *bytes <= output_limit)
            .ok_or_else(|| resource("output-bytes"))?;
        pieces[slot] = Some(Arc::clone(value));
    }
    Ok(pieces)
}

fn interpolation_capacity(
    pieces: &[Option<Arc<str>>],
    output_limit: usize,
) -> Result<usize, VmError> {
    let capacity = pieces.iter().flatten().try_fold(0_usize, |total, piece| {
        total
            .checked_add(piece.len())
            .ok_or_else(|| resource("output-bytes"))
    })?;
    if capacity > output_limit {
        return Err(resource("output-bytes"));
    }
    Ok(capacity)
}

fn interpolation_remaining(
    pieces: &[Option<Arc<str>>],
    output_limit: usize,
) -> Result<usize, VmError> {
    interpolation_capacity(pieces, output_limit).map(|used| output_limit - used)
}

fn generator_builtin(
    name: &str,
    input: &Value,
    output_limit: usize,
) -> Result<Option<Value>, VmError> {
    let selected = match name {
        "add" => {
            return fold_values(input, None, binary_add)
                .into_iter()
                .next()
                .transpose();
        }
        "arrays" => matches!(input, Value::Array(_)),
        "booleans" => matches!(input, Value::Bool(_)),
        "iterables" => matches!(input, Value::Array(_) | Value::Object(_)),
        "nulls" => matches!(input, Value::Null),
        "numbers" => matches!(input, Value::Number(_)),
        "objects" => matches!(input, Value::Object(_)),
        "scalars" => !matches!(input, Value::Array(_) | Value::Object(_)),
        "strings" => matches!(input, Value::String(_)),
        "values" => !matches!(input, Value::Null),
        "keys" | "keys_unsorted" => return keys(input, name == "keys").map(Some),
        "length" => return length(input).map(Some),
        "max" => return extrema(input, true).into_iter().next().transpose(),
        "min" => return extrema(input, false).into_iter().next().transpose(),
        "reverse" => return reverse(input).into_iter().next().transpose(),
        "sort" => return sort_values(input).into_iter().next().transpose(),
        "tonumber" => match input {
            Value::Number(_) => return Ok(Some(input.clone())),
            Value::String(value) => {
                return Number::parse(value)
                    .map(Value::Number)
                    .map(Some)
                    .map_err(|error| runtime(error.to_string()));
            }
            value => return Err(type_error("tonumber", value)),
        },
        "tostring" => {
            return interpolation_value(input, output_limit)
                .map(Value::String)
                .map(Some);
        }
        "type" => return Ok(Some(Value::string(type_name(input)))),
        "unique" => return unique_values(input).into_iter().next().transpose(),
        "utf8bytelength" => match input {
            Value::String(value) => return number_usize(value.len()).map(Some),
            value => return Err(type_error("utf8bytelength", value)),
        },
        _ => return Err(invalid("generator built-in left admitted subset")),
    };
    Ok(selected.then(|| input.clone()))
}

fn interpolation_value(value: &Value, output_limit: usize) -> Result<Arc<str>, VmError> {
    match value {
        Value::String(value) if value.len() <= output_limit => Ok(Arc::clone(value)),
        Value::String(_) => Err(resource("output-bytes")),
        value => bounded_json(value, output_limit).map(Arc::from),
    }
}

struct BoundedJsonWriter {
    output: String,
    limit: usize,
}

impl BoundedJsonWriter {
    fn new(limit: usize) -> Self {
        Self {
            output: String::new(),
            limit,
        }
    }

    fn push(&mut self, value: &str) -> Result<(), VmError> {
        let length = self
            .output
            .len()
            .checked_add(value.len())
            .filter(|length| *length <= self.limit)
            .ok_or_else(|| resource("output-bytes"))?;
        self.output
            .try_reserve_exact(length - self.output.len())
            .map_err(|_| resource("output-bytes"))?;
        self.output.push_str(value);
        Ok(())
    }
}

impl std::fmt::Write for BoundedJsonWriter {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        self.push(value).map_err(|_| std::fmt::Error)
    }
}

impl io::Write for BoundedJsonWriter {
    fn write(&mut self, value: &[u8]) -> io::Result<usize> {
        let value = std::str::from_utf8(value)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        self.push(value)
            .map_err(|_| io::Error::other("output byte limit exceeded"))?;
        Ok(value.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn write_jq_number(writer: &mut BoundedJsonWriter, value: &Number) -> Result<(), VmError> {
    let literal = value.to_string();
    let Some(exponent) = literal.find(['e', 'E']) else {
        return writer.push(&literal);
    };
    let sign = exponent + 1;
    if literal
        .as_bytes()
        .get(sign)
        .is_some_and(|byte| *byte == b'+' || *byte == b'-')
    {
        return writer.push(&literal);
    }
    writer.push(&literal[..sign])?;
    writer.push("+")?;
    writer.push(&literal[sign..])
}

enum JsonFrame<'a> {
    Value(&'a Value),
    Array(&'a [Value], usize),
    Object(&'a Object, usize),
}

fn bounded_json(value: &Value, output_limit: usize) -> Result<String, VmError> {
    let mut writer = BoundedJsonWriter::new(output_limit);
    let mut frames = vec![JsonFrame::Value(value)];
    while let Some(frame) = frames.pop() {
        match frame {
            JsonFrame::Value(Value::Null) => writer.push("null")?,
            JsonFrame::Value(Value::Bool(value)) => {
                writer.push(if *value { "true" } else { "false" })?;
            }
            JsonFrame::Value(Value::Number(value)) => write_jq_number(&mut writer, value)?,
            JsonFrame::Value(Value::String(value)) => {
                serde_json::to_writer(&mut writer, value.as_ref())
                    .map_err(|_| resource("output-bytes"))?;
            }
            JsonFrame::Value(Value::Array(values)) => {
                writer.push("[")?;
                frames.push(JsonFrame::Array(values, 0));
            }
            JsonFrame::Value(Value::Object(values)) => {
                writer.push("{")?;
                frames.push(JsonFrame::Object(values, 0));
            }
            JsonFrame::Array(values, next) => {
                if next == values.len() {
                    writer.push("]")?;
                } else {
                    if next > 0 {
                        writer.push(",")?;
                    }
                    frames.push(JsonFrame::Array(values, next + 1));
                    frames.push(JsonFrame::Value(&values[next]));
                }
            }
            JsonFrame::Object(values, next) => {
                if next == values.len() {
                    writer.push("}")?;
                } else {
                    if next > 0 {
                        writer.push(",")?;
                    }
                    let Some((key, value)) = values.get_index(next) else {
                        return Err(invalid("object entry missing during interpolation"));
                    };
                    serde_json::to_writer(&mut writer, key.as_ref())
                        .map_err(|_| resource("output-bytes"))?;
                    writer.push(":")?;
                    frames.push(JsonFrame::Object(values, next + 1));
                    frames.push(JsonFrame::Value(value));
                }
            }
        }
    }
    Ok(writer.output)
}

fn module_metadata(bytecode: &Bytecode, input: &Value) -> Result<Value, VmError> {
    let Value::String(requested) = input else {
        return Err(type_error("modulemeta", input));
    };
    bytecode
        .modules()
        .iter()
        .find(|module| module.name == requested.as_ref())
        .map(|module| module.metadata.clone())
        .ok_or_else(|| runtime(format!("module {requested} was not loaded")))
}

struct Evaluator<'a> {
    bytecode: &'a Bytecode,
    limits: VmLimits,
    observations: Cell<VmObservations>,
    cancellation: Option<&'a AtomicBool>,
    stop: &'a AtomicBool,
}

impl Evaluator<'_> {
    fn cancelled(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
            || self
                .cancellation
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
    }

    fn enter(&self, depth: usize) -> Result<(), VmError> {
        if self.cancelled() {
            return Err(VmError::Interrupted);
        }
        if depth >= self.limits.call_stack {
            return Err(resource("call-stack"));
        }
        let mut observations = self.observations.get();
        if observations.steps >= self.limits.steps {
            return Err(resource("vm-steps"));
        }
        observations.steps += 1;
        observations.call_stack_high_water = observations.call_stack_high_water.max(depth + 1);
        self.observations.set(observations);
        Ok(())
    }

    #[allow(
        clippy::too_many_lines,
        reason = "streaming operation dispatch mirrors the exhaustive evaluator"
    )]
    fn emit_node(
        &self,
        node: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let Some(instruction) = self.bytecode.instructions().get(node as usize) else {
            return emit(Err(invalid("tree instruction missing after validation")));
        };
        let operation = instruction.operation.clone();
        match operation {
            Operation::RecursiveDescent => self.emit_recursive(input, depth, emit),
            Operation::Interpolation(segments) => {
                self.emit_interpolation(&Arc::from(segments), input, environment, depth, emit)
            }
            Operation::AccessField { base, key } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                let key = match self.string(key) {
                    Ok(key) => Arc::clone(key),
                    Err(error) => return emit(Err(error)),
                };
                self.emit_node(base, input, environment, depth + 1, &mut |result| {
                    emit(result.and_then(|value| access_field(&value, &key)))
                })
            }
            Operation::AccessIndex { base, index } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(base, input, environment, depth + 1, &mut |base| {
                    let Ok(base) = base else {
                        return emit(base);
                    };
                    self.emit_node(index, &base, environment, depth + 1, &mut |index| {
                        emit(index.and_then(|index| access_index(&base, &index)))
                    })
                })
            }
            Operation::Iterate(base) => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(
                    base,
                    input,
                    environment,
                    depth + 1,
                    &mut |base| match base {
                        Ok(Value::Array(values)) => {
                            for value in values.iter().cloned() {
                                if !emit(Ok(value)) {
                                    return false;
                                }
                            }
                            true
                        }
                        Ok(Value::Object(values)) => {
                            for value in values.values().cloned() {
                                if !emit(Ok(value)) {
                                    return false;
                                }
                            }
                            true
                        }
                        Ok(value) => emit(Err(type_error("iterate", &value))),
                        Err(error) => emit(Err(error)),
                    },
                )
            }
            Operation::Optional(child) => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(child, input, environment, depth + 1, &mut |result| {
                    result.is_err() || emit(result)
                })
            }
            Operation::Pipe { left, right } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(left, input, environment, depth + 1, &mut |left| {
                    let Ok(left) = left else {
                        return emit(left);
                    };
                    self.emit_node(right, &left, environment, depth + 1, emit)
                })
            }
            Operation::Comma { left, right } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                let mut failed = false;
                let keep_going =
                    self.emit_node(left, input, environment, depth + 1, &mut |result| {
                        failed |= result.is_err();
                        emit(result)
                    });
                keep_going && !failed && self.emit_node(right, input, environment, depth + 1, emit)
            }
            Operation::Unary { operator, child } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(child, input, environment, depth + 1, &mut |result| {
                    emit(result.and_then(|value| unary(operator, &value)))
                })
            }
            Operation::Binary {
                operator,
                left,
                right,
            } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                if operator == BinaryOperator::Alternative {
                    let mut accepted = false;
                    let keep_going =
                        self.emit_node(left, input, environment, depth + 1, &mut |result| {
                            match result {
                                Ok(value) if value.is_truthy() => {
                                    accepted = true;
                                    emit(Ok(value))
                                }
                                Ok(_) | Err(_) => true,
                            }
                        });
                    return keep_going
                        && (accepted
                            || self.emit_node(right, input, environment, depth + 1, emit));
                }
                self.emit_node(left, input, environment, depth + 1, &mut |left| {
                    let Ok(left) = left else {
                        return emit(left);
                    };
                    if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
                        let left_truthy = left.is_truthy();
                        if (operator == BinaryOperator::And && !left_truthy)
                            || (operator == BinaryOperator::Or && left_truthy)
                        {
                            return emit(Ok(Value::Bool(operator == BinaryOperator::Or)));
                        }
                        return self.emit_node(
                            right,
                            input,
                            environment,
                            depth + 1,
                            &mut |right| emit(right.map(|value| Value::Bool(value.is_truthy()))),
                        );
                    }
                    self.emit_node(
                        right,
                        input,
                        environment,
                        depth + 1,
                        &mut |right| match right {
                            Ok(right) => emit(binary_value(operator, &left, &right)),
                            Err(error) => emit(Err(error)),
                        },
                    )
                })
            }
            Operation::Conditional {
                branches,
                alternative,
            } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                for (condition, body) in branches {
                    let conditions = self.node(condition, input, environment, depth + 1);
                    if let Some(error) = first_error(&conditions) {
                        return emit(Err(error));
                    }
                    if conditions
                        .iter()
                        .any(|value| value.as_ref().is_ok_and(Value::is_truthy))
                    {
                        return self.emit_node(body, input, environment, depth + 1, emit);
                    }
                }
                self.emit_node(alternative, input, environment, depth + 1, emit)
            }
            Operation::Bind { value, name, body } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                let name = match self.string(name) {
                    Ok(name) => Arc::clone(name),
                    Err(error) => return emit(Err(error)),
                };
                self.emit_node(value, input, environment, depth + 1, &mut |value| {
                    let Ok(value) = value else {
                        return emit(value);
                    };
                    let mut nested = environment.clone();
                    nested.insert(Arc::clone(&name), value);
                    self.emit_node(body, input, &nested, depth + 1, emit)
                })
            }
            Operation::Reduce {
                generator,
                name,
                initial,
                update,
            } => self.emit_fold(
                generator,
                name,
                initial,
                update,
                None,
                input,
                environment,
                depth,
                emit,
            ),
            Operation::Foreach {
                generator,
                name,
                initial,
                update,
                extract,
            } => self.emit_fold(
                generator,
                name,
                initial,
                update,
                Some(extract),
                input,
                environment,
                depth,
                emit,
            ),
            Operation::TryCatch { expression, catch } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(
                    expression,
                    input,
                    environment,
                    depth + 1,
                    &mut |result| match result {
                        Ok(value) => emit(Ok(value)),
                        Err(error) => catch.is_none_or(|catch| {
                            self.emit_node(
                                catch,
                                &Value::string(catch_value(&error)),
                                environment,
                                depth + 1,
                                emit,
                            )
                        }),
                    },
                )
            }
            Operation::Call { name, arguments } => {
                let name = match self.string(name) {
                    Ok(name) => Arc::clone(name),
                    Err(error) => return emit(Err(error)),
                };
                match name.as_ref() {
                    "empty" => self
                        .enter(depth)
                        .map_or_else(|error| emit(Err(error)), |()| true),
                    "select" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        let Some(argument) = arguments.first() else {
                            return emit(Err(invalid("select argument missing")));
                        };
                        self.emit_node(*argument, input, environment, depth + 1, &mut |selected| {
                            match selected {
                                Ok(value) if value.is_truthy() => emit(Ok(input.clone())),
                                Ok(_) => true,
                                Err(error) => emit(Err(error)),
                            }
                        })
                    }
                    "range" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        self.emit_range(&arguments, input, environment, depth + 1, emit)
                    }
                    _ => self
                        .node(node, input, environment, depth)
                        .into_iter()
                        .all(emit),
                }
            }
            _ => self
                .node(node, input, environment, depth)
                .into_iter()
                .all(emit),
        }
    }

    fn emit_recursive(
        &self,
        input: &Value,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let mut cursor = TraversalCursor::new(input.clone());
        loop {
            if cursor.depth() > self.limits.path_stack {
                return emit(Err(resource("path-stack")));
            }
            let mut observations = self.observations.get();
            observations.path_stack_high_water =
                observations.path_stack_high_water.max(cursor.depth());
            self.observations.set(observations);
            let value = match cursor.next(self.limits.path_stack) {
                Ok(Some(value)) => value,
                Ok(None) => return true,
                Err(error) => return emit(Err(error)),
            };
            if let Err(error) = self.enter(depth) {
                return emit(Err(error));
            }
            if !emit(Ok(value)) {
                return false;
            }
        }
    }

    fn emit_interpolation(
        &self,
        segments: &Arc<[InterpolationOperand]>,
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let expression_count = segments
            .iter()
            .filter(|segment| matches!(segment, InterpolationOperand::Expression(_)))
            .count();
        if expression_count >= self.limits.call_stack {
            return emit(Err(resource("call-stack")));
        }
        let pieces = match interpolation_pieces(segments, self.bytecode, self.limits.output_bytes) {
            Ok(pieces) => pieces,
            Err(error) => return emit(Err(error)),
        };

        let mut pending = vec![InterpolationWork::Expand {
            next: segments.len(),
            pieces,
        }];
        while let Some(work) = pending.pop() {
            let (mut next, pieces) = match work {
                InterpolationWork::Expand { next, pieces } => (next, pieces),
                InterpolationWork::Error(error) => return emit(Err(error)),
            };

            let mut expanded = false;
            while next > 0 {
                next -= 1;
                let InterpolationOperand::Expression(node) = &segments[next] else {
                    continue;
                };
                let slot = next;
                let available = self.limits.fork_stack.saturating_sub(pending.len());
                let mut outcomes = Vec::new();
                let mut overflow = false;
                self.emit_node(*node, input, environment, depth + 1, &mut |result| {
                    if outcomes.len() >= available {
                        overflow = true;
                        return false;
                    }
                    let failed = result.is_err();
                    outcomes.push(result);
                    !failed
                });
                if overflow {
                    return emit(Err(resource("fork-stack")));
                }
                for outcome in outcomes.into_iter().rev() {
                    match outcome {
                        Ok(value) => {
                            let mut nested = pieces.clone();
                            let piece = interpolation_remaining(&nested, self.limits.output_bytes)
                                .and_then(|remaining| interpolation_value(&value, remaining));
                            match piece {
                                Ok(piece) => {
                                    nested[slot] = Some(piece);
                                    pending.push(InterpolationWork::Expand {
                                        next,
                                        pieces: nested,
                                    });
                                }
                                Err(error) => pending.push(InterpolationWork::Error(error)),
                            }
                        }
                        Err(error) => pending.push(InterpolationWork::Error(error)),
                    }
                }
                let mut observations = self.observations.get();
                observations.fork_stack_high_water =
                    observations.fork_stack_high_water.max(pending.len());
                self.observations.set(observations);
                expanded = true;
                break;
            }
            if expanded {
                continue;
            }

            let capacity = match interpolation_capacity(&pieces, self.limits.output_bytes) {
                Ok(capacity) => capacity,
                Err(error) => return emit(Err(error)),
            };
            let mut output = String::with_capacity(capacity);
            for piece in pieces {
                let Some(piece) = piece else {
                    return emit(Err(invalid(
                        "interpolation segment missing after evaluation",
                    )));
                };
                output.push_str(&piece);
            }
            if !emit(Ok(Value::string(output))) {
                return false;
            }
        }
        true
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "fold bytecode carries four bodies plus the shared evaluator context"
    )]
    fn emit_fold(
        &self,
        generator: u32,
        name: u32,
        initial: u32,
        update: u32,
        extract: Option<u32>,
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if let Err(error) = self.enter_fold_frame(depth) {
            return emit(Err(error));
        }
        let name = match self.string(name) {
            Ok(name) => Arc::clone(name),
            Err(error) => return emit(Err(error)),
        };
        self.emit_node(
            initial,
            input,
            environment,
            depth + 1,
            &mut |initial_result| {
                let mut accumulator = match initial_result {
                    Ok(value) => value,
                    Err(error) => {
                        let _ = emit(Err(error));
                        return false;
                    }
                };
                let mut fold_failed = false;
                let keep_going =
                    self.emit_node(generator, input, environment, depth + 1, &mut |generated| {
                        let item = match generated {
                            Ok(value) => value,
                            Err(error) => {
                                fold_failed = true;
                                let _ = emit(Err(error));
                                return false;
                            }
                        };
                        if let Err(error) = self.enter(depth + 1) {
                            fold_failed = true;
                            let _ = emit(Err(error));
                            return false;
                        }
                        let mut nested = environment.clone();
                        nested.insert(Arc::clone(&name), item);
                        let mut next_accumulator = Value::Null;
                        let mut update_failed = false;
                        let update_kept_going = self.emit_node(
                            update,
                            &accumulator,
                            &nested,
                            depth + 1,
                            &mut |updated| {
                                let updated = match updated {
                                    Ok(value) => value,
                                    Err(error) => {
                                        update_failed = true;
                                        let _ = emit(Err(error));
                                        return false;
                                    }
                                };
                                next_accumulator = updated.clone();
                                let Some(extract) = extract else {
                                    return true;
                                };
                                let mut extract_failed = false;
                                let extracted = self.emit_node(
                                    extract,
                                    &updated,
                                    &nested,
                                    depth + 1,
                                    &mut |result| {
                                        extract_failed |= result.is_err();
                                        let accepted = emit(result);
                                        accepted && !extract_failed
                                    },
                                );
                                if extract_failed {
                                    update_failed = true;
                                }
                                extracted && !extract_failed
                            },
                        );
                        accumulator = next_accumulator;
                        if update_failed {
                            fold_failed = true;
                        }
                        update_kept_going && !update_failed
                    });
                if !keep_going || fold_failed {
                    return false;
                }
                extract.is_some() || emit(Ok(accumulator))
            },
        )
    }

    fn enter_fold_frame(&self, depth: usize) -> Result<(), VmError> {
        self.enter(depth)?;
        let frames = depth.saturating_add(1);
        if frames > self.limits.value_stack {
            return Err(resource("value-stack"));
        }
        let mut observations = self.observations.get();
        observations.value_stack_high_water = observations.value_stack_high_water.max(frames);
        self.observations.set(observations);
        Ok(())
    }

    fn emit_range(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let mut numbers = Vec::new();
        for argument in arguments {
            match first_value(self.node(*argument, input, environment, depth)) {
                Ok(Value::Number(number)) => numbers.push(number.as_f64()),
                Ok(value) => return emit(Err(type_error("range", &value))),
                Err(error) => return emit(Err(error)),
            }
        }
        let (mut current, end, step) = match numbers.as_slice() {
            [end] => (0.0, *end, 1.0),
            [start, end] => (*start, *end, 1.0),
            [start, end, step] => (*start, *end, *step),
            _ => return emit(Err(invalid("range arity"))),
        };
        if step == 0.0 {
            return emit(Err(runtime("range step cannot be zero".to_owned())));
        }
        while (step > 0.0 && current < end) || (step < 0.0 && current > end) {
            if let Err(error) = self.enter(depth) {
                return emit(Err(error));
            }
            let Some(value) = number_value(current).pop() else {
                return emit(Err(invalid("range produced no numeric value")));
            };
            if !emit(value) {
                return false;
            }
            let next = current + step;
            if next.to_bits() == current.to_bits() {
                return emit(Err(resource("vm-steps")));
            }
            current = next;
        }
        true
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the bytecode operation dispatch is intentionally exhaustive"
    )]
    fn node(&self, node: u32, input: &Value, environment: &Environment, depth: usize) -> Outcomes {
        if let Err(error) = self.enter(depth) {
            return one_error(error);
        }
        let Some(instruction) = self.bytecode.instructions().get(node as usize) else {
            return one_error(invalid("tree instruction missing after validation"));
        };
        match &instruction.operation {
            Operation::Identity => vec![Ok(input.clone())],
            Operation::Literal(index) => self
                .bytecode
                .constants()
                .get(*index as usize)
                .cloned()
                .map_or_else(
                    || one_error(invalid("literal missing after validation")),
                    |value| vec![Ok(value)],
                ),
            Operation::Variable(index) => match self.string(*index) {
                Ok(name) => environment.get(name).cloned().map_or_else(
                    || one_error(runtime(format!("variable ${name} has no value"))),
                    |value| vec![Ok(value)],
                ),
                Err(error) => one_error(error),
            },
            Operation::Empty => Vec::new(),
            Operation::RecursiveDescent => {
                let mut output = Vec::new();
                self.emit_recursive(input, depth, &mut |result| {
                    output.push(result);
                    true
                });
                output
            }
            Operation::Interpolation(segments) => {
                let mut output = Vec::new();
                self.emit_interpolation(
                    &Arc::from(segments.clone()),
                    input,
                    environment,
                    depth,
                    &mut |result| {
                        output.push(result);
                        true
                    },
                );
                output
            }
            Operation::AccessField { base, key } => {
                let key = match self.string(*key) {
                    Ok(key) => Arc::clone(key),
                    Err(error) => return one_error(error),
                };
                let bases = self.node(*base, input, environment, depth + 1);
                map_outcomes(bases, |value| access_field(value, &key))
            }
            Operation::AccessIndex { base, index } => {
                let bases = self.node(*base, input, environment, depth + 1);
                let mut output = Vec::new();
                for base in bases {
                    match base {
                        Ok(base) => {
                            for index in self.node(*index, &base, environment, depth + 1) {
                                output.push(index.and_then(|index| access_index(&base, &index)));
                            }
                        }
                        Err(error) => output.push(Err(error)),
                    }
                }
                output
            }
            Operation::Slice { base, start, end } => {
                let bases = self.node(*base, input, environment, depth + 1);
                let mut output = Vec::new();
                for base in bases {
                    let Ok(base) = base else {
                        output.push(base);
                        continue;
                    };
                    let starts = self.bound(*start, &base, environment, depth + 1);
                    let ends = self.bound(*end, &base, environment, depth + 1);
                    for start in &starts {
                        for end in &ends {
                            output.push(match (start, end) {
                                (Ok(start), Ok(end)) => slice(&base, *start, *end),
                                (Err(error), _) | (_, Err(error)) => Err(error.clone()),
                            });
                        }
                    }
                }
                output
            }
            Operation::Iterate(base) => {
                let bases = self.node(*base, input, environment, depth + 1);
                let mut output = Vec::new();
                for base in bases {
                    match base {
                        Ok(Value::Array(values)) => output.extend(values.iter().cloned().map(Ok)),
                        Ok(Value::Object(values)) => {
                            output.extend(values.values().cloned().map(Ok));
                        }
                        Ok(value) => output.push(Err(type_error("iterate", &value))),
                        Err(error) => output.push(Err(error)),
                    }
                }
                output
            }
            Operation::Optional(child) => self
                .node(*child, input, environment, depth + 1)
                .into_iter()
                .flatten()
                .map(Ok)
                .collect(),
            Operation::Pipe { left, right } => {
                let mut output = Vec::new();
                for value in self.node(*left, input, environment, depth + 1) {
                    match value {
                        Ok(value) => {
                            output.extend(self.node(*right, &value, environment, depth + 1));
                        }
                        Err(error) => output.push(Err(error)),
                    }
                }
                output
            }
            Operation::Comma { left, right } => {
                let mut output = self.node(*left, input, environment, depth + 1);
                if !ends_in_error(&output) {
                    output.extend(self.node(*right, input, environment, depth + 1));
                }
                output
            }
            Operation::Array(child) => {
                let results = self.node(*child, input, environment, depth + 1);
                match collect_values(results) {
                    Ok(values) => vec![Ok(Value::array(values))],
                    Err(error) => one_error(error),
                }
            }
            Operation::Object(entries) => {
                let mut candidates = vec![Object::new()];
                for entry in entries {
                    let keys = match &entry.key {
                        KeyOperand::Static(index) => match self.string(*index) {
                            Ok(key) => vec![Ok(Arc::clone(key))],
                            Err(error) => return one_error(error),
                        },
                        KeyOperand::Computed(node) => self
                            .node(*node, input, environment, depth + 1)
                            .into_iter()
                            .map(|value| value.and_then(computed_key))
                            .collect(),
                    };
                    let values = self.node(entry.value, input, environment, depth + 1);
                    let mut next = Vec::new();
                    for candidate in &candidates {
                        for key in &keys {
                            for value in &values {
                                match (key, value) {
                                    (Ok(key), Ok(value)) => {
                                        let mut object = candidate.clone();
                                        object.insert(Arc::clone(key), value.clone());
                                        next.push(object);
                                    }
                                    (Err(error), _) | (_, Err(error)) => {
                                        return one_error(error.clone());
                                    }
                                }
                            }
                        }
                    }
                    candidates = next;
                }
                candidates.into_iter().map(Value::object).map(Ok).collect()
            }
            Operation::Unary { operator, child } => {
                let values = self.node(*child, input, environment, depth + 1);
                map_outcomes(values, |value| unary(*operator, value))
            }
            Operation::Binary {
                operator,
                left,
                right,
            } => self.binary(*operator, *left, *right, input, environment, depth + 1),
            Operation::Conditional {
                branches,
                alternative,
            } => {
                for (condition, body) in branches {
                    let conditions = self.node(*condition, input, environment, depth + 1);
                    if let Some(error) = first_error(&conditions) {
                        return one_error(error);
                    }
                    if conditions
                        .iter()
                        .any(|value| value.as_ref().is_ok_and(Value::is_truthy))
                    {
                        return self.node(*body, input, environment, depth + 1);
                    }
                }
                self.node(*alternative, input, environment, depth + 1)
            }
            Operation::Bind { value, name, body } => {
                let name = match self.string(*name) {
                    Ok(name) => Arc::clone(name),
                    Err(error) => return one_error(error),
                };
                let mut output = Vec::new();
                for value in self.node(*value, input, environment, depth + 1) {
                    match value {
                        Ok(value) => {
                            let mut nested = environment.clone();
                            nested.insert(Arc::clone(&name), value);
                            output.extend(self.node(*body, input, &nested, depth + 1));
                        }
                        Err(error) => output.push(Err(error)),
                    }
                }
                output
            }
            Operation::Reduce {
                generator,
                name,
                initial,
                update,
            } => {
                let mut output = Vec::new();
                self.emit_fold(
                    *generator,
                    *name,
                    *initial,
                    *update,
                    None,
                    input,
                    environment,
                    depth,
                    &mut |result| {
                        output.push(result);
                        true
                    },
                );
                output
            }
            Operation::Foreach {
                generator,
                name,
                initial,
                update,
                extract,
            } => {
                let mut output = Vec::new();
                self.emit_fold(
                    *generator,
                    *name,
                    *initial,
                    *update,
                    Some(*extract),
                    input,
                    environment,
                    depth,
                    &mut |result| {
                        output.push(result);
                        true
                    },
                );
                output
            }
            Operation::Call { name, arguments } => match self.string(*name).cloned() {
                Ok(name) => self.call(&name, arguments, input, environment, depth + 1),
                Err(error) => one_error(error),
            },
            Operation::TryCatch { expression, catch } => {
                let values = self.node(*expression, input, environment, depth + 1);
                let mut output = Vec::new();
                for value in values {
                    match value {
                        Ok(value) => output.push(Ok(value)),
                        Err(error) => {
                            if let Some(catch) = catch {
                                output.extend(self.node(
                                    *catch,
                                    &Value::string(catch_value(&error)),
                                    environment,
                                    depth + 1,
                                ));
                            }
                        }
                    }
                }
                output
            }
            Operation::Assignment {
                operator,
                path,
                value,
            } => self.assignment(*operator, *path, *value, input, environment, depth + 1),
            operation => one_error(VmError::Unsupported {
                operation: format!("{operation:?}").into(),
            }),
        }
    }

    fn string(&self, index: u32) -> Result<&Arc<str>, VmError> {
        self.bytecode
            .string(index)
            .ok_or_else(|| invalid("string missing after validation"))
    }

    fn bound(
        &self,
        node: Option<u32>,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Vec<Result<Option<i64>, VmError>> {
        match node {
            None => vec![Ok(None)],
            Some(node) => self
                .node(node, input, environment, depth)
                .into_iter()
                .map(|value| {
                    value.and_then(|value| match value {
                        Value::Null => Ok(None),
                        Value::Number(number) => number.exact_index().map(Some).ok_or_else(|| {
                            runtime("slice bound must be an exact integer".to_owned())
                        }),
                        value => Err(type_error("slice", &value)),
                    })
                })
                .collect(),
        }
    }

    fn binary(
        &self,
        operator: BinaryOperator,
        left: u32,
        right: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let left_values = self.node(left, input, environment, depth);
        if operator == BinaryOperator::Alternative {
            let accepted = left_values
                .iter()
                .filter_map(|value| value.as_ref().ok())
                .filter(|value| value.is_truthy())
                .cloned()
                .map(Ok)
                .collect::<Vec<_>>();
            if !accepted.is_empty() {
                return accepted;
            }
            return self.node(right, input, environment, depth);
        }
        if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
            let mut output = Vec::new();
            for left in left_values {
                let Ok(left) = left else {
                    output.push(left);
                    continue;
                };
                let left_truthy = left.is_truthy();
                if (operator == BinaryOperator::And && !left_truthy)
                    || (operator == BinaryOperator::Or && left_truthy)
                {
                    output.push(Ok(Value::Bool(operator == BinaryOperator::Or)));
                } else {
                    output.extend(
                        self.node(right, input, environment, depth)
                            .into_iter()
                            .map(|value| value.map(|value| Value::Bool(value.is_truthy()))),
                    );
                }
            }
            return output;
        }
        let right_values = self.node(right, input, environment, depth);
        let mut output = Vec::new();
        for left in &left_values {
            for right in &right_values {
                output.push(match (left, right) {
                    (Ok(left), Ok(right)) => binary_value(operator, left, right),
                    (Err(error), _) | (_, Err(error)) => Err(error.clone()),
                });
            }
        }
        output
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the versioned built-in registry is executed in one exhaustive dispatch"
    )]
    fn call(
        &self,
        name: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        match name {
            "empty" => Vec::new(),
            "modulemeta" => vec![module_metadata(self.bytecode, input)],
            "type" => vec![Ok(Value::string(type_name(input)))],
            "length" => vec![length(input)],
            "utf8bytelength" => match input {
                Value::String(value) => vec![number_usize(value.len())],
                value => one_error(type_error("utf8bytelength", value)),
            },
            "keys" | "keys_unsorted" => vec![keys(input, name == "keys")],
            "has" => self.argument_values(arguments, input, environment, depth, 0, |key| {
                has(input, key)
            }),
            "in" => self.argument_values(arguments, input, environment, depth, 0, |container| {
                has(container, input)
            }),
            "arrays" => selector(input, matches!(input, Value::Array(_))),
            "booleans" => selector(input, matches!(input, Value::Bool(_))),
            "iterables" => selector(input, matches!(input, Value::Array(_) | Value::Object(_))),
            "nulls" => selector(input, matches!(input, Value::Null)),
            "numbers" => selector(input, matches!(input, Value::Number(_))),
            "objects" => selector(input, matches!(input, Value::Object(_))),
            "scalars" => selector(input, !matches!(input, Value::Array(_) | Value::Object(_))),
            "strings" => selector(input, matches!(input, Value::String(_))),
            "values" => selector(input, !matches!(input, Value::Null)),
            "select" => {
                let Some(argument) = arguments.first() else {
                    return one_error(invalid("select argument missing"));
                };
                let mut output = Vec::new();
                for selected in self.node(*argument, input, environment, depth) {
                    match selected {
                        Ok(value) if value.is_truthy() => output.push(Ok(input.clone())),
                        Ok(_) => {}
                        Err(error) => output.push(Err(error)),
                    }
                }
                output
            }
            "map" => self.map(arguments, input, environment, depth, false),
            "map_values" => self.map(arguments, input, environment, depth, true),
            "tonumber" => match input {
                Value::Number(_) => vec![Ok(input.clone())],
                Value::String(value) => {
                    Number::parse(value).map(Value::Number).map(Ok).map_or_else(
                        |error| one_error(runtime(error.to_string())),
                        |value| vec![value],
                    )
                }
                value => one_error(type_error("tonumber", value)),
            },
            "tostring" => {
                vec![interpolation_value(input, self.limits.output_bytes).map(Value::String)]
            }
            "range" => self.range(arguments, input, environment, depth),
            "add" => fold_values(input, None, binary_add),
            "min" => extrema(input, false),
            "max" => extrema(input, true),
            "sort" => sort_values(input),
            "sort_by" => self.sort_by(arguments, input, environment, depth, false),
            "unique" => unique_values(input),
            "unique_by" => self.sort_by(arguments, input, environment, depth, true),
            "reverse" => reverse(input),
            "flatten" => {
                let requested = if arguments.is_empty() {
                    None
                } else {
                    match first_value(self.node(arguments[0], input, environment, depth)) {
                        Ok(Value::Number(number)) => number.exact_index(),
                        Ok(value) => return one_error(type_error("flatten", &value)),
                        Err(error) => return one_error(error),
                    }
                };
                match flatten(input, requested) {
                    Ok(value) => vec![Ok(value)],
                    Err(error) => one_error(error),
                }
            }
            "error" => {
                let message = if let Some(argument) = arguments.first() {
                    match first_value(self.node(*argument, input, environment, depth)) {
                        Ok(Value::String(value)) => value,
                        Ok(value) => Arc::from(value.to_string()),
                        Err(error) => return one_error(error),
                    }
                } else {
                    Arc::from(input.to_string())
                };
                one_error(VmError::Runtime { message })
            }
            "test" | "match" | "capture" | "scan" | "split" | "splits" | "sub" | "gsub" => {
                self.regex_call(name, arguments, input, environment, depth)
            }
            "fromdate" | "fromdateiso8601" => vec![stdlib::fromdate_iso8601(input)],
            "todate" | "todateiso8601" => {
                vec![stdlib::todate_iso8601(input, self.limits.output_bytes)]
            }
            "gmtime" => vec![stdlib::gmtime(input)],
            "localtime" => vec![stdlib::localtime(input, ambient_platform(environment))],
            "mktime" => vec![stdlib::mktime(input)],
            "strptime" => {
                self.argument_values(
                    arguments,
                    input,
                    environment,
                    depth,
                    0,
                    |format| match format {
                        Value::String(format) => stdlib::strptime(input, format),
                        value => Err(type_error("strptime", value)),
                    },
                )
            }
            "strftime" => {
                self.argument_values(
                    arguments,
                    input,
                    environment,
                    depth,
                    0,
                    |format| match format {
                        Value::String(format) => {
                            stdlib::strftime(input, format, self.limits.output_bytes)
                        }
                        value => Err(type_error("strftime", value)),
                    },
                )
            }
            "strflocaltime" => self.argument_values(
                arguments,
                input,
                environment,
                depth,
                0,
                |format| match format {
                    Value::String(format) => stdlib::strflocaltime(
                        input,
                        format,
                        ambient_platform(environment),
                        self.limits.output_bytes,
                    ),
                    value => Err(type_error("strflocaltime", value)),
                },
            ),
            "now" => vec![stdlib::now(ambient_platform(environment))],
            "env" => vec![ambient_environment(environment)],
            "input_filename" => vec![ambient_value(environment, INPUT_FILENAME, "input_filename")],
            "input_line_number" => vec![ambient_value(
                environment,
                INPUT_LINE_NUMBER,
                "input_line_number",
            )],
            _ => one_error(VmError::Unsupported {
                operation: format!("builtin {name}").into(),
            }),
        }
    }

    fn regex_call(
        &self,
        name: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Value::String(input) = input else {
            return one_error(type_error(name, input));
        };
        let Some(pattern_node) = arguments.first() else {
            return one_error(invalid("regex pattern argument missing"));
        };
        let pattern_values = self.node(
            *pattern_node,
            &Value::String(Arc::clone(input)),
            environment,
            depth,
        );
        let flags_index = usize::from(matches!(name, "sub" | "gsub")) + 1;
        let flag_values = if let Some(flags) = arguments.get(flags_index) {
            self.node(
                *flags,
                &Value::String(Arc::clone(input)),
                environment,
                depth,
            )
        } else {
            vec![Ok(Value::string(""))]
        };
        let mut output = Vec::new();
        for pattern in &pattern_values {
            let pattern = match pattern {
                Ok(Value::String(pattern)) => pattern,
                Ok(value) => {
                    output.push(Err(type_error(name, value)));
                    continue;
                }
                Err(error) => {
                    output.push(Err(error.clone()));
                    continue;
                }
            };
            for flags in &flag_values {
                let flags = match flags {
                    Ok(Value::String(flags)) => flags,
                    Ok(value) => {
                        output.push(Err(type_error(name, value)));
                        continue;
                    }
                    Err(error) => {
                        output.push(Err(error.clone()));
                        continue;
                    }
                };
                let result = match name {
                    "test" => stdlib::regex_test(input, pattern, flags, self.limits)
                        .map(|value| vec![value]),
                    "match" => stdlib::regex_matches(input, pattern, flags, self.limits),
                    "capture" => stdlib::regex_capture(input, pattern, flags, self.limits),
                    "scan" => stdlib::regex_scan(input, pattern, flags, self.limits),
                    "split" => stdlib::regex_split(input, pattern, flags, false, self.limits),
                    "splits" => stdlib::regex_split(input, pattern, flags, true, self.limits),
                    "sub" | "gsub" => {
                        let Some(replacement_node) = arguments.get(1) else {
                            return one_error(invalid("regex replacement argument missing"));
                        };
                        stdlib::regex_substitute(
                            input,
                            pattern,
                            flags,
                            name == "gsub",
                            self.limits,
                            |context| match first_value(self.node(
                                *replacement_node,
                                context,
                                environment,
                                depth,
                            )) {
                                Ok(Value::String(value)) => Ok(value),
                                Ok(value) => Err(type_error(name, &value)),
                                Err(error) => Err(error),
                            },
                        )
                        .map(|value| vec![value])
                    }
                    _ => unreachable!("regex dispatch is exhaustive"),
                };
                match result {
                    Ok(values) => output.extend(values.into_iter().map(Ok)),
                    Err(error) => output.push(Err(error)),
                }
            }
        }
        output
    }

    fn argument_values(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        index: usize,
        apply: impl Fn(&Value) -> Result<Value, VmError>,
    ) -> Outcomes {
        let Some(argument) = arguments.get(index) else {
            return one_error(invalid("builtin argument missing"));
        };
        self.node(*argument, input, environment, depth)
            .into_iter()
            .map(|value| value.and_then(|value| apply(&value)))
            .collect()
    }

    fn map(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        values_only: bool,
    ) -> Outcomes {
        let Some(argument) = arguments.first() else {
            return one_error(invalid("map argument missing"));
        };
        match input {
            Value::Array(values) => {
                let mut mapped = Vec::new();
                for value in values.iter() {
                    match collect_values(self.node(*argument, value, environment, depth)) {
                        Ok(values) => mapped.extend(values),
                        Err(error) => return one_error(error),
                    }
                }
                vec![Ok(Value::array(mapped))]
            }
            Value::Object(object) if values_only => {
                let mut mapped = Object::new();
                for (key, value) in object.iter() {
                    match first_value(self.node(*argument, value, environment, depth)) {
                        Ok(value) => {
                            mapped.insert(Arc::clone(key), value);
                        }
                        Err(error) => return one_error(error),
                    }
                }
                vec![Ok(Value::object(mapped))]
            }
            value => one_error(type_error("map", value)),
        }
    }

    fn range(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let mut numbers = Vec::new();
        for argument in arguments {
            match first_value(self.node(*argument, input, environment, depth)) {
                Ok(Value::Number(number)) => numbers.push(number.as_f64()),
                Ok(value) => return one_error(type_error("range", &value)),
                Err(error) => return one_error(error),
            }
        }
        let (mut current, end, step) = match numbers.as_slice() {
            [end] => (0.0, *end, 1.0),
            [start, end] => (*start, *end, 1.0),
            [start, end, step] => (*start, *end, *step),
            _ => return one_error(invalid("range arity")),
        };
        if step == 0.0 {
            return one_error(runtime("range step cannot be zero".to_owned()));
        }
        let mut output = Vec::new();
        while (step > 0.0 && current < end) || (step < 0.0 && current > end) {
            output.extend(number_value(current));
            current += step;
            if output.len() >= self.limits.fork_stack {
                return one_error(resource("fork-stack"));
            }
        }
        output
    }

    fn sort_by(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        unique: bool,
    ) -> Outcomes {
        let Value::Array(values) = input else {
            return one_error(type_error("sort_by", input));
        };
        let Some(argument) = arguments.first() else {
            return one_error(invalid("sort_by argument missing"));
        };
        let mut keyed = Vec::new();
        for value in values.iter() {
            match collect_values(self.node(*argument, value, environment, depth)) {
                Ok(key_values) => keyed.push((Value::array(key_values), value.clone())),
                Err(error) => return one_error(error),
            }
        }
        sort_by_cached_key(&mut keyed);
        if unique {
            keyed.dedup_by(|left, right| left.0 == right.0);
        }
        vec![Ok(Value::array(
            keyed
                .into_iter()
                .map(|(_, value)| value)
                .collect::<Vec<_>>(),
        ))]
    }

    fn assignment(
        &self,
        operator: AssignmentOperator,
        path_node: u32,
        value_node: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let paths = match self.paths(path_node, input, environment, depth) {
            Ok(paths) => paths,
            Err(error) => return one_error(error),
        };
        let mut documents = vec![input.clone()];
        for path in paths {
            let mut next = Vec::new();
            for document in documents {
                let old = path.get(&document).cloned().unwrap_or(Value::Null);
                let expression_input = if operator == AssignmentOperator::Set {
                    input
                } else {
                    &old
                };
                let replacements = self.node(value_node, expression_input, environment, depth);
                for replacement in replacements {
                    match replacement {
                        Ok(replacement) => {
                            let replacement = match update_value(operator, &old, &replacement) {
                                Ok(value) => value,
                                Err(error) => return one_error(error),
                            };
                            match replace_or_create(&document, path.components(), replacement) {
                                Ok(value) => next.push(value),
                                Err(error) => return one_error(error),
                            }
                        }
                        Err(error) => return one_error(error),
                    }
                }
            }
            documents = next;
        }
        documents.into_iter().map(Ok).collect()
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the explicit path work stack keeps traversal and bounds in one auditable loop"
    )]
    fn paths(
        &self,
        node: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Result<Vec<Path>, VmError> {
        #[derive(Clone)]
        enum Segment {
            Field(Arc<str>),
            Index(u32),
            Iterate,
        }
        struct Work {
            node: u32,
            segments: Vec<Segment>,
            optional: bool,
        }

        let mut pending = vec![Work {
            node,
            segments: Vec::new(),
            optional: false,
        }];
        let mut output = Vec::new();
        'work: while let Some(mut work) = pending.pop() {
            self.enter(depth)?;
            if work.segments.len() >= self.limits.path_stack {
                return Err(resource("path-stack"));
            }
            let mut observations = self.observations.get();
            observations.path_stack_high_water =
                observations.path_stack_high_water.max(work.segments.len());
            observations.fork_stack_high_water =
                observations.fork_stack_high_water.max(pending.len());
            self.observations.set(observations);

            let operation = self
                .bytecode
                .instructions()
                .get(work.node as usize)
                .ok_or_else(|| invalid("path instruction missing"))?
                .operation
                .clone();
            match operation {
                Operation::Identity => {
                    let mut candidates = vec![Path::root()];
                    for segment in work.segments.into_iter().rev() {
                        let mut next = Vec::new();
                        for path in candidates {
                            match segment {
                                Segment::Field(ref key) => {
                                    let mut components = path.components().to_vec();
                                    components.push(PathComponent::Key(Arc::clone(key)));
                                    next.push(Path::new(components));
                                }
                                Segment::Index(index_node) => {
                                    let base = path.get(input).unwrap_or(&Value::Null);
                                    for index in self.node(
                                        index_node,
                                        base,
                                        environment,
                                        depth.saturating_add(1),
                                    ) {
                                        let component =
                                            match index.and_then(|value| path_component(&value)) {
                                                Ok(component) => component,
                                                Err(_) if work.optional => continue 'work,
                                                Err(error) => return Err(error),
                                            };
                                        let mut components = path.components().to_vec();
                                        components.push(component);
                                        next.push(Path::new(components));
                                    }
                                }
                                Segment::Iterate => match path.get(input) {
                                    Some(Value::Array(values)) => {
                                        for index in 0..values.len() {
                                            let mut components = path.components().to_vec();
                                            components.push(PathComponent::Index(index));
                                            next.push(Path::new(components));
                                        }
                                    }
                                    Some(Value::Object(values)) => {
                                        for key in values.keys() {
                                            let mut components = path.components().to_vec();
                                            components.push(PathComponent::Key(Arc::clone(key)));
                                            next.push(Path::new(components));
                                        }
                                    }
                                    Some(_) if work.optional => continue 'work,
                                    Some(value) => {
                                        return Err(type_error("update iteration", value));
                                    }
                                    None => {}
                                },
                            }
                        }
                        candidates = next;
                    }
                    output.extend(candidates);
                }
                Operation::AccessField { base, key } => {
                    work.segments
                        .push(Segment::Field(Arc::clone(self.string(key)?)));
                    work.node = base;
                    pending.push(work);
                }
                Operation::AccessIndex { base, index } => {
                    work.segments.push(Segment::Index(index));
                    work.node = base;
                    pending.push(work);
                }
                Operation::Iterate(base) => {
                    work.segments.push(Segment::Iterate);
                    work.node = base;
                    pending.push(work);
                }
                Operation::Comma { left, right } => {
                    if pending.len().saturating_add(2) > self.limits.fork_stack {
                        return Err(resource("fork-stack"));
                    }
                    pending.push(Work {
                        node: right,
                        segments: work.segments.clone(),
                        optional: work.optional,
                    });
                    work.node = left;
                    pending.push(work);
                }
                Operation::Optional(child) => {
                    work.optional = true;
                    work.node = child;
                    pending.push(work);
                }
                _ if work.optional => {}
                _ => return Err(runtime("assignment left side is not a path".to_owned())),
            }
        }
        Ok(output)
    }
}

fn catch_value(error: &VmError) -> String {
    match error {
        VmError::Runtime { message } => message.to_string(),
        _ => error.to_string(),
    }
}

fn map_outcomes(values: Outcomes, apply: impl Fn(&Value) -> Result<Value, VmError>) -> Outcomes {
    values
        .into_iter()
        .map(|value| value.and_then(|value| apply(&value)))
        .collect()
}

fn collect_values(values: Outcomes) -> Result<Vec<Value>, VmError> {
    values.into_iter().collect()
}

fn first_value(values: Outcomes) -> Result<Value, VmError> {
    values
        .into_iter()
        .next()
        .unwrap_or_else(|| Err(runtime("filter produced no value".to_owned())))
}

fn first_error(values: &Outcomes) -> Option<VmError> {
    values
        .iter()
        .find_map(|value| value.as_ref().err().cloned())
}

fn ends_in_error(values: &Outcomes) -> bool {
    values.last().is_some_and(Result::is_err)
}

fn one_error(error: VmError) -> Outcomes {
    vec![Err(error)]
}

fn runtime(message: String) -> VmError {
    VmError::Runtime {
        message: message.into(),
    }
}

fn invalid(message: &'static str) -> VmError {
    VmError::InvalidProgram { message }
}

fn resource(resource: &'static str) -> VmError {
    VmError::Resource { resource }
}

fn ambient_platform(environment: &Environment) -> bool {
    matches!(environment.get(AMBIENT_PLATFORM), Some(Value::Bool(true)))
}

fn ambient_environment(environment: &Environment) -> Result<Value, VmError> {
    match environment.get(AMBIENT_ENVIRONMENT) {
        Some(Value::Object(values)) => Ok(Value::Object(Arc::clone(values))),
        _ => Err(runtime(
            "env requires environment access permitted by capability policy".to_owned(),
        )),
    }
}

fn ambient_value(environment: &Environment, key: &str, operation: &str) -> Result<Value, VmError> {
    if !ambient_platform(environment) {
        return Err(runtime(format!(
            "{operation} requires platform access permitted by capability policy"
        )));
    }
    environment.get(key).cloned().ok_or_else(|| {
        runtime(format!(
            "{operation} metadata is unavailable for this input mode"
        ))
    })
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn type_error(operation: &str, value: &Value) -> VmError {
    runtime(format!(
        "{operation} cannot be applied to {}",
        type_name(value)
    ))
}

fn access_field(value: &Value, key: &Arc<str>) -> Result<Value, VmError> {
    match value {
        Value::Object(object) => Ok(object.get(key).cloned().unwrap_or(Value::Null)),
        Value::Null => Ok(Value::Null),
        value => Err(type_error("field access", value)),
    }
}

fn access_index(value: &Value, index: &Value) -> Result<Value, VmError> {
    match (value, index) {
        (Value::Object(object), Value::String(key)) => {
            Ok(object.get(key).cloned().unwrap_or(Value::Null))
        }
        (Value::Array(values), Value::Number(number)) => {
            let index = number
                .exact_index()
                .ok_or_else(|| runtime("array index must be an exact integer".to_owned()))?;
            let index = normalize_index(index, values.len());
            Ok(index
                .and_then(|index| values.get(index).cloned())
                .unwrap_or(Value::Null))
        }
        (Value::Null, Value::String(_) | Value::Number(_)) => Ok(Value::Null),
        (_, Value::String(_) | Value::Number(_)) => Err(type_error("index", value)),
        _ => Err(runtime(
            "index must be a string or exact integer".to_owned(),
        )),
    }
}

fn normalize_index(index: i64, length: usize) -> Option<usize> {
    if index >= 0 {
        usize::try_from(index).ok().filter(|index| *index < length)
    } else {
        let signed_length = i64::try_from(length).ok()?;
        usize::try_from(signed_length + index)
            .ok()
            .filter(|index| *index < length)
    }
}

fn slice(value: &Value, start: Option<i64>, end: Option<i64>) -> Result<Value, VmError> {
    match value {
        Value::Array(values) => {
            let (start, end) = slice_bounds(values.len(), start, end);
            Ok(Value::array(values[start..end].to_vec()))
        }
        Value::String(value) => {
            let characters = value.chars().collect::<Vec<_>>();
            let (start, end) = slice_bounds(characters.len(), start, end);
            Ok(Value::string(
                characters[start..end].iter().collect::<String>(),
            ))
        }
        value => Err(type_error("slice", value)),
    }
}

fn slice_bounds(length: usize, start: Option<i64>, end: Option<i64>) -> (usize, usize) {
    let length_i64 = i64::try_from(length).unwrap_or(i64::MAX);
    let normalize = |value: i64| {
        let normalized = if value < 0 {
            (length_i64 + value).clamp(0, length_i64)
        } else {
            value.clamp(0, length_i64)
        };
        usize::try_from(normalized).unwrap_or(length)
    };
    let start = start.map_or(0, normalize);
    let end = end.map_or(length, normalize);
    (start.min(end), end)
}

fn computed_key(value: Value) -> Result<Arc<str>, VmError> {
    match value {
        Value::String(value) => Ok(value),
        value => Err(type_error("computed object key", &value)),
    }
}

fn unary(operator: UnaryOperator, value: &Value) -> Result<Value, VmError> {
    match operator {
        UnaryOperator::Not => Ok(Value::Bool(!value.is_truthy())),
        UnaryOperator::Negate => match value {
            Value::Number(value) => Number::from_f64(-value.as_f64())
                .map(Value::Number)
                .map_err(|error| runtime(error.to_string())),
            value => Err(type_error("negation", value)),
        },
    }
}

fn binary_value(operator: BinaryOperator, left: &Value, right: &Value) -> Result<Value, VmError> {
    match operator {
        BinaryOperator::Equal => Ok(Value::Bool(left == right)),
        BinaryOperator::NotEqual => Ok(Value::Bool(left != right)),
        BinaryOperator::Less => Ok(Value::Bool(left < right)),
        BinaryOperator::LessEqual => Ok(Value::Bool(left <= right)),
        BinaryOperator::Greater => Ok(Value::Bool(left > right)),
        BinaryOperator::GreaterEqual => Ok(Value::Bool(left >= right)),
        BinaryOperator::Add => binary_add(left, right),
        BinaryOperator::Subtract => binary_subtract(left, right),
        BinaryOperator::Multiply => numeric(left, right, Number::multiply, "multiply"),
        BinaryOperator::Divide => numeric(left, right, Number::divide, "divide"),
        BinaryOperator::Remainder => match (left, right) {
            (Value::Number(left), Value::Number(right)) if right.as_f64() != 0.0 => {
                Number::from_f64(left.as_f64() % right.as_f64())
                    .map(Value::Number)
                    .map_err(|error| runtime(error.to_string()))
            }
            (Value::Number(_), Value::Number(_)) => {
                Err(runtime("cannot divide by zero".to_owned()))
            }
            _ => Err(runtime("remainder requires numbers".to_owned())),
        },
        BinaryOperator::Alternative | BinaryOperator::Or | BinaryOperator::And => {
            Err(invalid("short-circuit operator reached scalar dispatch"))
        }
    }
}

fn binary_add(left: &Value, right: &Value) -> Result<Value, VmError> {
    match (left, right) {
        (Value::Null, value) | (value, Value::Null) => Ok(value.clone()),
        (Value::Number(left), Value::Number(right)) => left
            .add(right)
            .map(Value::Number)
            .map_err(|error| runtime(error.to_string())),
        (Value::String(left), Value::String(right)) => Ok(Value::string(format!("{left}{right}"))),
        (Value::Array(left), Value::Array(right)) => Ok(Value::array(
            left.iter().chain(right.iter()).cloned().collect::<Vec<_>>(),
        )),
        (Value::Object(left), Value::Object(right)) => {
            let mut object = left.as_ref().clone();
            for (key, value) in right.iter() {
                object.insert(Arc::clone(key), value.clone());
            }
            Ok(Value::object(object))
        }
        _ => Err(runtime(format!(
            "cannot add {} and {}",
            type_name(left),
            type_name(right)
        ))),
    }
}

fn binary_subtract(left: &Value, right: &Value) -> Result<Value, VmError> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left
            .subtract(right)
            .map(Value::Number)
            .map_err(|error| runtime(error.to_string())),
        (Value::Array(left), Value::Array(right)) => Ok(Value::array(
            left.iter()
                .filter(|value| !right.contains(value))
                .cloned()
                .collect::<Vec<_>>(),
        )),
        _ => Err(runtime("subtraction requires numbers or arrays".to_owned())),
    }
}

fn numeric(
    left: &Value,
    right: &Value,
    apply: fn(&Number, &Number) -> Result<Number, crate::NumberError>,
    name: &str,
) -> Result<Value, VmError> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => apply(left, right)
            .map(Value::Number)
            .map_err(|error| runtime(error.to_string())),
        _ => Err(runtime(format!("{name} requires numbers"))),
    }
}

fn number_value(number: f64) -> Outcomes {
    Number::from_f64(number)
        .map(Value::Number)
        .map(Ok)
        .map_or_else(
            |error| one_error(runtime(error.to_string())),
            |value| vec![value],
        )
}

fn number_usize(number: usize) -> Result<Value, VmError> {
    Number::parse(&number.to_string())
        .map(Value::Number)
        .map_err(|error| runtime(error.to_string()))
}

fn length(value: &Value) -> Result<Value, VmError> {
    let length = match value {
        Value::Null => 0.0,
        Value::Number(number) => number.as_f64().abs(),
        Value::String(value) => return number_usize(value.chars().count()),
        Value::Array(value) => return number_usize(value.len()),
        Value::Object(value) => return number_usize(value.len()),
        Value::Bool(_) => return Err(type_error("length", value)),
    };
    Number::from_f64(length)
        .map(Value::Number)
        .map_err(|error| runtime(error.to_string()))
}

fn keys(value: &Value, sorted: bool) -> Result<Value, VmError> {
    let mut keys = match value {
        Value::Array(values) => (0..values.len())
            .map(|index| Number::parse(&index.to_string()).map(Value::Number))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| runtime(error.to_string()))?,
        Value::Object(values) => values
            .keys()
            .map(|key| Value::String(Arc::clone(key)))
            .collect(),
        value => return Err(type_error("keys", value)),
    };
    if sorted {
        stable_sort_values(&mut keys);
    }
    Ok(Value::array(keys))
}

fn has(container: &Value, key: &Value) -> Result<Value, VmError> {
    let found = match (container, key) {
        (Value::Object(values), Value::String(key)) => values.contains_key(key),
        (Value::Array(values), Value::Number(index)) => index
            .exact_index()
            .and_then(|index| normalize_index(index, values.len()))
            .is_some(),
        (Value::Object(_), _) => {
            return Err(runtime("object membership requires string key".to_owned()));
        }
        (Value::Array(_), _) => {
            return Err(runtime(
                "array membership requires integer index".to_owned(),
            ));
        }
        (value, _) => return Err(type_error("has", value)),
    };
    Ok(Value::Bool(found))
}

fn selector(input: &Value, selected: bool) -> Outcomes {
    if selected {
        vec![Ok(input.clone())]
    } else {
        Vec::new()
    }
}

fn fold_values(
    input: &Value,
    initial: Option<Value>,
    apply: fn(&Value, &Value) -> Result<Value, VmError>,
) -> Outcomes {
    let values = match input {
        Value::Array(values) => values.iter().collect::<Vec<_>>(),
        Value::Object(values) => values.values().collect::<Vec<_>>(),
        value => return one_error(type_error("add", value)),
    };
    let mut iterator = values.into_iter();
    let mut result = if let Some(initial) = initial {
        initial
    } else if let Some(first) = iterator.next() {
        first.clone()
    } else {
        return vec![Ok(Value::Null)];
    };
    for value in iterator {
        match apply(&result, value) {
            Ok(next) => result = next,
            Err(error) => return one_error(error),
        }
    }
    vec![Ok(result)]
}

fn extrema(input: &Value, maximum: bool) -> Outcomes {
    let Value::Array(values) = input else {
        return one_error(type_error(if maximum { "max" } else { "min" }, input));
    };
    let selected = if values.len() >= PARALLEL_REDUCTION_THRESHOLD {
        if maximum {
            values
                .par_iter()
                .enumerate()
                .max_by(|left, right| left.1.cmp(right.1).then_with(|| left.0.cmp(&right.0)))
                .map(|(_, value)| value)
        } else {
            values
                .par_iter()
                .enumerate()
                .min_by(|left, right| left.1.cmp(right.1).then_with(|| left.0.cmp(&right.0)))
                .map(|(_, value)| value)
        }
    } else if maximum {
        values.iter().max()
    } else {
        values.iter().min()
    };
    vec![Ok(selected.cloned().unwrap_or(Value::Null))]
}

fn sort_values(input: &Value) -> Outcomes {
    let Value::Array(values) = input else {
        return one_error(type_error("sort", input));
    };
    let mut values = values.to_vec();
    stable_sort_values(&mut values);
    vec![Ok(Value::array(values))]
}

fn unique_values(input: &Value) -> Outcomes {
    let Value::Array(values) = input else {
        return one_error(type_error("unique", input));
    };
    let mut values = values.to_vec();
    stable_sort_values(&mut values);
    values.dedup();
    vec![Ok(Value::array(values))]
}

/// Stably orders values using the shared thresholded parallel sort policy.
pub fn stable_sort_values(values: &mut [Value]) {
    if values.len() >= PARALLEL_SORT_THRESHOLD {
        values.par_sort();
    } else {
        values.sort();
    }
}

/// Number of workers in the shared Rayon execution pool.
#[must_use]
pub fn parallel_worker_count() -> usize {
    rayon::current_num_threads()
}

/// High-water observations from an overlapping stable-sort preparation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StableSortPipelineObservations {
    /// Number of independently sorted input runs.
    pub batches: usize,
    /// Maximum worker batches simultaneously in flight.
    pub in_flight_batches: usize,
    /// Maximum estimated bytes simultaneously in flight.
    pub in_flight_bytes: usize,
}

/// Bounded producer for stable sorted runs backed by the shared Rayon pool.
pub struct StableSortPipeline {
    batch: Vec<Value>,
    batch_bytes: usize,
    batch_values_limit: usize,
    batch_bytes_limit: usize,
    in_flight_batches_limit: usize,
    in_flight_bytes_limit: usize,
    in_flight_batches: usize,
    in_flight_bytes: usize,
    next_ordinal: usize,
    sender: Sender<(usize, usize, Vec<Value>, bool)>,
    receiver: Receiver<(usize, usize, Vec<Value>, bool)>,
    runs: BTreeMap<usize, Vec<Value>>,
    observations: StableSortPipelineObservations,
    cancellation: Option<Arc<AtomicBool>>,
}

impl StableSortPipeline {
    /// Creates a finite run queue. Every limit must be non-zero.
    #[must_use]
    pub fn new(
        batch_values_limit: usize,
        batch_bytes_limit: usize,
        in_flight_batches_limit: usize,
        in_flight_bytes_limit: usize,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            batch: Vec::with_capacity(batch_values_limit.max(1)),
            batch_bytes: 0,
            batch_values_limit: batch_values_limit.max(1),
            batch_bytes_limit: batch_bytes_limit.max(1),
            in_flight_batches_limit: in_flight_batches_limit.max(1),
            in_flight_bytes_limit: in_flight_bytes_limit.max(1),
            in_flight_batches: 0,
            in_flight_bytes: 0,
            next_ordinal: 0,
            sender,
            receiver,
            runs: BTreeMap::new(),
            observations: StableSortPipelineObservations::default(),
            cancellation: None,
        }
    }

    /// Adds cooperative cancellation shared with decoding and evaluation.
    #[must_use]
    pub fn with_cancellation(mut self, cancellation: Arc<AtomicBool>) -> Self {
        self.cancellation = Some(cancellation);
        self
    }

    /// Adds one producer result, blocking only when the finite worker queue is full.
    ///
    /// # Errors
    ///
    /// Returns a stable resource name when one value exceeds the byte envelope or
    /// a worker disconnects.
    pub fn push(&mut self, value: Value, estimated_bytes: usize) -> Result<(), &'static str> {
        self.check_cancellation()?;
        if estimated_bytes > self.in_flight_bytes_limit {
            return Err("hybrid-in-flight-bytes");
        }
        if !self.batch.is_empty()
            && (self.batch.len() >= self.batch_values_limit
                || self.batch_bytes.saturating_add(estimated_bytes) > self.batch_bytes_limit)
        {
            self.dispatch()?;
        }
        self.batch.push(value);
        self.batch_bytes = self.batch_bytes.saturating_add(estimated_bytes);
        if self.batch.len() >= self.batch_values_limit || self.batch_bytes >= self.batch_bytes_limit
        {
            self.dispatch()?;
        }
        Ok(())
    }

    /// Drains every run and performs deterministic pairwise stable merges.
    ///
    /// # Errors
    ///
    /// Returns a stable resource name if a worker disconnects.
    pub fn finish(mut self) -> Result<(Vec<Value>, StableSortPipelineObservations), &'static str> {
        self.check_cancellation()?;
        self.dispatch()?;
        while self.in_flight_batches != 0 {
            self.receive_one()?;
        }
        let runs = self.runs.into_values().collect::<Vec<_>>();
        Ok((
            merge_sorted_runs(runs, self.cancellation.as_ref())?,
            self.observations,
        ))
    }

    fn dispatch(&mut self) -> Result<(), &'static str> {
        self.check_cancellation()?;
        if self.batch.is_empty() {
            return Ok(());
        }
        let bytes = self.batch_bytes;
        while self.in_flight_batches >= self.in_flight_batches_limit
            || self.in_flight_bytes.saturating_add(bytes) > self.in_flight_bytes_limit
        {
            self.receive_one()?;
        }
        let ordinal = self.next_ordinal;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        let mut values =
            std::mem::replace(&mut self.batch, Vec::with_capacity(self.batch_values_limit));
        self.batch_bytes = 0;
        let sender = self.sender.clone();
        let cancellation = self.cancellation.clone();
        rayon::spawn_fifo(move || {
            if !cancellation
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
            {
                stable_sort_values(&mut values);
            }
            let cancelled = cancellation
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Relaxed));
            let _ = sender.send((ordinal, bytes, values, cancelled));
        });
        self.in_flight_batches = self.in_flight_batches.saturating_add(1);
        self.in_flight_bytes = self.in_flight_bytes.saturating_add(bytes);
        self.observations.batches = self.observations.batches.saturating_add(1);
        self.observations.in_flight_batches = self
            .observations
            .in_flight_batches
            .max(self.in_flight_batches);
        self.observations.in_flight_bytes =
            self.observations.in_flight_bytes.max(self.in_flight_bytes);
        Ok(())
    }

    fn receive_one(&mut self) -> Result<(), &'static str> {
        self.check_cancellation()?;
        let (ordinal, bytes, values, worker_cancelled) =
            self.receiver.recv().map_err(|_| "hybrid-sort-worker")?;
        if worker_cancelled {
            return Err("interrupted");
        }
        self.check_cancellation()?;
        self.in_flight_batches = self.in_flight_batches.saturating_sub(1);
        self.in_flight_bytes = self.in_flight_bytes.saturating_sub(bytes);
        self.runs.insert(ordinal, values);
        Ok(())
    }

    fn check_cancellation(&self) -> Result<(), &'static str> {
        if self
            .cancellation
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            Err("interrupted")
        } else {
            Ok(())
        }
    }
}

fn merge_sorted_runs(
    mut runs: Vec<Vec<Value>>,
    cancellation: Option<&Arc<AtomicBool>>,
) -> Result<Vec<Value>, &'static str> {
    while runs.len() > 1 {
        if cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err("interrupted");
        }
        let mut pairs = Vec::with_capacity(runs.len().div_ceil(2));
        let mut iterator = runs.into_iter();
        while let Some(left) = iterator.next() {
            pairs.push((left, iterator.next()));
        }
        runs = if let Some(cancellation) = cancellation {
            pairs
                .into_par_iter()
                .map(|(left, right)| match right {
                    Some(right) => stable_merge_cancellable(left, right, cancellation),
                    None => Ok(left),
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            pairs
                .into_par_iter()
                .map(|(left, right)| match right {
                    Some(right) => stable_merge(left, right),
                    None => left,
                })
                .collect()
        };
    }
    Ok(runs.pop().unwrap_or_default())
}

fn stable_merge_cancellable(
    left: Vec<Value>,
    right: Vec<Value>,
    cancellation: &AtomicBool,
) -> Result<Vec<Value>, &'static str> {
    let mut merged = Vec::with_capacity(left.len().saturating_add(right.len()));
    let mut left = left.into_iter().peekable();
    let mut right = right.into_iter().peekable();
    let mut comparisons_until_check = 0usize;
    while let (Some(left_value), Some(right_value)) = (left.peek(), right.peek()) {
        if comparisons_until_check == 0 {
            if cancellation.load(Ordering::Relaxed) {
                return Err("interrupted");
            }
            comparisons_until_check = 16 * 1024;
        }
        comparisons_until_check = comparisons_until_check.saturating_sub(1);
        if left_value <= right_value {
            merged.push(left.next().expect("left value was peeked"));
        } else {
            merged.push(right.next().expect("right value was peeked"));
        }
    }
    merged.extend(left);
    merged.extend(right);
    Ok(merged)
}

fn stable_merge(left: Vec<Value>, right: Vec<Value>) -> Vec<Value> {
    let mut merged = Vec::with_capacity(left.len().saturating_add(right.len()));
    let mut left = left.into_iter().peekable();
    let mut right = right.into_iter().peekable();
    while let (Some(left_value), Some(right_value)) = (left.peek(), right.peek()) {
        if left_value <= right_value {
            merged.push(left.next().expect("left value was peeked"));
        } else {
            merged.push(right.next().expect("right value was peeked"));
        }
    }
    merged.extend(left);
    merged.extend(right);
    merged
}

fn sort_by_cached_key(values: &mut [(Value, Value)]) {
    if values.len() >= PARALLEL_SORT_THRESHOLD {
        values.par_sort_by(|left, right| left.0.cmp(&right.0));
    } else {
        values.sort_by(|left, right| left.0.cmp(&right.0));
    }
}

fn reverse(input: &Value) -> Outcomes {
    match input {
        Value::Array(values) => vec![Ok(Value::array(
            values.iter().rev().cloned().collect::<Vec<_>>(),
        ))],
        Value::String(value) => vec![Ok(Value::string(value.chars().rev().collect::<String>()))],
        value => one_error(type_error("reverse", value)),
    }
}

fn flatten(input: &Value, depth: Option<i64>) -> Result<Value, VmError> {
    let Value::Array(values) = input else {
        return Err(type_error("flatten", input));
    };
    if depth.is_some_and(|depth| depth < 0) {
        return Err(runtime("flatten depth cannot be negative".to_owned()));
    }
    let mut output = Vec::new();
    let mut stack = values
        .iter()
        .rev()
        .cloned()
        .map(|value| (value, depth))
        .collect::<Vec<_>>();
    while let Some((value, depth)) = stack.pop() {
        match value {
            Value::Array(values) if depth != Some(0) => {
                let next_depth = depth.map(|depth| depth - 1);
                stack.extend(
                    values
                        .iter()
                        .rev()
                        .cloned()
                        .map(|value| (value, next_depth)),
                );
            }
            value => output.push(value),
        }
    }
    Ok(Value::array(output))
}

fn path_component(value: &Value) -> Result<PathComponent, VmError> {
    match value {
        Value::String(key) => Ok(PathComponent::Key(Arc::clone(key))),
        Value::Number(index) => index
            .exact_index()
            .and_then(|index| usize::try_from(index).ok())
            .map(PathComponent::Index)
            .ok_or_else(|| runtime("path index must be a non-negative exact integer".to_owned())),
        value => Err(type_error("path component", value)),
    }
}

fn update_value(
    operator: AssignmentOperator,
    old: &Value,
    replacement: &Value,
) -> Result<Value, VmError> {
    match operator {
        AssignmentOperator::Set | AssignmentOperator::Update => Ok(replacement.clone()),
        AssignmentOperator::Add => binary_add(old, replacement),
        AssignmentOperator::Subtract => binary_subtract(old, replacement),
        AssignmentOperator::Multiply => numeric(old, replacement, Number::multiply, "multiply"),
        AssignmentOperator::Divide => numeric(old, replacement, Number::divide, "divide"),
        AssignmentOperator::Alternative => Ok(if old.is_truthy() {
            old.clone()
        } else {
            replacement.clone()
        }),
    }
}

fn replace_or_create(
    root: &Value,
    components: &[PathComponent],
    replacement: Value,
) -> Result<Value, VmError> {
    let mut current = root.clone();
    let mut ancestors = Vec::with_capacity(components.len());
    for component in components {
        let child = match (component, &current) {
            (PathComponent::Key(key), Value::Object(object)) => {
                object.get(key).cloned().unwrap_or(Value::Null)
            }
            (PathComponent::Key(_) | PathComponent::Index(_), Value::Null) => Value::Null,
            (PathComponent::Key(_), value) => return Err(type_error("object assignment", value)),
            (PathComponent::Index(index), Value::Array(values)) => {
                values.get(*index).cloned().unwrap_or(Value::Null)
            }
            (PathComponent::Index(_), value) => return Err(type_error("array assignment", value)),
        };
        ancestors.push((current, component.clone()));
        current = child;
    }

    let mut rebuilt = replacement;
    while let Some((parent, component)) = ancestors.pop() {
        rebuilt = match component {
            PathComponent::Key(key) => {
                let mut object = match parent {
                    Value::Object(object) => object.as_ref().clone(),
                    Value::Null => IndexMap::new(),
                    value => return Err(type_error("object assignment", &value)),
                };
                object.insert(key, rebuilt);
                Value::object(object)
            }
            PathComponent::Index(index) => {
                let mut values = match parent {
                    Value::Array(values) => values.to_vec(),
                    Value::Null => Vec::new(),
                    value => return Err(type_error("array assignment", &value)),
                };
                values.resize(index.saturating_add(1), Value::Null);
                values[index] = rebuilt;
                Value::array(values)
            }
        };
    }
    Ok(rebuilt)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use indexmap::IndexMap;

    use super::{
        PARALLEL_REDUCTION_THRESHOLD, PARALLEL_SORT_THRESHOLD, StableSortPipeline, Value, extrema,
        sort_by_cached_key, stable_sort_values,
    };
    use crate::{ResolveOptions, Vm, VmLimits, analyze, parse, resolve};

    fn run(query: &str, input: &str) -> Vec<Result<Value, String>> {
        run_with_variables(query, input, BTreeMap::new())
    }

    fn run_with_variables(
        query: &str,
        input: &str,
        variables: BTreeMap<Arc<str>, Value>,
    ) -> Vec<Result<Value, String>> {
        let query = parse(query).unwrap();
        let query = resolve(
            query,
            &ResolveOptions {
                variables: variables.keys().cloned().collect::<BTreeSet<_>>(),
                ..ResolveOptions::default()
            },
        )
        .unwrap();
        let plan = analyze(query).compile().unwrap().document_plan();
        let input: Value = serde_json::from_str(input).unwrap();
        let mut vm = Vm::new_with_variables(&plan, input, VmLimits::default(), variables);
        let mut values = Vec::new();
        loop {
            match vm.next_result() {
                Ok(Some(value)) => values.push(Ok(value)),
                Ok(None) => break,
                Err(error) => {
                    values.push(Err(error.to_string()));
                    break;
                }
            }
        }
        values
    }

    fn first_error_with_limits(query: &str, input: &str, limits: VmLimits) -> crate::VmError {
        let plan = analyze(resolve(parse(query).unwrap(), &ResolveOptions::default()).unwrap())
            .compile()
            .unwrap()
            .document_plan();
        let input: Value = serde_json::from_str(input).unwrap();
        Vm::new(&plan, input, limits)
            .next_result()
            .expect_err("query should fail before producing a result")
    }

    fn json(values: Vec<Result<Value, String>>) -> Vec<String> {
        values
            .into_iter()
            .map(|value| {
                value.map_or_else(|error| format!("error:{error}"), |value| value.to_string())
            })
            .collect()
    }

    #[test]
    fn regex_builtins_match_jq_unicode_captures_scans_splits_and_substitution() {
        assert_eq!(
            json(run(r#"match("a"; "g")"#, r#""éaéa""#)),
            vec![
                r#"{"offset":1,"length":1,"string":"a","captures":[]}"#,
                r#"{"offset":3,"length":1,"string":"a","captures":[]}"#,
            ]
        );
        assert_eq!(
            json(run(r#"capture("(?<x>a)(?<y>z)?")"#, r#""a""#)),
            vec![r#"{"x":"a","y":null}"#]
        );
        assert_eq!(
            json(run(r#"scan("([a-z]+)([0-9]+)")"#, r#""ab12cd34""#)),
            vec![r#"["ab","12"]"#, r#"["cd","34"]"#]
        );
        assert_eq!(
            json(run(
                r#"sub("(?<x>[a-z]+)(?<n>[0-9]+)"; "\(.n)-\(.x)")"#,
                r#""abc123""#,
            )),
            vec![r#""123-abc""#]
        );
        assert!(json(run(r#"test("(?=a)")"#, r#""a""#))[0].contains("not supported"));
    }

    #[test]
    fn regex_pattern_input_and_compiled_program_limits_are_resources() {
        for (query, limits, expected) in [
            (
                r#"test("ab")"#,
                VmLimits {
                    regex_pattern_bytes: 1,
                    ..VmLimits::default()
                },
                "regex-pattern-bytes",
            ),
            (
                r#"test("ab")"#,
                VmLimits {
                    regex_input_bytes: 1,
                    ..VmLimits::default()
                },
                "regex-input-bytes",
            ),
            (
                r#"test("(?:[A-Za-z0-9_]{1,100}){100}")"#,
                VmLimits {
                    regex_compiled_bytes: 1,
                    ..VmLimits::default()
                },
                "regex-compiled-bytes",
            ),
        ] {
            let error = first_error_with_limits(query, r#""ab""#, limits);
            assert_eq!(error, crate::VmError::Resource { resource: expected });
        }
    }

    #[test]
    fn utc_date_builtins_round_trip_with_stable_arrays() {
        assert_eq!(
            json(run("fromdateiso8601", r#""2015-03-05T23:51:47Z""#)),
            vec!["1425599507"]
        );
        assert_eq!(
            json(run("gmtime", "1425599507")),
            vec!["[2015,2,5,23,51,47,4,63]"]
        );
        assert_eq!(
            json(run(
                r#"gmtime | strftime("%Y-%m-%dT%H:%M:%SZ")"#,
                "1425599507",
            )),
            vec![r#""2015-03-05T23:51:47Z""#]
        );
        assert!(matches!(
            first_error_with_limits("todateiso8601", "253402300800", VmLimits::default()),
            crate::VmError::NumericRange { .. }
        ));
    }

    #[test]
    fn regex_date_platform_release_host_contract_covers_utc_boundaries() {
        for timestamp in ["0000-01-01T00:00:00Z", "9999-12-30T22:00:00Z"] {
            let input = serde_json::to_string(timestamp).unwrap();
            assert_eq!(
                json(run("fromdateiso8601 | todateiso8601", &input)),
                [input]
            );
        }
    }

    #[test]
    fn ambient_builtins_require_reserved_policy_admission_without_echoing_values() {
        assert!(json(run("env", "null"))[0].contains("capability policy"));
        let mut variables = BTreeMap::new();
        variables.insert(
            Arc::from(super::AMBIENT_ENVIRONMENT),
            Value::object(IndexMap::from([(
                Arc::from("SECRET"),
                Value::string("redacted"),
            )])),
        );
        variables.insert(Arc::from(super::AMBIENT_PLATFORM), Value::Bool(true));
        variables.insert(
            Arc::from(super::INPUT_FILENAME),
            Value::string("fixture.json"),
        );
        assert_eq!(
            json(run_with_variables("env | type", "null", variables.clone())),
            vec![r#""object""#]
        );
        assert_eq!(
            json(run_with_variables("input_filename", "null", variables)),
            vec![r#""fixture.json""#]
        );
    }

    #[test]
    fn wave_one_navigation_and_generators_compose() {
        assert_eq!(json(run(".a.b", r#"{"a":{"b":2}}"#)), ["2"]);
        assert_eq!(json(run(".[-1]", "[1,2,3]")), ["3"]);
        assert_eq!(json(run(".[1:3]", "[0,1,2,3]")), ["[1,2]"]);
        assert_eq!(json(run(".[] | .x", r#"[{"x":1},{"x":2}]"#)), ["1", "2"]);
        assert_eq!(json(run(".a, .missing?", r#"{"a":1}"#)), ["1", "null"]);
        assert_eq!(run("empty", "null"), Vec::<Result<Value, String>>::new());
    }

    #[test]
    fn wave_two_construction_control_and_operators_compose() {
        assert_eq!(json(run("[.[] | . * 2]", "[1,2,3]")), ["[2,4,6]"]);
        assert_eq!(
            json(run(
                "{name, ok: (.age >= 18)}",
                r#"{"name":"Ada","age":30}"#
            )),
            [r#"{"name":"Ada","ok":true}"#]
        );
        assert_eq!(
            json(run("if . then \"yes\" else \"no\" end", "0")),
            [r#""yes""#]
        );
        assert_eq!(json(run("null // false // 7", "null")), ["7"]);
        assert_eq!(json(run(r#""a" + "b""#, "null")), [r#""ab""#]);
        assert_eq!(
            json(run(r#"{"a":1}+{"a":2,"b":3}"#, "null")),
            [r#"{"a":2,"b":3}"#]
        );
    }

    #[test]
    fn wave_three_variables_builtins_errors_and_updates_compose() {
        let variables = BTreeMap::from([(
            Arc::from("n"),
            Value::from_json(serde_json::json!(3)).unwrap(),
        )]);
        assert_eq!(json(run_with_variables("$n + 1", "null", variables)), ["4"]);
        assert_eq!(
            json(run(
                "map(.x) | sort | unique",
                r#"[{"x":2},{"x":1},{"x":2}]"#
            )),
            ["[1,2]"]
        );
        assert_eq!(json(run("type, length", r"[1,2]")), [r#""array""#, "2"]);
        assert_eq!(json(run("tostring | length", "1e4096")), ["7"]);
        assert_eq!(json(run(r#""\(.)""#, "1e4096")), [r#""1e+4096""#]);
        assert_eq!(
            json(run("try error(\"boom\") catch .", "null")),
            [r#""boom""#]
        );
        let error_after = json(run("1, error(\"later\")", "null"));
        assert_eq!(error_after[0], "1");
        assert!(error_after[1].starts_with("error:runtime error: later"));
        assert_eq!(json(run(".a.b = 2", "{}")), [r#"{"a":{"b":2}}"#]);
        assert_eq!(
            json(run("(.a, .b) += 10", r#"{"a":1,"b":2,"c":[3]}"#)),
            [r#"{"a":11,"b":12,"c":[3]}"#]
        );
    }

    #[test]
    fn keyed_sort_uses_every_value_from_its_filter() {
        assert_eq!(
            json(run(
                "sort_by(.a,.b)",
                r#"[{"a":1,"b":2},{"a":1,"b":1},{"a":0,"b":9}]"#,
            )),
            [r#"[{"a":0,"b":9},{"a":1,"b":1},{"a":1,"b":2}]"#]
        );
        assert_eq!(
            json(run(
                "unique_by(.a,.b)",
                r#"[{"a":1,"b":2,"id":"first"},{"a":1,"b":1},{"a":1,"b":2,"id":"last"}]"#,
            )),
            [r#"[{"a":1,"b":1},{"a":1,"b":2,"id":"first"}]"#]
        );
        assert_eq!(json(run("sort_by(empty)", "[3,1,2]")), ["[3,1,2]"]);
        assert_eq!(json(run("sort_by(.)", "[3,1,2]")), ["[1,2,3]"]);
        assert!(
            json(run(r#"sort_by(.a, error("bad"))"#, r#"[{"a":1}]"#,))[0]
                .starts_with("error:runtime error: bad")
        );
    }

    #[test]
    fn parallel_collections_preserve_order_and_extrema_ties() {
        let count = PARALLEL_SORT_THRESHOLD + 1;
        let original = (0..count)
            .map(|index| Value::string(format!("{index:05}")))
            .collect::<Vec<_>>();

        let mut values = original.iter().rev().cloned().collect::<Vec<_>>();
        stable_sort_values(&mut values);
        assert_eq!(values, original);

        let mut keyed = original
            .iter()
            .cloned()
            .map(|value| (Value::Null, value))
            .collect::<Vec<_>>();
        sort_by_cached_key(&mut keyed);
        assert!(keyed.iter().map(|(_, value)| value).eq(original.iter()));

        let first_equal = Value::object(IndexMap::from([
            (Arc::from("a"), Value::Null),
            (Arc::from("b"), Value::Null),
        ]));
        let last_equal = Value::object(IndexMap::from([
            (Arc::from("b"), Value::Null),
            (Arc::from("a"), Value::Null),
        ]));
        let mut equal_values = vec![first_equal.clone(); PARALLEL_REDUCTION_THRESHOLD];
        equal_values.push(last_equal.clone());
        let equal_values = Value::array(equal_values);
        let minimum = extrema(&equal_values, false).pop().unwrap().unwrap();
        let maximum = extrema(&equal_values, true).pop().unwrap().unwrap();
        assert!(minimum.shares_node_with(&first_equal));
        assert!(maximum.shares_node_with(&last_equal));
    }

    #[test]
    fn recursive_descent_is_depth_first_and_streams_scalars_in_object_order() {
        assert_eq!(
            json(run("..", r#"{"z":1,"a":[2,{"b":3}]}"#)),
            [
                r#"{"z":1,"a":[2,{"b":3}]}"#,
                "1",
                r#"[2,{"b":3}]"#,
                "2",
                r#"{"b":3}"#,
                "3",
            ]
        );
        assert_eq!(
            json(run(".. | scalars", r#"{"z":1,"a":[2,{"b":3}]}"#)),
            ["1", "2", "3"]
        );
        assert_eq!(json(run("..", "42")), ["42"]);
    }

    #[test]
    fn user_functions_preserve_generator_arguments_scope_and_managed_recursion() {
        assert_eq!(
            json(run("def pair($x; $y): $x, $y; pair((1,2); (3,4))", "null")),
            ["1", "3", "1", "4", "2", "3", "2", "4"]
        );
        assert_eq!(
            json(run("def pair($x; $y): $x, $y; pair(1,2; 3,4)", "null")),
            ["1", "3", "1", "4", "2", "3", "2", "4"]
        );
        assert_eq!(json(run("def twice(f): f | f; twice(. + 1)", "1")), ["3"]);
        assert_eq!(
            json(run("def wrapped(f): [f, (not)]; wrapped((1,2))", "null")),
            ["[1,2,true]"]
        );
        assert_eq!(
            json(run("def fallback: empty // 9; fallback", "null")),
            ["9"]
        );
        assert_eq!(
            json(run(
                "1 as $x | def captured: $x; 2 as $x | captured",
                "null"
            )),
            ["1"]
        );
        assert_eq!(
            json(run(
                "def down($n): if $n == 0 then 0 else $n, down($n - 1) end; down(3)",
                "null"
            )),
            ["3", "2", "1", "0"]
        );

        let plan = analyze(
            resolve(
                parse("def forever: forever; forever").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut limited = Vm::new(
            &plan,
            Value::Null,
            VmLimits {
                call_stack: 8,
                ..VmLimits::default()
            },
        );
        assert!(matches!(
            limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "call-stack"
            })
        ));
    }

    #[test]
    fn interpolation_matches_jq_conversion_generator_order_nesting_and_errors() {
        assert_eq!(
            json(run(
                r#""v=\(null),\(false),\(1),\("x"),\([1]),\({a:1})""#,
                "null",
            )),
            [r#""v=null,false,1,x,[1],{\"a\":1}""#]
        );
        assert_eq!(
            json(run(r#""x=\(1,2);y=\("a","b")""#, "null")),
            [
                r#""x=1;y=a""#,
                r#""x=2;y=a""#,
                r#""x=1;y=b""#,
                r#""x=2;y=b""#
            ]
        );
        assert_eq!(
            json(run(r#""\(if true then "x\(1,2)" else "z" end)""#, "null")),
            [r#""x1""#, r#""x2""#]
        );
        assert_eq!(json(run(r#""sum=\(1 + 2)""#, "null")), [r#""sum=3""#]);
        assert_eq!(
            json(run(r#""x=\((1,2) + 1);y=\(("a","b") + "!")""#, "null",)),
            [
                r#""x=2;y=a!""#,
                r#""x=3;y=a!""#,
                r#""x=2;y=b!""#,
                r#""x=3;y=b!""#,
            ]
        );
        let partial = json(run(r#""before=\(1,error("boom"),2)""#, "null"));
        assert_eq!(partial[0], r#""before=1""#);
        assert!(partial[1].starts_with("error:runtime error: boom"));
        assert_eq!(
            run(r#""\(empty)""#, "null"),
            Vec::<Result<Value, String>>::new()
        );
    }

    #[test]
    fn recursive_cursor_obeys_depth_work_and_cancellation_after_partial_output() {
        let plan = analyze(resolve(parse("..").unwrap(), &ResolveOptions::default()).unwrap())
            .compile()
            .unwrap()
            .document_plan();
        let mut input = Value::Null;
        for _ in 0..128 {
            input = Value::array([input]);
        }
        let mut depth_limited = Vm::new(
            &plan,
            input.clone(),
            VmLimits {
                path_stack: 16,
                ..VmLimits::default()
            },
        );
        for _ in 0..16 {
            assert!(depth_limited.next_result().unwrap().is_some());
        }
        assert!(matches!(
            depth_limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "path-stack"
            })
        ));

        let mut work_limited = Vm::new(
            &plan,
            input.clone(),
            VmLimits {
                steps: 2,
                ..VmLimits::default()
            },
        );
        assert!(work_limited.next_result().unwrap().is_some());
        assert!(matches!(
            work_limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "vm-steps"
            })
        ));

        let cancellation = Arc::new(AtomicBool::new(false));
        let mut cancelled =
            Vm::new(&plan, input, VmLimits::default()).with_cancellation(Arc::clone(&cancellation));
        assert!(cancelled.next_result().unwrap().is_some());
        cancellation.store(true, Ordering::Relaxed);
        assert!(matches!(
            cancelled.next_result(),
            Err(crate::VmError::Interrupted)
        ));
    }

    #[test]
    fn interpolation_generator_explosion_is_bounded_by_managed_forks() {
        let plan = analyze(
            resolve(
                parse(r#""\(1,2,3)-\("a","b")""#).unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut limited = Vm::new(
            &plan,
            Value::Null,
            VmLimits {
                fork_stack: 1,
                ..VmLimits::default()
            },
        );
        assert!(matches!(
            limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "fork-stack"
            })
        ));

        let fallback_plan = analyze(
            resolve(
                parse(r#""\(range(0;3) + 0)""#).unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut fallback_limited = Vm::new(
            &fallback_plan,
            Value::Null,
            VmLimits {
                fork_stack: 1,
                ..VmLimits::default()
            },
        );
        assert!(matches!(
            fallback_limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "fork-stack"
            })
        ));

        let output_plan = analyze(
            resolve(
                parse(r#""prefix=\(.)""#).unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut output_limited = Vm::new(
            &output_plan,
            Value::string("value"),
            VmLimits {
                output_bytes: 8,
                ..VmLimits::default()
            },
        );
        assert!(matches!(
            output_limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "output-bytes"
            })
        ));

        let wide = Value::array(vec![Value::string("0123456789"); 100_000]);
        let mut serialization_limited = Vm::new(
            &output_plan,
            wide,
            VmLimits {
                output_bytes: 16,
                ..VmLimits::default()
            },
        );
        assert!(matches!(
            serialization_limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "output-bytes"
            })
        ));
    }

    #[test]
    fn stable_sort_pipeline_bounds_runs_and_merges_deterministically() {
        let mut pipeline = StableSortPipeline::new(2, 256, 2, 512);
        for value in [3, 1, 2, 1, 3, 2] {
            pipeline
                .push(serde_json::from_str(&value.to_string()).unwrap(), 32)
                .unwrap();
        }
        let (values, observations) = pipeline.finish().unwrap();
        assert_eq!(
            values,
            [1, 1, 2, 2, 3, 3]
                .map(|value| serde_json::from_str(&value.to_string()).unwrap())
                .to_vec()
        );
        assert_eq!(observations.batches, 3);
        assert!(observations.in_flight_batches <= 2);
        assert!(observations.in_flight_bytes <= 512);
    }

    #[test]
    fn stable_sort_pipeline_cancels_with_workers_active() {
        let cancellation = Arc::new(AtomicBool::new(false));
        let mut pipeline =
            StableSortPipeline::new(1, 64, 2, 128).with_cancellation(Arc::clone(&cancellation));
        pipeline
            .push(serde_json::from_str("2").unwrap(), 32)
            .unwrap();
        cancellation.store(true, Ordering::Relaxed);
        assert_eq!(pipeline.finish().unwrap_err(), "interrupted");
    }

    #[test]
    fn stable_sort_pipeline_preserves_equal_object_order_across_runs() {
        let first: Value = serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();
        let second: Value = serde_json::from_str(r#"{"b":2,"a":1}"#).unwrap();
        assert_eq!(first.cmp(&second), std::cmp::Ordering::Equal);
        assert_ne!(first.to_string(), second.to_string());

        let run = || {
            let mut pipeline = StableSortPipeline::new(1, 256, 2, 512);
            pipeline.push(first.clone(), 64).unwrap();
            pipeline.push(second.clone(), 64).unwrap();
            pipeline
                .finish()
                .unwrap()
                .0
                .into_iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
        };
        let expected = vec![first.to_string(), second.to_string()];
        assert_eq!(run(), expected);
        assert_eq!(run(), expected);
    }
}
