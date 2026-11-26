//! Schedule path-sensitive operations and traverse input paths with bounded work.

use super::managed_work::{
    AssignmentState, FilterArgument, GeneratorContinuation, GeneratorTask, GeneratorWork, UserFrame,
};
use super::origins::{LexicalEnvironment, OriginToken, PathAliases, alias_path_for_input};
use super::structured_paths::{
    bind_pattern, getpath, initialize_pattern_variables, is_pattern_mismatch, path_value,
};
use super::value_ops::{slice_bounds, type_error};
use super::{UserFrames, invalid, raised, resource, runtime};
use crate::ast::ParameterKind;
use crate::bytecode::{BindingPatternOperand, Operation};
use crate::{Bytecode, Path, PathComponent, Value, VmError, VmLimits, VmObservations};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub(super) struct PathWork {
    pub(super) node: u32,
    pub(super) input: Value,
    pub(super) origin: Option<OriginToken>,
    pub(super) prefix: Path,
    pub(super) environment: Arc<LexicalEnvironment>,
    pub(super) frames: UserFrames,
    pub(super) continuations: Vec<GeneratorContinuation>,
}

#[derive(Clone)]
struct PathTraversalFrame {
    value: Value,
    path: Path,
    emitted: bool,
    next_child: usize,
}

#[derive(Clone)]
pub(super) struct PathTraversalCursor {
    frames: Vec<PathTraversalFrame>,
}

impl PathTraversalCursor {
    pub(super) fn new(value: Value, path: Path) -> Self {
        Self {
            frames: vec![PathTraversalFrame {
                value,
                path,
                emitted: false,
                next_child: 0,
            }],
        }
    }

    pub(super) fn without_root(value: Value) -> Self {
        Self {
            frames: vec![PathTraversalFrame {
                value,
                path: Path::root(),
                emitted: true,
                next_child: 0,
            }],
        }
    }

    pub(super) fn next(
        &mut self,
        depth_limit: usize,
        charge: &mut impl FnMut() -> Result<(), VmError>,
    ) -> Result<Option<Path>, VmError> {
        loop {
            charge()?;
            let Some(frame) = self.frames.last_mut() else {
                return Ok(None);
            };
            if !frame.emitted {
                frame.emitted = true;
                return Ok(Some(frame.path.clone()));
            }
            let next_child = frame.next_child;
            let child = match &frame.value {
                Value::Array(values) => values
                    .get(next_child)
                    .cloned()
                    .map(|child| (PathComponent::Index(next_child), child)),
                Value::Object(values) => values
                    .get_index(next_child)
                    .map(|(key, child)| (PathComponent::Key(Arc::clone(key)), child.clone())),
                Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
            };
            let Some((component, child)) = child else {
                self.frames.pop();
                continue;
            };
            frame.next_child = frame.next_child.saturating_add(1);
            let parent_path = frame.path.clone();
            let total = parent_path
                .components()
                .len()
                .checked_add(1)
                .ok_or_else(|| resource("path-stack"))?;
            if total > depth_limit {
                return Err(resource("path-stack"));
            }
            let mut components = Vec::new();
            components
                .try_reserve_exact(total)
                .map_err(|_| resource("path-stack"))?;
            for _ in 0..total {
                charge()?;
            }
            components.extend_from_slice(parent_path.components());
            components.push(component);
            self.frames
                .try_reserve(1)
                .map_err(|_| resource("path-stack"))?;
            let path = Path::new(components);
            self.frames.push(PathTraversalFrame {
                value: child,
                path,
                emitted: false,
                next_child: 0,
            });
        }
    }

