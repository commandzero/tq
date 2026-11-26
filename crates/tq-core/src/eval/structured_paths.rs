//! Bind patterns and apply values at jq paths.

use super::managed_work::{AssignmentState, AssignmentTarget};
use super::ordered::ensure_managed_container_size;
use super::value_ops::{
    access_index, binary_add, binary_multiply, binary_subtract, normalize_index, number_usize,
    numeric, type_error,
};
use super::{Environment, invalid, path, path_builtin, resource, runtime};
use crate::ast::AssignmentOperator;
use crate::bytecode::BindingPatternOperand;
use crate::{Bytecode, Number, Path, PathComponent, Value, VmError, VmLimits};
use std::sync::Arc;

pub(super) fn is_pattern_mismatch(error: &VmError) -> bool {
    matches!(error, VmError::Runtime { message } if message.contains("destructuring cannot be applied"))
}

pub(super) fn is_retryable_alternative_error(error: &VmError) -> bool {
    matches!(
        error,
        VmError::Runtime { .. }
            | VmError::CapabilityDenied { .. }
            | VmError::Raised { .. }
            | VmError::Break { .. }
    )
}

pub(super) fn bind_pattern(
    bytecode: &Bytecode,
    pattern: &BindingPatternOperand,
    value: &Value,
    environment: &mut Environment,
    depth: usize,
    charge: &mut dyn FnMut(usize) -> Result<(), VmError>,
) -> Result<(), VmError> {
    charge(depth)?;
    match pattern {
        BindingPatternOperand::Variable(name) => {
            let name = bytecode
                .string(*name)
                .ok_or_else(|| invalid("pattern variable missing after validation"))?;
            environment.insert(Arc::clone(name), value.clone());
            Ok(())
        }
        BindingPatternOperand::Array(patterns) => {
            match value {
                Value::Array(values) => {
                    for (index, pattern) in patterns.iter().enumerate() {
                        let value = values.get(index).cloned().unwrap_or(Value::Null);
                        bind_pattern(
                            bytecode,
                            pattern,
                            &value,
                            environment,
                            depth.saturating_add(1),
                            charge,
                        )?;
                    }
                }
                Value::Null => {
                    for pattern in patterns {
                        bind_pattern(
                            bytecode,
                            pattern,
                            &Value::Null,
                            environment,
                            depth.saturating_add(1),
                            charge,
                        )?;
                    }
                }
                value => return Err(type_error("array destructuring", value)),
            }
            Ok(())
        }
        BindingPatternOperand::Object(entries) => {
            match value {
                Value::Object(values) => {
                    for (key, pattern) in entries {
                        let key = bytecode
                            .string(*key)
                            .ok_or_else(|| invalid("pattern key missing after validation"))?;
                        let value = values.get(key).cloned().unwrap_or(Value::Null);
                        bind_pattern(
                            bytecode,
                            pattern,
                            &value,
                            environment,
                            depth.saturating_add(1),
                            charge,
                        )?;
                    }
                }
                Value::Null => {
                    for (_, pattern) in entries {
                        bind_pattern(
                            bytecode,
                            pattern,
                            &Value::Null,
                            environment,
                            depth.saturating_add(1),
                            charge,
                        )?;
                    }
                }
                value => return Err(type_error("object destructuring", value)),
            }
            Ok(())
        }
    }
}

pub(super) fn initialize_pattern_variables(
    bytecode: &Bytecode,
    patterns: &[BindingPatternOperand],
    environment: &mut Environment,
) -> Result<(), VmError> {
    for pattern in patterns {
        initialize_pattern_variables_one(bytecode, pattern, environment)?;
    }
    Ok(())
}

