//! Evaluate operations and builtins on the direct execution path.

use super::builtin_effects::{
    append_effect, builtin_signatures, debug_effect, halt_error_text, halt_status, module_metadata,
};
use super::managed_work::{InterpolationWork, TraversalCursor};
use super::ordered::{
    extrema, flatten, fold_values, reverse, sort_by_cached_key, sort_values, unique_values,
};
use super::schedule::{interpolation_capacity, interpolation_pieces, interpolation_remaining};
use super::structured_paths::{
    bind_pattern, compare_paths_for_deletion, emit_stream_root, getpath,
    initialize_pattern_variables, is_pattern_mismatch, is_retryable_alternative_error, jq_path,
    path_component_for_target, path_value, push_children, root_child_is_last, stream_event,
    tostream, update_value,
};
use super::value_ops::{
    absolute_value, access_field, access_index, binary_add, binary_value, computed_key, explode,
    from_entries, fromjson, has, implode, jq_compare, keys, length, math_argument, math_error,
    math_result_value, nth_index, number_usize, number_value, numeric_predicate,
    regex_pattern_argument, runtime_number, selector, slice, slice_bound_value, to_entries,
    type_error, type_name, unary,
};
use super::{
    Environment, INPUT_FILENAME, INPUT_LINE_NUMBER, Outcomes, ambient_environment,
    ambient_platform, ambient_value, catch_value, input_context_value, invalid, is_catchable_error,
    limit_count, path, path_builtin, raised, resource, runtime, scalar, sql, variable_value,
};
use crate::ast::{AssignmentOperator, BinaryOperator};
use crate::bytecode::{BindingPatternOperand, InterpolationOperand, KeyOperand, Operation};
use crate::vm::EffectState;
use crate::{
    Bytecode, InputCursor, Number, Object, Path, PathComponent, Value, VmError, VmLimits,
    VmObservations, collection, format, math, parse_jq_number, stdlib, string_compat,
};
use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) struct Evaluator<'a> {
    pub(super) bytecode: &'a Bytecode,
    pub(super) limits: VmLimits,
    pub(super) observations: Cell<VmObservations>,
    pub(super) cancellation: Option<&'a AtomicBool>,
    pub(super) stop: &'a AtomicBool,
    pub(super) input_cursor: Option<&'a InputCursor>,
    pub(super) effects: Arc<EffectState>,
}

enum WalkFrame {
    Scalar {
        value: Value,
        depth: usize,
    },
    Array {
        values: Arc<[Value]>,
        next: usize,
        children: Vec<Outcomes>,
        depth: usize,
    },
    Object {
        entries: Vec<(Arc<str>, Value)>,
        next: usize,
        children: Vec<Outcomes>,
        depth: usize,
    },
}

impl WalkFrame {
    fn new(value: Value, depth: usize) -> Self {
        match value {
            Value::Array(values) => Self::Array {
                values,
                next: 0,
                children: Vec::new(),
                depth,
            },
            Value::Object(values) => Self::Object {
                entries: values
                    .iter()
                    .map(|(key, value)| (Arc::clone(key), value.clone()))
                    .collect(),
                next: 0,
                children: Vec::new(),
                depth,
            },
            value => Self::Scalar { value, depth },
        }
    }

    fn next_value(&mut self) -> Option<Value> {
        match self {
            Self::Scalar { .. } => None,
            Self::Array { values, next, .. } => {
                let value = values.get(*next).cloned();
                *next = next.saturating_add(1);
                value
            }
            Self::Object { entries, next, .. } => {
                let value = entries.get(*next).map(|(_, value)| value.clone());
                *next = next.saturating_add(1);
                value
            }
        }
    }

    fn push_children(&mut self, children: Outcomes) {
        match self {
            Self::Scalar { .. } => {}
            Self::Array {
                values,
                next,
                children: collected,
                ..
            } => {
                let failed = children.first().is_some_and(Result::is_err);
                collected.push(children);
                if failed {
                    *next = values.len();
                }
            }
            Self::Object {
                entries,
                next,
                children: collected,
                ..
            } => {
                let failed = children.first().is_some_and(Result::is_err);
                collected.push(children);
                if failed {
                    *next = entries.len();
                }
            }
        }
    }

    fn finish(self, fork_limit: usize) -> Vec<Result<Value, VmError>> {
        match self {
            Self::Scalar { value, .. } => vec![Ok(value)],
            Self::Array { children, .. } => rebuild_walk_array(children),
            Self::Object {
                entries, children, ..
            } => rebuild_walk_object(entries, children, fork_limit),
        }
    }

    fn depth(&self) -> usize {
        match self {
            Self::Scalar { depth, .. } | Self::Array { depth, .. } | Self::Object { depth, .. } => {
                *depth
            }
        }
    }
}

fn rebuild_walk_array(children: Vec<Outcomes>) -> Outcomes {
    let mut values = Vec::new();
    for child in children {
        for result in child {
            match result {
                Ok(value) => values.push(value),
                Err(error) => return one_error(error),
            }
        }
    }
    vec![Ok(Value::array(values))]
}