    pub(super) fn depth(&self) -> usize {
        self.frames.len().saturating_sub(1)
    }
}
pub(super) fn current_path_aliases(continuations: &[GeneratorContinuation]) -> PathAliases {
    continuations
        .iter()
        .rev()
        .find_map(|continuation| match continuation {
            GeneratorContinuation::PathAliases(aliases) => Some(aliases.as_ref().clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn path_bind_candidate(bytecode: &Bytecode, node: u32) -> bool {
    fn visit(
        bytecode: &Bytecode,
        node: u32,
        memo: &mut BTreeMap<u32, bool>,
        active: &mut BTreeSet<u32>,
    ) -> bool {
        if let Some(candidate) = memo.get(&node) {
            return *candidate;
        }
        if !active.insert(node) {
            return false;
        }
        let result = match bytecode.instructions().get(node as usize) {
            Some(instruction) => match &instruction.operation {
                Operation::Identity
                | Operation::RecursiveDescent
                | Operation::Variable(_)
                | Operation::ParameterCall { .. } => true,
                Operation::AccessField { base, .. }
                | Operation::Iterate(base)
                | Operation::Optional(base) => visit(bytecode, *base, memo, active),
                Operation::AccessIndex { base, .. } | Operation::Slice { base, .. } => {
                    visit(bytecode, *base, memo, active)
                }
                Operation::Pipe { left, right } => {
                    visit(bytecode, *left, memo, active) && visit(bytecode, *right, memo, active)
                }
                Operation::Comma { left, right } => {
                    visit(bytecode, *left, memo, active) || visit(bytecode, *right, memo, active)
                }
                Operation::Conditional {
                    branches,
                    alternative,
                } => {
                    branches
                        .iter()
                        .any(|(_, body)| visit(bytecode, *body, memo, active))
                        || visit(bytecode, *alternative, memo, active)
                }
                Operation::TryCatch { expression, catch } => {
                    visit(bytecode, *expression, memo, active)
                        || catch.is_some_and(|catch| visit(bytecode, catch, memo, active))
                }
                Operation::Bind { body, .. } | Operation::BindAlternatives { body, .. } => {
                    visit(bytecode, *body, memo, active)
                }
                Operation::UserCall { symbol, .. } => bytecode
                    .functions()
                    .get(*symbol as usize)
                    .is_some_and(|function| visit(bytecode, function.body, memo, active)),
                Operation::Call { name, .. } => bytecode
                    .string(*name)
                    .is_some_and(|name| name.as_ref() == "select"),
                _ => false,
            },
            None => false,
        };
        active.remove(&node);
        memo.insert(node, result);
        result
    }

    visit(bytecode, node, &mut BTreeMap::new(), &mut BTreeSet::new())
}

pub(super) fn path_candidate_with_aliases(
    bytecode: &Bytecode,
    node: u32,
    continuations: &[GeneratorContinuation],
) -> bool {
    match bytecode
        .instructions()
        .get(node as usize)
        .map(|instruction| &instruction.operation)
    {
        Some(Operation::Variable(index)) => bytecode
            .string(*index)
            .is_some_and(|name| current_path_aliases(continuations).contains_key(name)),
        _ => path_bind_candidate(bytecode, node),
    }
}

pub(super) fn clear_path_aliases(
    bytecode: &Bytecode,
    pattern: &BindingPatternOperand,
    aliases: &mut PathAliases,
) {
    match pattern {
        BindingPatternOperand::Variable(index) => {
            if let Some(name) = bytecode.string(*index) {
                aliases.remove(name);
            }
        }
        BindingPatternOperand::Array(patterns) => {
            for pattern in patterns {
                clear_path_aliases(bytecode, pattern, aliases);
            }
        }
        BindingPatternOperand::Object(fields) => {
            for (_, pattern) in fields {
                clear_path_aliases(bytecode, pattern, aliases);
            }
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "path traversal keeps the selected root and managed continuation state explicit"
)]
pub(super) fn schedule_path_eval(
    bytecode: &Bytecode,
    work: PathWork,
    pending: &mut Vec<GeneratorTask>,
    next_boundary: &mut u64,
) -> Option<Result<Value, VmError>> {
    let operation = match bytecode.instructions().get(work.node as usize) {
        Some(instruction) => instruction.operation.clone(),
        None => return Some(Err(invalid("path instruction missing"))),
    };
    match operation {
        Operation::Identity => Some(Ok(path_value(&work.prefix))),
        Operation::Variable(index) => {
            let Some(name) = bytecode.string(index) else {
                return Some(Err(invalid("string missing after validation")));
            };
            if !work.environment.contains_key(name) {
                return Some(Err(runtime(format!("variable ${name} has no value"))));
            }
            let aliases = current_path_aliases(&work.continuations);
            if let Some(alias) = aliases.get(name) {
                if work.origin.as_ref() == Some(&alias.origin) {
                    let path = work
                        .origin
                        .as_ref()
                        .map_or_else(Path::root, |origin| alias_path_for_input(alias, origin));
                    return Some(Ok(path_value(&path)));
                }
                return Some(Err(runtime(
                    "assignment left side is not a path".to_owned(),
                )));
            }
            pending.push(GeneratorTask::Eval(GeneratorWork {
                node: work.node,
                input: work.input,
                origin: work.origin.clone(),
                environment: work.environment,
                frames: work.frames,
                continuations: work.continuations,
            }));
            None
        }
        Operation::AccessField { base, key } => {
            let key = match bytecode.string(key) {
                Some(key) => Arc::clone(key),
                None => return Some(Err(invalid("path field missing after validation"))),
            };
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathField(key));
            pending.push(GeneratorTask::PathEval(PathWork {
                node: base,
                input: work.input,
                origin: work.origin.clone(),
                prefix: work.prefix,
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::AccessIndex { base, index } => {
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathIndex {
                node: index,
                input: work.input.clone(),
                origin: work.origin.clone(),
                environment: Arc::clone(&work.environment),
            });
            pending.push(GeneratorTask::PathEval(PathWork {
                node: base,
                input: work.input,
                origin: work.origin.clone(),
                prefix: work.prefix,
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::Slice { base, start, end } => {
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathSliceBase {
                start,
                end,
                input: work.input.clone(),
                origin: work.origin.clone(),
                environment: Arc::clone(&work.environment),
                frames: Arc::clone(&work.frames),
            });
            pending.push(GeneratorTask::PathEval(PathWork {
                node: base,
                input: work.input,
                origin: work.origin.clone(),
                prefix: work.prefix,
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::Iterate(base) => {
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathIterate {
                input: work.input.clone(),
            });
            pending.push(GeneratorTask::PathEval(PathWork {
                node: base,
                input: work.input,
                origin: work.origin.clone(),
                prefix: work.prefix,
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::RecursiveDescent => {
            pending.push(GeneratorTask::PathTraverse {
                cursor: PathTraversalCursor::new(work.input, work.prefix),
                environment: work.environment,
                frames: work.frames,
                continuations: work.continuations,
            });
            None
        }
        Operation::Pipe { left, right } => {
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathPipe {
                node: right,
                input: work.input.clone(),
                environment: Arc::clone(&work.environment),
            });
            pending.push(GeneratorTask::PathEval(PathWork {
                node: left,
                input: work.input,
                origin: work.origin.clone(),
                prefix: work.prefix,
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::Comma { left, right } => {
            let right_task = if path_candidate_with_aliases(bytecode, right, &work.continuations) {
                GeneratorTask::PathEval(PathWork {
                    node: right,
                    input: work.input.clone(),
                    origin: work.origin.clone(),
                    prefix: work.prefix.clone(),
                    environment: Arc::clone(&work.environment),
                    frames: Arc::clone(&work.frames),
                    continuations: work.continuations.clone(),
                })
            } else {
                GeneratorTask::Eval(GeneratorWork {
                    node: right,
                    input: work.input.clone(),
                    origin: work.origin.clone(),
                    environment: Arc::clone(&work.environment),
                    frames: Arc::clone(&work.frames),
                    continuations: work.continuations.clone(),
                })
            };
            let left_task = if path_candidate_with_aliases(bytecode, left, &work.continuations) {
                GeneratorTask::PathEval(PathWork {
                    node: left,
                    input: work.input,
                    origin: work.origin.clone(),
                    prefix: work.prefix,
                    environment: work.environment,
                    frames: work.frames,
                    continuations: work.continuations,
                })
            } else {
                GeneratorTask::Eval(GeneratorWork {
                    node: left,
                    input: work.input,
                    origin: work.origin.clone(),
                    environment: work.environment,
                    frames: work.frames,
                    continuations: work.continuations,
                })
            };
            pending.push(right_task);
            pending.push(left_task);
            None
        }
        Operation::Optional(child) => {
            let boundary = *next_boundary;
            *next_boundary = next_boundary.saturating_add(1);
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::OptionalBoundary { boundary });
            pending.push(GeneratorTask::PathEval(PathWork {
                node: child,
                input: work.input,
                origin: work.origin.clone(),
                prefix: work.prefix,
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::Conditional {
            branches,
            alternative,
        } => {
            let Some(condition) = branches.first().map(|(condition, _)| *condition) else {
                return Some(Err(invalid("conditional has no alternative")));
            };
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathConditional {
                branches: Arc::from(branches),
                next: 0,
                alternative,
                input: work.input.clone(),
                origin: work.origin.clone(),
                prefix: work.prefix.clone(),
                environment: Arc::clone(&work.environment),
            });
            pending.push(GeneratorTask::Eval(GeneratorWork {
                node: condition,
                input: work.input,
                origin: work.origin.clone(),
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::TryCatch { expression, catch } => {
            let boundary = *next_boundary;
            *next_boundary = next_boundary.saturating_add(1);
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathCatch {
                boundary,
                node: catch,
                origin: work.origin.clone(),
                environment: Arc::clone(&work.environment),
            });
            let task = if path_candidate_with_aliases(bytecode, expression, &continuations) {
                GeneratorTask::PathEval(PathWork {
                    node: expression,
                    input: work.input,
                    origin: work.origin.clone(),
                    prefix: work.prefix,
                    environment: work.environment,
                    frames: work.frames,
                    continuations,
                })
            } else {
                GeneratorTask::Eval(GeneratorWork {
                    node: expression,
                    input: work.input,
                    origin: work.origin.clone(),
                    environment: work.environment,
                    frames: work.frames,
                    continuations,
                })
            };
            pending.push(task);
            None
        }
        Operation::ParameterCall {
            function,
            parameter,
        } => {
            let argument = work
                .frames
                .iter()
                .rev()
                .find(|frame| frame.symbol == function)
                .and_then(|frame| frame.filters.get(parameter as usize))
                .and_then(Clone::clone);
            let Some(argument) = argument else {
                return Some(Err(invalid(
                    "filter parameter missing from active user frame",
                )));
            };
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathUserReturn {
                environment: Arc::clone(&work.environment),
                frames: Arc::clone(&work.frames),
            });
            if path_candidate_with_aliases(bytecode, argument.node, &continuations) {
                pending.push(GeneratorTask::PathEval(PathWork {
                    node: argument.node,
                    input: work.input,
                    origin: work.origin.clone(),
                    prefix: work.prefix,
                    environment: argument.environment,
                    frames: argument.frames,
                    continuations,
                }));
            } else {
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: argument.node,
                    input: work.input,
                    origin: work.origin.clone(),
                    environment: argument.environment,
                    frames: argument.frames,
                    continuations,
                }));
            }
            None
        }
        Operation::BindAlternatives {
            value,
            patterns,
            body,
        } => {
            if !path_bind_candidate(bytecode, body) {
                return Some(Err(VmError::Unsupported {
                    operation: "path bind alternatives body".into(),
                }));
            }
            let boundary = *next_boundary;
            *next_boundary = next_boundary.saturating_add(1);
            let patterns: Arc<[BindingPatternOperand]> = Arc::from(patterns);
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathBindAlternatives {
                boundary,
                patterns,
                body,
                input: work.input.clone(),
                prefix: work.prefix.clone(),
                environment: Arc::clone(&work.environment),
            });
            if path_candidate_with_aliases(bytecode, value, &continuations) {
                pending.push(GeneratorTask::PathEval(PathWork {
                    node: value,
                    input: work.input,
                    origin: work.origin.clone(),
                    prefix: work.prefix,
                    environment: work.environment,
                    frames: work.frames,
                    continuations,
                }));
            } else {
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: value,
                    input: work.input,
                    origin: work.origin.clone(),
                    environment: work.environment,
                    frames: work.frames,
                    continuations,
                }));
            }
            None
        }
        Operation::Bind {
            value,
            pattern,
            body,
        } => {
            if path_candidate_with_aliases(bytecode, value, &work.continuations) {
                let mut continuations = work.continuations;
                continuations.push(GeneratorContinuation::PathBind {
                    pattern,
                    body,
                    input: work.input.clone(),
                    input_origin: work.origin.clone(),
                    prefix: work.prefix.clone(),
                    environment: Arc::clone(&work.environment),
                    entered_from_path: true,
                });
                pending.push(GeneratorTask::PathEval(PathWork {
                    node: value,
                    input: work.input,
                    origin: work.origin.clone(),
                    prefix: work.prefix,
                    environment: work.environment,
                    frames: work.frames,
                    continuations,
                }));
                return None;
            }
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathBindValue {
                pattern,
                body,
                input: work.input.clone(),
                input_origin: work.origin.clone(),
                prefix: work.prefix,
                environment: Arc::clone(&work.environment),
                entered_from_path: true,
            });
            pending.push(GeneratorTask::Eval(GeneratorWork {
                node: value,
                input: work.input,
                origin: work.origin.clone(),
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::UserCall { symbol, arguments } => {
            let Some(function) = bytecode.functions().get(symbol as usize) else {
                return Some(Err(invalid("user function missing after validation")));
            };
            if arguments.len() != function.parameters.len() {
                return Some(Err(invalid("user function arity changed after validation")));
            }
            let filters = arguments
                .iter()
                .zip(&function.parameters)
                .map(|(node, parameter)| {
                    (parameter.kind == ParameterKind::Filter).then(|| FilterArgument {
                        node: *node,
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                    })
                })
                .collect::<Vec<_>>();
            if let Some(next) = function
                .parameters
                .iter()
                .position(|parameter| parameter.kind == ParameterKind::Value)
            {
                let arguments: Arc<[u32]> = Arc::from(arguments);
                let from_path =
                    path_candidate_with_aliases(bytecode, arguments[next], &work.continuations);
                let mut continuations = work.continuations;
                continuations.push(GeneratorContinuation::PathUserArgument {
                    symbol,
                    arguments: Arc::clone(&arguments),
                    next,
                    input: work.input.clone(),
                    input_origin: work.origin.clone(),
                    prefix: work.prefix.clone(),
                    caller_environment: Arc::clone(&work.environment),
                    caller_frames: Arc::clone(&work.frames),
                    filters,
                    bindings: LexicalEnvironment::new(),
                    from_path,
                });
                if from_path {
                    pending.push(GeneratorTask::PathEval(PathWork {
                        node: arguments[next],
                        input: work.input,
                        origin: work.origin.clone(),
                        prefix: work.prefix,
                        environment: work.environment,
                        frames: work.frames,
                        continuations,
                    }));
                } else {
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: arguments[next],
                        input: work.input,
                        origin: work.origin.clone(),
                        environment: work.environment,
                        frames: work.frames,
                        continuations,
                    }));
                }
                return None;
            }
            let mut frames = work.frames.to_vec();
            frames.push(UserFrame { symbol, filters });
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathUserReturn {
                environment: Arc::clone(&work.environment),
                frames: Arc::clone(&work.frames),
            });
            pending.push(GeneratorTask::PathEval(PathWork {
                node: function.body,
                input: work.input,
                origin: work.origin.clone(),
                prefix: work.prefix,
                environment: work.environment,
                frames: frames.into(),
                continuations,
            }));
            None
        }
        Operation::Call { name, arguments } => {
            let Some(name) = bytecode.string(name) else {
                return Some(Err(invalid("path call name missing after validation")));
            };
            match name.as_ref() {
                "getpath" => {
                    let Some(argument) = arguments.first() else {
                        return Some(Err(invalid("getpath argument missing")));
                    };
                    if arguments.len() != 1 {
                        return Some(Err(invalid("getpath arity")));
                    }
                    let selected = match getpath(&work.input, work.prefix.components()) {
                        Ok(value) => value,
                        Err(error) => return Some(Err(error)),
                    };
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::PathGetPath {
                        base: work.prefix,
                        input: selected.clone(),
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: *argument,
                        input: selected,
                        origin: None,
                        environment: work.environment,
                        frames: work.frames,
                        continuations,
                    }));
                    None
                }
                "select" => {
                    let Some(argument) = arguments.first() else {
                        return Some(Err(invalid("select argument missing")));
                    };
                    let selected = match getpath(&work.input, work.prefix.components()) {
                        Ok(value) => value,
                        Err(error) => return Some(Err(error)),
                    };
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::PathSelect(work.prefix));
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: *argument,
                        input: selected,
                        origin: None,
                        environment: work.environment,
                        frames: work.frames,
                        continuations,
                    }));
                    None
                }
                "error" => {
                    let Some(argument) = arguments.first() else {
                        return Some(Err(raised(work.input)));
                    };
                    let selected = match getpath(&work.input, work.prefix.components()) {
                        Ok(value) => value,
                        Err(error) => return Some(Err(error)),
                    };
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::Raise);
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: *argument,
                        input: selected,
                        origin: None,
                        environment: work.environment,
                        frames: work.frames,
                        continuations,
                    }));
                    None
                }
                _ => Some(Err(runtime(
                    "assignment left side is not a path".to_owned(),
                ))),
            }
        }
        _ => Some(Err(runtime(
            "assignment left side is not a path".to_owned(),
        ))),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "slice path scheduling carries the shared managed continuation state"
)]
pub(super) fn schedule_path_slice(
    path: Path,
    target: Value,
    start: Option<f64>,
    end: Option<f64>,
    assignment: Option<Rc<RefCell<AssignmentState>>>,
    environment: Arc<LexicalEnvironment>,
    frames: UserFrames,
    continuations: Vec<GeneratorContinuation>,
    pending: &mut Vec<GeneratorTask>,
) -> Option<Result<Value, VmError>> {
    let Value::Array(values) = target else {
        return if matches!(target, Value::Null) {
            None
        } else {
            Some(Err(type_error("slice", &target)))
        };
    };
    let (start, end) = slice_bounds(values.len(), start, end);
    if start < end || assignment.is_some() {
        pending.push(GeneratorTask::PathSlice {
            path,
            start,
            end,
            next: start,
            assignment,
            environment,
            frames,
            continuations,
        });
    }
    None
}

#[allow(
    clippy::too_many_arguments,
    reason = "alternative binding scheduling carries the generator state needed for retry"
)]
pub(super) fn schedule_bind_alternative(
    bytecode: &Bytecode,
    patterns: Arc<[BindingPatternOperand]>,
    mut next: usize,
    value: Value,
    input: Value,
    environment: Arc<LexicalEnvironment>,
    body: u32,
    frames: UserFrames,
    mut continuations: Vec<GeneratorContinuation>,
    boundary: u64,
    pending: &mut Vec<GeneratorTask>,
    limits: VmLimits,
    cancellation: Option<&AtomicBool>,
    stop: &AtomicBool,
    observations: &mut VmObservations,
    path_mode: bool,
    prefix: Path,
) -> Result<(), VmError> {
    while let Some(pattern) = patterns.get(next) {
        let mut nested = environment.as_ref().clone();
        initialize_pattern_variables(bytecode, &patterns, &mut nested)?;
        let pattern_depth = continuations.len();
        let mut charge = |depth| {
            if stop.load(Ordering::Relaxed)
                || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
            {
                return Err(VmError::Interrupted);
            }
            if depth >= limits.call_stack {
                return Err(resource("call-stack"));
            }
            if observations.steps >= limits.steps {
                return Err(resource("vm-steps"));
            }
            observations.steps += 1;
            observations.call_stack_high_water = observations.call_stack_high_water.max(depth + 1);
            Ok(())
        };
        match bind_pattern(
            bytecode,
            pattern,
            &value,
            &mut nested,
            pattern_depth,
            &mut charge,
        ) {
            Ok(()) => {
                let body_input = input.clone();
                if path_mode {
                    continuations.push(GeneratorContinuation::PathBindAlternativeBody {
                        boundary,
                        patterns,
                        next: next.saturating_add(1),
                        body,
                        value,
                        input,
                        prefix: prefix.clone(),
                        environment,
                    });
                    pending.push(GeneratorTask::PathEval(PathWork {
                        node: body,
                        input: body_input,
                        origin: None,
                        prefix,
                        environment: Arc::new(nested),
                        frames,
                        continuations,
                    }));
                } else {
                    continuations.push(GeneratorContinuation::BindAlternativeBody {
                        boundary,
                        patterns,
                        next: next.saturating_add(1),
                        body,
                        value,
                        input,
                        environment,
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: body,
                        input: body_input,
                        origin: None,
                        environment: Arc::new(nested),
                        frames,
                        continuations,
                    }));
                }
                return Ok(());
            }
            Err(error) => {
                if !is_pattern_mismatch(&error) {
                    return Err(error);
                }
                next = next.saturating_add(1);
                if next >= patterns.len() {
                    return Err(error);
                }
            }
        }
    }
    Err(runtime(
        "destructuring alternative has no patterns".to_owned(),
    ))
}