fn initialize_pattern_variables_one(
    bytecode: &Bytecode,
    pattern: &BindingPatternOperand,
    environment: &mut Environment,
) -> Result<(), VmError> {
    match pattern {
        BindingPatternOperand::Variable(name) => {
            let name = bytecode
                .string(*name)
                .ok_or_else(|| invalid("pattern variable missing after validation"))?;
            environment.entry(Arc::clone(name)).or_insert(Value::Null);
        }
        BindingPatternOperand::Array(patterns) => {
            for pattern in patterns {
                initialize_pattern_variables_one(bytecode, pattern, environment)?;
            }
        }
        BindingPatternOperand::Object(entries) => {
            for (_, pattern) in entries {
                initialize_pattern_variables_one(bytecode, pattern, environment)?;
            }
        }
    }
    Ok(())
}
pub(super) fn path_component(value: &Value) -> Result<PathComponent, VmError> {
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

pub(super) fn path_component_for_target(
    target: &Value,
    value: &Value,
) -> Result<PathComponent, VmError> {
    match value {
        Value::Number(index) if index.as_f64() < 0.0 => {
            let Value::Array(values) = target else {
                return path_component(value);
            };
            let index = index
                .exact_index()
                .and_then(|index| normalize_index(index, values.len()))
                .ok_or_else(|| runtime("path index is outside the target array".to_owned()))?;
            Ok(PathComponent::Index(index))
        }
        _ => path_component(value),
    }
}

pub(super) fn jq_path(value: &Value) -> Result<Vec<PathComponent>, VmError> {
    let Value::Array(components) = value else {
        return Err(type_error("path", value));
    };
    components.iter().map(path_component).collect()
}

pub(super) fn stream_event(value: &Value) -> Result<(Vec<PathComponent>, Option<Value>), VmError> {
    let Value::Array(parts) = value else {
        return Err(runtime("stream event must be an array".to_owned()));
    };
    if !(1..=2).contains(&parts.len()) {
        return Err(runtime(
            "stream event must contain a path and optional value".to_owned(),
        ));
    }
    let path = jq_path(&parts[0])?;
    Ok((path, parts.get(1).cloned()))
}

pub(super) fn emit_stream_root(
    root: &mut Option<Value>,
    emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
) -> bool {
    root.take().is_none_or(|root| emit(Ok(root)))
}

pub(super) fn path_value(path: &Path) -> Value {
    Value::array(
        path.components()
            .iter()
            .map(|component| match component {
                PathComponent::Key(key) => Value::String(Arc::clone(key)),
                PathComponent::Index(index) => number_usize(*index),
            })
            .collect::<Vec<_>>(),
    )
}

pub(super) fn root_child_is_last(input: &Value, path: &[PathComponent]) -> bool {
    let Some(component) = path.first() else {
        return false;
    };
    match (input, component) {
        (Value::Array(values), PathComponent::Index(index)) => {
            index.saturating_add(1) == values.len()
        }
        (Value::Object(values), PathComponent::Key(key)) => {
            values.keys().last().is_some_and(|last| last == key)
        }
        _ => false,
    }
}

pub(super) fn getpath(input: &Value, path: &[PathComponent]) -> Result<Value, VmError> {
    let mut current = input.clone();
    for component in path {
        current = match component {
            PathComponent::Key(key) => access_index(&current, &Value::String(Arc::clone(key)))?,
            PathComponent::Index(index) => access_index(&current, &number_usize(*index))?,
        };
    }
    Ok(current)
}

pub(super) fn push_children(
    value: &Value,
    prefix: &[PathComponent],
    pending: &mut Vec<(Value, Vec<PathComponent>)>,
) {
    match value {
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate().rev() {
                let mut path = prefix.to_vec();
                path.push(PathComponent::Index(index));
                pending.push((child.clone(), path));
            }
        }
        Value::Object(values) => {
            for (key, child) in values.iter().rev() {
                let mut path = prefix.to_vec();
                path.push(PathComponent::Key(Arc::clone(key)));
                pending.push((child.clone(), path));
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

pub(super) fn tostream(input: &Value, limit: usize) -> Result<Vec<Value>, VmError> {
    let mut output = Vec::new();
    tostream_at(input, Vec::new(), true, limit, &mut output)?;
    Ok(output)
}

fn tostream_at(
    value: &Value,
    path: Vec<PathComponent>,
    root: bool,
    limit: usize,
    output: &mut Vec<Value>,
) -> Result<(), VmError> {
    if path.len() > limit {
        return Err(resource("path-stack"));
    }
    let empty_container = matches!(value, Value::Array(values) if values.is_empty())
        || matches!(value, Value::Object(values) if values.is_empty());
    match value {
        Value::Array(values) if !values.is_empty() => {
            for (index, child) in values.iter().enumerate() {
                let mut child_path = path.clone();
                child_path.push(PathComponent::Index(index));
                tostream_at(child, child_path, false, limit, output)?;
            }
        }
        Value::Object(values) if !values.is_empty() => {
            for (key, child) in values.iter() {
                let mut child_path = path.clone();
                child_path.push(PathComponent::Key(Arc::clone(key)));
                tostream_at(child, child_path, false, limit, output)?;
            }
        }
        _ => output.push(Value::array(vec![
            path_value(&Path::new(path.clone())),
            value.clone(),
        ])),
    }
    if !root {
        output.push(Value::array(vec![path_value(&Path::new(path))]));
    } else if root && empty_container {
        // Empty roots were already emitted as scalar-like stream records.
    }
    Ok(())
}

pub(super) fn update_value(
    operator: AssignmentOperator,
    old: &Value,
    replacement: &Value,
) -> Result<Value, VmError> {
    match operator {
        AssignmentOperator::Set | AssignmentOperator::Update => Ok(replacement.clone()),
        AssignmentOperator::Add => binary_add(old, replacement),
        AssignmentOperator::Subtract => binary_subtract(old, replacement),
        AssignmentOperator::Multiply => binary_multiply(old, replacement),
        AssignmentOperator::Divide => numeric(old, replacement, Number::divide, "divide"),
        AssignmentOperator::Alternative => Ok(if old.is_truthy() {
            old.clone()
        } else {
            replacement.clone()
        }),
    }
}

pub(super) fn apply_plain_assignment(
    state: &AssignmentState,
    replacement: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let mut document = state.input.clone();
    for target in &state.targets {
        let old = assignment_target_value(&state.input, target, limits, charge)?;
        let replacement = update_value(state.operator, &old, replacement)?;
        document = apply_assignment_target(&document, target, replacement, limits, charge)?;
    }
    Ok(document)
}

pub(super) fn assignment_target_value(
    root: &Value,
    target: &AssignmentTarget,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    match target {
        AssignmentTarget::Path(path) => {
            path::getpath_bounded(root, path.components(), limits, charge)
        }
        AssignmentTarget::Slice { path, start, end } => {
            match path::getpath_bounded(root, path.components(), limits, charge)? {
                Value::Array(values) => {
                    let Some(values) = values.get(*start..*end) else {
                        return Ok(Value::Null);
                    };
                    charge()?;
                    ensure_managed_container_size(values.len(), limits)?;
                    let mut selected = Vec::new();
                    selected
                        .try_reserve_exact(values.len())
                        .map_err(|_| resource("output-bytes"))?;
                    for value in values {
                        charge()?;
                        selected.push(value.clone());
                    }
                    Ok(Value::array(selected))
                }
                _ => Ok(Value::Null),
            }
        }
    }
}

pub(super) fn apply_assignment_target(
    root: &Value,
    target: &AssignmentTarget,
    replacement: Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    match target {
        AssignmentTarget::Path(path) => {
            path::replace_or_create_bounded(root, path.components(), replacement, limits, charge)
        }
        AssignmentTarget::Slice { path, start, end } => path::replace_slice_bounded(
            root,
            path.components(),
            *start,
            *end,
            replacement,
            limits,
            charge,
        ),
    }
}

pub(super) fn retain_assignment_deletion_target(
    accumulator: &mut path_builtin::PathAccumulator,
    target: &AssignmentTarget,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<(), VmError> {
    match target {
        AssignmentTarget::Path(path) => accumulator.push(path.components(), limits, charge),
        AssignmentTarget::Slice { path, start, end } => {
            let total = path
                .components()
                .len()
                .checked_add(1)
                .ok_or_else(|| resource("path-stack"))?;
            if total > limits.path_stack {
                return Err(resource("path-stack"));
            }
            for index in *start..*end {
                let mut components = Vec::new();
                components
                    .try_reserve_exact(total)
                    .map_err(|_| resource("path-stack"))?;
                for component in path.components() {
                    charge()?;
                    components.push(component.clone());
                }
                charge()?;
                components.push(PathComponent::Index(index));
                accumulator.push(&components, limits, charge)?;
            }
            Ok(())
        }
    }
}

pub(super) fn compare_paths_for_deletion(left: &Path, right: &Path) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    for (left, right) in left.components().iter().zip(right.components()) {
        let ordering = match (left, right) {
            (PathComponent::Key(left), PathComponent::Key(right)) => left.cmp(right),
            (PathComponent::Index(left), PathComponent::Index(right)) => left.cmp(right),
            (PathComponent::Key(_), PathComponent::Index(_)) => Ordering::Less,
            (PathComponent::Index(_), PathComponent::Key(_)) => Ordering::Greater,
        };
        if ordering != Ordering::Equal {
            return ordering.reverse();
        }
    }
    right.components().len().cmp(&left.components().len())
}
