//! Schedule managed evaluation of calls, objects, and interpolation.

use super::managed_work::{
    FilterArgument, GeneratorContinuation, GeneratorTask, GeneratorWork, SqlInState, SqlIndexState,
    UserFrame,
};
use super::origins::{LexicalEnvironment, OriginToken};
use super::{UserFrames, invalid, iterate, resource, runtime, sql};
use crate::ast::ParameterKind;
use crate::bytecode::{InterpolationOperand, KeyOperand};
use crate::{Bytecode, Object, Value, VmError, VmLimits};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[allow(
    clippy::too_many_arguments,
    reason = "SQL scheduling carries the caller context needed by every callback phase"
)]
#[allow(
    clippy::too_many_lines,
    reason = "SQL call scheduling keeps the right-to-left callback phases together"
)]
pub(super) fn schedule_sql_call(
    name: &str,
    arguments: &[u32],
    input: Value,
    origin: Option<OriginToken>,
    environment: Arc<LexicalEnvironment>,
    frames: UserFrames,
    continuations: Vec<GeneratorContinuation>,
    limits: VmLimits,
    next_boundary: &mut u64,
    pending: &mut Vec<GeneratorTask>,
) -> Option<Result<Value, VmError>> {
    let boundary = *next_boundary;
    *next_boundary = next_boundary.saturating_add(1);
    match name {
        "IN" => {
            let Some(source) = arguments.first().copied() else {
                return Some(Err(invalid("IN argument missing")));
            };
            if arguments.len() > 2 {
                return Some(Err(invalid("IN accepts at most two arguments")));
            }
            let state = Rc::new(RefCell::new(SqlInState { matched: false }));
            pending.push(GeneratorTask::SqlInFinish {
                boundary,
                state: Rc::clone(&state),
                environment: Arc::clone(&environment),
                frames: Arc::clone(&frames),
                continuations: continuations.clone(),
            });
            let mut next_continuations = continuations;
            if let Some(needle) = arguments.get(1).copied() {
                next_continuations.push(GeneratorContinuation::SqlInRight {
                    boundary,
                    state,
                    source,
                    input: input.clone(),
                    origin: origin.clone(),
                    environment: Arc::clone(&environment),
                    frames: Arc::clone(&frames),
                });
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: needle,
                    input,
                    origin,
                    environment,
                    frames,
                    continuations: next_continuations,
                }));
            } else {
                next_continuations.push(GeneratorContinuation::SqlInLeft {
                    boundary,
                    state,
                    needle: input.clone(),
                });
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: source,
                    input,
                    origin,
                    environment,
                    frames,
                    continuations: next_continuations,
                }));
            }
            None
        }
        "INDEX" => {
            if !(1..=2).contains(&arguments.len()) {
                return Some(Err(invalid("INDEX expects one or two arguments")));
            }
            let key = *arguments.last().expect("INDEX arity was checked");
            let cursor =
                (arguments.len() == 1).then(|| iterate::ContainerCursor::new(input.clone()));
            if cursor.as_ref().is_some_and(|cursor| !cursor.is_container()) {
                return Some(Err(runtime(format!("Cannot iterate over {input}"))));
            }
            let state = Rc::new(RefCell::new(SqlIndexState {
                builder: Some(sql::SqlIndexBuilder::new(limits)),
            }));
            pending.push(GeneratorTask::SqlIndexFinish {
                boundary,
                state: Rc::clone(&state),
                environment: Arc::clone(&environment),
                frames: Arc::clone(&frames),
                continuations: continuations.clone(),
            });
            let mut next_continuations = continuations;
            next_continuations.push(GeneratorContinuation::SqlIndexRow {
                boundary,
                state,
                key,
            });
            if let Some(cursor) = cursor {
                pending.push(GeneratorTask::IterateCursor {
                    cursor,
                    origin,
                    environment,
                    frames,
                    continuations: next_continuations,
                });
            } else {
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: arguments[0],
                    input,
                    origin,
                    environment,
                    frames,
                    continuations: next_continuations,
                }));
            }
            None
        }
        "JOIN" => {
            if !(2..=4).contains(&arguments.len()) {
                return Some(Err(invalid("JOIN expects two to four arguments")));
            }
            let key = if arguments.len() == 2 {
                arguments[1]
            } else {
                arguments[2]
            };
            let stream = arguments.get(1).copied().filter(|_| arguments.len() >= 3);
            let join = arguments.get(3).copied();
            let index_input = input.clone();
            let index_environment = Arc::clone(&environment);
            let index_frames = Arc::clone(&frames);
            let mut next_continuations = continuations;
            next_continuations.push(GeneratorContinuation::SqlJoinIndex {
                boundary,
                key,
                stream,
                join,
                input,
                origin: origin.clone(),
                environment,
                frames,
            });
            pending.push(GeneratorTask::Eval(GeneratorWork {
                node: arguments[0],
                input: index_input,
                origin,
                environment: index_environment,
                frames: index_frames,
                continuations: next_continuations,
            }));
            None
        }
        _ => Some(Err(invalid("unsupported SQL builtin"))),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "object construction carries each branch's bounded continuation state explicitly"
)]
pub(super) fn schedule_object_entry(
    bytecode: &Bytecode,
    entries: &Arc<[crate::bytecode::ObjectOperand]>,
    next: usize,
    object: Arc<Object>,
    input: Value,
    origin: Option<OriginToken>,
    environment: Arc<LexicalEnvironment>,
    frames: UserFrames,
    mut continuations: Vec<GeneratorContinuation>,
    pending: &mut Vec<GeneratorTask>,
) -> Option<Result<Value, VmError>> {
    let Some(entry) = entries.get(next) else {
        return Some(Ok(Value::Object(object)));
    };
    match &entry.key {
        KeyOperand::Static(index) => {
            let Some(key) = bytecode.string(*index) else {
                return Some(Err(invalid("object key string missing after validation")));
            };
            continuations.push(GeneratorContinuation::ObjectValue {
                entries: Arc::clone(entries),
                next: next.saturating_add(1),
                object,
                key: Arc::clone(key),
                input: input.clone(),
                origin: origin.clone(),
                environment: Arc::clone(&environment),
                frames: Arc::clone(&frames),
            });
            pending.push(GeneratorTask::Eval(GeneratorWork {
                node: entry.value,
                input,
                origin,
                environment,
                frames,
                continuations,
            }));
        }
        KeyOperand::Computed(node) => {
            continuations.push(GeneratorContinuation::ObjectKey {
                entries: Arc::clone(entries),
                next,
                object,
                input: input.clone(),
                origin: origin.clone(),
                environment: Arc::clone(&environment),
                frames: Arc::clone(&frames),
            });
            pending.push(GeneratorTask::Eval(GeneratorWork {
                node: *node,
                input,
                origin,
                environment,
                frames,
                continuations,
            }));
        }
    }
    None
}
#[allow(
    clippy::too_many_arguments,
    reason = "user-call scheduling keeps every captured VM resource explicit"
)]
pub(super) fn schedule_user_call(
    symbol: u32,
    arguments: Arc<[u32]>,
    mut next: usize,
    input: Value,
    input_origin: Option<OriginToken>,
    caller_environment: Arc<LexicalEnvironment>,
    caller_frames: UserFrames,
    mut filters: Vec<Option<FilterArgument>>,
    bindings: LexicalEnvironment,
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
                    input_origin: input_origin.clone(),
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
                    origin: input_origin.clone(),
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
        origin: input_origin,
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
pub(super) fn schedule_interpolation(
    segments: &Arc<[InterpolationOperand]>,
    mut next: usize,
    mut pieces: Vec<Option<Arc<str>>>,
    input: Value,
    environment: Arc<LexicalEnvironment>,
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
                    origin: None,
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

pub(super) fn interpolation_pieces(
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

pub(super) fn interpolation_capacity(
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

pub(super) fn interpolation_remaining(
    pieces: &[Option<Arc<str>>],
    output_limit: usize,
) -> Result<usize, VmError> {
    interpolation_capacity(pieces, output_limit).map(|used| output_limit - used)
}