fn rebuild_walk_object(
    entries: Vec<(Arc<str>, Value)>,
    children: Vec<Outcomes>,
    fork_limit: usize,
) -> Outcomes {
    let mut object = Object::new();
    for ((key, _), child) in entries.into_iter().zip(children) {
        match child.first() {
            Some(Ok(value)) => {
                if object.len() >= fork_limit {
                    return one_error(resource("fork-stack"));
                }
                object.insert(key, value.clone());
            }
            Some(Err(error)) => return one_error(error.clone()),
            None => {}
        }
    }
    vec![Ok(Value::object(object))]
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
    pub(super) fn emit_node(
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
                    self.emit_node(index, input, environment, depth + 1, &mut |index| {
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
                self.emit_node(
                    child,
                    input,
                    environment,
                    depth + 1,
                    &mut |result| match result {
                        Err(error) if is_catchable_error(&error) => true,
                        result => emit(result),
                    },
                )
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
                self.emit_conditional(
                    &branches,
                    0,
                    alternative,
                    input,
                    environment,
                    depth + 1,
                    emit,
                )
            }
            Operation::Bind {
                value,
                pattern,
                body,
            } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(value, input, environment, depth + 1, &mut |value| {
                    let Ok(value) = value else {
                        return emit(value);
                    };
                    let mut nested = environment.clone();
                    let mut charge = |pattern_depth| self.enter(pattern_depth);
                    if let Err(error) = bind_pattern(
                        self.bytecode,
                        &pattern,
                        &value,
                        &mut nested,
                        depth.saturating_add(1),
                        &mut charge,
                    ) {
                        return emit(Err(error));
                    }
                    self.emit_node(body, input, &nested, depth + 1, emit)
                })
            }
            Operation::BindAlternatives {
                value,
                patterns,
                body,
            } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(value, input, environment, depth + 1, &mut |value| {
                    let Ok(value) = value else {
                        return emit(value);
                    };
                    self.emit_bind_alternatives(
                        &patterns,
                        &value,
                        input,
                        body,
                        environment,
                        depth,
                        emit,
                    )
                })
            }
            Operation::Reduce {
                generator,
                pattern,
                initial,
                update,
            } => self.emit_fold(
                generator,
                &pattern,
                initial,
                update,
                None,
                false,
                input,
                environment,
                depth,
                emit,
            ),
            Operation::Foreach {
                generator,
                pattern,
                initial,
                update,
                extract,
            } => self.emit_fold(
                generator,
                &pattern,
                initial,
                update,
                extract,
                true,
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
                        Err(error) if is_catchable_error(&error) => catch.is_none_or(|catch| {
                            self.emit_node(
                                catch,
                                &catch_value(&error),
                                environment,
                                depth + 1,
                                emit,
                            )
                        }),
                        Err(error) => emit(Err(error)),
                    },
                )
            }
            Operation::Label { symbol, body } => {
                self.emit_node(body, input, environment, depth + 1, &mut |result| {
                    if matches!(result, Err(VmError::Break { label }) if label == symbol) {
                        false
                    } else {
                        emit(result)
                    }
                });
                true
            }
            Operation::Break(symbol) => emit(Err(VmError::Break { label: symbol })),
            Operation::Call { name, arguments } => {
                let name = match self.string(name) {
                    Ok(name) => Arc::clone(name),
                    Err(error) => return emit(Err(error)),
                };
                match name.as_ref() {
                    "IN" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        self.emit_sql_in(&arguments, input, environment, depth + 1, emit)
                    }
                    "combinations" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        self.emit_combinations(&arguments, input, environment, depth + 1, emit)
                    }
                    "JOIN" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        self.emit_sql_join(&arguments, input, environment, depth + 1, emit)
                    }
                    "empty" => self
                        .enter(depth)
                        .map_or_else(|error| emit(Err(error)), |()| true),
                    "first" | "last" | "nth" | "skip" | "isempty" => self.emit_pull_consumer(
                        name.as_ref(),
                        &arguments,
                        input,
                        environment,
                        depth + 1,
                        emit,
                    ),
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
                    "error" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        let Some(argument) = arguments.first() else {
                            return emit(Err(raised(input.clone())));
                        };
                        let mut completed = false;
                        self.emit_node(*argument, input, environment, depth + 1, &mut |result| {
                            if completed {
                                return false;
                            }
                            completed = true;
                            emit(result.map_or_else(Err, |value| Err(raised(value))));
                            false
                        })
                    }
                    "input" | "inputs" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        let Some(cursor) = self.input_cursor else {
                            return if name.as_ref() == "input" {
                                emit(Err(runtime("break".to_owned())))
                            } else {
                                true
                            };
                        };
                        if name.as_ref() == "input" {
                            return match cursor.next_value() {
                                Ok(Some(value)) => emit(Ok(value)),
                                Ok(None) => emit(Err(runtime("break".to_owned()))),
                                Err(error) => emit(Err(error)),
                            };
                        }
                        loop {
                            match cursor.next_value() {
                                Ok(Some(value)) => {
                                    if !emit(Ok(value)) {
                                        return true;
                                    }
                                }
                                Ok(None) => return true,
                                Err(error) => return emit(Err(error)),
                            }
                        }
                    }
                    "paths" => self.emit_paths(&arguments, input, environment, depth + 1, emit),
                    "fromstream" => {
                        self.emit_fromstream(&arguments, input, environment, depth + 1, emit)
                    }
                    "tostream" => self.emit_tostream(input, depth + 1, emit),
                    "truncate_stream" => {
                        self.emit_truncate_stream(&arguments, input, environment, depth + 1, emit)
                    }
                    "limit" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        let Some(count_node) = arguments.first() else {
                            return emit(Err(invalid("limit count missing")));
                        };
                        let Some(expression) = arguments.get(1) else {
                            return emit(Err(invalid("limit expression missing")));
                        };
                        let mut keep_going = true;
                        self.emit_node(*count_node, input, environment, depth + 1, &mut |count| {
                            let count = match count.and_then(|value| limit_count(&value)) {
                                Ok(count) => count,
                                Err(error) => {
                                    keep_going = emit(Err(error));
                                    return keep_going;
                                }
                            };
                            let mut emitted = 0usize;
                            if count > 0 {
                                self.emit_node(
                                    *expression,
                                    input,
                                    environment,
                                    depth + 1,
                                    &mut |result| {
                                        let success = result.is_ok();
                                        keep_going = emit(result);
                                        if success {
                                            emitted += 1;
                                        }
                                        keep_going && emitted < count
                                    },
                                );
                            }
                            keep_going
                        });
                        true
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

    fn emit_descendant_paths(
        &self,
        input: &Value,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let mut pending = Vec::new();
        push_children(input, &[], &mut pending);
        while let Some((value, components)) = pending.pop() {
            if components.len() > self.limits.path_stack {
                return emit(Err(resource("path-stack")));
            }
            if let Err(error) = self.enter(depth.saturating_add(components.len())) {
                return emit(Err(error));
            }
            if !emit(Ok(path_value(&Path::new(components.clone())))) {
                return true;
            }
            push_children(&value, &components, &mut pending);
        }
        true
    }

    fn emit_paths(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let Some(filter) = arguments.first() else {
            return self.emit_descendant_paths(input, depth, emit);
        };
        let candidates = match self.descendant_path_values(input, depth) {
            Ok(candidates) => candidates,
            Err(error) => return emit(Err(error)),
        };
        for (path, value) in candidates.into_iter().skip(1) {
            let mut continue_paths = true;
            self.emit_node(
                *filter,
                &value,
                environment,
                depth + 1,
                &mut |result| match result {
                    Ok(value) if value.is_truthy() => {
                        continue_paths = emit(Ok(path_value(&path)));
                        continue_paths
                    }
                    Ok(_) => true,
                    Err(error) => {
                        let _ = emit(Err(error));
                        continue_paths = false;
                        false
                    }
                },
            );
            if !continue_paths {
                return false;
            }
        }
        true
    }

    fn emit_tostream(
        &self,
        input: &Value,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        enum Frame {
            Visit(Value, Vec<PathComponent>, bool),
            Close(Vec<PathComponent>),
        }

        let mut pending = vec![Frame::Visit(input.clone(), Vec::new(), true)];
        while let Some(frame) = pending.pop() {
            match frame {
                Frame::Close(path) => {
                    if let Err(error) = self.enter(depth.saturating_add(path.len())) {
                        return emit(Err(error));
                    }
                    if !emit(Ok(Value::array(vec![path_value(&Path::new(path))]))) {
                        return true;
                    }
                }
                Frame::Visit(value, path, root) => {
                    if path.len() > self.limits.path_stack {
                        return emit(Err(resource("path-stack")));
                    }
                    if let Err(error) = self.enter(depth.saturating_add(path.len())) {
                        return emit(Err(error));
                    }
                    match &value {
                        Value::Array(values) if !values.is_empty() => {
                            if !root && (path.len() != 1 || root_child_is_last(input, &path)) {
                                pending.push(Frame::Close(path.clone()));
                            }
                            for (index, child) in values.iter().enumerate().rev() {
                                let mut child_path = path.clone();
                                child_path.push(PathComponent::Index(index));
                                pending.push(Frame::Visit(child.clone(), child_path, false));
                            }
                        }
                        Value::Object(values) if !values.is_empty() => {
                            if !root && (path.len() != 1 || root_child_is_last(input, &path)) {
                                pending.push(Frame::Close(path.clone()));
                            }
                            for (key, child) in values.iter().rev() {
                                let mut child_path = path.clone();
                                child_path.push(PathComponent::Key(Arc::clone(key)));
                                pending.push(Frame::Visit(child.clone(), child_path, false));
                            }
                        }
                        _ => {
                            let record =
                                Value::array(vec![path_value(&Path::new(path.clone())), value]);
                            if !emit(Ok(record)) {
                                return false;
                            }
                            if !root
                                && (path.len() != 1 || root_child_is_last(input, &path))
                                && !emit(Ok(Value::array(vec![path_value(&Path::new(path))])))
                            {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
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
                                .and_then(|remaining| format::text(&value, remaining));
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
        pattern: &BindingPatternOperand,
        initial: u32,
        update: u32,
        extract: Option<u32>,
        emit_each_update: bool,
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if let Err(error) = self.enter_fold_frame(depth) {
            return emit(Err(error));
        }
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
                        let mut charge = |pattern_depth| self.enter(pattern_depth);
                        if let Err(error) = bind_pattern(
                            self.bytecode,
                            pattern,
                            &item,
                            &mut nested,
                            depth.saturating_add(1),
                            &mut charge,
                        ) {
                            fold_failed = true;
                            let _ = emit(Err(error));
                            return false;
                        }
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
                                    return if emit_each_update {
                                        emit(Ok(updated))
                                    } else {
                                        true
                                    };
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
                extract.is_some() || emit_each_update || emit(Ok(accumulator))
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

    fn emit_current_values(
        &self,
        input: &Value,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        match input {
            Value::Array(values) => {
                for value in values.iter().cloned() {
                    if let Err(error) = self.enter(depth) {
                        return emit(Err(error));
                    }
                    if !emit(Ok(value)) {
                        return false;
                    }
                }
                true
            }
            Value::Object(values) => {
                for value in values.values().cloned() {
                    if let Err(error) = self.enter(depth) {
                        return emit(Err(error));
                    }
                    if !emit(Ok(value)) {
                        return false;
                    }
                }
                true
            }
            value => emit(Err(type_error("index", value))),
        }
    }

    fn emit_pull_consumer(
        &self,
        name: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if let Err(error) = self.enter(depth) {
            return emit(Err(error));
        }
        match name {
            "first" if arguments.is_empty() => Self::emit_direct_index(0, input, emit),
            "first" => self.emit_first(arguments, input, environment, depth, emit),
            "last" if arguments.is_empty() => Self::emit_direct_index(-1, input, emit),
            "last" => self.emit_last(arguments, input, environment, depth, emit),
            "nth" | "skip" => {
                self.emit_nth_or_skip(name, arguments, input, environment, depth, emit)
            }
            "isempty" => self.emit_isempty(arguments, input, environment, depth, emit),
            _ => emit(Err(invalid("unknown pull consumer"))),
        }
    }

    fn emit_direct_index(
        index: i64,
        input: &Value,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let index = Value::Number(
            Number::parse(&index.to_string()).expect("literal consumer index is valid"),
        );
        emit(access_index(input, &index))
    }

    fn emit_pull_generator(
        &self,
        name: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let generator_argument = match name {
            "nth" | "skip" => arguments.len() == 2,
            _ => arguments.len() == 1,
        };
        if generator_argument && let Some(argument) = arguments.last().copied() {
            return self.emit_node(argument, input, environment, depth + 1, emit);
        }
        self.emit_current_values(input, depth + 1, emit)
    }

    fn emit_first(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if arguments.len() > 1 {
            return emit(Err(invalid("first arity")));
        }
        let mut accepted = true;
        self.emit_pull_generator(
            "first",
            arguments,
            input,
            environment,
            depth,
            &mut |result| {
                accepted = emit(result);
                false
            },
        );
        accepted
    }

    fn emit_last(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if arguments.len() > 1 {
            return emit(Err(invalid("last arity")));
        }
        let mut latest = None;
        let mut failure = None;
        self.emit_pull_generator(
            "last",
            arguments,
            input,
            environment,
            depth,
            &mut |result| {
                match result {
                    Ok(value) => latest = Some(value),
                    Err(error) => {
                        failure = Some(error);
                        return false;
                    }
                }
                true
            },
        );
        if let Some(error) = failure {
            emit(Err(error))
        } else if let Some(value) = latest {
            emit(Ok(value))
        } else {
            true
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "nth and skip retain the count expression and generator evaluation context"
    )]
    fn emit_nth_or_skip(
        &self,
        name: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let Some(count_node) = arguments.first().copied() else {
            return emit(Err(invalid(if name == "nth" {
                "nth count missing"
            } else {
                "skip count missing"
            })));
        };
        if name == "nth" && arguments.len() == 1 {
            let mut accepted = true;
            self.emit_node(count_node, input, environment, depth, &mut |count| {
                accepted = emit(count.and_then(|count| nth_index(input, &count)));
                accepted
            });
            return accepted;
        }
        if name == "nth" && arguments.len() > 2 {
            return emit(Err(invalid("nth arity")));
        }
        if name == "skip" && arguments.len() != 2 {
            return emit(Err(invalid("skip arity")));
        }
        let mut accepted = true;
        self.emit_node(count_node, input, environment, depth, &mut |count| {
            let count = match count {
                Ok(count) => match limit_count(&count) {
                    Ok(count) => count,
                    Err(error) => {
                        accepted = emit(Err(error));
                        return accepted;
                    }
                },
                Err(error) => {
                    accepted = emit(Err(error));
                    return accepted;
                }
            };
            let mut remaining = count;
            self.emit_pull_generator(name, arguments, input, environment, depth, &mut |result| {
                match result {
                    Ok(value) if name == "nth" && remaining == 0 => {
                        accepted = emit(Ok(value));
                        false
                    }
                    Ok(_) if name == "nth" => {
                        remaining = remaining.saturating_sub(1);
                        true
                    }
                    Ok(value) if remaining == 0 => {
                        accepted = emit(Ok(value));
                        accepted
                    }
                    Ok(_) => {
                        remaining = remaining.saturating_sub(1);
                        true
                    }
                    Err(error) => {
                        accepted = emit(Err(error));
                        false
                    }
                }
            });
            accepted
        });
        accepted
    }

    fn emit_isempty(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if arguments.len() != 1 {
            return emit(Err(invalid("isempty arity")));
        }
        let mut accepted = true;
        let mut found = false;
        self.emit_pull_generator(
            "isempty",
            arguments,
            input,
            environment,
            depth,
            &mut |result| {
                found = true;
                accepted = match result {
                    Ok(_) => emit(Ok(Value::Bool(false))),
                    Err(error) => emit(Err(error)),
                };
                false
            },
        );
        if found {
            accepted
        } else {
            emit(Ok(Value::Bool(true)))
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "alternative binding keeps retry and resource state explicit"
    )]
    fn emit_bind_alternatives(
        &self,
        patterns: &[BindingPatternOperand],
        value: &Value,
        input: &Value,
        body: u32,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let mut last_error = None;
        for pattern in patterns {
            let mut nested = environment.clone();
            if let Err(error) = initialize_pattern_variables(self.bytecode, patterns, &mut nested) {
                last_error = Some(error);
                continue;
            }
            let mut charge = |pattern_depth| self.enter(pattern_depth);
            if let Err(error) = bind_pattern(
                self.bytecode,
                pattern,
                value,
                &mut nested,
                depth.saturating_add(1),
                &mut charge,
            ) {
                if !is_pattern_mismatch(&error) {
                    return emit(Err(error));
                }
                last_error = Some(error);
            } else {
                let mut body_failed = None;
                let keep_going =
                    self.emit_node(
                        body,
                        input,
                        &nested,
                        depth + 1,
                        &mut |result| match result {
                            Ok(value) => emit(Ok(value)),
                            Err(error) => {
                                body_failed = Some(error);
                                true
                            }
                        },
                    );
                if let Some(error) = body_failed {
                    if !is_retryable_alternative_error(&error) {
                        return emit(Err(error));
                    }
                    last_error = Some(error);
                    continue;
                }
                return keep_going;
            }
        }
        emit(Err(last_error.unwrap_or_else(|| {
            runtime("destructuring alternative has no patterns".to_owned())
        })))
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
                Ok(name) => vec![variable_value(environment, name)],
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
                            for index in self.node(*index, input, environment, depth + 1) {
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
                    let starts = self.bound(*start, input, environment, depth + 1);
                    let ends = self.bound(*end, input, environment, depth + 1);
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
            Operation::Bind {
                value,
                pattern,
                body,
            } => {
                let mut output = Vec::new();
                for value in self.node(*value, input, environment, depth + 1) {
                    match value {
                        Ok(value) => {
                            let mut nested = environment.clone();
                            let mut charge = |pattern_depth| self.enter(pattern_depth);
                            match bind_pattern(
                                self.bytecode,
                                pattern,
                                &value,
                                &mut nested,
                                depth.saturating_add(1),
                                &mut charge,
                            ) {
                                Ok(()) => {
                                    output.extend(self.node(*body, input, &nested, depth + 1));
                                }
                                Err(error) => output.push(Err(error)),
                            }
                        }
                        Err(error) => output.push(Err(error)),
                    }
                }
                output
            }
            Operation::BindAlternatives {
                value,
                patterns,
                body,
            } => {
                let mut output = Vec::new();
                for value in self.node(*value, input, environment, depth + 1) {
                    match value {
                        Ok(value) => {
                            self.emit_bind_alternatives(
                                patterns,
                                &value,
                                input,
                                *body,
                                environment,
                                depth,
                                &mut |result| {
                                    output.push(result);
                                    true
                                },
                            );
                        }
                        Err(error) => output.push(Err(error)),
                    }
                }
                output
            }
            Operation::Reduce {
                generator,
                pattern,
                initial,
                update,
            } => {
                let mut output = Vec::new();
                self.emit_fold(
                    *generator,
                    pattern,
                    *initial,
                    *update,
                    None,
                    false,
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
                pattern,
                initial,
                update,
                extract,
            } => {
                let mut output = Vec::new();
                self.emit_fold(
                    *generator,
                    pattern,
                    *initial,
                    *update,
                    *extract,
                    true,
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
                                    &catch_value(&error),
                                    environment,
                                    depth + 1,
                                ));
                            }
                        }
                    }
                }
                output
            }
            Operation::Label { symbol, body } => {
                let mut output = Vec::new();
                for result in self.node(*body, input, environment, depth + 1) {
                    if matches!(result, Err(VmError::Break { label }) if label == *symbol) {
                        break;
                    }
                    output.push(result);
                }
                output
            }
            Operation::Break(symbol) => one_error(VmError::Break { label: *symbol }),
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

    #[allow(
        clippy::too_many_arguments,
        reason = "the streaming branch callback carries the evaluator context explicitly"
    )]
    fn emit_conditional(
        &self,
        branches: &[(u32, u32)],
        index: usize,
        alternative: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let Some((condition, body)) = branches.get(index).copied() else {
            return self.emit_node(alternative, input, environment, depth, emit);
        };
        self.emit_node(
            condition,
            input,
            environment,
            depth,
            &mut |result| match result {
                Ok(value) if value.is_truthy() => {
                    self.emit_node(body, input, environment, depth, emit)
                }
                Ok(_) => self.emit_conditional(
                    branches,
                    index.saturating_add(1),
                    alternative,
                    input,
                    environment,
                    depth,
                    emit,
                ),
                Err(error) => emit(Err(error)),
            },
        )
    }

    fn bound(
        &self,
        node: Option<u32>,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Vec<Result<Option<f64>, VmError>> {
        match node {
            None => vec![Ok(None)],
            Some(node) => self
                .node(node, input, environment, depth)
                .into_iter()
                .map(|value| value.and_then(|value| slice_bound_value(&value)))
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

    fn walk(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(callback) = arguments.first() else {
            return one_error(invalid("walk argument missing"));
        };
        let root_depth = depth.saturating_add(1);
        if root_depth > self.limits.path_stack {
            return one_error(resource("path-stack"));
        }
        let mut observations = self.observations.get();
        observations.path_stack_high_water = observations.path_stack_high_water.max(root_depth);
        self.observations.set(observations);
        let mut frames = vec![WalkFrame::new(input.clone(), root_depth)];
        let mut completed = None;

        loop {
            if let Some(outcomes) = completed.take() {
                if let Some(parent) = frames.last_mut() {
                    parent.push_children(outcomes);
                    continue;
                }
                return outcomes;
            }

            let Some(frame) = frames.last_mut() else {
                return Vec::new();
            };
            let Some(value) = frame.next_value() else {
                let frame = frames
                    .pop()
                    .expect("walk frame exists after last frame lookup");
                let callback_depth = frame.depth();
                let mut outcomes = Vec::new();
                for rebuilt in frame.finish(self.limits.fork_stack) {
                    match rebuilt {
                        Ok(value) => outcomes.extend(self.node(
                            *callback,
                            &value,
                            environment,
                            callback_depth,
                        )),
                        Err(error) => outcomes.push(Err(error)),
                    }
                    if outcomes.len() >= self.limits.fork_stack {
                        outcomes = one_error(resource("fork-stack"));
                        break;
                    }
                }
                completed = Some(outcomes);
                continue;
            };

            let child_depth = frame.depth().saturating_add(1);
            if let Err(error) = self.enter(child_depth) {
                completed = Some(one_error(error));
                continue;
            }
            if child_depth > self.limits.path_stack {
                completed = Some(one_error(resource("path-stack")));
                continue;
            }
            let mut observations = self.observations.get();
            observations.path_stack_high_water =
                observations.path_stack_high_water.max(child_depth);
            self.observations.set(observations);
            frames.push(WalkFrame::new(value, child_depth));
        }
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
            "first" | "last" | "nth" | "skip" | "isempty" => {
                let mut output = Vec::new();
                self.emit_pull_consumer(
                    name,
                    arguments,
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
            "empty" => Vec::new(),
            "modulemeta" => vec![module_metadata(
                self.bytecode,
                input,
                self.cancellation,
                self.stop,
            )],
            "type" => vec![Ok(Value::string(type_name(input)))],
            "length" => vec![length(input)],
            "abs" => vec![absolute_value(input)],
            "utf8bytelength" => match input {
                Value::String(value) => vec![Ok(number_usize(value.len()))],
                value => one_error(type_error("utf8bytelength", value)),
            },
            "keys" | "keys_unsorted" => vec![keys(input, name == "keys")],
            "builtins" => vec![Ok(builtin_signatures())],
            "not" => vec![Ok(Value::Bool(!input.is_truthy()))],
            "has" => self.argument_values(arguments, input, environment, depth, 0, |key| {
                has(input, key)
            }),
            "in" => self.argument_values(arguments, input, environment, depth, 0, |container| {
                has(container, input)
            }),
            "IN" => self.sql_in(arguments, input, environment, depth),
            "INDEX" => self.sql_index(arguments, input, environment, depth),
            "JOIN" => self.sql_join(arguments, input, environment, depth),
            "contains" => self.argument_values(arguments, input, environment, depth, 0, |needle| {
                collection::contains(input, needle, self.limits, &|| self.enter(depth))
            }),
            "inside" => {
                self.argument_values(arguments, input, environment, depth, 0, |container| {
                    collection::inside(input, container, self.limits, &|| self.enter(depth))
                })
            }
            "combinations" => self.combinations(arguments, input, environment, depth),
            "transpose" => self.transpose(input, depth),
            "bsearch" => self.bsearch(arguments, input, environment, depth),
            "indices" => self.argument_values(arguments, input, environment, depth, 0, |needle| {
                collection::indices(input, needle, self.limits, &|| self.enter(depth))
            }),
            "index" => self.argument_values(arguments, input, environment, depth, 0, |needle| {
                collection::index(input, needle, false, self.limits, &|| self.enter(depth))
            }),
            "rindex" => self.argument_values(arguments, input, environment, depth, 0, |needle| {
                collection::index(input, needle, true, self.limits, &|| self.enter(depth))
            }),
            "arrays" => selector(input, matches!(input, Value::Array(_))),
            "booleans" => selector(input, matches!(input, Value::Bool(_))),
            "finites" => selector(
                input,
                matches!(input, Value::Number(number) if !number.as_f64().is_infinite()),
            ),
            "iterables" => selector(input, matches!(input, Value::Array(_) | Value::Object(_))),
            "nulls" => selector(input, matches!(input, Value::Null)),
            "normals" => selector(
                input,
                matches!(input, Value::Number(number) if number.as_f64().is_normal()),
            ),
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
            "to_entries" => vec![to_entries(input)],
            "from_entries" => match input {
                Value::Array(entries) => vec![from_entries(entries, "from_entries")],
                value => one_error(type_error("from_entries", value)),
            },
            "with_entries" => self.with_entries(arguments, input, environment, depth),
            "tonumber" => match input {
                Value::Number(_) => vec![Ok(input.clone())],
                Value::String(value) => parse_jq_number(value)
                    .map(Value::Number)
                    .map(Ok)
                    .map_or_else(
                        |error| one_error(runtime(error.to_string())),
                        |value| vec![value],
                    ),
                value => one_error(type_error("tonumber", value)),
            },
            "tostring" => vec![format::text(input, self.limits.output_bytes).map(Value::String)],
            "@urid" => vec![string_compat::urid(input, self.limits.output_bytes)],
            name if name.starts_with('@') => {
                vec![format::apply(name, input, self.limits.output_bytes)]
            }
            "tojson" => {
                vec![format::bounded_json(input, self.limits.output_bytes).map(Value::string)]
            }
            "fromjson" => {
                let mut checkpoint = || {
                    if self.cancelled() {
                        Err(VmError::Interrupted)
                    } else {
                        Ok(())
                    }
                };
                vec![fromjson(input, self.limits, &mut checkpoint)]
            }
            "range" => self.range(arguments, input, environment, depth),
            "add" => self.add(arguments, input, environment, depth),
            "min" => extrema(input, false),
            "max" => extrema(input, true),
            "sort" => sort_values(input),
            "sort_by" => self.sort_by(arguments, input, environment, depth, false),
            "group_by" => self.group_by(arguments, input, environment, depth),
            "min_by" => self.keyed_extrema(arguments, input, environment, depth, false),
            "max_by" => self.keyed_extrema(arguments, input, environment, depth, true),
            "unique" => unique_values(input),
            "unique_by" => self.sort_by(arguments, input, environment, depth, true),
            "reverse" => reverse(input),
            "flatten" => self.flatten(arguments, input, environment, depth),
            "walk" => self.walk(arguments, input, environment, depth),
            "limit" => self.limit(arguments, input, environment, depth),
            "any" => self.any_all(arguments, input, environment, depth, true),
            "all" => self.any_all(arguments, input, environment, depth, false),
            "trim" => vec![string_compat::trim(input)],
            "ltrim" => vec![string_compat::ltrim(input)],
            "rtrim" => vec![string_compat::rtrim(input)],
            "ltrimstr" => self.argument_values(arguments, input, environment, depth, 0, |prefix| {
                string_compat::ltrimstr(input, prefix)
            }),
            "rtrimstr" => self.argument_values(arguments, input, environment, depth, 0, |suffix| {
                string_compat::rtrimstr(input, suffix)
            }),
            "startswith" => {
                self.argument_values(arguments, input, environment, depth, 0, |prefix| {
                    string_compat::startswith(input, prefix)
                })
            }
            "endswith" => self.argument_values(arguments, input, environment, depth, 0, |suffix| {
                string_compat::endswith(input, suffix)
            }),
            "trimstr" => self.argument_values(arguments, input, environment, depth, 0, |affix| {
                string_compat::trimstr(input, affix)
            }),
            "join" => self.argument_values(arguments, input, environment, depth, 0, |separator| {
                string_compat::join(input, separator, self.limits.output_bytes)
            }),
            "toboolean" => vec![string_compat::toboolean(input)],
            "ascii_upcase" => vec![string_compat::ascii_upcase(input)],
            "ascii_downcase" => vec![string_compat::ascii_downcase(input)],
            "explode" => vec![explode(input)],
            "implode" => vec![implode(input)],
            "nan" => vec![Ok(runtime_number(f64::NAN))],
            "infinite" => vec![Ok(runtime_number(f64::INFINITY))],
            // tq's hybrid Number retains decimal literal provenance and
            // therefore provides the decimal-number capability advertised by
            // jq's have_decnum/0 and have_literal_numbers/0 predicates.
            "have_decnum" | "have_literal_numbers" => vec![Ok(Value::Bool(true))],
            "isnan" | "isinfinite" | "isfinite" | "isnormal" => {
                vec![numeric_predicate(name, input)]
            }
            "acos" | "acosh" | "asin" | "asinh" | "atan" | "atan2" | "atanh" | "cbrt" | "ceil"
            | "copysign" | "cos" | "cosh" | "drem" | "erf" | "erfc" | "exp" | "exp10" | "exp2"
            | "expm1" | "fabs" | "fdim" | "fma" | "fmax" | "fmin" | "fmod" | "floor" | "frexp"
            | "gamma" | "hypot" | "j0" | "j1" | "jn" | "ldexp" | "lgamma" | "log" | "log10"
            | "log1p" | "log2" | "logb" | "modf" | "nearbyint" | "nextafter" | "nexttoward"
            | "pow" | "remainder" | "rint" | "round" | "scalb" | "scalbln" | "significand"
            | "sin" | "sinh" | "sqrt" | "tan" | "tanh" | "tgamma" | "trunc" | "y0" | "y1"
            | "yn" => self.math_call(name, arguments, input, environment, depth),
            "paths" if arguments.is_empty() => match self.descendant_paths(input, depth + 1) {
                Ok(paths) => paths
                    .into_iter()
                    .map(|path| Ok(path_value(&path)))
                    .collect(),
                Err(error) => one_error(error),
            },
            "paths" => self.paths_filter(arguments, input, environment, depth),
            "path" => self.path_outcomes(arguments, input, environment, depth),
            "pick" => self.pick(arguments, input, environment, depth),
            "del" => self.delete(arguments, input, environment, depth),
            "delpaths" => self.delete_paths(arguments, input, environment, depth),
            "getpath" => {
                let path_limit = self.limits.path_stack;
                self.argument_values(arguments, input, environment, depth, 0, |path| {
                    let path = jq_path(path)?;
                    if path.len() > path_limit {
                        return Err(resource("path-stack"));
                    }
                    getpath(input, &path)
                })
            }
            "setpath" => self.setpath(arguments, input, environment, depth),
            "fromstream" => self.fromstream(arguments, input, environment, depth),
            "tostream" => match tostream(input, self.limits.path_stack) {
                Ok(records) => records.into_iter().map(Ok).collect(),
                Err(error) => one_error(error),
            },
            "truncate_stream" => self.truncate_stream(arguments, input, environment, depth),
            "stderr" => match format::text(input, self.limits.output_bytes) {
                Ok(text) => append_effect(&self.effects, text.as_bytes(), self.limits.output_bytes)
                    .map_or_else(one_error, |()| vec![Ok(input.clone())]),
                Err(error) => one_error(error),
            },
            "debug" => {
                let values = arguments.first().map_or_else(
                    || vec![Ok(input.clone())],
                    |argument| self.node(*argument, input, environment, depth + 1),
                );
                let mut output = Vec::new();
                for value in values {
                    let value = match value {
                        Ok(value) => value,
                        Err(error) => return one_error(error),
                    };
                    if let Err(error) =
                        debug_effect(&value, self.limits.output_bytes).and_then(|bytes| {
                            append_effect(&self.effects, &bytes, self.limits.output_bytes)
                        })
                    {
                        return one_error(error);
                    }
                }
                output.push(Ok(input.clone()));
                output
            }
            "halt" => vec![Err(VmError::Halt {
                status: 0,
                stderr: Arc::from([]),
            })],
            "halt_error" => {
                let stderr = match halt_error_text(input, self.limits.output_bytes) {
                    Ok(stderr) => stderr,
                    Err(error) => return one_error(error),
                };
                let status_values = arguments.first().map_or_else(
                    || {
                        vec![Ok(Value::Number(
                            Number::parse("5").expect("halt_error default status"),
                        ))]
                    },
                    |argument| self.node(*argument, input, environment, depth + 1),
                );
                status_values
                    .into_iter()
                    .map(|value| {
                        value.and_then(|value| {
                            Err(VmError::Halt {
                                status: halt_status(&value)?,
                                stderr: Arc::from(stderr.as_bytes()),
                            })
                        })
                    })
                    .collect()
            }
            "error" => {
                let Some(argument) = arguments.first() else {
                    return vec![Err(raised(input.clone()))];
                };
                let Some(result) = self
                    .node(*argument, input, environment, depth)
                    .into_iter()
                    .next()
                else {
                    return Vec::new();
                };
                vec![result.map_or_else(Err, |value| Err(raised(value)))]
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
            "env" => vec![ambient_environment(environment, "env")],
            "input_filename" => vec![ambient_value(
                environment,
                INPUT_FILENAME,
                "input_filename",
                self.input_cursor
                    .and_then(InputCursor::current_context)
                    .map(|context| Value::string(context.identity)),
            )],
            "input_line_number" => vec![
                self.input_cursor
                    .and_then(InputCursor::current_context)
                    .map_or_else(
                        || input_context_value(environment, INPUT_LINE_NUMBER, "input_line_number"),
                        |context| {
                            Ok(Value::Number(
                                Number::parse(&context.line_number.to_string())
                                    .expect("a source line number is an admitted exact integer"),
                            ))
                        },
                    ),
            ],
            "input" => vec![
                self.input_cursor
                    .map_or(Ok(None), InputCursor::next_value)
                    .and_then(|value| {
                        value.ok_or_else(|| VmError::Runtime {
                            message: "break".into(),
                        })
                    }),
            ],
            "inputs" => {
                let Some(cursor) = self.input_cursor else {
                    return if name == "input" {
                        one_error(runtime("break".to_owned()))
                    } else {
                        Vec::new()
                    };
                };
                if name == "input" {
                    return match cursor.next_value() {
                        Ok(Some(value)) => vec![Ok(value)],
                        Ok(None) => one_error(runtime("break".to_owned())),
                        Err(error) => one_error(error),
                    };
                }
                let mut values = Vec::new();
                loop {
                    match cursor.next_value() {
                        Ok(Some(value)) => values.push(Ok(value)),
                        Ok(None) => break,
                        Err(error) => return one_error(error),
                    }
                }
                values
            }
            _ => one_error(VmError::Unsupported {
                operation: format!("builtin {name}").into(),
            }),
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "regex dispatch keeps bounded engine and jq overload handling together"
    )]
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
        let mut output = Vec::new();
        for pattern in &pattern_values {
            let array_allowed = matches!(name, "test" | "match" | "capture")
                && arguments.get(flags_index).is_none();
            let (pattern, array_flags) = match pattern
                .as_ref()
                .map_err(Clone::clone)
                .and_then(|value| regex_pattern_argument(name, value, array_allowed))
            {
                Ok(parsed) => parsed,
                Err(error) => {
                    output.push(Err(error));
                    continue;
                }
            };
            let flag_values = if let Some(flags) = array_flags {
                vec![Ok(Value::String(flags))]
            } else if let Some(flags) = arguments.get(flags_index) {
                self.node(
                    *flags,
                    &Value::String(Arc::clone(input)),
                    environment,
                    depth,
                )
            } else {
                vec![Ok(Value::string(""))]
            };
            for flags in &flag_values {
                let flags = match flags {
                    Ok(Value::String(flags)) => Arc::clone(flags),
                    Ok(Value::Null) => Arc::from(""),
                    Ok(value) => {
                        output.push(Err(type_error(name, value)));
                        continue;
                    }
                    Err(error) => {
                        output.push(Err(error.clone()));
                        continue;
                    }
                };
                let checkpoint = || self.enter(depth);
                let result = match name {
                    "test" => stdlib::regex_test(input, &pattern, &flags, self.limits, &checkpoint)
                        .map(|value| vec![value]),
                    "match" => {
                        stdlib::regex_matches(input, &pattern, &flags, self.limits, &checkpoint)
                    }
                    "capture" => {
                        stdlib::regex_capture(input, &pattern, &flags, self.limits, &checkpoint)
                    }
                    "scan" => stdlib::regex_scan(input, &pattern, &flags, self.limits, &checkpoint),
                    "split" => stdlib::regex_split(
                        input,
                        &pattern,
                        &flags,
                        false,
                        arguments.get(flags_index).is_none(),
                        self.limits,
                        &checkpoint,
                    ),
                    "splits" => stdlib::regex_split(
                        input,
                        &pattern,
                        &flags,
                        true,
                        false,
                        self.limits,
                        &checkpoint,
                    ),
                    "sub" | "gsub" => {
                        let Some(replacement_node) = arguments.get(1) else {
                            return one_error(invalid("regex replacement argument missing"));
                        };
                        stdlib::regex_substitute(
                            input,
                            &pattern,
                            &flags,
                            name == "gsub",
                            self.limits,
                            &checkpoint,
                            |context| {
                                self.regex_replacement_values(
                                    name,
                                    *replacement_node,
                                    context,
                                    environment,
                                    depth,
                                )
                            },
                        )
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

    fn regex_replacement_values(
        &self,
        name: &str,
        node: u32,
        context: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Result<Vec<Arc<str>>, VmError> {
        let mut values = Vec::new();
        for value in self.node(node, context, environment, depth) {
            if values.len() >= self.limits.regex_replacement_limit {
                return Err(resource("regex-replacement-count"));
            }
            values.push(match value? {
                Value::String(value) => value,
                Value::Null => Arc::from(""),
                value => return Err(type_error(name, &value)),
            });
        }
        Ok(values)
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

    fn sql_in(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let mut output = Vec::new();
        self.emit_sql_in(arguments, input, environment, depth, &mut |result| {
            output.push(result);
            true
        });
        output
    }

    fn emit_sql_in(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let (Some(source), maybe_needle) = (arguments.first(), arguments.get(1)) else {
            return emit(Err(invalid("IN argument missing")));
        };
        if let Some(needle) = maybe_needle {
            let mut matched = false;
            let mut search_error = None;
            self.emit_node(*needle, input, environment, depth, &mut |needle| {
                if matched || search_error.is_some() {
                    return false;
                }
                let needle = match needle {
                    Ok(needle) => needle,
                    Err(error) => {
                        search_error = Some(error);
                        return false;
                    }
                };
                self.emit_node(
                    *source,
                    input,
                    environment,
                    depth,
                    &mut |source| match source {
                        Ok(source) => {
                            let mut charge = || self.enter(depth);
                            match sql::equal_bounded(&source, &needle, self.limits, &mut charge) {
                                Ok(true) => {
                                    matched = true;
                                    false
                                }
                                Ok(false) => true,
                                Err(error) => {
                                    search_error = Some(error);
                                    false
                                }
                            }
                        }
                        Err(error) => {
                            search_error = Some(error);
                            false
                        }
                    },
                );
                !matched && search_error.is_none()
            });
            if let Some(error) = search_error {
                return emit(Err(error));
            }
            return emit(Ok(Value::Bool(matched)));
        }

        let mut matched = false;
        let mut source_error = None;
        self.emit_node(
            *source,
            input,
            environment,
            depth,
            &mut |source| match source {
                Ok(source) => {
                    let mut charge = || self.enter(depth);
                    match sql::equal_bounded(input, &source, self.limits, &mut charge) {
                        Ok(true) => {
                            matched = true;
                            false
                        }
                        Ok(false) => true,
                        Err(error) => {
                            source_error = Some(error);
                            false
                        }
                    }
                }
                Err(error) => {
                    source_error = Some(error);
                    false
                }
            },
        );
        if let Some(error) = source_error {
            return emit(Err(error));
        }
        emit(Ok(Value::Bool(matched)))
    }

    fn sql_index(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(key_node) = arguments.last().copied() else {
            return one_error(invalid("INDEX argument missing"));
        };
        let rows = if arguments.len() == 1 {
            match self.sql_iterable_rows(input, depth) {
                Ok(rows) => rows.into_iter().map(Ok).collect(),
                Err(error) => return one_error(error),
            }
        } else {
            self.node(arguments[0], input, environment, depth)
        };
        let mut index = Object::new();
        for row in rows {
            let row = match row {
                Ok(row) => row,
                Err(error) => return one_error(error),
            };
            if let Err(error) = self.enter(depth) {
                return one_error(error);
            }
            for key in self.node(key_node, &row, environment, depth) {
                let key = match key {
                    Ok(key) => key,
                    Err(error) => return one_error(error),
                };
                let key = match format::text(&key, self.limits.output_bytes) {
                    Ok(key) => key,
                    Err(error) => return one_error(error),
                };
                index.insert(key, row.clone());
            }
        }
        vec![Ok(Value::object(index))]
    }

    fn sql_join(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let mut output = Vec::new();
        self.emit_sql_join(arguments, input, environment, depth, &mut |result| {
            output.push(result);
            true
        });
        output
    }

    fn emit_sql_join(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let Some(index_node) = arguments.first().copied() else {
            return emit(Err(invalid("JOIN index argument missing")));
        };
        let Some(key_node) = arguments
            .get(if arguments.len() == 2 { 1 } else { 2 })
            .copied()
        else {
            return emit(Err(invalid("JOIN key argument missing")));
        };
        self.emit_node(index_node, input, environment, depth, &mut |index| {
            let index = match index {
                Ok(index) => index,
                Err(error) => return emit(Err(error)),
            };
            if arguments.len() == 2 {
                let rows = match self.sql_iterable_rows(input, depth) {
                    Ok(rows) => rows,
                    Err(error) => return emit(Err(error)),
                };
                let mut grouped = Vec::new();
                for row in rows {
                    let mut key_error = None;
                    let mut keys = Vec::new();
                    if let Err(error) = self.enter(depth) {
                        return emit(Err(error));
                    }
                    self.emit_node(key_node, &row, environment, depth, &mut |key| match key {
                        Ok(key) => {
                            keys.push(key);
                            true
                        }
                        Err(error) => {
                            key_error = Some(error);
                            false
                        }
                    });
                    if let Some(error) = key_error {
                        return emit(Err(error));
                    }
                    for key in keys {
                        let match_value = match access_index(&index, &key) {
                            Ok(value) => value,
                            Err(error) => return emit(Err(error)),
                        };
                        grouped.push(Value::array(vec![row.clone(), match_value]));
                    }
                }
                return emit(Ok(Value::array(grouped)));
            }

            let Some(stream_node) = arguments.get(1).copied() else {
                return emit(Err(invalid("JOIN stream argument missing")));
            };
            self.emit_node(stream_node, input, environment, depth, &mut |row| {
                let row = match row {
                    Ok(row) => row,
                    Err(error) => return emit(Err(error)),
                };
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(key_node, &row, environment, depth, &mut |key| {
                    let key = match key {
                        Ok(key) => key,
                        Err(error) => return emit(Err(error)),
                    };
                    let match_value = match access_index(&index, &key) {
                        Ok(value) => value,
                        Err(error) => return emit(Err(error)),
                    };
                    let pair = Value::array(vec![row.clone(), match_value]);
                    if let Some(join_node) = arguments.get(3).copied() {
                        self.emit_node(join_node, &pair, environment, depth, emit)
                    } else {
                        emit(Ok(pair))
                    }
                })
            })
        })
    }

    fn sql_iterable_rows(&self, input: &Value, depth: usize) -> Result<Vec<Value>, VmError> {
        let values = match input {
            Value::Array(values) => values.iter().cloned().collect::<Vec<_>>(),
            Value::Object(values) => values.values().cloned().collect::<Vec<_>>(),
            value => return Err(runtime(format!("Cannot iterate over {value}"))),
        };
        for _ in &values {
            self.enter(depth)?;
        }
        Ok(values)
    }

    fn combinations(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let mut output = Vec::new();
        self.emit_combinations(arguments, input, environment, depth, &mut |result| {
            output.push(result);
            true
        });
        output
    }

    fn emit_combinations(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if let Some(argument) = arguments.first() {
            let Value::Array(base_values) = input else {
                return emit(Err(type_error("combinations", input)));
            };
            let mut dimensions = Vec::new();
            let mut argument_error = None;
            self.emit_node(*argument, input, environment, depth, &mut |value| {
                if let Err(error) = self.enter(depth) {
                    argument_error = Some(error);
                    return false;
                }
                let value = match value {
                    Ok(value) => value,
                    Err(error) => {
                        argument_error = Some(error);
                        return false;
                    }
                };
                let count = match value {
                    Value::Number(number) if number.as_f64().is_finite() => {
                        let count = number.as_f64().ceil();
                        if count <= 0.0 {
                            0
                        } else {
                            match count.to_string().parse::<usize>() {
                                Ok(count) if count <= self.limits.fork_stack => count,
                                _ => {
                                    argument_error = Some(resource("fork-stack"));
                                    return false;
                                }
                            }
                        }
                    }
                    Value::Number(_) => {
                        argument_error = Some(resource("fork-stack"));
                        return false;
                    }
                    value => {
                        argument_error = Some(type_error("combinations", &value));
                        return false;
                    }
                };
                if dimensions.len().saturating_add(count) > self.limits.fork_stack {
                    argument_error = Some(resource("fork-stack"));
                    return false;
                }
                for _ in 0..count {
                    if let Err(error) = self.enter(depth) {
                        argument_error = Some(error);
                        return false;
                    }
                    dimensions.push(base_values.as_ref());
                }
                true
            });
            if let Some(error) = argument_error {
                return emit(Err(error));
            }
            let mut current = Vec::new();
            if current.try_reserve(dimensions.len()).is_err() {
                return emit(Err(resource("fork-stack")));
            }
            self.emit_combination_product(&dimensions, &mut current, depth, emit)
        } else {
            let Value::Array(values) = input else {
                return emit(Err(type_error("combinations", input)));
            };
            if values.len() > self.limits.fork_stack {
                return emit(Err(resource("fork-stack")));
            }
            let mut dimensions = Vec::new();
            if dimensions.try_reserve(values.len()).is_err() {
                return emit(Err(resource("fork-stack")));
            }
            for value in values.iter() {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                let Value::Array(dimension) = value else {
                    return emit(Err(type_error("combinations", value)));
                };
                dimensions.push(dimension.as_ref());
            }
            let mut current = Vec::new();
            if current.try_reserve(dimensions.len()).is_err() {
                return emit(Err(resource("fork-stack")));
            }
            self.emit_combination_product(&dimensions, &mut current, depth, emit)
        }
    }

    fn emit_combination_product(
        &self,
        dimensions: &[&[Value]],
        current: &mut Vec<Value>,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if dimensions.is_empty() {
            if let Err(error) = self.enter(depth) {
                return emit(Err(error));
            }
            return emit(Ok(Value::array(Vec::new())));
        }

        // Iterate an odometer rather than recursing once per dimension. The
        // dimension count is user-controlled, so native call-stack depth must
        // not become an unmetered VM resource.
        if dimensions.iter().any(|dimension| dimension.is_empty()) {
            return true;
        }

        let mut indices: Vec<usize> = Vec::new();
        if indices.try_reserve(dimensions.len()).is_err() {
            return emit(Err(resource("fork-stack")));
        }
        indices.resize(dimensions.len(), 0);
        current.clear();
        for dimension in dimensions {
            current.push(dimension[0].clone());
        }

        loop {
            if let Err(error) = self.enter(depth) {
                return emit(Err(error));
            }
            if !emit(Ok(Value::array(current.clone()))) {
                return false;
            }

            let mut position = dimensions.len();
            loop {
                if position == 0 {
                    return true;
                }
                position -= 1;
                indices[position] = indices[position].saturating_add(1);
                if indices[position] < dimensions[position].len() {
                    current[position] = dimensions[position][indices[position]].clone();
                    break;
                }
                indices[position] = 0;
                current[position] = dimensions[position][0].clone();
            }
        }
    }

    fn transpose(&self, input: &Value, depth: usize) -> Outcomes {
        let mut charge = || self.enter(depth);
        scalar::transpose_value(input, self.limits, &mut charge)
            .map_or_else(one_error, |value| vec![Ok(value)])
    }

    fn bsearch(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        self.argument_values(arguments, input, environment, depth, 0, |needle| {
            let mut charge = || self.enter(depth);
            scalar::bsearch_value(input, needle, self.limits, &mut charge)
        })
    }

    fn math_call(
        &self,
        name: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        if arguments.is_empty() {
            let value = match math_argument(name, input) {
                Ok(value) => value,
                Err(error) => return one_error(error),
            };
            return self
                .math_result(name, &[value], depth)
                .map_or_else(one_error, |value| vec![Ok(value)]);
        }

        let mut argument_values = Vec::with_capacity(arguments.len());
        for argument in arguments {
            argument_values.push(
                self.node(*argument, input, environment, depth)
                    .into_iter()
                    .map(|value| value.and_then(|value| math_argument(name, &value)))
                    .collect::<Vec<_>>(),
            );
        }

        // jq evaluates explicit arguments left-to-right, but emits the
        // Cartesian product with the rightmost argument as the outer loop.
        // Retain errors from a later argument when an earlier argument is
        // empty (`pow(empty; error("later"))`), while an earlier error is
        // suppressed if a later argument is empty.
        let mut combinations = argument_values
            .pop()
            .unwrap_or_default()
            .into_iter()
            .map(|value| value.map(|value| vec![value]))
            .collect::<Vec<_>>();
        for values in argument_values.into_iter().rev() {
            if values.is_empty() {
                combinations.retain(Result::is_err);
                continue;
            }
            let suffixes = combinations;
            let mut next = Vec::new();
            for suffix in suffixes {
                for value in &values {
                    let combination = match (value, &suffix) {
                        (Ok(value), Ok(suffix)) => {
                            let mut combination = Vec::with_capacity(suffix.len() + 1);
                            combination.push(*value);
                            combination.extend(suffix.iter().copied());
                            Ok(combination)
                        }
                        (_, Err(error)) | (Err(error), Ok(_)) => Err(error.clone()),
                    };
                    next.push(combination);
                    if next.len() > self.limits.fork_stack {
                        return one_error(resource("fork-stack"));
                    }
                }
            }
            combinations = next;
        }

        combinations
            .into_iter()
            .map(|values| values.and_then(|values| self.math_result(name, &values, depth)))
            .collect()
    }

    fn math_result(&self, name: &str, arguments: &[f64], depth: usize) -> Result<Value, VmError> {
        if matches!(name, "jn" | "yn") {
            self.charge_bessel_work(arguments, depth)?;
        }
        math::evaluate(name, arguments)
            .map(math_result_value)
            .map_err(|error| math_error(name, &error))
    }

    fn charge_bessel_work(&self, arguments: &[f64], depth: usize) -> Result<(), VmError> {
        let Some(order) = arguments.first().copied() else {
            return Ok(());
        };
        if !order.is_finite() || order.abs() > 1_024.0 {
            return Ok(());
        }
        let mut remaining = order.abs().trunc();
        while remaining >= 1.0 {
            self.enter(depth)?;
            remaining -= 1.0;
        }
        Ok(())
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
                    if values_only {
                        match self
                            .node(*argument, value, environment, depth)
                            .into_iter()
                            .next()
                        {
                            Some(Ok(value)) => mapped.push(value),
                            Some(Err(error)) => return one_error(error),
                            None => {}
                        }
                    } else {
                        match collect_values(self.node(*argument, value, environment, depth)) {
                            Ok(values) => mapped.extend(values),
                            Err(error) => return one_error(error),
                        }
                    }
                }
                vec![Ok(Value::array(mapped))]
            }
            Value::Object(object) if values_only => {
                let mut mapped = Object::new();
                for (key, value) in object.iter() {
                    let values = self.node(*argument, value, environment, depth);
                    match values.into_iter().next() {
                        Some(Ok(value)) => {
                            mapped.insert(Arc::clone(key), value);
                        }
                        Some(Err(error)) => return one_error(error),
                        None => {}
                    }
                }
                vec![Ok(Value::object(mapped))]
            }
            Value::Object(object) => {
                let mut mapped = Vec::new();
                for value in object.values() {
                    match collect_values(self.node(*argument, value, environment, depth)) {
                        Ok(values) => mapped.extend(values),
                        Err(error) => return one_error(error),
                    }
                }
                vec![Ok(Value::array(mapped))]
            }
            value => one_error(type_error(
                if values_only { "map_values" } else { "map" },
                value,
            )),
        }
    }

    fn add(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(generator) = arguments.first() else {
            return fold_values(input, None, binary_add);
        };
        let mut result = None;
        for value in self.node(*generator, input, environment, depth) {
            let value = match value {
                Ok(value) => value,
                Err(error) => return one_error(error),
            };
            result = Some(match result {
                None => value,
                Some(result) => match binary_add(&result, &value) {
                    Ok(value) => value,
                    Err(error) => return one_error(error),
                },
            });
        }
        vec![Ok(result.unwrap_or(Value::Null))]
    }

    fn flatten(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        if arguments.is_empty() {
            return flatten(input, None).map_or_else(one_error, |value| vec![Ok(value)]);
        }
        let mut output = Vec::new();
        for requested in self.node(arguments[0], input, environment, depth) {
            let requested = match requested {
                Ok(Value::Number(number)) => match number.exact_index() {
                    Some(depth) => Some(depth),
                    None if number.as_f64().is_sign_negative() => {
                        output.push(Err(runtime(
                            "flatten depth must not be negative".to_owned(),
                        )));
                        break;
                    }
                    None => None,
                },
                Ok(value) => {
                    output.push(Err(type_error("flatten", &value)));
                    break;
                }
                Err(error) => {
                    output.push(Err(error));
                    break;
                }
            };
            match flatten(input, requested) {
                Ok(value) => output.push(Ok(value)),
                Err(error) => {
                    output.push(Err(error));
                    break;
                }
            }
        }
        output
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
            keyed.dedup_by(|left, right| collection::jq_equal(&left.0, &right.0));
        }
        vec![Ok(Value::array(
            keyed
                .into_iter()
                .map(|(_, value)| value)
                .collect::<Vec<_>>(),
        ))]
    }

    fn with_entries(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(argument) = arguments.first() else {
            return one_error(invalid("with_entries argument missing"));
        };
        let entries = match to_entries(input) {
            Ok(Value::Array(entries)) => entries,
            Ok(_) => unreachable!("to_entries always returns an array"),
            Err(error) => return one_error(error),
        };
        let mut mapped = Vec::new();
        for entry in entries.iter() {
            match collect_values(self.node(*argument, entry, environment, depth)) {
                Ok(values) => mapped.extend(values),
                Err(error) => return one_error(error),
            }
        }
        vec![from_entries(&mapped, "with_entries")]
    }

    fn keyed_values(
        &self,
        operation: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Result<Vec<(Value, Value)>, VmError> {
        let Value::Array(values) = input else {
            return Err(type_error(operation, input));
        };
        let Some(argument) = arguments.first() else {
            return Err(invalid("keyed builtin argument missing"));
        };
        let mut keyed = Vec::with_capacity(values.len());
        for value in values.iter() {
            let keys = collect_values(self.node(*argument, value, environment, depth))?;
            keyed.push((Value::array(keys), value.clone()));
        }
        Ok(keyed)
    }

    fn group_by(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let mut keyed = match self.keyed_values("group_by", arguments, input, environment, depth) {
            Ok(keyed) => keyed,
            Err(error) => return one_error(error),
        };
        sort_by_cached_key(&mut keyed);
        let mut groups: Vec<Vec<Value>> = Vec::new();
        let mut previous: Option<Value> = None;
        for (key, value) in keyed {
            if previous
                .as_ref()
                .is_none_or(|previous| !collection::jq_equal(previous, &key))
            {
                groups.push(Vec::new());
                previous = Some(key);
            }
            groups.last_mut().expect("group was created").push(value);
        }
        vec![Ok(Value::array(
            groups.into_iter().map(Value::array).collect::<Vec<_>>(),
        ))]
    }

    fn keyed_extrema(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        maximum: bool,
    ) -> Outcomes {
        let operation = if maximum { "max_by" } else { "min_by" };
        let keyed = match self.keyed_values(operation, arguments, input, environment, depth) {
            Ok(keyed) => keyed,
            Err(error) => return one_error(error),
        };
        let selected = if maximum {
            keyed
                .into_iter()
                .max_by(|left, right| jq_compare(&left.0, &right.0))
        } else {
            keyed
                .into_iter()
                .min_by(|left, right| jq_compare(&left.0, &right.0))
        };
        vec![Ok(selected.map_or(Value::Null, |(_, value)| value))]
    }

    fn limit(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(count_node) = arguments.first() else {
            return one_error(invalid("limit count missing"));
        };
        let Some(expression) = arguments.get(1) else {
            return one_error(invalid("limit expression missing"));
        };
        let mut output = Vec::new();
        for count in self.node(*count_node, input, environment, depth) {
            let count = match count.and_then(|value| limit_count(&value)) {
                Ok(count) => count,
                Err(error) => {
                    output.push(Err(error));
                    break;
                }
            };
            if count == 0 {
                continue;
            }
            let mut emitted = 0usize;
            self.emit_node(*expression, input, environment, depth, &mut |result| {
                let success = result.is_ok();
                output.push(result);
                if success {
                    emitted += 1;
                }
                success && emitted < count
            });
        }
        output
    }

    fn any_all(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        any: bool,
    ) -> Outcomes {
        let (generator, condition) = match arguments {
            [] => (None, None),
            [condition] => (None, Some(*condition)),
            [generator, condition] => (Some(*generator), Some(*condition)),
            _ => return one_error(invalid("predicate arity")),
        };
        let mut decision = None;
        let mut failure = None;
        let mut test = |generated: Result<Value, VmError>| {
            let generated = match generated {
                Ok(value) => value,
                Err(error) => {
                    failure = Some(error);
                    return false;
                }
            };
            if let Some(condition) = condition {
                self.emit_node(condition, &generated, environment, depth, &mut |tested| {
                    let tested = match tested {
                        Ok(value) => value.is_truthy(),
                        Err(error) => {
                            failure = Some(error);
                            return false;
                        }
                    };
                    if any == tested {
                        decision = Some(any);
                        false
                    } else {
                        true
                    }
                });
                decision.is_none() && failure.is_none()
            } else if any == generated.is_truthy() {
                decision = Some(any);
                false
            } else {
                true
            }
        };
        if let Some(generator) = generator {
            self.emit_node(generator, input, environment, depth, &mut test);
        } else {
            self.emit_iterable_values(input, depth, &mut test);
        }
        if let Some(error) = failure {
            one_error(error)
        } else {
            vec![Ok(Value::Bool(decision.unwrap_or(!any)))]
        }
    }

    fn emit_iterable_values(
        &self,
        input: &Value,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if let Err(error) = self.enter(depth) {
            return emit(Err(error));
        }
        match input {
            Value::Array(values) => {
                for value in values.iter().cloned() {
                    if let Err(error) = self.enter(depth) {
                        return emit(Err(error));
                    }
                    if !emit(Ok(value)) {
                        return false;
                    }
                }
                true
            }
            Value::Object(values) => {
                for value in values.values().cloned() {
                    if let Err(error) = self.enter(depth) {
                        return emit(Err(error));
                    }
                    if !emit(Ok(value)) {
                        return false;
                    }
                }
                true
            }
            value => emit(Err(type_error("iterate", value))),
        }
    }

    fn setpath(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(path_node) = arguments.first() else {
            return one_error(invalid("setpath path missing"));
        };
        let Some(value_node) = arguments.get(1) else {
            return one_error(invalid("setpath value missing"));
        };
        let paths = self.node(*path_node, input, environment, depth);
        let values = self.node(*value_node, input, environment, depth);
        let mut output = Vec::new();
        for value in &values {
            match value {
                Ok(value) => {
                    for path in &paths {
                        let path = match path {
                            Ok(path) => match jq_path(path) {
                                Ok(path) => path,
                                Err(error) => return one_error(error),
                            },
                            Err(error) => return one_error(error.clone()),
                        };
                        if path.len() > self.limits.path_stack {
                            return one_error(resource("path-stack"));
                        }
                        let mut charge = || self.enter(depth);
                        output.push(path::replace_or_create_bounded(
                            input,
                            &path,
                            value.clone(),
                            self.limits,
                            &mut charge,
                        ));
                    }
                }
                Err(error) => return one_error(error.clone()),
            }
        }
        output
    }

    fn paths_filter(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(filter) = arguments.first() else {
            return one_error(invalid("paths filter missing"));
        };
        let candidates = match self.descendant_path_values(input, depth + 1) {
            Ok(candidates) => candidates,
            Err(error) => return one_error(error),
        };
        let mut output = Vec::new();
        for (path, value) in candidates.into_iter().skip(1) {
            let values = self.node(*filter, &value, environment, depth + 1);
            for value in values {
                match value {
                    Ok(value) if value.is_truthy() => output.push(Ok(path_value(&path))),
                    Ok(_) => {}
                    Err(error) => {
                        output.push(Err(error));
                        return output;
                    }
                }
            }
        }
        output
    }

    fn pick(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(path_node) = arguments.first() else {
            return one_error(invalid("pick path expression missing"));
        };
        let paths = match self.paths(*path_node, input, environment, depth) {
            Ok(paths) => paths,
            Err(error) => return one_error(error),
        };
        let mut picked = Value::Null;
        for path in paths {
            let value = path.get(input).cloned().unwrap_or(Value::Null);
            let mut charge = || self.enter(depth);
            picked = match path::replace_or_create_bounded(
                &picked,
                path.components(),
                value,
                self.limits,
                &mut charge,
            ) {
                Ok(value) => value,
                Err(error) => return one_error(error),
            };
        }
        vec![Ok(picked)]
    }

    fn delete(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(path_node) = arguments.first() else {
            return one_error(invalid("del path expression missing"));
        };
        let mut paths = match self.paths(*path_node, input, environment, depth) {
            Ok(paths) => paths,
            Err(error) => return one_error(error),
        };
        paths.sort_by(compare_paths_for_deletion);
        paths.dedup_by(|left, right| left == right);
        let mut deleted = input.clone();
        for path in paths {
            let mut charge = || self.enter(depth);
            deleted = match path::delete_path_bounded(
                &deleted,
                path.components(),
                self.limits,
                &mut charge,
            ) {
                Ok(value) => value,
                Err(error) => return one_error(error),
            };
        }
        vec![Ok(deleted)]
    }

    fn delete_paths(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(paths_node) = arguments.first() else {
            return one_error(invalid("delpaths paths missing"));
        };
        let paths_values = self.node(*paths_node, input, environment, depth);
        let mut output = Vec::new();
        for paths_value in paths_values {
            let paths_value = match paths_value {
                Ok(value) => value,
                Err(error) => {
                    output.push(Err(error));
                    return output;
                }
            };
            let Value::Array(path_values) = paths_value else {
                return one_error(type_error("delpaths", &paths_value));
            };
            let mut paths = Vec::new();
            for path_value in path_values.iter() {
                match jq_path(path_value) {
                    Ok(path) if path.len() <= self.limits.path_stack => paths.push(Path::new(path)),
                    Ok(_) => return one_error(resource("path-stack")),
                    Err(error) => return one_error(error),
                }
            }
            paths.sort_by(compare_paths_for_deletion);
            paths.dedup_by(|left, right| left == right);
            let mut deleted = input.clone();
            for path in paths {
                let mut charge = || self.enter(depth);
                deleted = match path::delete_path_bounded(
                    &deleted,
                    path.components(),
                    self.limits,
                    &mut charge,
                ) {
                    Ok(value) => value,
                    Err(error) => return one_error(error),
                };
            }
            output.push(Ok(deleted));
        }
        output
    }

    fn fromstream(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let mut output = Vec::new();
        self.emit_fromstream(arguments, input, environment, depth, &mut |result| {
            output.push(result);
            true
        });
        output
    }

    fn emit_fromstream(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let Some(stream) = arguments.first() else {
            return emit(Err(invalid("fromstream stream argument missing")));
        };
        let mut root = None;
        self.emit_node(*stream, input, environment, depth, &mut |event| {
            let event = match event {
                Ok(event) => event,
                Err(error) => {
                    return emit(Err(error));
                }
            };
            let (path, value) = match stream_event(&event) {
                Ok(event) => event,
                Err(error) => {
                    return emit(Err(error));
                }
            };
            if path.len() > self.limits.path_stack {
                return emit(Err(resource("path-stack")));
            }
            if let Err(error) = self.enter(depth.saturating_add(path.len())) {
                return emit(Err(error));
            }
            match value {
                Some(value) if path.is_empty() => emit(Ok(value)),
                Some(value) => {
                    let base = root.take().unwrap_or(Value::Null);
                    let mut charge = || self.enter(depth.saturating_add(path.len()));
                    match path::replace_or_create_bounded(
                        &base,
                        &path,
                        value,
                        self.limits,
                        &mut charge,
                    ) {
                        Ok(rebuilt) => {
                            root = Some(rebuilt);
                            true
                        }
                        Err(error) => emit(Err(error)),
                    }
                }
                None if path.len() == 1 => emit_stream_root(&mut root, emit),
                None => true,
            }
        })
    }

    fn truncate_stream(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let mut output = Vec::new();
        self.emit_truncate_stream(arguments, input, environment, depth, &mut |result| {
            output.push(result);
            true
        });
        output
    }

    fn emit_truncate_stream(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let count = match limit_count(input) {
            Ok(count) => count,
            Err(error) => return emit(Err(error)),
        };
        for event_node in arguments {
            let keep_going = self.emit_node(*event_node, input, environment, depth, &mut |event| {
                let event = match event {
                    Ok(event) => event,
                    Err(error) => return emit(Err(error)),
                };
                let (mut path, value) = match stream_event(&event) {
                    Ok(event) => event,
                    Err(error) => return emit(Err(error)),
                };
                if path.len() <= count {
                    return true;
                }
                path.drain(..count);
                if path.len() > self.limits.path_stack {
                    return emit(Err(resource("path-stack")));
                }
                let mut record = vec![path_value(&Path::new(path))];
                if let Some(value) = value {
                    record.push(value);
                }
                emit(Ok(Value::array(record)))
            });
            if !keep_going {
                return false;
            }
        }
        true
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
        if operator == AssignmentOperator::Update {
            let mut document = input.clone();
            let mut deletions = path_builtin::PathAccumulator::new();
            for path in paths {
                let old = path.get(&document).cloned().unwrap_or(Value::Null);
                let mut values = self.node(value_node, &old, environment, depth);
                match values.drain(..).next() {
                    Some(Ok(replacement)) => {
                        let mut charge = || self.enter(depth);
                        document = match path::replace_or_create_bounded(
                            &document,
                            path.components(),
                            replacement,
                            self.limits,
                            &mut charge,
                        ) {
                            Ok(value) => value,
                            Err(error) => return one_error(error),
                        };
                    }
                    Some(Err(error)) => return one_error(error),
                    None => {
                        let mut charge = || self.enter(depth);
                        if let Err(error) =
                            deletions.push(path.components(), self.limits, &mut charge)
                        {
                            return one_error(error);
                        }
                    }
                }
            }
            let mut charge = || self.enter(depth);
            document = match path_builtin::delete_paths_bounded(
                &document,
                deletions,
                self.limits,
                &mut charge,
            ) {
                Ok(value) => value,
                Err(error) => return one_error(error),
            };
            return vec![Ok(document)];
        }

        let values = self.node(value_node, input, environment, depth);
        let mut documents = Vec::new();
        for replacement in values {
            let replacement = match replacement {
                Ok(replacement) => replacement,
                Err(error) => {
                    documents.push(Err(error));
                    break;
                }
            };
            let mut document = input.clone();
            for path in &paths {
                let old = path.get(input).cloned().unwrap_or(Value::Null);
                let replacement = match update_value(operator, &old, &replacement) {
                    Ok(value) => value,
                    Err(error) => {
                        documents.push(Err(error));
                        return documents;
                    }
                };
                let mut charge = || self.enter(depth);
                document = match path::replace_or_create_bounded(
                    &document,
                    path.components(),
                    replacement,
                    self.limits,
                    &mut charge,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        documents.push(Err(error));
                        return documents;
                    }
                };
            }
            documents.push(Ok(document));
        }
        documents
    }

    fn descendant_paths(&self, input: &Value, depth: usize) -> Result<Vec<Path>, VmError> {
        Ok(self
            .descendant_path_values(input, depth)?
            .into_iter()
            .skip(1)
            .map(|(path, _)| path)
            .collect())
    }

    fn descendant_path_values(
        &self,
        input: &Value,
        depth: usize,
    ) -> Result<Vec<(Path, Value)>, VmError> {
        let mut output = Vec::new();
        let mut pending = vec![(input.clone(), Vec::new())];
        while let Some((value, components)) = pending.pop() {
            if components.len() > self.limits.path_stack {
                return Err(resource("path-stack"));
            }
            self.enter(depth.saturating_add(components.len()))?;
            let mut observations = self.observations.get();
            observations.path_stack_high_water =
                observations.path_stack_high_water.max(components.len());
            self.observations.set(observations);
            output.push((Path::new(components.clone()), value.clone()));
            push_children(&value, &components, &mut pending);
        }
        Ok(output)
    }

    fn paths(
        &self,
        node: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Result<Vec<Path>, VmError> {
        self.path_values(node, input, environment, depth)
            .into_iter()
            .map(|result| {
                result.and_then(|(path, _)| {
                    let length = path.components().len();
                    if length > self.limits.path_stack {
                        Err(resource("path-stack"))
                    } else {
                        let mut observations = self.observations.get();
                        observations.path_stack_high_water =
                            observations.path_stack_high_water.max(length);
                        self.observations.set(observations);
                        Ok(path)
                    }
                })
            })
            .collect()
    }

    fn path_outcomes(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(node) = arguments.first() else {
            return one_error(invalid("path argument missing"));
        };
        self.path_values(*node, input, environment, depth)
            .into_iter()
            .map(|result| result.map(|(path, _)| path_value(&path)))
            .collect()
    }

    #[allow(
        clippy::too_many_lines,
        reason = "path traversal keeps assignment operations exhaustive in one bounded interpreter"
    )]
    fn path_values(
        &self,
        node: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Vec<Result<(Path, Value), VmError>> {
        if let Err(error) = self.enter(depth) {
            return vec![Err(error)];
        }
        let operation = match self.bytecode.instructions().get(node as usize) {
            Some(instruction) => instruction.operation.clone(),
            None => return vec![Err(invalid("path instruction missing"))],
        };
        match operation {
            Operation::Identity => vec![Ok((Path::root(), input.clone()))],
            Operation::AccessField { base, key } => {
                let key = match self.string(key) {
                    Ok(key) => Arc::clone(key),
                    Err(error) => return vec![Err(error)],
                };
                let mut output = Vec::new();
                for result in self.path_values(base, input, environment, depth + 1) {
                    let (path, value) = match result {
                        Ok(value) => value,
                        Err(error) => {
                            output.push(Err(error));
                            return output;
                        }
                    };
                    let selected = match access_field(&value, &key) {
                        Ok(selected) => selected,
                        Err(error) => {
                            output.push(Err(error));
                            return output;
                        }
                    };
                    let mut components = path.components().to_vec();
                    components.push(PathComponent::Key(Arc::clone(&key)));
                    output.push(Ok((Path::new(components), selected)));
                }
                output
            }
            Operation::AccessIndex { base, index } => {
                let mut output = Vec::new();
                for result in self.path_values(base, input, environment, depth + 1) {
                    let (path, value) = match result {
                        Ok(value) => value,
                        Err(error) => {
                            output.push(Err(error));
                            return output;
                        }
                    };
                    for index in self.node(index, input, environment, depth + 1) {
                        let index = match index {
                            Ok(index) => index,
                            Err(error) => {
                                output.push(Err(error));
                                return output;
                            }
                        };
                        let component = match path_component_for_target(&value, &index) {
                            Ok(component) => component,
                            Err(error) => {
                                output.push(Err(error));
                                return output;
                            }
                        };
                        let selected = match access_index(&value, &index) {
                            Ok(selected) => selected,
                            Err(error) => {
                                output.push(Err(error));
                                return output;
                            }
                        };
                        let mut components = path.components().to_vec();
                        components.push(component);
                        output.push(Ok((Path::new(components), selected)));
                    }
                }
                output
            }
            Operation::Iterate(base) => {
                let mut output = Vec::new();
                for result in self.path_values(base, input, environment, depth + 1) {
                    let (path, value) = match result {
                        Ok(value) => value,
                        Err(error) => {
                            output.push(Err(error));
                            return output;
                        }
                    };
                    match value {
                        Value::Array(values) => {
                            for (index, selected) in values.iter().enumerate() {
                                let mut components = path.components().to_vec();
                                components.push(PathComponent::Index(index));
                                output.push(Ok((Path::new(components), selected.clone())));
                            }
                        }
                        Value::Object(values) => {
                            for (key, selected) in values.iter() {
                                let mut components = path.components().to_vec();
                                components.push(PathComponent::Key(Arc::clone(key)));
                                output.push(Ok((Path::new(components), selected.clone())));
                            }
                        }
                        value => {
                            output.push(Err(type_error("update iteration", &value)));
                            return output;
                        }
                    }
                }
                output
            }
            Operation::RecursiveDescent => match self.descendant_path_values(input, depth + 1) {
                Ok(values) => values.into_iter().map(Ok).collect(),
                Err(error) => vec![Err(error)],
            },
            Operation::Pipe { left, right } => {
                let mut output = Vec::new();
                for result in self.path_values(left, input, environment, depth + 1) {
                    let (prefix, value) = match result {
                        Ok(value) => value,
                        Err(error) => {
                            output.push(Err(error));
                            return output;
                        }
                    };
                    for result in self.path_values(right, &value, environment, depth + 1) {
                        let (relative, selected) = match result {
                            Ok(value) => value,
                            Err(error) => {
                                output.push(Err(error));
                                return output;
                            }
                        };
                        let mut components = prefix.components().to_vec();
                        components.extend(relative.components().iter().cloned());
                        output.push(Ok((Path::new(components), selected)));
                    }
                }
                output
            }
            Operation::Comma { left, right } => {
                let mut output = self.path_values(left, input, environment, depth + 1);
                if output.iter().any(Result::is_err) {
                    return output;
                }
                output.extend(self.path_values(right, input, environment, depth + 1));
                output
            }
            Operation::Optional(child) => {
                let values = self.path_values(child, input, environment, depth + 1);
                if values
                    .last()
                    .is_some_and(|result| result.as_ref().is_err_and(is_catchable_error))
                {
                    values
                        .into_iter()
                        .filter_map(|result| result.ok().map(Ok))
                        .collect::<Vec<_>>()
                } else {
                    values
                }
            }
            Operation::Call { name, arguments } => {
                let is_select = match self.string(name) {
                    Ok(name) => name.as_ref() == "select",
                    Err(error) => return vec![Err(error)],
                };
                if !is_select {
                    if self.string(name).is_ok_and(|name| name.as_ref() == "error") {
                        let Some(argument) = arguments.first() else {
                            return vec![Err(raised(input.clone()))];
                        };
                        return self
                            .node(*argument, input, environment, depth + 1)
                            .into_iter()
                            .map(|result| result.map_or_else(Err, |value| Err(raised(value))))
                            .collect();
                    }
                    return vec![Err(runtime(
                        "assignment left side is not a path".to_owned(),
                    ))];
                }
                let Some(argument) = arguments.first() else {
                    return vec![Err(invalid("select argument missing"))];
                };
                let mut output = Vec::new();
                for result in self.node(*argument, input, environment, depth + 1) {
                    match result {
                        Ok(value) if value.is_truthy() => {
                            output.push(Ok((Path::root(), input.clone())));
                        }
                        Ok(_) => {}
                        Err(error) => {
                            output.push(Err(error));
                            return output;
                        }
                    }
                }
                output
            }
            _ => vec![Err(runtime(
                "assignment left side is not a path".to_owned(),
            ))],
        }
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

pub(super) fn one_error(error: VmError) -> Outcomes {
    vec![Err(error)]
}
