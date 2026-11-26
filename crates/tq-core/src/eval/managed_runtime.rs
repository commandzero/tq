//! Run managed tasks, bytecode instructions, and continuations in evaluation order.

use super::admission::{builtin_argument_order, regex_builtin_shape, supports_builtin_call};
use super::builtin_effects::{
    append_effect, debug_effect, generator_builtin, halt_error_text, halt_status, module_metadata,
};
use super::managed_paths::{
    PathTraversalCursor, PathWork, clear_path_aliases, current_path_aliases,
    path_candidate_with_aliases, schedule_bind_alternative, schedule_path_eval,
    schedule_path_slice,
};
use super::managed_work::{
    AddState, AssignmentState, AssignmentTarget, FoldState, GeneratorContinuation, GeneratorTask,
    GeneratorWork, KeyedCollectionMode, ManagedKeyedValue, PathCollectionKind, PullConsumerKind,
    PullConsumerState, SqlJoinState, TraversalCursor, UserFrame, recursive_child, regex_error_work,
    task_owned_by_assignment_rhs, task_owned_by_limit, task_owned_by_pull_consumer,
    task_owned_by_pull_consumer_work, task_owned_by_sql,
};
use super::ordered::project_managed_keyed_values;
use super::origins::{
    LexicalEnvironment, OriginToken, PathAlias, fresh_origin, getpath_managed,
    origin_at_path_bounded, record_pattern_aliases, record_pattern_origin, relative_alias_path,
    scalar_result_origin,
};
use super::schedule::{
    interpolation_pieces, interpolation_remaining, schedule_interpolation, schedule_object_entry,
    schedule_sql_call, schedule_user_call,
};
use super::structured_paths::{
    apply_assignment_target, apply_plain_assignment, assignment_target_value, bind_pattern,
    jq_path, path_component_for_target, path_value, retain_assignment_deletion_target,
    update_value,
};
use super::unwind::ErrorUnwind;
use super::value_ops::{
    access_field, access_index, binary_value, computed_key, has, index_origin, nth_index,
    regex_pattern_argument, slice, slice_bound_value, type_error, unary,
};
use super::{
    Environment, Evaluation, INPUT_FILENAME, INPUT_LINE_NUMBER, ambient_platform, ambient_value,
    capability_denied, charge_managed_step, error_message, generator, input_context_value, invalid,
    iterate, limit_count, path_builtin, resource, runtime, scalar, sql, variable_value,
};
use crate::ast::{AssignmentOperator, BinaryOperator, ParameterKind};
use crate::bytecode::{InterpolationOperand, Operation};
use crate::{
    InputCursor, Number, Object, Path, PathComponent, Value, VmError, VmObservations, format,
    stdlib,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[allow(
    clippy::too_many_lines,
    reason = "keep the hot task, instruction, and continuation dispatch on the same stack frame"
)]
pub(super) fn evaluate(
    evaluation: Evaluation<'_>,
    input: &Value,
    variables: &Arc<Environment>,
    mut emit: impl FnMut(Result<Value, VmError>, VmObservations) -> bool,
) -> VmObservations {
    let Evaluation {
        bytecode,
        limits,
        cancellation,
        stop,
        input_cursor,
        effects,
    } = evaluation;
    let environment = Arc::new(LexicalEnvironment::from_values(Arc::clone(variables)));
    let mut next_origin = 1_u64;
    // Immediate evaluation does not need the suspended-task queue's larger payload or allocation.
    let mut current = Some(GeneratorWork {
        node: bytecode.root(),
        input: input.clone(),
        origin: Some(OriginToken {
            root: 0,
            path: Path::root(),
        }),
        environment,
        frames: Arc::from([]),
        continuations: Vec::new(),
    });
    let mut pending = Vec::new();
    let mut observations = VmObservations::default();
    let mut next_boundary = 0_u64;

    'generator: while current.is_some() || !pending.is_empty() {
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
        if limits.value_stack == 0 {
            let _ = emit(Err(resource("value-stack")), observations);
            break;
        }
        observations.steps += 1;
        let mut delivered_from_path = false;
        let (mut work, delivered) = if let Some(work) = current.take() {
            (work, None)
        } else {
            match pending.pop().expect("pending work is present") {
                GeneratorTask::Eval(work) => (work, None),
                GeneratorTask::InvokeBuiltin {
                    name,
                    arguments,
                    source_arity,
                    input,
                    origin,
                    environment,
                    frames,
                    continuations,
                } => match generator::start(&name, &input, &arguments, source_arity, limits) {
                    Ok(Some(state)) => {
                        pending.push(GeneratorTask::BuiltinGenerator {
                            state,
                            environment,
                            frames,
                            continuations,
                        });
                        continue;
                    }
                    Err(error) => (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Err(error)),
                    ),
                    Ok(None) => {
                        let mut charge = || {
                            if stop.load(Ordering::Relaxed)
                                || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                            {
                                return Err(VmError::Interrupted);
                            }
                            if observations.steps >= limits.steps {
                                return Err(resource("vm-steps"));
                            }
                            observations.steps += 1;
                            Ok(())
                        };
                        let (scalar_result, getpath_path): (
                            Result<Option<Value>, VmError>,
                            Option<Option<Path>>,
                        ) = if name.as_ref() == "getpath" {
                            match arguments.first() {
                                None => (Err(invalid("getpath argument missing")), Some(None)),
                                Some(path_value) => {
                                    match scalar::bounded_getpath_resolution(
                                        &input,
                                        path_value,
                                        limits,
                                        &mut charge,
                                    ) {
                                        Ok(resolution) => {
                                            (Ok(Some(resolution.value)), Some(resolution.path))
                                        }
                                        Err(error) => (Err(error), Some(None)),
                                    }
                                }
                            }
                        } else {
                            let result = if scalar::supports_ambient(&name, arguments.len()) {
                                scalar::evaluate_ambient(
                                    &name,
                                    &input,
                                    &arguments,
                                    &environment,
                                    limits,
                                    &mut charge,
                                )
                            } else {
                                scalar::evaluate(&name, &input, &arguments, limits, &mut charge)
                            };
                            (result, None)
                        };
                        match scalar_result {
                            Ok(Some(value)) => {
                                let origin_result = match getpath_path {
                                    Some(Some(path)) => origin_at_path_bounded(
                                        origin.as_ref(),
                                        &path,
                                        limits,
                                        &mut charge,
                                    ),
                                    Some(None) => fresh_origin(&mut next_origin).map(Some),
                                    None => scalar_result_origin(
                                        &name,
                                        &input,
                                        &arguments,
                                        origin,
                                        &mut next_origin,
                                    ),
                                };
                                let origin = match origin_result {
                                    Ok(origin) => origin,
                                    Err(error) => {
                                        return {
                                            let _ = emit(Err(error), observations);
                                            observations
                                        };
                                    }
                                };
                                (
                                    GeneratorWork {
                                        node: bytecode.root(),
                                        input: Value::Null,
                                        origin,
                                        environment,
                                        frames,
                                        continuations,
                                    },
                                    Some(Ok(value)),
                                )
                            }
                            Ok(None) => continue,
                            Err(error) => (
                                GeneratorWork {
                                    node: bytecode.root(),
                                    input: Value::Null,
                                    origin: None,
                                    environment,
                                    frames,
                                    continuations,
                                },
                                Some(Err(error)),
                            ),
                        }
                    }
                },
                GeneratorTask::FinishBuiltinArguments {
                    name,
                    source_arity,
                    values,
                    input,
                    origin,
                    environment,
                    frames,
                    continuations,
                } => {
                    pending.push(GeneratorTask::InvokeBuiltin {
                        name,
                        arguments: values.take(),
                        source_arity,
                        input,
                        origin,
                        environment,
                        frames,
                        continuations,
                    });
                    continue;
                }
                GeneratorTask::SqlInFinish {
                    boundary: _,
                    state,
                    environment,
                    frames,
                    continuations,
                } => {
                    let value = Value::Bool(state.borrow().matched);
                    let (origin, result) = match fresh_origin(&mut next_origin) {
                        Ok(origin) => (Some(origin), Ok(value)),
                        Err(error) => (None, Err(error)),
                    };
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(result),
                    )
                }
                GeneratorTask::SqlIndexFinish {
                    boundary: _,
                    state,
                    environment,
                    frames,
                    continuations,
                } => {
                    let result = state
                        .borrow_mut()
                        .builder
                        .take()
                        .map(sql::SqlIndexBuilder::finish)
                        .ok_or_else(|| invalid("SQL INDEX finished more than once"));
                    let (origin, result) = match result.and_then(|value| {
                        fresh_origin(&mut next_origin).map(|origin| (Some(origin), Ok(value)))
                    }) {
                        Ok((origin, result)) => (origin, result),
                        Err(error) => (None, Err(error)),
                    };
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(result),
                    )
                }
                GeneratorTask::SqlJoinFinish {
                    boundary: _,
                    state,
                    environment,
                    frames,
                    continuations,
                } => {
                    let result = state
                        .borrow_mut()
                        .collector
                        .take()
                        .map(sql::SqlPairCollector::finish)
                        .ok_or_else(|| invalid("SQL JOIN finished more than once"));
                    let (origin, result) = match result.and_then(|value| {
                        fresh_origin(&mut next_origin).map(|origin| (Some(origin), Ok(value)))
                    }) {
                        Ok((origin, result)) => (origin, result),
                        Err(error) => (None, Err(error)),
                    };
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(result),
                    )
                }
                GeneratorTask::InvokeRegex {
                    name,
                    arguments,
                    replacement,
                    input,
                    origin: _origin,
                    environment,
                    frames,
                    continuations,
                } => 'regex: {
                    let flags_index = usize::from(matches!(name.as_ref(), "sub" | "gsub")) + 1;
                    let Some(pattern_value) = arguments.first() else {
                        break 'regex regex_error_work(
                            bytecode,
                            &environment,
                            &frames,
                            &continuations,
                            invalid("regex pattern argument missing"),
                        );
                    };
                    let array_allowed = matches!(name.as_ref(), "test" | "match" | "capture")
                        && arguments.get(flags_index).is_none();
                    let (pattern, array_flags) =
                        match regex_pattern_argument(&name, pattern_value, array_allowed) {
                            Ok(parsed) => parsed,
                            Err(error) => {
                                break 'regex regex_error_work(
                                    bytecode,
                                    &environment,
                                    &frames,
                                    &continuations,
                                    error,
                                );
                            }
                        };
                    let flags = match array_flags {
                        Some(flags) => Ok(flags),
                        None => match arguments.get(flags_index) {
                            None | Some(Value::Null) => Ok(Arc::from("")),
                            Some(Value::String(flags)) => Ok(Arc::clone(flags)),
                            Some(value) => Err(type_error(&name, value)),
                        },
                    };
                    let flags = match flags {
                        Ok(flags) => flags,
                        Err(error) => {
                            break 'regex regex_error_work(
                                bytecode,
                                &environment,
                                &frames,
                                &continuations,
                                error,
                            );
                        }
                    };
                    let Value::String(input) = &input else {
                        break 'regex regex_error_work(
                            bytecode,
                            &environment,
                            &frames,
                            &continuations,
                            type_error(&name, &input),
                        );
                    };
                    let checkpoint_steps = Cell::new(observations.steps);
                    let checkpoint = || {
                        if stop.load(Ordering::Relaxed)
                            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                        {
                            return Err(VmError::Interrupted);
                        }
                        if checkpoint_steps.get() >= limits.steps {
                            return Err(resource("vm-steps"));
                        }
                        checkpoint_steps.set(checkpoint_steps.get().saturating_add(1));
                        Ok(())
                    };
                    match name.as_ref() {
                        "test" => {
                            let test =
                                stdlib::regex_test(input, &pattern, &flags, limits, &checkpoint);
                            observations.steps = checkpoint_steps.get();
                            let value = match test {
                                Ok(value) => value,
                                Err(error) => {
                                    break 'regex regex_error_work(
                                        bytecode,
                                        &environment,
                                        &frames,
                                        &continuations,
                                        error,
                                    );
                                }
                            };
                            let origin = match fresh_origin(&mut next_origin) {
                                Ok(origin) => Some(origin),
                                Err(error) => {
                                    break 'regex regex_error_work(
                                        bytecode,
                                        &environment,
                                        &frames,
                                        &continuations,
                                        error,
                                    );
                                }
                            };
                            (
                                GeneratorWork {
                                    node: bytecode.root(),
                                    input: Value::Null,
                                    origin,
                                    environment,
                                    frames,
                                    continuations,
                                },
                                Some(Ok(value)),
                            )
                        }
                        "match" | "capture" | "scan" => {
                            let kind = match name.as_ref() {
                                "match" => stdlib::RegexPullKind::Match,
                                "capture" => stdlib::RegexPullKind::Capture,
                                _ => stdlib::RegexPullKind::Scan,
                            };
                            match stdlib::regex_cursor(
                                Arc::clone(input),
                                &pattern,
                                &flags,
                                kind,
                                name.as_ref() == "scan",
                                limits,
                            ) {
                                Ok(cursor) => {
                                    pending.push(GeneratorTask::RegexCursor {
                                        cursor: Rc::new(RefCell::new(cursor)),
                                        environment,
                                        frames,
                                        continuations,
                                    });
                                    continue 'generator;
                                }
                                Err(error) => {
                                    break 'regex regex_error_work(
                                        bytecode,
                                        &environment,
                                        &frames,
                                        &continuations,
                                        error,
                                    );
                                }
                            }
                        }
                        "splits" => {
                            match stdlib::regex_cursor(
                                Arc::clone(input),
                                &pattern,
                                &flags,
                                stdlib::RegexPullKind::Span,
                                true,
                                limits,
                            ) {
                                Ok(cursor) => {
                                    pending.push(GeneratorTask::RegexSplitCursor {
                                        cursor: Rc::new(RefCell::new(cursor)),
                                        input: Arc::clone(input),
                                        copied: 0,
                                        emitted_tail: false,
                                        environment,
                                        frames,
                                        continuations,
                                    });
                                    continue 'generator;
                                }
                                Err(error) => {
                                    break 'regex regex_error_work(
                                        bytecode,
                                        &environment,
                                        &frames,
                                        &continuations,
                                        error,
                                    );
                                }
                            }
                        }
                        "split" => {
                            let split = stdlib::regex_split(
                                input,
                                &pattern,
                                &flags,
                                false,
                                arguments.get(flags_index).is_none(),
                                limits,
                                &checkpoint,
                            );
                            observations.steps = checkpoint_steps.get();
                            match split {
                                Ok(values) => {
                                    let origin = match fresh_origin(&mut next_origin) {
                                        Ok(origin) => Some(origin),
                                        Err(error) => {
                                            break 'regex regex_error_work(
                                                bytecode,
                                                &environment,
                                                &frames,
                                                &continuations,
                                                error,
                                            );
                                        }
                                    };
                                    (
                                        GeneratorWork {
                                            node: bytecode.root(),
                                            input: Value::Null,
                                            origin,
                                            environment,
                                            frames,
                                            continuations,
                                        },
                                        Some(Ok(values
                                            .into_iter()
                                            .next()
                                            .unwrap_or_else(|| Value::array(Vec::new())))),
                                    )
                                }
                                Err(error) => {
                                    break 'regex regex_error_work(
                                        bytecode,
                                        &environment,
                                        &frames,
                                        &continuations,
                                        error,
                                    );
                                }
                            }
                        }
                        "sub" | "gsub" => {
                            let Some(replacement) = replacement else {
                                break 'regex regex_error_work(
                                    bytecode,
                                    &environment,
                                    &frames,
                                    &continuations,
                                    invalid("regex replacement argument missing"),
                                );
                            };
                            match stdlib::regex_substitution(
                                Arc::clone(input),
                                &pattern,
                                &flags,
                                name.as_ref() == "gsub",
                                limits,
                            ) {
                                Ok(state) => {
                                    pending.push(GeneratorTask::RegexSubstitution {
                                        name: Arc::clone(&name),
                                        state: Rc::new(RefCell::new(state)),
                                        replacement,
                                        environment,
                                        frames,
                                        continuations,
                                    });
                                    continue 'generator;
                                }
                                Err(error) => {
                                    break 'regex regex_error_work(
                                        bytecode,
                                        &environment,
                                        &frames,
                                        &continuations,
                                        error,
                                    );
                                }
                            }
                        }
                        _ => unreachable!("regex dispatch is exhaustive"),
                    }
                }
                GeneratorTask::RegexCursor {
                    cursor,
                    environment,
                    frames,
                    continuations,
                } => {
                    let checkpoint_steps = Cell::new(observations.steps);
                    let checkpoint = || {
                        if stop.load(Ordering::Relaxed)
                            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                        {
                            return Err(VmError::Interrupted);
                        }
                        if checkpoint_steps.get() >= limits.steps {
                            return Err(resource("vm-steps"));
                        }
                        checkpoint_steps.set(checkpoint_steps.get().saturating_add(1));
                        Ok(())
                    };
                    let pull = cursor.borrow_mut().next(&checkpoint);
                    observations.steps = checkpoint_steps.get();
                    match pull {
                        Ok(Some(pull)) => {
                            let value = match pull {
                                stdlib::RegexPull::Match { value }
                                | stdlib::RegexPull::Capture { value }
                                | stdlib::RegexPull::Scan { value } => value,
                                stdlib::RegexPull::Replacement { .. }
                                | stdlib::RegexPull::Span { .. } => {
                                    let _ = emit(
                                        Err(invalid("unexpected regex cursor result")),
                                        observations,
                                    );
                                    return observations;
                                }
                            };
                            let origin = match fresh_origin(&mut next_origin) {
                                Ok(origin) => Some(origin),
                                Err(error) => {
                                    let _ = emit(Err(error), observations);
                                    return observations;
                                }
                            };
                            pending.push(GeneratorTask::RegexCursor {
                                cursor,
                                environment: Arc::clone(&environment),
                                frames: Arc::clone(&frames),
                                continuations: continuations.clone(),
                            });
                            (
                                GeneratorWork {
                                    node: bytecode.root(),
                                    input: Value::Null,
                                    origin,
                                    environment,
                                    frames,
                                    continuations,
                                },
                                Some(Ok(value)),
                            )
                        }
                        Ok(None) => continue,
                        Err(error) => (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin: None,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Err(error)),
                        ),
                    }
                }
                GeneratorTask::RegexSplitCursor {
                    cursor,
                    input,
                    copied,
                    emitted_tail,
                    environment,
                    frames,
                    continuations,
                } => {
                    if emitted_tail {
                        continue;
                    }
                    let checkpoint_steps = Cell::new(observations.steps);
                    let checkpoint = || {
                        if stop.load(Ordering::Relaxed)
                            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                        {
                            return Err(VmError::Interrupted);
                        }
                        if checkpoint_steps.get() >= limits.steps {
                            return Err(resource("vm-steps"));
                        }
                        checkpoint_steps.set(checkpoint_steps.get().saturating_add(1));
                        Ok(())
                    };
                    let pull = cursor.borrow_mut().next(&checkpoint);
                    observations.steps = checkpoint_steps.get();
                    match pull {
                        Ok(Some(stdlib::RegexPull::Span { start, end })) => 'split: {
                            let value = match stdlib::regex_split_piece(
                                &input[copied..start],
                                limits,
                                &checkpoint,
                            ) {
                                Ok(value) => value,
                                Err(error) => {
                                    observations.steps = checkpoint_steps.get();
                                    break 'split regex_error_work(
                                        bytecode,
                                        &environment,
                                        &frames,
                                        &continuations,
                                        error,
                                    );
                                }
                            };
                            observations.steps = checkpoint_steps.get();
                            match fresh_origin(&mut next_origin) {
                                Ok(origin) => {
                                    pending.push(GeneratorTask::RegexSplitCursor {
                                        cursor,
                                        input: Arc::clone(&input),
                                        copied: end,
                                        emitted_tail: false,
                                        environment: Arc::clone(&environment),
                                        frames: Arc::clone(&frames),
                                        continuations: continuations.clone(),
                                    });
                                    (
                                        GeneratorWork {
                                            node: bytecode.root(),
                                            input: Value::Null,
                                            origin: Some(origin),
                                            environment,
                                            frames,
                                            continuations,
                                        },
                                        Some(Ok(value)),
                                    )
                                }
                                Err(error) => regex_error_work(
                                    bytecode,
                                    &environment,
                                    &frames,
                                    &continuations,
                                    error,
                                ),
                            }
                        }
                        Ok(None) => 'split: {
                            let value = match stdlib::regex_split_piece(
                                &input[copied..],
                                limits,
                                &checkpoint,
                            ) {
                                Ok(value) => value,
                                Err(error) => {
                                    observations.steps = checkpoint_steps.get();
                                    break 'split regex_error_work(
                                        bytecode,
                                        &environment,
                                        &frames,
                                        &continuations,
                                        error,
                                    );
                                }
                            };
                            observations.steps = checkpoint_steps.get();
                            match fresh_origin(&mut next_origin) {
                                Ok(origin) => {
                                    pending.push(GeneratorTask::RegexSplitCursor {
                                        cursor,
                                        input: Arc::clone(&input),
                                        copied,
                                        emitted_tail: true,
                                        environment: Arc::clone(&environment),
                                        frames: Arc::clone(&frames),
                                        continuations: continuations.clone(),
                                    });
                                    (
                                        GeneratorWork {
                                            node: bytecode.root(),
                                            input: Value::Null,
                                            origin: Some(origin),
                                            environment,
                                            frames,
                                            continuations,
                                        },
                                        Some(Ok(value)),
                                    )
                                }
                                Err(error) => regex_error_work(
                                    bytecode,
                                    &environment,
                                    &frames,
                                    &continuations,
                                    error,
                                ),
                            }
                        }
                        Ok(Some(_)) => regex_error_work(
                            bytecode,
                            &environment,
                            &frames,
                            &continuations,
                            invalid("unexpected regex split cursor result"),
                        ),
                        Err(error) => {
                            regex_error_work(bytecode, &environment, &frames, &continuations, error)
                        }
                    }
                }
                GeneratorTask::RegexSubstitution {
                    name,
                    state,
                    replacement,
                    environment,
                    frames,
                    continuations,
                } => {
                    let checkpoint_steps = Cell::new(observations.steps);
                    let checkpoint = || {
                        if stop.load(Ordering::Relaxed)
                            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                        {
                            return Err(VmError::Interrupted);
                        }
                        if checkpoint_steps.get() >= limits.steps {
                            return Err(resource("vm-steps"));
                        }
                        checkpoint_steps.set(checkpoint_steps.get().saturating_add(1));
                        Ok(())
                    };
                    let next = state.borrow_mut().next(&checkpoint);
                    observations.steps = checkpoint_steps.get();
                    match next {
                        Ok(Some(stdlib::RegexSubstitutionEvent::Replace { context })) => {
                            pending.push(GeneratorTask::RegexReplacementFinish {
                                name: Arc::clone(&name),
                                state: Rc::clone(&state),
                                replacement,
                                environment: Arc::clone(&environment),
                                frames: Arc::clone(&frames),
                                continuations: continuations.clone(),
                            });
                            let mut replacement_continuations = continuations;
                            replacement_continuations.push(
                                GeneratorContinuation::RegexReplacementItem {
                                    name: Arc::clone(&name),
                                    state,
                                },
                            );
                            pending.push(GeneratorTask::Eval(GeneratorWork {
                                node: replacement,
                                input: context,
                                origin: None,
                                environment,
                                frames,
                                continuations: replacement_continuations,
                            }));
                            continue;
                        }
                        Ok(Some(stdlib::RegexSubstitutionEvent::Output(value))) => {
                            let origin = match fresh_origin(&mut next_origin) {
                                Ok(origin) => Some(origin),
                                Err(error) => {
                                    let _ = emit(Err(error), observations);
                                    return observations;
                                }
                            };
                            pending.push(GeneratorTask::RegexSubstitution {
                                name,
                                state,
                                replacement,
                                environment: Arc::clone(&environment),
                                frames: Arc::clone(&frames),
                                continuations: continuations.clone(),
                            });
                            (
                                GeneratorWork {
                                    node: bytecode.root(),
                                    input: Value::Null,
                                    origin,
                                    environment,
                                    frames,
                                    continuations,
                                },
                                Some(Ok(value)),
                            )
                        }
                        Ok(None) => continue,
                        Err(error) => (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin: None,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Err(error)),
                        ),
                    }
                }
                GeneratorTask::RegexReplacementFinish {
                    name,
                    state,
                    replacement,
                    environment,
                    frames,
                    continuations,
                } => {
                    let checkpoint_steps = Cell::new(observations.steps);
                    let checkpoint = || {
                        if stop.load(Ordering::Relaxed)
                            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                        {
                            return Err(VmError::Interrupted);
                        }
                        if checkpoint_steps.get() >= limits.steps {
                            return Err(resource("vm-steps"));
                        }
                        checkpoint_steps.set(checkpoint_steps.get().saturating_add(1));
                        Ok(())
                    };
                    let finish = state.borrow_mut().finish_replacement_stream(&checkpoint);
                    observations.steps = checkpoint_steps.get();
                    match finish {
                        Ok(()) => {
                            pending.push(GeneratorTask::RegexSubstitution {
                                name,
                                state,
                                replacement,
                                environment,
                                frames,
                                continuations,
                            });
                            continue;
                        }
                        Err(error) => (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin: None,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Err(error)),
                        ),
                    }
                }
                GeneratorTask::BuiltinGenerator {
                    mut state,
                    environment,
                    frames,
                    continuations,
                    ..
                } => {
                    let mut charge = || {
                        if stop.load(Ordering::Relaxed)
                            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                        {
                            return Err(VmError::Interrupted);
                        }
                        if observations.steps >= limits.steps {
                            return Err(resource("vm-steps"));
                        }
                        observations.steps += 1;
                        Ok(())
                    };
                    match state.next(&mut charge) {
                        Ok(Some(value)) => match fresh_origin(&mut next_origin) {
                            Ok(origin) => {
                                pending.push(GeneratorTask::BuiltinGenerator {
                                    state,
                                    environment: Arc::clone(&environment),
                                    frames: Arc::clone(&frames),
                                    continuations: continuations.clone(),
                                });
                                (
                                    GeneratorWork {
                                        node: bytecode.root(),
                                        input: Value::Null,
                                        origin: Some(origin),
                                        environment,
                                        frames,
                                        continuations,
                                    },
                                    Some(Ok(value)),
                                )
                            }
                            Err(error) => (
                                GeneratorWork {
                                    node: bytecode.root(),
                                    input: Value::Null,
                                    origin: None,
                                    environment,
                                    frames,
                                    continuations,
                                },
                                Some(Err(error)),
                            ),
                        },
                        Ok(None) => continue,
                        Err(error) => (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin: None,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Err(error)),
                        ),
                    }
                }
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
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Ok(value)),
                    )
                }
                GeneratorTask::IterateCursor {
                    mut cursor,
                    origin,
                    environment,
                    frames,
                    continuations,
                } => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    match cursor.next_child(&mut charge) {
                        Ok(Some((value, component))) => {
                            match origin
                                .as_ref()
                                .map(|origin| origin.child_bounded(component, limits, &mut charge))
                                .transpose()
                            {
                                Ok(child_origin) => {
                                    pending.push(GeneratorTask::IterateCursor {
                                        cursor,
                                        origin,
                                        environment: Arc::clone(&environment),
                                        frames: Arc::clone(&frames),
                                        continuations: continuations.clone(),
                                    });
                                    (
                                        GeneratorWork {
                                            node: bytecode.root(),
                                            input: Value::Null,
                                            origin: child_origin,
                                            environment,
                                            frames,
                                            continuations,
                                        },
                                        Some(Ok(value)),
                                    )
                                }
                                Err(error) => (
                                    GeneratorWork {
                                        node: bytecode.root(),
                                        input: Value::Null,
                                        origin: None,
                                        environment,
                                        frames,
                                        continuations,
                                    },
                                    Some(Err(error)),
                                ),
                            }
                        }
                        Ok(None) => continue,
                        Err(error) => (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin: None,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Err(error)),
                        ),
                    }
                }
                GeneratorTask::InputValues {
                    cursor,
                    environment,
                    frames,
                    continuations,
                } => match cursor.next_value() {
                    Ok(Some(value)) => match fresh_origin(&mut next_origin) {
                        Ok(origin) => {
                            pending.push(GeneratorTask::InputValues {
                                cursor,
                                environment: Arc::clone(&environment),
                                frames: Arc::clone(&frames),
                                continuations: continuations.clone(),
                            });
                            (
                                GeneratorWork {
                                    node: bytecode.root(),
                                    input: Value::Null,
                                    origin: Some(origin),
                                    environment,
                                    frames,
                                    continuations,
                                },
                                Some(Ok(value)),
                            )
                        }
                        Err(error) => (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin: None,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Err(error)),
                        ),
                    },
                    Ok(None) => continue,
                    Err(error) => (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Err(error)),
                    ),
                },
                GeneratorTask::FinishArray {
                    values,
                    environment,
                    frames,
                    continuations,
                } => {
                    let (origin, result) = match fresh_origin(&mut next_origin) {
                        Ok(origin) => (Some(origin), Ok(Value::array(values.take()))),
                        Err(error) => (None, Err(error)),
                    };
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(result),
                    )
                }
                GeneratorTask::MapArray {
                    inputs,
                    next,
                    argument,
                    values,
                    first_only,
                    environment,
                    frames,
                    continuations,
                } => {
                    if let Some(value) = inputs.get(next) {
                        pending.push(GeneratorTask::MapArray {
                            inputs: Arc::clone(&inputs),
                            next: next + 1,
                            argument,
                            values: Rc::clone(&values),
                            first_only,
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: continuations.clone(),
                        });
                        let mut callback_continuations = continuations;
                        if first_only {
                            callback_continuations.push(GeneratorContinuation::IgnoreError);
                            callback_continuations.push(GeneratorContinuation::ArrayUpdateItem {
                                values,
                                accepted: Rc::new(Cell::new(false)),
                            });
                        } else {
                            callback_continuations.push(GeneratorContinuation::ArrayItem(values));
                        }
                        (
                            GeneratorWork {
                                node: argument,
                                input: value.clone(),
                                origin: None,
                                environment,
                                frames,
                                continuations: callback_continuations,
                            },
                            None,
                        )
                    } else {
                        let (origin, result) = match fresh_origin(&mut next_origin) {
                            Ok(origin) => (Some(origin), Ok(Value::array(values.take()))),
                            Err(error) => (None, Err(error)),
                        };
                        (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(result),
                        )
                    }
                }
                GeneratorTask::MapObject {
                    entries,
                    next,
                    argument,
                    values,
                    environment,
                    frames,
                    continuations,
                } => {
                    if let Some((key, value)) = entries.get(next) {
                        pending.push(GeneratorTask::MapObject {
                            entries: Arc::clone(&entries),
                            next: next + 1,
                            argument,
                            values: Rc::clone(&values),
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: continuations.clone(),
                        });
                        let mut callback_continuations = continuations;
                        callback_continuations.push(GeneratorContinuation::IgnoreError);
                        callback_continuations.push(GeneratorContinuation::ObjectItem {
                            key: Arc::clone(key),
                            values,
                            accepted: Rc::new(Cell::new(false)),
                        });
                        (
                            GeneratorWork {
                                node: argument,
                                input: value.clone(),
                                origin: None,
                                environment,
                                frames,
                                continuations: callback_continuations,
                            },
                            None,
                        )
                    } else {
                        let output = std::mem::take(&mut *values.borrow_mut());
                        let (origin, result) = match fresh_origin(&mut next_origin) {
                            Ok(origin) => (Some(origin), Ok(Value::object(output))),
                            Err(error) => (None, Err(error)),
                        };
                        (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(result),
                        )
                    }
                }
                GeneratorTask::CollectSortKeys {
                    values,
                    next,
                    argument,
                    keyed_values,
                    mode,
                    origin,
                    environment,
                    frames,
                    continuations,
                } => {
                    if let Some(value) = values.get(next) {
                        let mut charge =
                            || charge_managed_step(&mut observations, limits, cancellation, stop);
                        let callback_origin = origin
                            .as_ref()
                            .map(|origin| {
                                origin.child_bounded(
                                    PathComponent::Index(next),
                                    limits,
                                    &mut charge,
                                )
                            })
                            .transpose();
                        match callback_origin {
                            Err(error) => (
                                GeneratorWork {
                                    node: bytecode.root(),
                                    input: Value::Null,
                                    origin: None,
                                    environment,
                                    frames,
                                    continuations,
                                },
                                Some(Err(error)),
                            ),
                            Ok(callback_origin) => {
                                pending.push(GeneratorTask::CollectSortKeys {
                                    values: Arc::clone(&values),
                                    next: next + 1,
                                    argument,
                                    keyed_values: Rc::clone(&keyed_values),
                                    mode,
                                    origin: origin.clone(),
                                    environment: Arc::clone(&environment),
                                    frames: Arc::clone(&frames),
                                    continuations: continuations.clone(),
                                });
                                let keys = Rc::new(RefCell::new(Vec::new()));
                                pending.push(GeneratorTask::FinishSortKey {
                                    input: value.clone(),
                                    origin: callback_origin.clone(),
                                    keys: Rc::clone(&keys),
                                    keyed_values,
                                    continuations: continuations.clone(),
                                });
                                let mut callback_continuations = continuations;
                                callback_continuations
                                    .push(GeneratorContinuation::SortKeyItem(Rc::clone(&keys)));
                                (
                                    GeneratorWork {
                                        node: argument,
                                        input: value.clone(),
                                        origin: callback_origin,
                                        environment,
                                        frames,
                                        continuations: callback_continuations,
                                    },
                                    None,
                                )
                            }
                        }
                    } else {
                        let keyed_values = std::mem::take(&mut *keyed_values.borrow_mut());
                        let mut charge =
                            || charge_managed_step(&mut observations, limits, cancellation, stop);
                        let (origin, result) = match project_managed_keyed_values(
                            mode,
                            keyed_values,
                            limits,
                            &mut charge,
                            &mut next_origin,
                        ) {
                            Ok((origin, output)) => (origin, Ok(output)),
                            Err(error) => (None, Err(error)),
                        };
                        (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(result),
                        )
                    }
                }
                GeneratorTask::FinishSortKey {
                    input,
                    origin,
                    keys,
                    keyed_values,
                    continuations: _,
                } => {
                    let keys = std::mem::take(&mut *keys.borrow_mut());
                    keyed_values.borrow_mut().push(ManagedKeyedValue {
                        key: Value::array(keys),
                        value: input,
                        origin,
                    });
                    continue;
                }
                GeneratorTask::FinishAlternative {
                    matched,
                    right,
                    input,
                    origin,
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
                            origin,
                            environment,
                            frames,
                            continuations,
                        },
                        None,
                    )
                }
                GeneratorTask::FinishPullConsumer {
                    state,
                    environment,
                    frames,
                    continuations,
                } => {
                    let (result, origin) = {
                        let mut state = state.borrow_mut();
                        match state.kind {
                            PullConsumerKind::First | PullConsumerKind::Skip => (None, None),
                            PullConsumerKind::Last | PullConsumerKind::Nth => {
                                (state.latest.take().map(Ok), state.latest_origin.take())
                            }
                            PullConsumerKind::IsEmpty => (Some(Ok(Value::Bool(!state.seen))), None),
                            PullConsumerKind::Any | PullConsumerKind::All => (
                                Some(Ok(Value::Bool(
                                    state
                                        .decision
                                        .unwrap_or(matches!(state.kind, PullConsumerKind::All)),
                                ))),
                                None,
                            ),
                        }
                    };
                    let Some(result) = result else {
                        continue;
                    };
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(result),
                    )
                }
                GeneratorTask::FinishAdd {
                    state,
                    environment,
                    frames,
                    continuations,
                } => {
                    let value = state.borrow().accumulator.clone().unwrap_or(Value::Null);
                    let origin = match fresh_origin(&mut next_origin) {
                        Ok(origin) => Some(origin),
                        Err(error) => {
                            return {
                                let _ = emit(Err(error), observations);
                                observations
                            };
                        }
                    };
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Ok(value)),
                    )
                }
                GeneratorTask::FinishDebug {
                    input,
                    origin,
                    environment,
                    frames,
                    continuations,
                } => (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(Ok(input)),
                ),
                GeneratorTask::FoldGenerator {
                    generator,
                    pattern,
                    update,
                    extract,
                    emit_each_update,
                    state,
                    input,
                    environment,
                    frames,
                    continuations,
                } => {
                    pending.push(GeneratorTask::FinishFold {
                        state: Rc::clone(&state),
                        emit_final: !emit_each_update,
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: continuations.clone(),
                    });
                    let mut generator_continuations = continuations;
                    generator_continuations.push(GeneratorContinuation::FoldItem {
                        state,
                        pattern,
                        update,
                        extract,
                        emit_each_update,
                        environment: Arc::clone(&environment),
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: generator,
                        input,
                        origin: None,
                        environment,
                        frames,
                        continuations: generator_continuations,
                    }));
                    continue;
                }
                GeneratorTask::FinishFold {
                    state,
                    emit_final,
                    environment,
                    frames,
                    continuations,
                } => {
                    if !emit_final {
                        continue;
                    }
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Ok(state.borrow().accumulator.clone())),
                    )
                }
                GeneratorTask::PathEval(path_work) => {
                    delivered_from_path = true;
                    let environment = Arc::clone(&path_work.environment);
                    let frames = Arc::clone(&path_work.frames);
                    let origin = path_work.origin.clone();
                    let continuations = path_work.continuations.clone();
                    let Some(result) =
                        schedule_path_eval(bytecode, path_work, &mut pending, &mut next_boundary)
                    else {
                        continue;
                    };
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(result),
                    )
                }
                GeneratorTask::PathChildren {
                    value,
                    path,
                    next,
                    environment,
                    frames,
                    continuations,
                } => {
                    delivered_from_path = true;
                    let child = match &value {
                        Value::Array(values) => values
                            .get(next)
                            .cloned()
                            .map(|child| (PathComponent::Index(next), child)),
                        Value::Object(values) => values.get_index(next).map(|(key, child)| {
                            (PathComponent::Key(Arc::clone(key)), child.clone())
                        }),
                        _ => None,
                    };
                    let Some((component, _)) = child else {
                        continue;
                    };
                    pending.push(GeneratorTask::PathChildren {
                        value,
                        path: path.clone(),
                        next: next.saturating_add(1),
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: continuations.clone(),
                    });
                    let mut components = path.components().to_vec();
                    components.push(component);
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Ok(path_value(&Path::new(components)))),
                    )
                }
                GeneratorTask::PathTraverse {
                    mut cursor,
                    environment,
                    frames,
                    continuations,
                } => {
                    delivered_from_path = true;
                    if cursor.depth() > limits.path_stack {
                        (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin: None,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Err(resource("path-stack"))),
                        )
                    } else {
                        let mut charge =
                            || charge_managed_step(&mut observations, limits, cancellation, stop);
                        match cursor.next(limits.path_stack, &mut charge) {
                            Ok(Some(path)) => {
                                pending.push(GeneratorTask::PathTraverse {
                                    cursor,
                                    environment: Arc::clone(&environment),
                                    frames: Arc::clone(&frames),
                                    continuations: continuations.clone(),
                                });
                                let (origin, result) = match fresh_origin(&mut next_origin) {
                                    Ok(origin) => (Some(origin), Ok(path_value(&path))),
                                    Err(error) => (None, Err(error)),
                                };
                                (
                                    GeneratorWork {
                                        node: bytecode.root(),
                                        input: Value::Null,
                                        origin,
                                        environment,
                                        frames,
                                        continuations,
                                    },
                                    Some(result),
                                )
                            }
                            Ok(None) => continue,
                            Err(error) => (
                                GeneratorWork {
                                    node: bytecode.root(),
                                    input: Value::Null,
                                    origin: None,
                                    environment,
                                    frames,
                                    continuations,
                                },
                                Some(Err(error)),
                            ),
                        }
                    }
                }
                GeneratorTask::FinishPathCollection {
                    boundary: _,
                    kind,
                    input,
                    paths,
                    environment,
                    frames,
                    continuations,
                } => {
                    let accumulator = std::mem::replace(
                        &mut *paths.borrow_mut(),
                        path_builtin::PathAccumulator::new(),
                    );
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let result = match kind {
                        PathCollectionKind::Delete => path_builtin::delete_paths_bounded(
                            &input,
                            accumulator,
                            limits,
                            &mut charge,
                        ),
                        PathCollectionKind::Pick => path_builtin::pick_paths_bounded(
                            &input,
                            accumulator,
                            limits,
                            &mut charge,
                        ),
                    };
                    let (origin, result) = match result {
                        Ok(value) => match fresh_origin(&mut next_origin) {
                            Ok(origin) => (Some(origin), Ok(value)),
                            Err(error) => (None, Err(error)),
                        },
                        Err(error) => (None, Err(error)),
                    };
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(result),
                    )
                }
                GeneratorTask::PathSlice {
                    path,
                    start,
                    end,
                    next,
                    assignment,
                    environment,
                    frames,
                    continuations,
                } => {
                    delivered_from_path = true;
                    if let Some(state) = assignment {
                        state.borrow_mut().targets.push(AssignmentTarget::Slice {
                            path,
                            start,
                            end,
                        });
                        state.borrow_mut().target_origins.push(None);
                        continue;
                    }
                    if next >= end {
                        continue;
                    }
                    pending.push(GeneratorTask::PathSlice {
                        path: path.clone(),
                        start,
                        end,
                        next: next.saturating_add(1),
                        assignment: None,
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: continuations.clone(),
                    });
                    let mut components = path.components().to_vec();
                    components.push(PathComponent::Index(next));
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Ok(path_value(&Path::new(components)))),
                    )
                }
                GeneratorTask::FinishAssignmentPaths {
                    state,
                    environment,
                    frames,
                    continuations,
                } => {
                    let (operator, value_node, input) = {
                        let state = state.borrow();
                        (state.operator, state.value_node, state.input.clone())
                    };
                    if operator == AssignmentOperator::Update {
                        pending.push(GeneratorTask::AssignmentUpdate {
                            state,
                            next: 0,
                            environment,
                            frames,
                            continuations,
                        });
                        continue;
                    }
                    let mut continuations = continuations;
                    continuations.push(GeneratorContinuation::AssignmentRhs(state));
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: value_node,
                        input,
                        origin: None,
                        environment,
                        frames,
                        continuations,
                    }));
                    continue;
                }
                GeneratorTask::AssignmentUpdate {
                    state,
                    next,
                    environment,
                    frames,
                    continuations,
                } => {
                    if next >= state.borrow().targets.len() {
                        let document = state.borrow().document.clone();
                        let deletions = std::mem::replace(
                            &mut state.borrow_mut().deletions,
                            path_builtin::PathAccumulator::new(),
                        );
                        let mut charge =
                            || charge_managed_step(&mut observations, limits, cancellation, stop);
                        let result = path_builtin::delete_paths_bounded(
                            &document,
                            deletions,
                            limits,
                            &mut charge,
                        );
                        let mut result = result;
                        let origin = if result.is_ok() {
                            match fresh_origin(&mut next_origin) {
                                Ok(origin) => Some(origin),
                                Err(error) => {
                                    result = Err(error);
                                    None
                                }
                            }
                        } else {
                            None
                        };
                        if let Ok(document) = &result {
                            state.borrow_mut().document = document.clone();
                        }
                        (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(result),
                        )
                    } else {
                        let target = state
                            .borrow()
                            .targets
                            .get(next)
                            .cloned()
                            .expect("target index checked above");
                        let (value_node, root, origin) = {
                            let state_ref = state.borrow();
                            (
                                state_ref.value_node,
                                state_ref.document.clone(),
                                state_ref.target_origins.get(next).cloned().flatten(),
                            )
                        };
                        let mut charge =
                            || charge_managed_step(&mut observations, limits, cancellation, stop);
                        let old = match assignment_target_value(&root, &target, limits, &mut charge)
                        {
                            Ok(old) => old,
                            Err(error) => {
                                return {
                                    let _ = emit(Err(error), observations);
                                    observations
                                };
                            }
                        };
                        let input = old.clone();
                        let boundary = next_boundary;
                        next_boundary = next_boundary.saturating_add(1);
                        let accepted = Rc::new(Cell::new(false));
                        pending.push(GeneratorTask::AssignmentUpdateContinue {
                            state: Rc::clone(&state),
                            next: next.saturating_add(1),
                            boundary,
                            accepted: Rc::clone(&accepted),
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: continuations.clone(),
                        });
                        let mut rhs_continuations = continuations;
                        rhs_continuations.push(GeneratorContinuation::AssignmentUpdateRhs {
                            state,
                            target,
                            old,
                            next: next.saturating_add(1),
                            boundary,
                            accepted,
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: value_node,
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: rhs_continuations,
                        }));
                        continue;
                    }
                }
                GeneratorTask::AssignmentUpdateContinue {
                    state,
                    next,
                    boundary: _,
                    accepted,
                    environment,
                    frames,
                    continuations,
                } => {
                    if accepted.get() {
                        continue;
                    }
                    let Some(target) = state.borrow().targets.get(next - 1).cloned() else {
                        continue;
                    };
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    if let Err(error) = retain_assignment_deletion_target(
                        &mut state.borrow_mut().deletions,
                        &target,
                        limits,
                        &mut charge,
                    ) {
                        return {
                            let _ = emit(Err(error), observations);
                            observations
                        };
                    }
                    pending.push(GeneratorTask::AssignmentUpdate {
                        state,
                        next,
                        environment,
                        frames,
                        continuations,
                    });
                    continue;
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
                                    origin: None,
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
                GeneratorTask::RecurseExpand {
                    input,
                    filter,
                    condition,
                    depth,
                    environment,
                    frames,
                    continuations,
                } => {
                    if depth > limits.path_stack {
                        let _ = emit(Err(resource("path-stack")), observations);
                        break;
                    }
                    observations.path_stack_high_water =
                        observations.path_stack_high_water.max(depth);
                    if let Some(filter) = filter {
                        let structural = matches!(input, Value::Array(_) | Value::Object(_));
                        let mut callback_continuations = continuations;
                        callback_continuations.push(GeneratorContinuation::RecurseChild {
                            filter,
                            condition,
                            depth,
                            structural,
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                        });
                        (
                            GeneratorWork {
                                node: filter,
                                input,
                                origin: None,
                                environment,
                                frames,
                                continuations: callback_continuations,
                            },
                            None,
                        )
                    } else {
                        pending.push(GeneratorTask::RecurseChildren {
                            input,
                            next: 0,
                            depth,
                            environment,
                            frames,
                            continuations,
                        });
                        continue;
                    }
                }
                GeneratorTask::RecurseChildren {
                    input,
                    next,
                    depth,
                    environment,
                    frames,
                    continuations,
                } => {
                    let Some(child) = recursive_child(&input, next) else {
                        continue;
                    };
                    pending.push(GeneratorTask::RecurseChildren {
                        input,
                        next: next.saturating_add(1),
                        depth,
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: continuations.clone(),
                    });
                    pending.push(GeneratorTask::RecurseValue {
                        value: child,
                        filter: None,
                        condition: None,
                        depth: depth.saturating_add(1),
                        environment,
                        frames,
                        continuations,
                    });
                    continue;
                }
                GeneratorTask::RecurseValue {
                    value,
                    filter,
                    condition,
                    depth,
                    environment,
                    frames,
                    continuations,
                } => {
                    if depth > limits.path_stack {
                        let _ = emit(Err(resource("path-stack")), observations);
                        break;
                    }
                    pending.push(GeneratorTask::RecurseExpand {
                        input: value.clone(),
                        filter,
                        condition,
                        depth,
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: continuations.clone(),
                    });
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Ok(value)),
                    )
                }
                GeneratorTask::Repeat {
                    expression,
                    input,
                    environment,
                    frames,
                    continuations,
                } => {
                    // jq defines repeat(exp) as `exp, _repeat`: the recursive
                    // call is a sibling of exp and therefore receives the same
                    // input, rather than the values emitted by exp.  Queue the
                    // next iteration before evaluating exp so all of exp's
                    // results are emitted first.  The VM's step budget bounds an
                    // empty or otherwise unbounded repeat without native stack
                    // growth.
                    pending.push(GeneratorTask::Repeat {
                        expression,
                        input: input.clone(),
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: continuations.clone(),
                    });
                    let mut item_continuations = continuations;
                    item_continuations.push(GeneratorContinuation::RepeatItem);
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: expression,
                        input,
                        origin: None,
                        environment,
                        frames,
                        continuations: item_continuations,
                    }));
                    continue;
                }
                GeneratorTask::WalkValue {
                    input,
                    callback,
                    depth,
                    environment,
                    frames,
                    continuations,
                } => {
                    if depth > limits.path_stack {
                        let _ = emit(Err(resource("path-stack")), observations);
                        break;
                    }
                    observations.path_stack_high_water =
                        observations.path_stack_high_water.max(depth);
                    match input {
                        Value::Array(values) => {
                            pending.push(GeneratorTask::WalkArray {
                                inputs: values,
                                next: 0,
                                callback,
                                depth,
                                values: Rc::new(RefCell::new(Vec::new())),
                                environment,
                                frames,
                                continuations,
                            });
                            continue;
                        }
                        Value::Object(values) => {
                            let entries = values
                                .iter()
                                .map(|(key, value)| (Arc::clone(key), value.clone()))
                                .collect::<Vec<_>>();
                            pending.push(GeneratorTask::WalkObject {
                                entries: Arc::from(entries),
                                next: 0,
                                callback,
                                depth,
                                values: Rc::new(RefCell::new(Object::new())),
                                environment,
                                frames,
                                continuations,
                            });
                            continue;
                        }
                        input => (
                            GeneratorWork {
                                node: callback,
                                input,
                                origin: None,
                                environment,
                                frames,
                                continuations,
                            },
                            None,
                        ),
                    }
                }
                GeneratorTask::WalkArray {
                    inputs,
                    next,
                    callback,
                    depth,
                    values,
                    environment,
                    frames,
                    continuations,
                } => {
                    if let Some(value) = inputs.get(next).cloned() {
                        pending.push(GeneratorTask::WalkArray {
                            inputs,
                            next: next + 1,
                            callback,
                            depth,
                            values: Rc::clone(&values),
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: continuations.clone(),
                        });
                        let mut child_continuations = continuations;
                        child_continuations.push(GeneratorContinuation::ArrayItem(values));
                        pending.push(GeneratorTask::WalkValue {
                            input: value,
                            callback,
                            depth: depth.saturating_add(1),
                            environment,
                            frames,
                            continuations: child_continuations,
                        });
                        continue;
                    }
                    (
                        GeneratorWork {
                            node: callback,
                            input: Value::array(values.take()),
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        None,
                    )
                }
                GeneratorTask::WalkObject {
                    entries,
                    next,
                    callback,
                    depth,
                    values,
                    environment,
                    frames,
                    continuations,
                } => {
                    if let Some((key, value)) = entries.get(next).cloned() {
                        pending.push(GeneratorTask::WalkObject {
                            entries,
                            next: next + 1,
                            callback,
                            depth,
                            values: Rc::clone(&values),
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: continuations.clone(),
                        });
                        let mut child_continuations = continuations;
                        child_continuations.push(GeneratorContinuation::ObjectItem {
                            key,
                            values,
                            accepted: Rc::new(Cell::new(false)),
                        });
                        pending.push(GeneratorTask::WalkValue {
                            input: value,
                            callback,
                            depth: depth.saturating_add(1),
                            environment,
                            frames,
                            continuations: child_continuations,
                        });
                        continue;
                    }
                    let rebuilt = std::mem::take(&mut *values.borrow_mut());
                    (
                        GeneratorWork {
                            node: callback,
                            input: Value::object(rebuilt),
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        None,
                    )
                }
            }
        };
        let mut result_origin = work.origin.clone();
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
                Operation::Literal(index) => {
                    result_origin = match fresh_origin(&mut next_origin) {
                        Ok(origin) => Some(origin),
                        Err(error) => {
                            return {
                                let _ = emit(Err(error), observations);
                                observations
                            };
                        }
                    };
                    Some(
                        bytecode
                            .constants()
                            .get(*index as usize)
                            .cloned()
                            .ok_or_else(|| invalid("literal missing after validation")),
                    )
                }
                Operation::Variable(index) => Some(
                    bytecode
                        .string(*index)
                        .ok_or_else(|| invalid("string missing after validation"))
                        .and_then(|name| {
                            result_origin = work.environment.origins.get(name).cloned();
                            variable_value(&work.environment, name)
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
                        current = Some(work);
                        continue 'generator;
                    }
                    None => Some(Err(invalid("string missing after validation"))),
                },
                Operation::AccessIndex { base, index } => {
                    work.continuations.push(GeneratorContinuation::AccessIndex {
                        node: *index,
                        input: work.input.clone(),
                        origin: work.origin.clone(),
                        environment: Arc::clone(&work.environment),
                    });
                    work.node = *base;
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Slice { base, start, end } => {
                    work.continuations.push(GeneratorContinuation::SliceBase {
                        start: *start,
                        end: *end,
                        input: work.input.clone(),
                        origin: work.origin.clone(),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                    });
                    work.node = *base;
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Iterate(base) => {
                    work.continuations.push(GeneratorContinuation::Iterate);
                    work.node = *base;
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Optional(child) => {
                    let boundary = next_boundary;
                    next_boundary = next_boundary.saturating_add(1);
                    work.continuations
                        .push(GeneratorContinuation::OptionalBoundary { boundary });
                    work.node = *child;
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Pipe { left, right } => {
                    work.continuations.push(GeneratorContinuation::Pipe {
                        node: *right,
                        environment: Arc::clone(&work.environment),
                    });
                    work.node = *left;
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Comma { left, right } => {
                    if pending.len().saturating_add(2) > limits.fork_stack {
                        Some(Err(resource("fork-stack")))
                    } else {
                        let right_work = GeneratorWork {
                            node: *right,
                            input: work.input.clone(),
                            origin: work.origin.clone(),
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
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Object(entries) => {
                    let entries: Arc<[crate::bytecode::ObjectOperand]> = Arc::from(entries.clone());
                    if entries.is_empty() {
                        result_origin = match fresh_origin(&mut next_origin) {
                            Ok(origin) => Some(origin),
                            Err(error) => {
                                return {
                                    let _ = emit(Err(error), observations);
                                    observations
                                };
                            }
                        };
                    }
                    schedule_object_entry(
                        bytecode,
                        &entries,
                        0,
                        Arc::new(Object::new()),
                        work.input.clone(),
                        work.origin.clone(),
                        Arc::clone(&work.environment),
                        Arc::clone(&work.frames),
                        work.continuations.clone(),
                        &mut pending,
                    )
                }
                Operation::Unary { operator, child } => {
                    work.continuations
                        .push(GeneratorContinuation::Unary(*operator));
                    work.node = *child;
                    current = Some(work);
                    continue 'generator;
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
                            origin: work.origin.clone(),
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
                            origin: work.origin.clone(),
                            environment: Arc::clone(&work.environment),
                        });
                    }
                    work.node = *left;
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Bind {
                    value,
                    pattern,
                    body,
                } => {
                    work.continuations.push(GeneratorContinuation::Bind {
                        pattern: pattern.clone(),
                        body: *body,
                        input: work.input.clone(),
                        input_origin: work.origin.clone(),
                        environment: Arc::clone(&work.environment),
                        origin: None,
                    });
                    work.node = *value;
                    current = Some(work);
                    continue 'generator;
                }
                Operation::BindAlternatives {
                    value,
                    patterns,
                    body,
                } => {
                    work.continuations
                        .push(GeneratorContinuation::BindAlternatives {
                            patterns: Arc::from(patterns.clone()),
                            body: *body,
                            input: work.input.clone(),
                            environment: Arc::clone(&work.environment),
                        });
                    work.node = *value;
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Reduce {
                    generator,
                    pattern,
                    initial,
                    update,
                } => {
                    work.continuations.push(GeneratorContinuation::FoldInitial {
                        generator: *generator,
                        pattern: pattern.clone(),
                        update: *update,
                        extract: None,
                        emit_each_update: false,
                        input: work.input.clone(),
                        environment: Arc::clone(&work.environment),
                    });
                    work.node = *initial;
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Foreach {
                    generator,
                    pattern,
                    initial,
                    update,
                    extract,
                } => {
                    work.continuations.push(GeneratorContinuation::FoldInitial {
                        generator: *generator,
                        pattern: pattern.clone(),
                        update: *update,
                        extract: *extract,
                        emit_each_update: true,
                        input: work.input.clone(),
                        environment: Arc::clone(&work.environment),
                    });
                    work.node = *initial;
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Assignment {
                    operator,
                    path,
                    value,
                } => {
                    let state = Rc::new(RefCell::new(AssignmentState {
                        operator: *operator,
                        value_node: *value,
                        input: work.input.clone(),
                        targets: Vec::new(),
                        target_origins: Vec::new(),
                        document: work.input.clone(),
                        deletions: path_builtin::PathAccumulator::new(),
                    }));
                    pending.push(GeneratorTask::FinishAssignmentPaths {
                        state: Rc::clone(&state),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    let mut path_continuations = work.continuations.clone();
                    path_continuations.push(GeneratorContinuation::AssignmentPath(state));
                    pending.push(GeneratorTask::PathEval(PathWork {
                        node: *path,
                        input: work.input.clone(),
                        origin: work.origin.clone(),
                        prefix: Path::root(),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: path_continuations,
                    }));
                    None
                }
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
                            origin: work.origin.clone(),
                            environment: Arc::clone(&work.environment),
                        });
                        work.node = *condition;
                    } else {
                        work.node = *alternative;
                    }
                    current = Some(work);
                    continue 'generator;
                }
                Operation::UserCall { symbol, arguments } => schedule_user_call(
                    *symbol,
                    Arc::from(arguments.clone()),
                    0,
                    work.input.clone(),
                    work.origin.clone(),
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
                            current = Some(work);
                            continue 'generator;
                        }
                        None => Some(Err(invalid(
                            "filter parameter missing from active user frame",
                        ))),
                    }
                }
                Operation::TryCatch { expression, catch } => {
                    let boundary = next_boundary;
                    next_boundary = next_boundary.saturating_add(1);
                    work.continuations.push(GeneratorContinuation::Catch {
                        boundary,
                        node: *catch,
                        origin: work.origin.clone(),
                        environment: Arc::clone(&work.environment),
                    });
                    work.node = *expression;
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Label { symbol, body } => {
                    work.continuations
                        .push(GeneratorContinuation::Label(*symbol));
                    work.node = *body;
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Break(symbol) => Some(Err(VmError::Break { label: *symbol })),
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .and_then(|name| regex_builtin_shape(name, arguments.len()))
                        .is_some() =>
                {
                    let Some(name) = bytecode.string(*name).cloned() else {
                        return {
                            let _ = emit(
                                Err(invalid("string missing after validation")),
                                observations,
                            );
                            observations
                        };
                    };
                    let Some((order, replacement_slot)) =
                        regex_builtin_shape(&name, arguments.len())
                    else {
                        return {
                            let _ = emit(Err(invalid("unsupported regex call")), observations);
                            observations
                        };
                    };
                    if arguments.len() > limits.fork_stack {
                        return {
                            let _ = emit(Err(resource("fork-stack")), observations);
                            observations
                        };
                    }
                    let mut values = Vec::new();
                    if values.try_reserve(arguments.len()).is_err() {
                        return {
                            let _ = emit(Err(resource("fork-stack")), observations);
                            observations
                        };
                    }
                    values.resize(arguments.len(), None);
                    let first = order[0];
                    let mut continuations = std::mem::take(&mut work.continuations);
                    continuations.push(GeneratorContinuation::RegexArguments {
                        name,
                        arguments: Arc::from(arguments.clone()),
                        order,
                        next: 1,
                        values,
                        replacement: replacement_slot.and_then(|slot| arguments.get(slot).copied()),
                        input: work.input.clone(),
                        origin: work.origin.clone(),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: arguments[first],
                        input: work.input.clone(),
                        origin: work.origin.clone(),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations,
                    }));
                    None
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| matches!(name.as_ref(), "IN" | "INDEX" | "JOIN")) =>
                {
                    let Some(name) = bytecode.string(*name) else {
                        return {
                            let _ = emit(
                                Err(invalid("string missing after validation")),
                                observations,
                            );
                            observations
                        };
                    };
                    schedule_sql_call(
                        name,
                        arguments,
                        work.input.clone(),
                        work.origin.clone(),
                        Arc::clone(&work.environment),
                        Arc::clone(&work.frames),
                        std::mem::take(&mut work.continuations),
                        limits,
                        &mut next_boundary,
                        &mut pending,
                    )
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| matches!(name.as_ref(), "any" | "all")) =>
                {
                    let kind = if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "any")
                    {
                        PullConsumerKind::Any
                    } else {
                        PullConsumerKind::All
                    };
                    let (generator, condition) = match arguments.as_slice() {
                        [] => (None, None),
                        [condition] => (None, Some(*condition)),
                        [generator, condition] => (Some(*generator), Some(*condition)),
                        _ => {
                            let _ = emit(Err(invalid("predicate arity")), observations);
                            return observations;
                        }
                    };
                    let boundary = next_boundary;
                    next_boundary = next_boundary.saturating_add(1);
                    let state = Rc::new(RefCell::new(PullConsumerState {
                        boundary,
                        kind,
                        seen: false,
                        latest: None,
                        latest_origin: None,
                        decision: None,
                        remaining: 0,
                    }));
                    pending.push(GeneratorTask::FinishPullConsumer {
                        state: Rc::clone(&state),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    let mut predicate_continuations = work.continuations.clone();
                    predicate_continuations.push(GeneratorContinuation::PredicateItem {
                        state,
                        condition,
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                    });
                    if let Some(generator) = generator {
                        work.node = generator;
                        work.continuations = predicate_continuations;
                        pending.push(GeneratorTask::Eval(work.clone()));
                    } else {
                        let values = match &work.input {
                            Value::Array(values) => Arc::clone(values),
                            Value::Object(values) => {
                                values.values().cloned().collect::<Vec<_>>().into()
                            }
                            value => {
                                let _ = emit(Err(type_error("iterate", value)), observations);
                                return observations;
                            }
                        };
                        pending.push(GeneratorTask::Iterate {
                            values,
                            next: 0,
                            environment: Arc::clone(&work.environment),
                            frames: Arc::clone(&work.frames),
                            continuations: predicate_continuations,
                        });
                    }
                    None
                }
                Operation::Call { name, arguments }
                    if bytecode.string(*name).is_some_and(|name| {
                        matches!(name.as_ref(), "path" | "paths" | "pick" | "del")
                    }) =>
                {
                    let Some(name) = bytecode.string(*name).cloned() else {
                        return {
                            let _ = emit(
                                Err(invalid("string missing after validation")),
                                observations,
                            );
                            observations
                        };
                    };
                    match name.as_ref() {
                        "path" => {
                            let Some(argument) = arguments.first() else {
                                return {
                                    let _ =
                                        emit(Err(invalid("path argument missing")), observations);
                                    observations
                                };
                            };
                            if arguments.len() != 1 {
                                return {
                                    let _ = emit(Err(invalid("path arity")), observations);
                                    observations
                                };
                            }
                            let mut continuations = work.continuations.clone();
                            continuations.push(GeneratorContinuation::FreshOrigin);
                            pending.push(GeneratorTask::PathEval(PathWork {
                                node: *argument,
                                input: work.input.clone(),
                                origin: work.origin.clone(),
                                prefix: Path::root(),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations,
                            }));
                            None
                        }
                        "paths" => {
                            if arguments.len() > 1 {
                                return {
                                    let _ = emit(Err(invalid("paths arity")), observations);
                                    observations
                                };
                            }
                            let mut continuations = work.continuations.clone();
                            if let Some(filter) = arguments.first().copied() {
                                continuations.push(GeneratorContinuation::PathFilter {
                                    root: work.input.clone(),
                                    filter,
                                    origin: work.origin.clone(),
                                    environment: Arc::clone(&work.environment),
                                    frames: work.frames.clone(),
                                });
                            }
                            pending.push(GeneratorTask::PathTraverse {
                                cursor: PathTraversalCursor::without_root(work.input.clone()),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations,
                            });
                            None
                        }
                        "pick" | "del" => {
                            let Some(argument) = arguments.first() else {
                                return {
                                    let _ =
                                        emit(Err(invalid("path callback missing")), observations);
                                    observations
                                };
                            };
                            if arguments.len() != 1 {
                                return {
                                    let _ = emit(Err(invalid("path callback arity")), observations);
                                    observations
                                };
                            }
                            let kind = if name.as_ref() == "pick" {
                                PathCollectionKind::Pick
                            } else {
                                PathCollectionKind::Delete
                            };
                            let boundary = next_boundary;
                            next_boundary = next_boundary.saturating_add(1);
                            let paths = Rc::new(RefCell::new(path_builtin::PathAccumulator::new()));
                            let continuations = work.continuations.clone();
                            pending.push(GeneratorTask::FinishPathCollection {
                                boundary,
                                kind,
                                input: work.input.clone(),
                                paths: Rc::clone(&paths),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: continuations.clone(),
                            });
                            let mut callback_continuations = continuations;
                            callback_continuations
                                .push(GeneratorContinuation::PathCollect { boundary, paths });
                            pending.push(GeneratorTask::PathEval(PathWork {
                                node: *argument,
                                input: work.input.clone(),
                                origin: work.origin.clone(),
                                prefix: Path::root(),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: callback_continuations,
                            }));
                            None
                        }
                        _ => unreachable!("path builtin guard is exhaustive"),
                    }
                }
                Operation::Call { name, arguments }
                    if arguments.is_empty()
                        && bytecode
                            .string(*name)
                            .is_some_and(|name| scalar::supports(name, 0)) =>
                {
                    let name = bytecode.string(*name).expect("validated scalar name");
                    // Retain the charge for the InvokeBuiltin task avoided by this path.
                    if let Err(error) =
                        charge_managed_step(&mut observations, limits, cancellation, stop)
                    {
                        let _ = emit(Err(error), observations);
                        return observations;
                    }
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    match scalar::evaluate(name, &work.input, &[], limits, &mut charge) {
                        Ok(Some(value)) => {
                            result_origin = match scalar_result_origin(
                                name,
                                &work.input,
                                &[],
                                work.origin.clone(),
                                &mut next_origin,
                            ) {
                                Ok(origin) => origin,
                                Err(error) => {
                                    let _ = emit(Err(error), observations);
                                    return observations;
                                }
                            };
                            Some(Ok(value))
                        }
                        Ok(None) => None,
                        Err(error) => {
                            result_origin = None;
                            Some(Err(error))
                        }
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| supports_builtin_call(name, arguments.len())) =>
                {
                    let Some(name) = bytecode.string(*name).cloned() else {
                        return {
                            let _ = emit(
                                Err(invalid("string missing after validation")),
                                observations,
                            );
                            observations
                        };
                    };
                    let source_arity = arguments.len();
                    let input = work.input.clone();
                    let origin = work.origin.clone();
                    let environment = Arc::clone(&work.environment);
                    let frames = Arc::clone(&work.frames);
                    if arguments.is_empty() {
                        pending.push(GeneratorTask::InvokeBuiltin {
                            name,
                            arguments: Vec::new(),
                            source_arity,
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: std::mem::take(&mut work.continuations),
                        });
                        None
                    } else if name.as_ref() == "combinations" && arguments.len() == 1 {
                        let values = Rc::new(RefCell::new(Vec::new()));
                        pending.push(GeneratorTask::FinishBuiltinArguments {
                            name,
                            source_arity,
                            values: Rc::clone(&values),
                            input: input.clone(),
                            origin: origin.clone(),
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: work.continuations.clone(),
                        });
                        let mut continuations = std::mem::take(&mut work.continuations);
                        continuations.push(GeneratorContinuation::ArrayItem(values));
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: arguments[0],
                            input,
                            origin,
                            environment,
                            frames,
                            continuations,
                        }));
                        None
                    } else {
                        if arguments.len() > limits.fork_stack {
                            return {
                                let _ = emit(Err(resource("fork-stack")), observations);
                                observations
                            };
                        }
                        let mut values = Vec::new();
                        if values.try_reserve(arguments.len()).is_err() {
                            return {
                                let _ = emit(Err(resource("fork-stack")), observations);
                                observations
                            };
                        }
                        values.resize(arguments.len(), None);
                        let order = builtin_argument_order(&name, arguments.len());
                        let first = order[0];
                        let mut continuations = std::mem::take(&mut work.continuations);
                        continuations.push(GeneratorContinuation::BuiltinArguments {
                            name,
                            arguments: Arc::from(arguments.clone()),
                            order,
                            next: 1,
                            values,
                            input: input.clone(),
                            origin: origin.clone(),
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: arguments[first],
                            input,
                            origin,
                            environment,
                            frames,
                            continuations,
                        }));
                        None
                    }
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
                        current = Some(work);
                        continue 'generator;
                    }
                    Some(Err(VmError::Raised {
                        message: error_message(&work.input),
                        value: work.input.clone(),
                    }))
                }
                Operation::Call { name, .. }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "stderr") =>
                {
                    match format::text(&work.input, limits.output_bytes).and_then(|text| {
                        append_effect(effects, text.as_bytes(), limits.output_bytes).map(|()| text)
                    }) {
                        Ok(_) => Some(Ok(work.input)),
                        Err(error) => Some(Err(error)),
                    }
                }
                Operation::Call { name, .. }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| matches!(name.as_ref(), "input" | "inputs")) =>
                'input: {
                    let Some(cursor) = input_cursor.cloned() else {
                        break 'input if bytecode
                            .string(*name)
                            .is_some_and(|name| name.as_ref() == "input")
                        {
                            Some(Err(runtime("break".to_owned())))
                        } else {
                            None
                        };
                    };
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "input")
                    {
                        match cursor.next_value() {
                            Ok(Some(value)) => match fresh_origin(&mut next_origin) {
                                Ok(origin) => {
                                    work.origin = Some(origin);
                                    Some(Ok(value))
                                }
                                Err(error) => Some(Err(error)),
                            },
                            Ok(None) => Some(Err(runtime("break".to_owned()))),
                            Err(error) => Some(Err(error)),
                        }
                    } else {
                        pending.push(GeneratorTask::InputValues {
                            cursor,
                            environment: Arc::clone(&work.environment),
                            frames: Arc::clone(&work.frames),
                            continuations: std::mem::take(&mut work.continuations),
                        });
                        None
                    }
                }
                Operation::Call { name, .. }
                    if bytecode.string(*name).is_some_and(|name| {
                        matches!(name.as_ref(), "input_filename" | "input_line_number")
                    }) =>
                {
                    let Some(name) = bytecode.string(*name) else {
                        return {
                            let _ = emit(
                                Err(invalid("string missing after validation")),
                                observations,
                            );
                            observations
                        };
                    };
                    let value = if name.as_ref() == "input_filename" {
                        if ambient_platform(&work.environment) {
                            input_cursor
                                .and_then(InputCursor::current_context)
                                .map_or_else(
                                    || {
                                        ambient_value(
                                            &work.environment,
                                            INPUT_FILENAME,
                                            "input_filename",
                                            None,
                                        )
                                    },
                                    |context| Ok(Value::string(context.identity)),
                                )
                        } else {
                            Err(capability_denied(format!(
                                "{name} requires platform access permitted by capability policy"
                            )))
                        }
                    } else {
                        input_cursor
                            .and_then(InputCursor::current_context)
                            .map_or_else(
                                || {
                                    input_context_value(
                                        &work.environment,
                                        INPUT_LINE_NUMBER,
                                        "input_line_number",
                                    )
                                },
                                |context| {
                                    Ok(Value::Number(
                                        Number::parse(&context.line_number.to_string()).expect(
                                            "a source line number is an admitted exact integer",
                                        ),
                                    ))
                                },
                            )
                    };
                    match value {
                        Ok(value) => match fresh_origin(&mut next_origin) {
                            Ok(origin) => {
                                result_origin = Some(origin);
                                Some(Ok(value))
                            }
                            Err(error) => Some(Err(error)),
                        },
                        Err(error) => Some(Err(error)),
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "debug") =>
                {
                    if let Some(argument) = arguments.first() {
                        pending.push(GeneratorTask::FinishDebug {
                            input: work.input.clone(),
                            origin: work.origin.clone(),
                            environment: Arc::clone(&work.environment),
                            frames: Arc::clone(&work.frames),
                            continuations: work.continuations.clone(),
                        });
                        work.continuations.push(GeneratorContinuation::DebugItem);
                        work.node = *argument;
                        current = Some(work);
                        continue 'generator;
                    }
                    match debug_effect(&work.input, limits.output_bytes)
                        .and_then(|bytes| append_effect(effects, &bytes, limits.output_bytes))
                    {
                        Ok(()) => Some(Ok(work.input)),
                        Err(error) => Some(Err(error)),
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "add")
                        && arguments.len() == 1 =>
                {
                    let state = Rc::new(RefCell::new(AddState { accumulator: None }));
                    pending.push(GeneratorTask::FinishAdd {
                        state: Rc::clone(&state),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    work.continuations
                        .push(GeneratorContinuation::AddItem(state));
                    work.node = arguments[0];
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Call { name, .. }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "halt") =>
                {
                    Some(Err(VmError::Halt {
                        status: 0,
                        stderr: Arc::from([]),
                    }))
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "halt_error") =>
                {
                    if let Some(argument) = arguments.first() {
                        work.continuations.push(GeneratorContinuation::HaltError {
                            input: work.input.clone(),
                        });
                        work.node = *argument;
                        current = Some(work);
                        continue 'generator;
                    }
                    match halt_error_text(&work.input, limits.output_bytes) {
                        Ok(stderr) => Some(Err(VmError::Halt {
                            status: 5,
                            stderr: Arc::from(stderr.as_bytes()),
                        })),
                        Err(error) => Some(Err(error)),
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode.string(*name).is_some_and(|name| {
                        matches!(name.as_ref(), "first" | "last" | "isempty")
                    }) && arguments.len() == 1 =>
                {
                    let name = bytecode.string(*name).map_or("first", AsRef::as_ref);
                    let kind = match name {
                        "first" => PullConsumerKind::First,
                        "last" => PullConsumerKind::Last,
                        _ => PullConsumerKind::IsEmpty,
                    };
                    let boundary = next_boundary;
                    next_boundary = next_boundary.saturating_add(1);
                    let state = Rc::new(RefCell::new(PullConsumerState {
                        boundary,
                        kind,
                        seen: false,
                        latest: None,
                        latest_origin: None,
                        decision: None,
                        remaining: 0,
                    }));
                    pending.push(GeneratorTask::FinishPullConsumer {
                        state: Rc::clone(&state),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    work.continuations
                        .push(GeneratorContinuation::PullConsumer { state });
                    work.node = arguments[0];
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| matches!(name.as_ref(), "first" | "last"))
                        && arguments.is_empty() =>
                {
                    let index = if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "first")
                    {
                        0_i64
                    } else {
                        -1_i64
                    };
                    let index_value = Value::Number(
                        Number::parse(&index.to_string())
                            .expect("literal pull index is a valid number"),
                    );
                    let mut result = access_index(&work.input, &index_value);
                    if result.is_ok() {
                        let mut charge =
                            || charge_managed_step(&mut observations, limits, cancellation, stop);
                        result_origin = match index_origin(
                            &work.input,
                            #[allow(clippy::cast_precision_loss)]
                            {
                                index as f64
                            },
                            work.origin.as_ref(),
                            limits,
                            &mut charge,
                        ) {
                            Ok(origin) => origin,
                            Err(error) => {
                                result = Err(error);
                                None
                            }
                        };
                    }
                    Some(result)
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| matches!(name.as_ref(), "nth" | "skip")) =>
                {
                    let kind = if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "nth")
                    {
                        PullConsumerKind::Nth
                    } else {
                        PullConsumerKind::Skip
                    };
                    let Some(count) = arguments.first().copied() else {
                        return {
                            let _ = emit(Err(invalid("pull count missing")), observations);
                            observations
                        };
                    };
                    let generator = arguments.get(1).copied();
                    if kind == PullConsumerKind::Skip && generator.is_none() {
                        return {
                            let _ = emit(Err(invalid("skip arity")), observations);
                            observations
                        };
                    }
                    if arguments.len() > 2 {
                        return {
                            let _ = emit(Err(invalid("pull arity")), observations);
                            observations
                        };
                    }
                    work.continuations.push(GeneratorContinuation::PullCount {
                        kind,
                        input: work.input.clone(),
                        origin: work.origin.clone(),
                        generator,
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                    });
                    work.node = count;
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "fromstream")
                        && arguments.len() == 1 =>
                {
                    let state = Rc::new(RefCell::new(generator::FromStreamState::new()));
                    work.continuations
                        .push(GeneratorContinuation::FromStreamItem(state));
                    work.node = arguments[0];
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "truncate_stream")
                        && arguments.len() == 1 =>
                {
                    work.continuations
                        .push(GeneratorContinuation::TruncateStreamItem {
                            count: work.input.clone(),
                        });
                    work.node = arguments[0];
                    current = Some(work);
                    continue 'generator;
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "recurse") =>
                {
                    let filter = arguments.first().copied();
                    let condition = arguments.get(1).copied();
                    pending.push(GeneratorTask::RecurseExpand {
                        input: work.input.clone(),
                        filter,
                        condition,
                        depth: 1,
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    Some(Ok(work.input))
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| matches!(name.as_ref(), "while" | "until")) =>
                {
                    match (arguments.first(), arguments.get(1)) {
                        (Some(condition), Some(update)) => {
                            work.continuations
                                .push(GeneratorContinuation::LoopCondition {
                                    condition: *condition,
                                    update: *update,
                                    input: work.input.clone(),
                                    until: bytecode
                                        .string(*name)
                                        .is_some_and(|name| name.as_ref() == "until"),
                                    environment: Arc::clone(&work.environment),
                                    frames: Arc::clone(&work.frames),
                                });
                            work.node = *condition;
                            current = Some(work);
                            continue 'generator;
                        }
                        _ => Some(Err(invalid("loop arguments missing"))),
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "repeat") =>
                {
                    match arguments.first() {
                        Some(expression) => {
                            pending.push(GeneratorTask::Repeat {
                                expression: *expression,
                                input: work.input.clone(),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: work.continuations.clone(),
                            });
                            None
                        }
                        None => Some(Err(invalid("repeat argument missing"))),
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "limit") =>
                {
                    match (arguments.first(), arguments.get(1)) {
                        (Some(count), Some(expression)) => {
                            work.continuations.push(GeneratorContinuation::LimitCount {
                                expression: *expression,
                                input: work.input.clone(),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                            });
                            work.node = *count;
                            current = Some(work);
                            continue 'generator;
                        }
                        _ => Some(Err(invalid("limit arguments missing"))),
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "walk") =>
                {
                    if let Some(callback) = arguments.first() {
                        pending.push(GeneratorTask::WalkValue {
                            input: work.input.clone(),
                            callback: *callback,
                            depth: 1,
                            environment: Arc::clone(&work.environment),
                            frames: Arc::clone(&work.frames),
                            continuations: work.continuations.clone(),
                        });
                        None
                    } else {
                        Some(Err(invalid("walk argument missing")))
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "map") =>
                {
                    match (arguments.first(), &work.input) {
                        (None, _) => Some(Err(invalid("map argument missing"))),
                        (
                            Some(_),
                            value @ (Value::Null
                            | Value::Bool(_)
                            | Value::Number(_)
                            | Value::String(_)),
                        ) => Some(Err(type_error("map", value))),
                        (Some(argument), Value::Array(values)) => {
                            pending.push(GeneratorTask::MapArray {
                                inputs: Arc::clone(values),
                                next: 0,
                                argument: *argument,
                                values: Rc::new(RefCell::new(Vec::new())),
                                first_only: false,
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: work.continuations.clone(),
                            });
                            None
                        }
                        (Some(argument), Value::Object(values)) => {
                            let inputs = values.values().cloned().collect::<Vec<_>>();
                            pending.push(GeneratorTask::MapArray {
                                inputs: Arc::from(inputs),
                                next: 0,
                                argument: *argument,
                                values: Rc::new(RefCell::new(Vec::new())),
                                first_only: false,
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: work.continuations.clone(),
                            });
                            None
                        }
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "with_entries") =>
                {
                    let Some(argument) = arguments.first() else {
                        let _ = emit(Err(invalid("with_entries argument missing")), observations);
                        return observations;
                    };
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let entries = match scalar::bounded_to_entries(&work.input, limits, &mut charge)
                    {
                        Ok(Value::Array(entries)) => entries,
                        Ok(_) => unreachable!("to_entries always returns an array"),
                        Err(error) => {
                            let _ = emit(Err(error), observations);
                            return observations;
                        }
                    };
                    let mut continuations = work.continuations.clone();
                    continuations.push(GeneratorContinuation::FromEntries);
                    pending.push(GeneratorTask::MapArray {
                        inputs: entries,
                        next: 0,
                        argument: *argument,
                        values: Rc::new(RefCell::new(Vec::new())),
                        first_only: false,
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations,
                    });
                    None
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "map_values") =>
                {
                    match (arguments.first(), &work.input) {
                        (None, _) => Some(Err(invalid("map_values argument missing"))),
                        (Some(argument), Value::Object(values)) => {
                            let entries = values
                                .iter()
                                .map(|(key, value)| (Arc::clone(key), value.clone()))
                                .collect::<Vec<_>>();
                            pending.push(GeneratorTask::MapObject {
                                entries: Arc::from(entries),
                                next: 0,
                                argument: *argument,
                                values: Rc::new(RefCell::new(Object::new())),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: work.continuations.clone(),
                            });
                            None
                        }
                        (Some(argument), Value::Array(values)) => {
                            pending.push(GeneratorTask::MapArray {
                                inputs: Arc::clone(values),
                                next: 0,
                                argument: *argument,
                                values: Rc::new(RefCell::new(Vec::new())),
                                first_only: true,
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: work.continuations.clone(),
                            });
                            None
                        }
                        (Some(_), value) => Some(Err(type_error("map_values", value))),
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "select") =>
                {
                    if let Some(argument) = arguments.first() {
                        work.continuations.push(GeneratorContinuation::Select {
                            input: work.input.clone(),
                            origin: work.origin.clone(),
                        });
                        work.node = *argument;
                        current = Some(work);
                        continue 'generator;
                    }
                    Some(Err(invalid("select argument missing")))
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| matches!(name.as_ref(), "has" | "in")) =>
                {
                    if let Some(argument) = arguments.first() {
                        let container_from_result = bytecode
                            .string(*name)
                            .is_some_and(|name| name.as_ref() == "in");
                        work.continuations.push(GeneratorContinuation::HasArgument {
                            input: work.input.clone(),
                            container_from_result,
                        });
                        work.node = *argument;
                        current = Some(work);
                        continue 'generator;
                    }
                    Some(Err(invalid("has/in argument missing")))
                }
                Operation::Call { name, arguments }
                    if bytecode.string(*name).is_some_and(|name| {
                        matches!(
                            name.as_ref(),
                            "sort_by" | "unique_by" | "group_by" | "min_by" | "max_by"
                        )
                    }) =>
                {
                    let name = bytecode.string(*name).map_or("sort_by", AsRef::as_ref);
                    let mode = match name {
                        "sort_by" => KeyedCollectionMode::Sort,
                        "unique_by" => KeyedCollectionMode::Unique,
                        "group_by" => KeyedCollectionMode::Group,
                        "min_by" => KeyedCollectionMode::Min,
                        "max_by" => KeyedCollectionMode::Max,
                        _ => unreachable!("collection mode guard is exhaustive"),
                    };
                    match (arguments.first(), &work.input) {
                        (None, _) => Some(Err(invalid("sort key argument missing"))),
                        (Some(argument), Value::Array(values)) => {
                            pending.push(GeneratorTask::CollectSortKeys {
                                values: Arc::clone(values),
                                next: 0,
                                argument: *argument,
                                keyed_values: Rc::new(RefCell::new(Vec::new())),
                                mode,
                                origin: work.origin.clone(),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: work.continuations.clone(),
                            });
                            None
                        }
                        (Some(_), value) => Some(Err(type_error(name, value))),
                    }
                }
                Operation::Call { name, arguments }
                    if arguments.is_empty()
                        && bytecode
                            .string(*name)
                            .is_some_and(|name| name.as_ref() == "modulemeta") =>
                {
                    let result = module_metadata(bytecode, &work.input, cancellation, stop);
                    if result.is_ok() {
                        result_origin = match fresh_origin(&mut next_origin) {
                            Ok(origin) => Some(origin),
                            Err(error) => {
                                return {
                                    let _ = emit(Err(error), observations);
                                    observations
                                };
                            }
                        };
                    }
                    Some(result)
                }
                Operation::Call { name, arguments } if arguments.is_empty() => {
                    let result = bytecode
                        .string(*name)
                        .ok_or_else(|| invalid("string missing after validation"))
                        .and_then(|name| generator_builtin(name, &work.input, limits.output_bytes))
                        .transpose();
                    if matches!(result, Some(Ok(_))) {
                        result_origin = match fresh_origin(&mut next_origin) {
                            Ok(origin) => Some(origin),
                            Err(error) => {
                                return {
                                    let _ = emit(Err(error), observations);
                                    observations
                                };
                            }
                        };
                    }
                    result
                }
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
                let unwind = ErrorUnwind {
                    bytecode,
                    limits,
                    cancellation,
                    stop,
                    pending: &mut pending,
                    observations: &mut observations,
                };
                if let Some(error) = unwind.recover(work.frames, work.continuations, error) {
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
            let Ok(value) = result.as_ref() else {
                unreachable!("error handled before continuation dispatch")
            };
            let value = value.clone();
            match continuation {
                GeneratorContinuation::IgnoreError => {
                    // map_values drops an item when its filter errors, just as
                    // it drops an item when the filter emits empty. The next
                    // mapping task is already queued.
                    result = Ok(value);
                }
                GeneratorContinuation::AccessField(key) => {
                    result = access_field(&value, &key);
                    if result.is_ok()
                        && let Some(origin) = result_origin.take()
                    {
                        let mut charge =
                            || charge_managed_step(&mut observations, limits, cancellation, stop);
                        match origin.child_bounded(
                            PathComponent::Key(Arc::clone(&key)),
                            limits,
                            &mut charge,
                        ) {
                            Ok(child) => result_origin = Some(child),
                            Err(error) => result = Err(error),
                        }
                    }
                }
                GeneratorContinuation::ApplyIndex { base, origin } => {
                    let component = path_component_for_target(&base, &value);
                    result = access_index(&base, &value);
                    if result.is_ok() {
                        result_origin = match (origin, component) {
                            (Some(origin), Ok(component)) => {
                                let mut charge = || {
                                    charge_managed_step(
                                        &mut observations,
                                        limits,
                                        cancellation,
                                        stop,
                                    )
                                };
                                match origin.child_bounded(component, limits, &mut charge) {
                                    Ok(child) => Some(child),
                                    Err(error) => {
                                        result = Err(error);
                                        None
                                    }
                                }
                            }
                            (origin, _) => origin,
                        };
                    }
                }
                GeneratorContinuation::AccessIndex {
                    node,
                    input,
                    origin,
                    environment,
                } => {
                    work.continuations.push(GeneratorContinuation::ApplyIndex {
                        base: value,
                        origin: result_origin,
                    });
                    current = Some(GeneratorWork {
                        node,
                        input,
                        origin,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    });
                    break;
                }
                GeneratorContinuation::SliceBase {
                    start,
                    end,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    if let Some(start) = start {
                        work.continuations.push(GeneratorContinuation::SliceStart {
                            base: value,
                            end,
                            input: input.clone(),
                            origin: origin.clone(),
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: start,
                            input,
                            origin: origin.clone(),
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                        break;
                    }
                    if let Some(end) = end {
                        work.continuations.push(GeneratorContinuation::SliceEnd {
                            base: value,
                            start: None,
                            origin: origin.clone(),
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: end,
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                        break;
                    }
                    result = slice(&value, None, None);
                }
                GeneratorContinuation::SliceStart {
                    base,
                    end,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    let start = match slice_bound_value(&value) {
                        Ok(start) => start,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    if let Some(end) = end {
                        work.continuations.push(GeneratorContinuation::SliceEnd {
                            base,
                            start,
                            origin: origin.clone(),
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: end,
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                        break;
                    }
                    result = slice(&base, start, None);
                }
                GeneratorContinuation::SliceEnd {
                    base,
                    start,
                    origin,
                } => {
                    result_origin = origin;
                    result = match slice_bound_value(&value) {
                        Ok(end) => slice(&base, start, end),
                        Err(error) => Err(error),
                    };
                }
                GeneratorContinuation::ObjectKey {
                    entries,
                    next,
                    object,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    let key = match computed_key(value) {
                        Ok(key) => key,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let Some(entry) = entries.get(next) else {
                        result = Err(invalid("object value missing after validation"));
                        continue;
                    };
                    let value_node = entry.value;
                    work.continuations.push(GeneratorContinuation::ObjectValue {
                        entries,
                        next: next.saturating_add(1),
                        object,
                        key,
                        input: input.clone(),
                        origin: origin.clone(),
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: value_node,
                        input,
                        origin,
                        environment,
                        frames,
                        continuations: work.continuations.clone(),
                    }));
                    break;
                }
                GeneratorContinuation::ObjectValue {
                    entries,
                    next,
                    mut object,
                    key,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    Arc::make_mut(&mut object).insert(key, value);
                    if let Some(next_result) = schedule_object_entry(
                        bytecode,
                        &entries,
                        next,
                        object,
                        input,
                        origin,
                        environment,
                        frames,
                        work.continuations.clone(),
                        &mut pending,
                    ) {
                        // A completed object is a normal generator result;
                        // continue through outer continuations so user-call
                        // frames and pull consumers are restored correctly.
                        if next_result.is_ok() {
                            result_origin = match fresh_origin(&mut next_origin) {
                                Ok(origin) => Some(origin),
                                Err(error) => {
                                    result = Err(error);
                                    continue;
                                }
                            };
                        }
                        result = next_result;
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::FoldInitial {
                    generator,
                    pattern,
                    update,
                    extract,
                    emit_each_update,
                    input,
                    environment,
                } => {
                    let state = Rc::new(RefCell::new(FoldState { accumulator: value }));
                    pending.push(GeneratorTask::FoldGenerator {
                        generator,
                        pattern,
                        update,
                        extract,
                        emit_each_update,
                        state,
                        input,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    });
                    break;
                }
                GeneratorContinuation::FoldItem {
                    state,
                    pattern,
                    update,
                    extract,
                    emit_each_update,
                    environment,
                } => {
                    let mut nested = environment.as_ref().clone();
                    let pattern_depth = work.continuations.len();
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
                        observations.call_stack_high_water =
                            observations.call_stack_high_water.max(depth + 1);
                        Ok(())
                    };
                    if let Err(error) = bind_pattern(
                        bytecode,
                        &pattern,
                        &value,
                        &mut nested,
                        pattern_depth,
                        &mut charge,
                    ) {
                        result = Err(error);
                        continue;
                    }
                    let accumulator = state.borrow().accumulator.clone();
                    state.borrow_mut().accumulator = Value::Null;
                    work.continuations.push(GeneratorContinuation::FoldUpdate {
                        state,
                        extract,
                        emit_each_update,
                        environment: Arc::new(nested.clone()),
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: update,
                        input: accumulator,
                        origin: None,
                        environment: Arc::new(nested),
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::FoldUpdate {
                    state,
                    extract,
                    emit_each_update,
                    environment,
                } => {
                    state.borrow_mut().accumulator = value.clone();
                    if let Some(extract) = extract {
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: extract,
                            input: value,
                            origin: None,
                            environment,
                            frames: work.frames,
                            continuations: work.continuations,
                        }));
                        break;
                    }
                    if emit_each_update {
                        result = Ok(value);
                    } else {
                        break;
                    }
                }
                GeneratorContinuation::PathField(key) => {
                    let Value::Array(path) = value else {
                        result = Err(runtime("assignment left side is not a path".to_owned()));
                        continue;
                    };
                    let mut components = match jq_path(&Value::Array(path)) {
                        Ok(components) => components,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    components.push(PathComponent::Key(key));
                    if components.len() > limits.path_stack {
                        result = Err(resource("path-stack"));
                        continue;
                    }
                    observations.path_stack_high_water =
                        observations.path_stack_high_water.max(components.len());
                    result = Ok(path_value(&Path::new(components)));
                }
                GeneratorContinuation::PathIndex {
                    node,
                    input,
                    origin,
                    environment,
                } => {
                    let path = match jq_path(&value) {
                        Ok(path) => Path::new(path),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let target = match getpath_managed(
                        &input,
                        path.components(),
                        limits,
                        &mut observations,
                        cancellation,
                        stop,
                    ) {
                        Ok(target) => target,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    work.continuations
                        .push(GeneratorContinuation::PathIndexValue { path, target });
                    work.continuations.push(GeneratorContinuation::PathOrigin);
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node,
                        input,
                        origin,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::PathIndexValue { path, target } => {
                    let component = match path_component_for_target(&target, &value) {
                        Ok(component) => component,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let mut components = path.components().to_vec();
                    components.push(component);
                    result = Ok(path_value(&Path::new(components)));
                }
                GeneratorContinuation::PathIterate { input } => {
                    let path = match jq_path(&value) {
                        Ok(path) => Path::new(path),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let selected = match getpath_managed(
                        &input,
                        path.components(),
                        limits,
                        &mut observations,
                        cancellation,
                        stop,
                    ) {
                        Ok(selected) => selected,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    match selected {
                        Value::Array(_) | Value::Object(_) => {
                            pending.push(GeneratorTask::PathChildren {
                                value: selected,
                                path,
                                next: 0,
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: work.continuations.clone(),
                            });
                            break;
                        }
                        selected => {
                            result = Err(type_error("update iteration", &selected));
                        }
                    }
                }
                GeneratorContinuation::PathPipe {
                    node,
                    input,
                    environment,
                } => {
                    let path = match jq_path(&value) {
                        Ok(path) => Path::new(path),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let selected = match getpath_managed(
                        &input,
                        path.components(),
                        limits,
                        &mut observations,
                        cancellation,
                        stop,
                    ) {
                        Ok(selected) => selected,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    work.continuations
                        .push(GeneratorContinuation::PathJoin(path.clone()));
                    let mut origin_charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let selected_origin = match origin_at_path_bounded(
                        work.origin.as_ref(),
                        &path,
                        limits,
                        &mut origin_charge,
                    ) {
                        Ok(origin) => origin,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    pending.push(GeneratorTask::PathEval(PathWork {
                        node,
                        input: selected,
                        origin: selected_origin,
                        prefix: Path::root(),
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::PathJoin(prefix) => {
                    let relative = match jq_path(&value) {
                        Ok(path) => path,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let mut components = prefix.components().to_vec();
                    components.extend(relative);
                    result = Ok(path_value(&Path::new(components)));
                }
                GeneratorContinuation::PathSelect(path)
                | GeneratorContinuation::PathFilterResult(path) => {
                    if value.is_truthy() {
                        delivered_from_path = true;
                        result = Ok(path_value(&path));
                    } else {
                        break;
                    }
                }
                GeneratorContinuation::PathOrigin => {
                    delivered_from_path = true;
                    result = Ok(value);
                }
                GeneratorContinuation::PathGetPath { base, input } => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let resolution =
                        scalar::bounded_getpath_resolution(&input, &value, limits, &mut charge);
                    let resolution = match resolution {
                        Ok(resolution) => resolution,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let Some(relative) = resolution.path else {
                        result = Err(runtime("assignment left side is not a path".to_owned()));
                        continue;
                    };
                    let Some(total) = base
                        .components()
                        .len()
                        .checked_add(relative.components().len())
                    else {
                        result = Err(resource("path-stack"));
                        continue;
                    };
                    if total > limits.path_stack {
                        result = Err(resource("path-stack"));
                        continue;
                    }
                    if let Err(error) = charge() {
                        result = Err(error);
                        continue;
                    }
                    let mut components = Vec::new();
                    if components.try_reserve_exact(total).is_err() {
                        result = Err(resource("path-stack"));
                        continue;
                    }
                    components.extend_from_slice(base.components());
                    components.extend_from_slice(relative.components());
                    delivered_from_path = true;
                    result = Ok(path_value(&Path::new(components)));
                }
                GeneratorContinuation::PathCollect { paths, .. } => {
                    let path = match jq_path(&value) {
                        Ok(path) => path,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    if let Err(error) = paths.borrow_mut().push(&path, limits, &mut charge) {
                        result = Err(error);
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::FreshOrigin => match fresh_origin(&mut next_origin) {
                    Ok(origin) => result_origin = Some(origin),
                    Err(error) => result = Err(error),
                },
                GeneratorContinuation::PathFilter {
                    root,
                    filter,
                    origin,
                    environment,
                    frames,
                } => {
                    let path = match jq_path(&value) {
                        Ok(path) => Path::new(path),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let selected = match getpath_managed(
                        &root,
                        path.components(),
                        limits,
                        &mut observations,
                        cancellation,
                        stop,
                    ) {
                        Ok(selected) => selected,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let mut origin_charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let selected_origin = match origin_at_path_bounded(
                        origin.as_ref(),
                        &path,
                        limits,
                        &mut origin_charge,
                    ) {
                        Ok(origin) => origin,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    work.continuations
                        .push(GeneratorContinuation::PathFilterResult(path));
                    work.continuations.push(GeneratorContinuation::FreshOrigin);
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: filter,
                        input: selected,
                        origin: selected_origin,
                        environment,
                        frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::PathConditional {
                    branches,
                    next,
                    alternative,
                    input,
                    origin,
                    prefix,
                    environment,
                } => {
                    let node = if value.is_truthy() {
                        branches[next].1
                    } else if let Some((condition, _)) = branches.get(next + 1) {
                        work.continuations
                            .push(GeneratorContinuation::PathConditional {
                                branches: Arc::clone(&branches),
                                next: next + 1,
                                alternative,
                                input: input.clone(),
                                origin: origin.clone(),
                                prefix: prefix.clone(),
                                environment: Arc::clone(&environment),
                            });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: *condition,
                            input,
                            origin: origin.clone(),
                            environment,
                            frames: work.frames,
                            continuations: work.continuations,
                        }));
                        break;
                    } else {
                        alternative
                    };
                    if path_candidate_with_aliases(bytecode, node, &work.continuations) {
                        pending.push(GeneratorTask::PathEval(PathWork {
                            node,
                            input,
                            origin: origin.clone(),
                            prefix,
                            environment,
                            frames: work.frames,
                            continuations: work.continuations,
                        }));
                    } else {
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node,
                            input,
                            origin,
                            environment,
                            frames: work.frames,
                            continuations: work.continuations,
                        }));
                    }
                    break;
                }
                GeneratorContinuation::PathBind {
                    pattern,
                    body,
                    input,
                    input_origin,
                    prefix,
                    environment,
                    entered_from_path,
                } => {
                    if !delivered_from_path {
                        work.continuations
                            .push(GeneratorContinuation::PathBindValue {
                                pattern,
                                body,
                                input,
                                input_origin,
                                prefix,
                                environment,
                                entered_from_path,
                            });
                        result = Ok(value);
                        continue;
                    }
                    let path = match jq_path(&value) {
                        Ok(path) => Path::new(path),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let selected = match getpath_managed(
                        &input,
                        path.components(),
                        limits,
                        &mut observations,
                        cancellation,
                        stop,
                    ) {
                        Ok(selected) => selected,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let mut nested = environment.as_ref().clone();
                    let pattern_depth = work.continuations.len();
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
                        observations.call_stack_high_water =
                            observations.call_stack_high_water.max(depth + 1);
                        Ok(())
                    };
                    if let Err(error) = bind_pattern(
                        bytecode,
                        &pattern,
                        &selected,
                        &mut nested,
                        pattern_depth,
                        &mut charge,
                    ) {
                        result = Err(error);
                        continue;
                    }
                    let mut origin_charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let selected_origin = match origin_at_path_bounded(
                        work.origin.as_ref(),
                        &path,
                        limits,
                        &mut origin_charge,
                    ) {
                        Ok(origin) => origin,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let mut pattern_origin_charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    if let Err(error) = record_pattern_origin(
                        bytecode,
                        &pattern,
                        selected_origin.clone(),
                        &mut nested.origins,
                        limits,
                        &mut pattern_origin_charge,
                    ) {
                        result = Err(error);
                        continue;
                    }
                    let mut aliases = current_path_aliases(&work.continuations);
                    clear_path_aliases(bytecode, &pattern, &mut aliases);
                    if let Err(error) = record_pattern_aliases(
                        bytecode,
                        &pattern,
                        &path,
                        selected_origin.as_ref(),
                        &mut aliases,
                        limits,
                        &mut pattern_origin_charge,
                    ) {
                        result = Err(error);
                        continue;
                    }
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::PathAliases(Arc::new(aliases)));
                    if entered_from_path
                        && path_candidate_with_aliases(bytecode, body, &continuations)
                    {
                        pending.push(GeneratorTask::PathEval(PathWork {
                            node: body,
                            input,
                            origin: input_origin.clone(),
                            prefix,
                            environment: Arc::new(nested),
                            frames: work.frames,
                            continuations,
                        }));
                    } else {
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: body,
                            input,
                            origin: input_origin,
                            environment: Arc::new(nested),
                            frames: work.frames,
                            continuations,
                        }));
                    }
                    break;
                }
                GeneratorContinuation::PathBindAlternatives {
                    boundary,
                    patterns,
                    body,
                    input,
                    prefix,
                    environment,
                } => {
                    match schedule_bind_alternative(
                        bytecode,
                        patterns,
                        0,
                        value,
                        input,
                        environment,
                        body,
                        work.frames.clone(),
                        work.continuations.clone(),
                        boundary,
                        &mut pending,
                        limits,
                        cancellation,
                        stop,
                        &mut observations,
                        true,
                        prefix,
                    ) {
                        Ok(()) => break,
                        Err(error) => result = Err(error),
                    }
                }
                GeneratorContinuation::PathBindValue {
                    pattern,
                    body,
                    input,
                    input_origin,
                    prefix,
                    environment,
                    entered_from_path,
                } => {
                    let mut nested = environment.as_ref().clone();
                    let pattern_depth = work.continuations.len();
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
                        observations.call_stack_high_water =
                            observations.call_stack_high_water.max(depth + 1);
                        Ok(())
                    };
                    if let Err(error) = bind_pattern(
                        bytecode,
                        &pattern,
                        &value,
                        &mut nested,
                        pattern_depth,
                        &mut charge,
                    ) {
                        result = Err(error);
                        continue;
                    }
                    let mut pattern_origin_charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    if let Err(error) = record_pattern_origin(
                        bytecode,
                        &pattern,
                        result_origin.clone(),
                        &mut nested.origins,
                        limits,
                        &mut pattern_origin_charge,
                    ) {
                        result = Err(error);
                        continue;
                    }
                    let mut aliases = current_path_aliases(&work.continuations);
                    clear_path_aliases(bytecode, &pattern, &mut aliases);
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::PathAliases(Arc::new(aliases)));
                    if entered_from_path
                        && path_candidate_with_aliases(bytecode, body, &continuations)
                    {
                        pending.push(GeneratorTask::PathEval(PathWork {
                            node: body,
                            input,
                            origin: input_origin,
                            prefix,
                            environment: Arc::new(nested),
                            frames: work.frames,
                            continuations,
                        }));
                    } else {
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: body,
                            input,
                            origin: input_origin,
                            environment: Arc::new(nested),
                            frames: work.frames,
                            continuations,
                        }));
                    }
                    break;
                }
                GeneratorContinuation::PathSliceBase {
                    start,
                    end,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    let path = match jq_path(&value) {
                        Ok(path) => Path::new(path),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let target = match getpath_managed(
                        &input,
                        path.components(),
                        limits,
                        &mut observations,
                        cancellation,
                        stop,
                    ) {
                        Ok(target) => target,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let assignment = work.continuations.last().and_then(|continuation| {
                        if let GeneratorContinuation::AssignmentPath(state) = continuation {
                            Some(Rc::clone(state))
                        } else {
                            None
                        }
                    });
                    let mut path_continuations = work.continuations.clone();
                    if assignment.is_some() {
                        path_continuations.pop();
                    }
                    if let Some(start) = start {
                        let mut slice_continuations = path_continuations;
                        slice_continuations.push(GeneratorContinuation::PathSliceStart {
                            path,
                            target,
                            end,
                            assignment,
                            input: input.clone(),
                            origin: origin.clone(),
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: start,
                            input,
                            origin: origin.clone(),
                            environment,
                            frames,
                            continuations: slice_continuations,
                        }));
                        break;
                    }
                    if let Some(end) = end {
                        let mut slice_continuations = path_continuations;
                        slice_continuations.push(GeneratorContinuation::PathSliceEnd {
                            path,
                            target,
                            start: None,
                            assignment,
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: end,
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: slice_continuations,
                        }));
                        break;
                    }
                    if let Some(next_result) = schedule_path_slice(
                        path,
                        target,
                        None,
                        None,
                        assignment,
                        environment,
                        frames,
                        path_continuations,
                        &mut pending,
                    ) {
                        result = next_result;
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::PathSliceStart {
                    path,
                    target,
                    end,
                    assignment,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    let start = match slice_bound_value(&value) {
                        Ok(start) => start,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    if let Some(end) = end {
                        let mut slice_continuations = work.continuations;
                        slice_continuations.push(GeneratorContinuation::PathSliceEnd {
                            path,
                            target,
                            start,
                            assignment,
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: end,
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: slice_continuations,
                        }));
                        break;
                    }
                    if let Some(next_result) = schedule_path_slice(
                        path,
                        target,
                        start,
                        None,
                        assignment,
                        environment,
                        frames,
                        work.continuations.clone(),
                        &mut pending,
                    ) {
                        result = next_result;
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::PathSliceEnd {
                    path,
                    target,
                    start,
                    assignment,
                } => {
                    let end = match slice_bound_value(&value) {
                        Ok(end) => end,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    if let Some(next_result) = schedule_path_slice(
                        path,
                        target,
                        start,
                        end,
                        assignment,
                        Arc::clone(&work.environment),
                        Arc::clone(&work.frames),
                        work.continuations.clone(),
                        &mut pending,
                    ) {
                        result = next_result;
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::PathUserReturn {
                    environment,
                    frames,
                }
                | GeneratorContinuation::ReturnUser {
                    environment,
                    frames,
                } => {
                    work.environment = environment;
                    work.frames = frames;
                    result = Ok(value);
                }
                GeneratorContinuation::AssignmentPath(state) => {
                    if !delivered_from_path {
                        result = Err(runtime("assignment left side is not a path".to_owned()));
                        continue;
                    }
                    let path = match jq_path(&value) {
                        Ok(path) => Path::new(path),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    if path.components().len() > limits.path_stack {
                        result = Err(resource("path-stack"));
                        continue;
                    }
                    state
                        .borrow_mut()
                        .targets
                        .push(AssignmentTarget::Path(path));
                    state
                        .borrow_mut()
                        .target_origins
                        .push(result_origin.clone());
                    break;
                }
                GeneratorContinuation::AssignmentRhs(state) => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let output =
                        apply_plain_assignment(&state.borrow(), &value, limits, &mut charge);
                    result = output;
                }
                GeneratorContinuation::AssignmentUpdateRhs {
                    state,
                    target,
                    old,
                    next,
                    boundary,
                    accepted,
                } => {
                    if accepted.replace(true) {
                        break;
                    }
                    pending.retain(|task| !task_owned_by_assignment_rhs(task, boundary));
                    let replacement = match update_value(state.borrow().operator, &old, &value) {
                        Ok(replacement) => replacement,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let updated = match apply_assignment_target(
                        &state.borrow().document,
                        &target,
                        replacement,
                        limits,
                        &mut charge,
                    ) {
                        Ok(updated) => updated,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    state.borrow_mut().document = updated;
                    pending.push(GeneratorTask::AssignmentUpdate {
                        state,
                        next,
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations,
                    });
                    break;
                }
                GeneratorContinuation::Pipe { node, environment } => {
                    current = Some(GeneratorWork {
                        node,
                        input: value,
                        origin: result_origin,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    });
                    break;
                }
                GeneratorContinuation::BinaryLeft {
                    operator,
                    right,
                    input,
                    origin,
                    environment,
                } => {
                    if (operator == BinaryOperator::And && !value.is_truthy())
                        || (operator == BinaryOperator::Or && value.is_truthy())
                    {
                        result_origin = match fresh_origin(&mut next_origin) {
                            Ok(origin) => Some(origin),
                            Err(error) => {
                                result = Err(error);
                                continue;
                            }
                        };
                        result = Ok(Value::Bool(operator == BinaryOperator::Or));
                        continue;
                    }
                    work.continuations.push(GeneratorContinuation::BinaryRight {
                        operator,
                        left: value,
                    });
                    current = Some(GeneratorWork {
                        node: right,
                        input,
                        origin,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    });
                    break;
                }
                GeneratorContinuation::BinaryRight { operator, left } => {
                    let computed = if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
                        Ok(Value::Bool(value.is_truthy()))
                    } else {
                        binary_value(operator, &left, &value)
                    };
                    match fresh_origin(&mut next_origin) {
                        Ok(origin) => {
                            result_origin = Some(origin);
                            result = computed;
                        }
                        Err(error) => result = Err(error),
                    }
                }
                GeneratorContinuation::Unary(operator) => match fresh_origin(&mut next_origin) {
                    Ok(origin) => {
                        result_origin = Some(origin);
                        result = unary(operator, &value);
                    }
                    Err(error) => result = Err(error),
                },
                GeneratorContinuation::ArrayItem(values) => {
                    values.borrow_mut().push(value);
                    break;
                }
                GeneratorContinuation::ArrayUpdateItem { values, accepted } => {
                    if !accepted.replace(true) {
                        values.borrow_mut().push(value);
                    }
                    break;
                }
                GeneratorContinuation::ObjectItem {
                    key,
                    values,
                    accepted,
                } => {
                    if !accepted.replace(true) {
                        values.borrow_mut().insert(key, value);
                    }
                    break;
                }
                GeneratorContinuation::Select { input, origin } => {
                    if value.is_truthy() {
                        result_origin = origin;
                        result = Ok(input);
                    } else {
                        break;
                    }
                }
                GeneratorContinuation::HasArgument {
                    input,
                    container_from_result,
                } => {
                    let computed = if container_from_result {
                        has(&value, &input)
                    } else {
                        has(&input, &value)
                    };
                    match fresh_origin(&mut next_origin) {
                        Ok(origin) => {
                            result_origin = Some(origin);
                            result = computed;
                        }
                        Err(error) => result = Err(error),
                    }
                }
                GeneratorContinuation::SortKeyItem(keys) => {
                    keys.borrow_mut().push(value);
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
                    pattern,
                    body,
                    input,
                    input_origin,
                    environment,
                    origin,
                } => {
                    let mut nested = environment.as_ref().clone();
                    let pattern_depth = work.continuations.len();
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
                        observations.call_stack_high_water =
                            observations.call_stack_high_water.max(depth + 1);
                        Ok(())
                    };
                    match bind_pattern(
                        bytecode,
                        &pattern,
                        &value,
                        &mut nested,
                        pattern_depth,
                        &mut charge,
                    ) {
                        Ok(()) => {
                            let mut pattern_origin_charge = || {
                                charge_managed_step(&mut observations, limits, cancellation, stop)
                            };
                            if let Err(error) = record_pattern_origin(
                                bytecode,
                                &pattern,
                                result_origin.clone(),
                                &mut nested.origins,
                                limits,
                                &mut pattern_origin_charge,
                            ) {
                                result = Err(error);
                                continue;
                            }
                            let mut aliases = current_path_aliases(&work.continuations);
                            clear_path_aliases(bytecode, &pattern, &mut aliases);
                            if let Some(origin) = result_origin.clone().or(origin) {
                                let alias_path = match relative_alias_path(
                                    input_origin.as_ref(),
                                    &origin,
                                    limits,
                                    &mut pattern_origin_charge,
                                ) {
                                    Ok(path) => path,
                                    Err(error) => {
                                        result = Err(error);
                                        continue;
                                    }
                                };
                                if let Err(error) = record_pattern_aliases(
                                    bytecode,
                                    &pattern,
                                    &alias_path,
                                    Some(&origin),
                                    &mut aliases,
                                    limits,
                                    &mut pattern_origin_charge,
                                ) {
                                    result = Err(error);
                                    continue;
                                }
                            }
                            let mut continuations = work.continuations;
                            continuations
                                .push(GeneratorContinuation::PathAliases(Arc::new(aliases)));
                            pending.push(GeneratorTask::Eval(GeneratorWork {
                                node: body,
                                input,
                                origin: input_origin,
                                environment: Arc::new(nested),
                                frames: work.frames,
                                continuations,
                            }));
                            break;
                        }
                        Err(error) => result = Err(error),
                    }
                }
                GeneratorContinuation::BindAlternatives {
                    patterns,
                    body,
                    input,
                    environment,
                } => {
                    let boundary = next_boundary;
                    next_boundary = next_boundary.saturating_add(1);
                    match schedule_bind_alternative(
                        bytecode,
                        patterns,
                        0,
                        value,
                        input,
                        environment,
                        body,
                        work.frames.clone(),
                        work.continuations.clone(),
                        boundary,
                        &mut pending,
                        limits,
                        cancellation,
                        stop,
                        &mut observations,
                        false,
                        Path::root(),
                    ) {
                        Ok(()) => break,
                        Err(error) => result = Err(error),
                    }
                }
                GeneratorContinuation::Conditional {
                    branches,
                    next,
                    alternative,
                    input,
                    origin,
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
                            origin: origin.clone(),
                            environment: Arc::clone(&environment),
                        });
                        condition
                    } else {
                        alternative
                    };
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node,
                        input,
                        origin,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::Iterate => {
                    let cursor = iterate::ContainerCursor::new(value.clone());
                    if !cursor.is_container() {
                        result = Err(type_error("iterate", &value));
                        continue;
                    }
                    pending.push(GeneratorTask::IterateCursor {
                        cursor,
                        origin: result_origin,
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    observations.fork_stack_high_water =
                        observations.fork_stack_high_water.max(pending.len());
                    break;
                }
                GeneratorContinuation::PullConsumer { state } => {
                    let mut state = state.borrow_mut();
                    state.seen = true;
                    let boundary = state.boundary;
                    match state.kind {
                        PullConsumerKind::First => {
                            result = Ok(value);
                            drop(state);
                            pending.retain(|task| !task_owned_by_pull_consumer(task, boundary));
                        }
                        PullConsumerKind::Last => {
                            state.latest = Some(value);
                            state.latest_origin = result_origin;
                            break;
                        }
                        PullConsumerKind::IsEmpty => {
                            result = Ok(Value::Bool(false));
                            drop(state);
                            pending.retain(|task| !task_owned_by_pull_consumer(task, boundary));
                        }
                        PullConsumerKind::Nth => {
                            if state.remaining > 0 {
                                state.remaining = state.remaining.saturating_sub(1);
                                break;
                            }
                            state.latest = Some(value);
                            state.latest_origin = result_origin;
                            drop(state);
                            pending
                                .retain(|task| !task_owned_by_pull_consumer_work(task, boundary));
                            break;
                        }
                        PullConsumerKind::Skip => {
                            if state.remaining > 0 {
                                state.remaining = state.remaining.saturating_sub(1);
                                break;
                            }
                            result = Ok(value);
                        }
                        PullConsumerKind::Any | PullConsumerKind::All => {
                            drop(state);
                            result = Err(invalid("predicate consumer continuation mismatch"));
                        }
                    }
                }
                GeneratorContinuation::PredicateItem {
                    state,
                    condition,
                    environment,
                    frames,
                } => {
                    if state.borrow().decision.is_some() {
                        break;
                    }
                    if let Some(condition) = condition {
                        work.continuations
                            .push(GeneratorContinuation::PredicateCondition { state });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: condition,
                            input: value,
                            origin: result_origin,
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                        break;
                    }
                    let mut state = state.borrow_mut();
                    state.seen = true;
                    let matches = value.is_truthy();
                    let decisive = match state.kind {
                        PullConsumerKind::Any => matches,
                        PullConsumerKind::All => !matches,
                        _ => false,
                    };
                    if decisive {
                        state.decision = Some(matches);
                        let boundary = state.boundary;
                        drop(state);
                        pending.retain(|task| !task_owned_by_pull_consumer_work(task, boundary));
                    }
                    break;
                }
                GeneratorContinuation::PredicateCondition { state } => {
                    if state.borrow().decision.is_some() {
                        break;
                    }
                    let mut state = state.borrow_mut();
                    state.seen = true;
                    let matches = value.is_truthy();
                    let decisive = match state.kind {
                        PullConsumerKind::Any => matches,
                        PullConsumerKind::All => !matches,
                        _ => false,
                    };
                    if decisive {
                        state.decision = Some(matches);
                        let boundary = state.boundary;
                        drop(state);
                        pending.retain(|task| !task_owned_by_pull_consumer_work(task, boundary));
                    }
                    break;
                }
                GeneratorContinuation::PullCount {
                    kind,
                    input,
                    origin,
                    generator,
                    environment,
                    frames,
                } => {
                    if kind == PullConsumerKind::Nth && generator.is_none() {
                        result = nth_index(&input, &value);
                        if result.is_ok() {
                            result_origin = match &value {
                                Value::Number(number) => {
                                    let mut charge = || {
                                        charge_managed_step(
                                            &mut observations,
                                            limits,
                                            cancellation,
                                            stop,
                                        )
                                    };
                                    match index_origin(
                                        &input,
                                        number.as_f64().trunc(),
                                        origin.as_ref(),
                                        limits,
                                        &mut charge,
                                    ) {
                                        Ok(origin) => origin,
                                        Err(error) => {
                                            result = Err(error);
                                            None
                                        }
                                    }
                                }
                                _ => None,
                            };
                        }
                        continue;
                    }
                    let count = match limit_count(&value) {
                        Ok(count) => count,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let Some(generator) = generator else {
                        result = Err(invalid("skip arity"));
                        continue;
                    };
                    let boundary = next_boundary;
                    next_boundary = next_boundary.saturating_add(1);
                    let state = Rc::new(RefCell::new(PullConsumerState {
                        boundary,
                        kind,
                        seen: false,
                        latest: None,
                        latest_origin: None,
                        decision: None,
                        remaining: count,
                    }));
                    if kind == PullConsumerKind::Nth {
                        pending.push(GeneratorTask::FinishPullConsumer {
                            state: Rc::clone(&state),
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: work.continuations.clone(),
                        });
                    }
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::PullConsumer { state });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: generator,
                        input,
                        origin,
                        environment,
                        frames,
                        continuations,
                    }));
                    break;
                }
                GeneratorContinuation::FromStreamItem(state) => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    match state.borrow_mut().feed(&value, limits, &mut charge) {
                        Ok(Some(value)) => {
                            result = Ok(value);
                            result_origin = match fresh_origin(&mut next_origin) {
                                Ok(origin) => Some(origin),
                                Err(error) => {
                                    result = Err(error);
                                    None
                                }
                            };
                        }
                        Ok(None) => break,
                        Err(error) => {
                            result = Err(error);
                        }
                    }
                }
                GeneratorContinuation::TruncateStreamItem { count } => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    match generator::truncate_stream_event(&value, &count, limits, &mut charge) {
                        Ok(Some(value)) => {
                            result = Ok(value);
                            result_origin = match fresh_origin(&mut next_origin) {
                                Ok(origin) => Some(origin),
                                Err(error) => {
                                    result = Err(error);
                                    None
                                }
                            };
                        }
                        Ok(None) => break,
                        Err(error) => {
                            result = Err(error);
                        }
                    }
                }
                GeneratorContinuation::AddItem(state) => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let mut state = state.borrow_mut();
                    state.accumulator = Some(match state.accumulator.take() {
                        None => value,
                        Some(accumulator) => match scalar::bounded_binary_add(
                            &accumulator,
                            &value,
                            limits,
                            &mut charge,
                        ) {
                            Ok(value) => value,
                            Err(error) => {
                                result = Err(error);
                                continue;
                            }
                        },
                    });
                    break;
                }
                GeneratorContinuation::DebugItem => {
                    let bytes = match debug_effect(&value, limits.output_bytes) {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    match append_effect(effects, &bytes, limits.output_bytes) {
                        Ok(()) => {
                            work.continuations.push(GeneratorContinuation::DebugItem);
                            break;
                        }
                        Err(error) => {
                            result = Err(error);
                        }
                    }
                }
                GeneratorContinuation::FromEntries => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    result = match &value {
                        Value::Array(entries) => scalar::bounded_from_entries(
                            entries,
                            "with_entries",
                            limits,
                            &mut charge,
                        ),
                        value => Err(type_error("with_entries", value)),
                    };
                }
                GeneratorContinuation::HaltError { input } => {
                    result = match halt_error_text(&input, limits.output_bytes) {
                        Ok(stderr) => match halt_status(&value) {
                            Ok(status) => Err(VmError::Halt {
                                status,
                                stderr: Arc::from(stderr.as_bytes()),
                            }),
                            Err(error) => Err(error),
                        },
                        Err(error) => Err(error),
                    };
                }
                GeneratorContinuation::BindAlternativeBody { .. }
                | GeneratorContinuation::PathBindAlternativeBody { .. }
                | GeneratorContinuation::OptionalBoundary { .. }
                | GeneratorContinuation::Catch { .. }
                | GeneratorContinuation::PathCatch { .. }
                | GeneratorContinuation::Label(_)
                | GeneratorContinuation::PathAliases(_)
                | GeneratorContinuation::RepeatItem => {
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
                        .and_then(|remaining| format::text(&value, remaining));
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
                GeneratorContinuation::RegexReplacementItem { name, state } => {
                    let text = match value {
                        Value::String(text) => text,
                        Value::Null => Arc::from(""),
                        value => {
                            result = Err(type_error(&name, &value));
                            continue;
                        }
                    };
                    let checkpoint_steps = Cell::new(observations.steps);
                    let checkpoint = || {
                        if stop.load(Ordering::Relaxed)
                            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                        {
                            return Err(VmError::Interrupted);
                        }
                        if checkpoint_steps.get() >= limits.steps {
                            return Err(resource("vm-steps"));
                        }
                        checkpoint_steps.set(checkpoint_steps.get().saturating_add(1));
                        Ok(())
                    };
                    let pushed = state.borrow_mut().push_replacement(text, &checkpoint);
                    observations.steps = checkpoint_steps.get();
                    if let Err(error) = pushed {
                        result = Err(error);
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::RegexArguments {
                    name,
                    arguments,
                    order,
                    next,
                    mut values,
                    replacement,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    let Some(slot) = next.checked_sub(1).and_then(|index| order.get(index)) else {
                        result = Err(invalid("regex argument continuation is inconsistent"));
                        continue;
                    };
                    let Some(slot_value) = values.get_mut(*slot) else {
                        result = Err(invalid("regex argument slot is missing"));
                        continue;
                    };
                    *slot_value = Some(value);
                    if next < order.len() {
                        let argument = order[next];
                        work.continuations
                            .push(GeneratorContinuation::RegexArguments {
                                name,
                                arguments: Arc::clone(&arguments),
                                order: Arc::clone(&order),
                                next: next.saturating_add(1),
                                values,
                                replacement,
                                input: input.clone(),
                                origin: origin.clone(),
                                environment: Arc::clone(&environment),
                                frames: Arc::clone(&frames),
                            });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: arguments[argument],
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                        break;
                    }
                    let arguments = values
                        .into_iter()
                        .map(|value| value.unwrap_or(Value::Null))
                        .collect::<Vec<_>>();
                    pending.push(GeneratorTask::InvokeRegex {
                        name,
                        arguments,
                        replacement,
                        input,
                        origin,
                        environment,
                        frames,
                        continuations: work.continuations,
                    });
                    break;
                }
                GeneratorContinuation::PathUserArgument {
                    symbol,
                    arguments,
                    next,
                    input,
                    input_origin,
                    prefix,
                    caller_environment,
                    caller_frames,
                    filters,
                    mut bindings,
                    from_path,
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
                    let _bound_origin = if from_path {
                        let path = match jq_path(&value) {
                            Ok(path) => Path::new(path),
                            Err(error) => {
                                result = Err(error);
                                continue;
                            }
                        };
                        let mut origin_charge =
                            || charge_managed_step(&mut observations, limits, cancellation, stop);
                        let selected_origin = match origin_at_path_bounded(
                            work.origin.as_ref(),
                            &path,
                            limits,
                            &mut origin_charge,
                        ) {
                            Ok(origin) => origin,
                            Err(error) => {
                                result = Err(error);
                                continue;
                            }
                        };
                        match getpath_managed(
                            &input,
                            path.components(),
                            limits,
                            &mut observations,
                            cancellation,
                            stop,
                        ) {
                            Ok(value) => {
                                let mut aliases = current_path_aliases(&work.continuations);
                                aliases.remove(name);
                                if let Some(origin) = selected_origin.clone() {
                                    aliases.insert(
                                        Arc::clone(name),
                                        PathAlias {
                                            path: path.clone(),
                                            origin,
                                        },
                                    );
                                }
                                let mut continuations = work.continuations.clone();
                                continuations
                                    .push(GeneratorContinuation::PathAliases(Arc::new(aliases)));
                                work.continuations = continuations;
                                bindings.origins.remove(name);
                                if let Some(origin) = selected_origin.clone() {
                                    bindings.origins.insert(Arc::clone(name), origin);
                                }
                                bindings.insert(Arc::clone(name), value);
                                selected_origin
                            }
                            Err(error) => {
                                result = Err(error);
                                continue;
                            }
                        }
                    } else {
                        let mut aliases = current_path_aliases(&work.continuations);
                        aliases.remove(name);
                        let mut continuations = work.continuations.clone();
                        continuations.push(GeneratorContinuation::PathAliases(Arc::new(aliases)));
                        work.continuations = continuations;
                        let origin = result_origin.clone();
                        bindings.origins.remove(name);
                        if let Some(origin) = origin.clone() {
                            bindings.origins.insert(Arc::clone(name), origin);
                        }
                        bindings.insert(Arc::clone(name), value);
                        origin
                    };
                    let mut next_value = next.saturating_add(1);
                    while function
                        .parameters
                        .get(next_value)
                        .is_some_and(|parameter| parameter.kind == ParameterKind::Filter)
                    {
                        next_value = next_value.saturating_add(1);
                    }
                    if function
                        .parameters
                        .get(next_value)
                        .is_some_and(|parameter| parameter.kind == ParameterKind::Value)
                    {
                        let mut next_continuations = work.continuations;
                        let from_path = path_candidate_with_aliases(
                            bytecode,
                            arguments[next_value],
                            &next_continuations,
                        );
                        next_continuations.push(GeneratorContinuation::PathUserArgument {
                            symbol,
                            arguments: Arc::clone(&arguments),
                            next: next_value,
                            input: input.clone(),
                            input_origin: input_origin.clone(),
                            prefix: prefix.clone(),
                            caller_environment: Arc::clone(&caller_environment),
                            caller_frames: Arc::clone(&caller_frames),
                            filters,
                            bindings,
                            from_path,
                        });
                        if from_path {
                            pending.push(GeneratorTask::PathEval(PathWork {
                                node: arguments[next_value],
                                input,
                                origin: input_origin.clone(),
                                prefix,
                                environment: caller_environment,
                                frames: caller_frames,
                                continuations: next_continuations,
                            }));
                        } else {
                            pending.push(GeneratorTask::Eval(GeneratorWork {
                                node: arguments[next_value],
                                input,
                                origin: input_origin.clone(),
                                environment: caller_environment,
                                frames: caller_frames,
                                continuations: next_continuations,
                            }));
                        }
                        break;
                    }
                    let mut frames = caller_frames.to_vec();
                    frames.push(UserFrame { symbol, filters });
                    let mut next_continuations = work.continuations;
                    next_continuations.push(GeneratorContinuation::PathUserReturn {
                        environment: Arc::clone(&caller_environment),
                        frames: Arc::clone(&caller_frames),
                    });
                    pending.push(GeneratorTask::PathEval(PathWork {
                        node: function.body,
                        input,
                        origin: input_origin,
                        prefix,
                        environment: Arc::new(bindings),
                        frames: frames.into(),
                        continuations: next_continuations,
                    }));
                    break;
                }
                GeneratorContinuation::SqlInRight {
                    boundary,
                    state,
                    source,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::SqlInLeft {
                        boundary,
                        state,
                        needle: value,
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: source,
                        input,
                        origin,
                        environment,
                        frames,
                        continuations,
                    }));
                    break;
                }
                GeneratorContinuation::SqlInLeft {
                    boundary,
                    state,
                    needle,
                } => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let equal = match sql::equal_bounded(&needle, &value, limits, &mut charge) {
                        Ok(equal) => equal,
                        Err(error) => {
                            pending.retain(|task| !task_owned_by_sql(task, boundary));
                            result = Err(error);
                            continue;
                        }
                    };
                    if equal {
                        state.borrow_mut().matched = true;
                        pending.retain(|task| !task_owned_by_sql(task, boundary));
                        match fresh_origin(&mut next_origin) {
                            Ok(origin) => {
                                result_origin = Some(origin);
                                result = Ok(Value::Bool(true));
                            }
                            Err(error) => result = Err(error),
                        }
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::SqlIndexRow {
                    boundary,
                    state,
                    key,
                } => {
                    work.continuations.push(GeneratorContinuation::SqlIndexKey {
                        boundary,
                        state,
                        row: value.clone(),
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: key,
                        input: value,
                        origin: result_origin.clone(),
                        environment: work.environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::SqlIndexKey {
                    boundary,
                    state,
                    row,
                    ..
                } => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let inserted = state
                        .borrow_mut()
                        .builder
                        .as_mut()
                        .ok_or_else(|| invalid("SQL INDEX state already finished"))
                        .and_then(|builder| builder.insert(&value, row, &mut charge));
                    match inserted {
                        Ok(()) => break,
                        Err(error) => {
                            pending.retain(|task| !task_owned_by_sql(task, boundary));
                            result = Err(error);
                        }
                    }
                }
                GeneratorContinuation::SqlJoinIndex {
                    boundary,
                    key,
                    stream,
                    join,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    if let Some(stream) = stream {
                        let mut continuations = work.continuations;
                        continuations.push(GeneratorContinuation::SqlJoinRow {
                            boundary,
                            index: value,
                            key,
                            join,
                            state: None,
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: stream,
                            input,
                            origin,
                            environment,
                            frames,
                            continuations,
                        }));
                        break;
                    }

                    let cursor = iterate::ContainerCursor::new(input.clone());
                    if !cursor.is_container() {
                        result = Err(runtime(format!("Cannot iterate over {input}")));
                        continue;
                    }
                    let state = Rc::new(RefCell::new(SqlJoinState {
                        collector: Some(sql::SqlPairCollector::new(limits)),
                    }));
                    pending.push(GeneratorTask::SqlJoinFinish {
                        boundary,
                        state: Rc::clone(&state),
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: work.continuations.clone(),
                    });
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::SqlJoinRow {
                        boundary,
                        index: value,
                        key,
                        join,
                        state: Some(state),
                    });
                    pending.push(GeneratorTask::IterateCursor {
                        cursor,
                        origin,
                        environment,
                        frames,
                        continuations,
                    });
                    break;
                }
                GeneratorContinuation::SqlJoinRow {
                    boundary,
                    index,
                    key,
                    join,
                    state,
                } => {
                    work.continuations.push(GeneratorContinuation::SqlJoinKey {
                        boundary,
                        index,
                        row: value.clone(),
                        join,
                        state,
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: key,
                        input: value,
                        origin: result_origin.clone(),
                        environment: work.environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::SqlJoinKey {
                    boundary,
                    index,
                    row,
                    join,
                    state,
                } => {
                    let matched = match sql::typed_lookup(&index, &value) {
                        Ok(matched) => matched,
                        Err(error) => {
                            pending.retain(|task| !task_owned_by_sql(task, boundary));
                            result = Err(error);
                            continue;
                        }
                    };
                    if let Some(state) = state {
                        let mut charge =
                            || charge_managed_step(&mut observations, limits, cancellation, stop);
                        let pushed = state
                            .borrow_mut()
                            .collector
                            .as_mut()
                            .ok_or_else(|| invalid("SQL JOIN state already finished"))
                            .and_then(|collector| collector.push(row, matched, &mut charge));
                        match pushed {
                            Ok(()) => break,
                            Err(error) => {
                                pending.retain(|task| !task_owned_by_sql(task, boundary));
                                result = Err(error);
                                continue;
                            }
                        }
                    }
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let pair = match sql::pair_bounded(row, matched, limits, &mut charge) {
                        Ok(pair) => pair,
                        Err(error) => {
                            pending.retain(|task| !task_owned_by_sql(task, boundary));
                            result = Err(error);
                            continue;
                        }
                    };
                    if let Some(join) = join {
                        let origin = match fresh_origin(&mut next_origin) {
                            Ok(origin) => Some(origin),
                            Err(error) => {
                                result = Err(error);
                                continue;
                            }
                        };
                        let mut continuations = work.continuations;
                        continuations.push(GeneratorContinuation::SqlJoinCallback { boundary });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: join,
                            input: pair,
                            origin,
                            environment: work.environment,
                            frames: work.frames,
                            continuations,
                        }));
                        break;
                    }
                    result_origin = match fresh_origin(&mut next_origin) {
                        Ok(origin) => Some(origin),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    result = Ok(pair);
                }
                GeneratorContinuation::SqlJoinCallback { .. } => {}
                GeneratorContinuation::BuiltinArguments {
                    name,
                    arguments,
                    order,
                    next,
                    mut values,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    let Some(slot) = next.checked_sub(1).and_then(|index| order.get(index)) else {
                        result = Err(invalid("builtin argument continuation is inconsistent"));
                        continue;
                    };
                    let Some(slot_value) = values.get_mut(*slot) else {
                        result = Err(invalid("builtin argument slot is missing"));
                        continue;
                    };
                    *slot_value = Some(value);
                    if next < order.len() {
                        let argument = order[next];
                        work.continuations
                            .push(GeneratorContinuation::BuiltinArguments {
                                name,
                                arguments: Arc::clone(&arguments),
                                order: Arc::clone(&order),
                                next: next.saturating_add(1),
                                values,
                                input: input.clone(),
                                origin: origin.clone(),
                                environment: Arc::clone(&environment),
                                frames: Arc::clone(&frames),
                            });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: arguments[argument],
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                        break;
                    }
                    let Some(arguments) = values.into_iter().collect::<Option<Vec<_>>>() else {
                        result = Err(invalid("builtin argument result is missing"));
                        continue;
                    };
                    pending.push(GeneratorTask::InvokeBuiltin {
                        name,
                        arguments,
                        source_arity: order.len(),
                        input,
                        origin,
                        environment,
                        frames,
                        continuations: work.continuations,
                    });
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
                    input_origin,
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
                    bindings.origins.remove(name);
                    if let Some(origin) = result_origin.clone() {
                        bindings.origins.insert(Arc::clone(name), origin);
                    }
                    let mut aliases = current_path_aliases(&work.continuations);
                    aliases.remove(name);
                    if let Some(origin) = result_origin.clone()
                        && origin.path.components().is_empty()
                    {
                        aliases.insert(
                            Arc::clone(name),
                            PathAlias {
                                path: Path::root(),
                                origin,
                            },
                        );
                    }
                    let mut continuations = work.continuations.clone();
                    continuations.push(GeneratorContinuation::PathAliases(Arc::new(aliases)));
                    if let Some(next_result) = schedule_user_call(
                        symbol,
                        arguments,
                        next + 1,
                        input,
                        input_origin,
                        caller_environment,
                        caller_frames,
                        filters,
                        bindings,
                        continuations,
                        bytecode,
                        limits.call_stack,
                        &mut pending,
                    ) {
                        result = next_result;
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::RecurseChild {
                    filter,
                    condition,
                    depth,
                    structural,
                    environment,
                    frames,
                } => {
                    if let Some(condition) = condition {
                        work.continuations
                            .push(GeneratorContinuation::RecurseCondition {
                                child: value.clone(),
                                filter,
                                condition,
                                depth,
                                structural,
                                environment: Arc::clone(&environment),
                                frames: Arc::clone(&frames),
                            });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: condition,
                            input: value,
                            origin: None,
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                    } else {
                        pending.push(GeneratorTask::RecurseValue {
                            value,
                            filter: Some(filter),
                            condition: None,
                            // A scalar filter chain retains one generator
                            // task, while a filter applied to a container
                            // consumes one structural path component.
                            depth: depth.saturating_add(usize::from(structural)),
                            environment,
                            frames,
                            continuations: work.continuations,
                        });
                    }
                    break;
                }
                GeneratorContinuation::RecurseCondition {
                    child,
                    filter,
                    condition,
                    depth,
                    structural,
                    environment,
                    frames,
                } => {
                    if value.is_truthy() {
                        pending.push(GeneratorTask::RecurseValue {
                            value: child,
                            filter: Some(filter),
                            condition: Some(condition),
                            depth: depth.saturating_add(usize::from(structural)),
                            environment,
                            frames,
                            continuations: work.continuations,
                        });
                    }
                    break;
                }
                GeneratorContinuation::LoopCondition {
                    condition,
                    update,
                    input,
                    until,
                    environment,
                    frames,
                } => {
                    let condition_satisfied = value.is_truthy();
                    if until && condition_satisfied {
                        result = Ok(input);
                    } else if !until && !condition_satisfied {
                        break;
                    } else {
                        let mut update_continuations = work.continuations.clone();
                        update_continuations.push(GeneratorContinuation::LoopUpdate {
                            condition,
                            update,
                            until,
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: update,
                            input: input.clone(),
                            origin: None,
                            environment,
                            frames,
                            continuations: update_continuations,
                        }));
                        if until {
                            break;
                        }
                        result = Ok(input);
                    }
                }
                GeneratorContinuation::LoopUpdate {
                    condition,
                    update,
                    until,
                    environment,
                    frames,
                } => {
                    work.continuations
                        .push(GeneratorContinuation::LoopCondition {
                            condition,
                            update,
                            input: value.clone(),
                            until,
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                        });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: condition,
                        input: value,
                        origin: None,
                        environment,
                        frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::LimitCount {
                    expression,
                    input,
                    environment,
                    frames,
                } => {
                    let count = match limit_count(&value) {
                        Ok(count) => count,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    if count > 0 {
                        let boundary = next_boundary;
                        next_boundary = next_boundary.saturating_add(1);
                        work.continuations.push(GeneratorContinuation::LimitItem {
                            boundary,
                            remaining: Rc::new(Cell::new(count)),
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: expression,
                            input,
                            origin: None,
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                    }
                    break;
                }
                GeneratorContinuation::LimitItem {
                    boundary,
                    remaining,
                } => {
                    let next = remaining.get().saturating_sub(1);
                    remaining.set(next);
                    if next == 0 {
                        pending.retain(|task| !task_owned_by_limit(task, boundary));
                    }
                    result = Ok(value);
                }
                GeneratorContinuation::Raise => {
                    result = Err(VmError::Raised {
                        message: error_message(&value),
                        value,
                    });
                }
            }
        }
    }
    observations
}
