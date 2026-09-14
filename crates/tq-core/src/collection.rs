//! jq-compatible collection containment and index operations.

use std::sync::Arc;

use crate::{Number, Value, VmError, VmLimits};

pub(crate) fn contains(
    input: &Value,
    needle: &Value,
    limits: VmLimits,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    charge()?;
    let result = match (input, needle) {
        (Value::String(input), Value::String(needle)) => string_contains(input, needle, charge)?,
        (Value::Array(_), Value::Array(_))
        | (Value::Object(_), Value::Object(_))
        | (Value::Number(_), Value::Number(_)) => contains_value(input, needle, limits, charge)?,
        (Value::Null, Value::Null) => true,
        (Value::Bool(left), Value::Bool(right)) if left == right => true,
        _ => return Err(containment_error(input, needle)),
    };
    Ok(Value::Bool(result))
}

pub(crate) fn inside(
    input: &Value,
    container: &Value,
    limits: VmLimits,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    contains(container, input, limits, charge)
}

pub(crate) fn indices(
    input: &Value,
    needle: &Value,
    limits: VmLimits,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    charge()?;
    let positions = match (input, needle) {
        (Value::String(input), Value::String(needle)) => {
            string_indices(input, needle, limits.output_bytes, charge)?
        }
        (Value::Array(input), Value::Array(needle)) => {
            array_indices(input, needle, limits, charge)?
        }
        (Value::Array(input), needle) => {
            array_indices(input, std::slice::from_ref(needle), limits, charge)?
        }
        _ => return Err(index_error("indices", input, needle)),
    };
    positions_value(positions, limits.output_bytes)
}

pub(crate) fn index(
    input: &Value,
    needle: &Value,
    reverse: bool,
    limits: VmLimits,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let position = match (input, needle) {
        (Value::String(input), Value::String(needle)) => {
            string_index(input, needle, reverse, charge)?
        }
        (Value::Array(input), Value::Array(needle)) => {
            array_index(input, needle, reverse, limits, charge)?
        }
        (Value::Array(input), needle) => {
            array_index(input, std::slice::from_ref(needle), reverse, limits, charge)?
        }
        _ => return Err(index_error("index", input, needle)),
    };
    position
        .map(|position| {
            Number::parse(&position.to_string())
                .map(Value::Number)
                .map_err(|error| runtime(error.to_string()))
        })
        .transpose()
        .map(|position| position.unwrap_or(Value::Null))
}

enum ContainsTask<'a> {
    Compare {
        input: &'a Value,
        needle: &'a Value,
        depth: usize,
    },
    ArrayNeedle {
        input: &'a [Value],
        needle: &'a [Value],
        next: usize,
        depth: usize,
        pending: bool,
    },
    ArraySearch {
        input: &'a [Value],
        needle: &'a Value,
        next: usize,
        depth: usize,
    },
    ObjectNeedle {
        input: &'a crate::Object,
        needle: crate::value::ObjectIter<'a>,
        depth: usize,
        pending: bool,
    },
}

enum EqualityTask<'a> {
    Compare {
        left: &'a Value,
        right: &'a Value,
        depth: usize,
    },
    Array {
        left: &'a [Value],
        right: &'a [Value],
        next: usize,
        depth: usize,
    },
    Object {
        right: &'a crate::Object,
        entries: crate::value::ObjectIter<'a>,
        depth: usize,
    },
}

fn push_contains_task<'a>(
    tasks: &mut Vec<ContainsTask<'a>>,
    task: ContainsTask<'a>,
) -> Result<(), VmError> {
    tasks.try_reserve(1).map_err(|_| resource("call-stack"))?;
    tasks.push(task);
    Ok(())
}

fn push_equality_task<'a>(
    tasks: &mut Vec<EqualityTask<'a>>,
    task: EqualityTask<'a>,
) -> Result<(), VmError> {
    tasks.try_reserve(1).map_err(|_| resource("call-stack"))?;
    tasks.push(task);
    Ok(())
}

fn push_result(results: &mut Vec<bool>, result: bool) -> Result<(), VmError> {
    results.try_reserve(1).map_err(|_| resource("call-stack"))?;
    results.push(result);
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    reason = "the explicit traversal keeps collection depth and work checks in one bounded state machine"
)]
fn contains_value(
    input: &Value,
    needle: &Value,
    limits: VmLimits,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<bool, VmError> {
    let mut tasks = Vec::new();
    push_contains_task(
        &mut tasks,
        ContainsTask::Compare {
            input,
            needle,
            depth: 0,
        },
    )?;
    let mut results = Vec::new();

    while let Some(task) = tasks.pop() {
        match task {
            ContainsTask::Compare {
                input,
                needle,
                depth,
            } => {
                if depth >= limits.call_stack {
                    return Err(resource("call-stack"));
                }
                charge()?;
                match (input, needle) {
                    (Value::String(input), Value::String(needle)) => {
                        push_result(&mut results, string_contains(input, needle, charge)?)?;
                    }
                    (Value::Array(input), Value::Array(needle)) => {
                        push_contains_task(
                            &mut tasks,
                            ContainsTask::ArrayNeedle {
                                input,
                                needle,
                                next: 0,
                                depth,
                                pending: false,
                            },
                        )?;
                    }
                    (Value::Object(input), Value::Object(needle)) => {
                        push_contains_task(
                            &mut tasks,
                            ContainsTask::ObjectNeedle {
                                input: input.as_ref(),
                                needle: needle.iter(),
                                depth,
                                pending: false,
                            },
                        )?;
                    }
                    _ => push_result(
                        &mut results,
                        jq_equal_bounded(input, needle, depth, limits, charge)?,
                    )?,
                }
            }
            ContainsTask::ArrayNeedle {
                input,
                needle,
                next,
                depth,
                pending,
            } => {
                if pending && !results.pop().ok_or_else(internal_collection_error)? {
                    push_result(&mut results, false)?;
                    continue;
                }
                if next == needle.len() {
                    push_result(&mut results, true)?;
                    continue;
                }
                charge()?;
                push_contains_task(
                    &mut tasks,
                    ContainsTask::ArrayNeedle {
                        input,
                        needle,
                        next: next + 1,
                        depth,
                        pending: true,
                    },
                )?;
                push_contains_task(
                    &mut tasks,
                    ContainsTask::ArraySearch {
                        input,
                        needle: &needle[next],
                        next: 0,
                        depth,
                    },
                )?;
            }
            ContainsTask::ArraySearch {
                input,
                needle,
                next,
                depth,
            } => {
                if let Some(found) = results.pop()
                    && found
                {
                    push_result(&mut results, true)?;
                    continue;
                }
                if next == input.len() {
                    push_result(&mut results, false)?;
                    continue;
                }
                charge()?;
                push_contains_task(
                    &mut tasks,
                    ContainsTask::ArraySearch {
                        input,
                        needle,
                        next: next + 1,
                        depth,
                    },
                )?;
                push_contains_task(
                    &mut tasks,
                    ContainsTask::Compare {
                        input: &input[next],
                        needle,
                        depth: depth.saturating_add(1),
                    },
                )?;
            }
            ContainsTask::ObjectNeedle {
                input,
                mut needle,
                depth,
                pending,
            } => {
                if pending && !results.pop().ok_or_else(internal_collection_error)? {
                    push_result(&mut results, false)?;
                    continue;
                }
                let Some((key, needle_value)) = needle.next() else {
                    push_result(&mut results, true)?;
                    continue;
                };
                charge()?;
                let Some(child_input) = input.get(key) else {
                    push_result(&mut results, false)?;
                    continue;
                };
                push_contains_task(
                    &mut tasks,
                    ContainsTask::ObjectNeedle {
                        input,
                        needle,
                        depth,
                        pending: true,
                    },
                )?;
                push_contains_task(
                    &mut tasks,
                    ContainsTask::Compare {
                        input: child_input,
                        needle: needle_value,
                        depth: depth.saturating_add(1),
                    },
                )?;
            }
        }
    }
    results.pop().ok_or_else(internal_collection_error)
}

#[allow(
    clippy::too_many_lines,
    reason = "the explicit traversal keeps collection depth and work checks in one bounded state machine"
)]
fn jq_equal_bounded(
    left: &Value,
    right: &Value,
    depth: usize,
    limits: VmLimits,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<bool, VmError> {
    if depth >= limits.call_stack {
        return Err(resource("call-stack"));
    }
    match (left, right) {
        (Value::Null, Value::Null) => {
            charge()?;
            return Ok(true);
        }
        (Value::Bool(left), Value::Bool(right)) => {
            charge()?;
            return Ok(left == right);
        }
        (Value::Number(left), Value::Number(right)) => {
            charge()?;
            return Ok(jq_number_equal(left, right));
        }
        (Value::String(left), Value::String(right)) => {
            charge()?;
            return Ok(left == right);
        }
        (Value::Array(left), Value::Array(right)) if Arc::ptr_eq(left, right) => {
            charge()?;
            return Ok(true);
        }
        (Value::Object(left), Value::Object(right)) if Arc::ptr_eq(left, right) => {
            charge()?;
            return Ok(true);
        }
        (left, right)
            if !matches!(left, Value::Array(_) | Value::Object(_))
                || !matches!(right, Value::Array(_) | Value::Object(_)) =>
        {
            charge()?;
            return Ok(false);
        }
        _ => {}
    }

    let mut tasks = Vec::new();
    push_equality_task(&mut tasks, EqualityTask::Compare { left, right, depth })?;
    let mut results = Vec::new();

    while let Some(task) = tasks.pop() {
        match task {
            EqualityTask::Compare { left, right, depth } => {
                if depth >= limits.call_stack {
                    return Err(resource("call-stack"));
                }
                charge()?;
                match (left, right) {
                    (Value::Null, Value::Null) => push_result(&mut results, true)?,
                    (Value::Bool(left), Value::Bool(right)) => {
                        push_result(&mut results, left == right)?;
                    }
                    (Value::Number(left), Value::Number(right)) => {
                        push_result(&mut results, jq_number_equal(left, right))?;
                    }
                    (Value::String(left), Value::String(right)) => {
                        push_result(&mut results, left == right)?;
                    }
                    (Value::Array(left), Value::Array(right)) => {
                        if Arc::ptr_eq(left, right) {
                            push_result(&mut results, true)?;
                        } else if left.len() != right.len() {
                            push_result(&mut results, false)?;
                        } else if left.is_empty() {
                            push_result(&mut results, true)?;
                        } else {
                            push_equality_task(
                                &mut tasks,
                                EqualityTask::Array {
                                    left,
                                    right,
                                    next: 1,
                                    depth,
                                },
                            )?;
                            push_equality_task(
                                &mut tasks,
                                EqualityTask::Compare {
                                    left: &left[0],
                                    right: &right[0],
                                    depth: depth.saturating_add(1),
                                },
                            )?;
                        }
                    }
                    (Value::Object(left), Value::Object(right)) => {
                        if Arc::ptr_eq(left, right) {
                            push_result(&mut results, true)?;
                        } else if left.len() != right.len() {
                            push_result(&mut results, false)?;
                        } else {
                            let left_object = left.as_ref();
                            let right_object = right.as_ref();
                            let mut entries = left_object.iter();
                            let Some((key, left_value)) = entries.next() else {
                                push_result(&mut results, true)?;
                                continue;
                            };
                            let Some(right_value) = right_object.get(key) else {
                                push_result(&mut results, false)?;
                                continue;
                            };
                            push_equality_task(
                                &mut tasks,
                                EqualityTask::Object {
                                    right: right_object,
                                    entries,
                                    depth,
                                },
                            )?;
                            push_equality_task(
                                &mut tasks,
                                EqualityTask::Compare {
                                    left: left_value,
                                    right: right_value,
                                    depth: depth.saturating_add(1),
                                },
                            )?;
                        }
                    }
                    _ => push_result(&mut results, false)?,
                }
            }
            EqualityTask::Array {
                left,
                right,
                next,
                depth,
            } => {
                if !results.pop().ok_or_else(internal_collection_error)? {
                    push_result(&mut results, false)?;
                } else if next == left.len() {
                    push_result(&mut results, true)?;
                } else {
                    push_equality_task(
                        &mut tasks,
                        EqualityTask::Array {
                            left,
                            right,
                            next: next + 1,
                            depth,
                        },
                    )?;
                    push_equality_task(
                        &mut tasks,
                        EqualityTask::Compare {
                            left: &left[next],
                            right: &right[next],
                            depth: depth.saturating_add(1),
                        },
                    )?;
                }
            }
            EqualityTask::Object {
                right,
                mut entries,
                depth,
            } => {
                if !results.pop().ok_or_else(internal_collection_error)? {
                    push_result(&mut results, false)?;
                    continue;
                }
                let Some((key, left_value)) = entries.next() else {
                    push_result(&mut results, true)?;
                    continue;
                };
                let Some(right_value) = right.get(key) else {
                    push_result(&mut results, false)?;
                    continue;
                };
                push_equality_task(
                    &mut tasks,
                    EqualityTask::Object {
                        right,
                        entries,
                        depth,
                    },
                )?;
                push_equality_task(
                    &mut tasks,
                    EqualityTask::Compare {
                        left: left_value,
                        right: right_value,
                        depth: depth.saturating_add(1),
                    },
                )?;
            }
        }
    }
    results.pop().ok_or_else(internal_collection_error)
}

pub(crate) fn jq_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Number(left), Value::Number(right)) => jq_number_equal(left, right),
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Array(left), Value::Array(right)) => {
            Arc::ptr_eq(left, right)
                || (left.len() == right.len()
                    && left
                        .iter()
                        .zip(right.iter())
                        .all(|(left, right)| jq_equal(left, right)))
        }
        (Value::Object(left), Value::Object(right)) => {
            Arc::ptr_eq(left, right)
                || (left.len() == right.len()
                    && left.iter().all(|(key, value)| {
                        right.get(key).is_some_and(|right| jq_equal(value, right))
                    }))
        }
        _ => false,
    }
}

fn jq_number_equal(left: &Number, right: &Number) -> bool {
    let left_value = left.as_f64();
    let right_value = right.as_f64();
    !left_value.is_nan() && !right_value.is_nan() && left == right
}

fn string_contains(
    input: &str,
    needle: &str,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<bool, VmError> {
    let input = input.chars().collect::<Vec<_>>();
    let needle = needle.chars().collect::<Vec<_>>();
    if needle.is_empty() {
        return Ok(true);
    }
    if needle.len() > input.len() {
        return Ok(false);
    }
    for window in input.windows(needle.len()) {
        charge()?;
        let mut matches = true;
        for (left, right) in window.iter().zip(needle.iter()) {
            charge()?;
            if left != right {
                matches = false;
                break;
            }
        }
        if matches {
            return Ok(true);
        }
    }
    Ok(false)
}

fn string_indices(
    input: &str,
    needle: &str,
    output_limit: usize,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<Vec<usize>, VmError> {
    let input = input.chars().collect::<Vec<_>>();
    let needle = needle.chars().collect::<Vec<_>>();
    if needle.is_empty() || needle.len() > input.len() {
        return Ok(Vec::new());
    }
    let mut positions = Vec::new();
    for (index, window) in input.windows(needle.len()).enumerate() {
        charge()?;
        let mut matches = true;
        for (left, right) in window.iter().zip(needle.iter()) {
            charge()?;
            if left != right {
                matches = false;
                break;
            }
        }
        if matches {
            push_position(&mut positions, index, output_limit)?;
        }
    }
    Ok(positions)
}

fn array_indices(
    input: &[Value],
    needle: &[Value],
    limits: VmLimits,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<Vec<usize>, VmError> {
    if needle.is_empty() || needle.len() > input.len() {
        return Ok(Vec::new());
    }
    let mut positions = Vec::new();
    for (index, window) in input.windows(needle.len()).enumerate() {
        charge()?;
        let mut matches = true;
        for (input, needle) in window.iter().zip(needle.iter()) {
            charge()?;
            if !jq_equal_bounded(input, needle, 0, limits, charge)? {
                matches = false;
                break;
            }
        }
        if matches {
            push_position(&mut positions, index, limits.output_bytes)?;
        }
    }
    Ok(positions)
}

fn string_index(
    input: &str,
    needle: &str,
    reverse: bool,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<Option<usize>, VmError> {
    let input = input.chars().collect::<Vec<_>>();
    let needle = needle.chars().collect::<Vec<_>>();
    if needle.is_empty() || needle.len() > input.len() {
        return Ok(None);
    }
    if reverse {
        for (index, window) in input.windows(needle.len()).enumerate().rev() {
            charge()?;
            if string_window_matches(window, &needle, charge)? {
                return Ok(Some(index));
            }
        }
    } else {
        for (index, window) in input.windows(needle.len()).enumerate() {
            charge()?;
            if string_window_matches(window, &needle, charge)? {
                return Ok(Some(index));
            }
        }
    }
    Ok(None)
}

fn string_window_matches(
    window: &[char],
    needle: &[char],
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<bool, VmError> {
    for (left, right) in window.iter().zip(needle.iter()) {
        charge()?;
        if left != right {
            return Ok(false);
        }
    }
    Ok(true)
}

fn array_index(
    input: &[Value],
    needle: &[Value],
    reverse: bool,
    limits: VmLimits,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<Option<usize>, VmError> {
    if needle.is_empty() || needle.len() > input.len() {
        return Ok(None);
    }
    if reverse {
        for (index, window) in input.windows(needle.len()).enumerate().rev() {
            charge()?;
            if array_window_matches(window, needle, limits, charge)? {
                return Ok(Some(index));
            }
        }
    } else {
        for (index, window) in input.windows(needle.len()).enumerate() {
            charge()?;
            if array_window_matches(window, needle, limits, charge)? {
                return Ok(Some(index));
            }
        }
    }
    Ok(None)
}

fn array_window_matches(
    window: &[Value],
    needle: &[Value],
    limits: VmLimits,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<bool, VmError> {
    for (input, needle) in window.iter().zip(needle.iter()) {
        charge()?;
        if !jq_equal_bounded(input, needle, 0, limits, charge)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn push_position(
    positions: &mut Vec<usize>,
    position: usize,
    output_limit: usize,
) -> Result<(), VmError> {
    let entries = positions
        .len()
        .checked_add(1)
        .ok_or_else(|| resource("output-bytes"))?;
    ensure_positions_lower_bound(entries, output_limit)?;
    positions
        .try_reserve(1)
        .map_err(|_| resource("output-bytes"))?;
    positions.push(position);
    Ok(())
}

fn positions_value(positions: Vec<usize>, output_limit: usize) -> Result<Value, VmError> {
    ensure_positions_lower_bound(positions.len(), output_limit)?;
    positions
        .into_iter()
        .map(|position| {
            Number::parse(&position.to_string())
                .map(Value::Number)
                .map_err(|error| runtime(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Value::array)
}

fn ensure_positions_lower_bound(entries: usize, output_limit: usize) -> Result<(), VmError> {
    let lower_bound = entries
        .checked_add(2)
        .ok_or_else(|| resource("output-bytes"))?;
    if lower_bound > output_limit {
        return Err(resource("output-bytes"));
    }
    Ok(())
}

fn containment_error(input: &Value, needle: &Value) -> VmError {
    runtime(format!(
        "{} ({}) and {} ({}) cannot have their containment checked",
        input_type(input),
        input,
        input_type(needle),
        needle
    ))
}

fn index_error(operation: &str, input: &Value, needle: &Value) -> VmError {
    runtime(format!(
        "{operation} cannot be applied to {} and {}",
        input_type(input),
        input_type(needle)
    ))
}

fn input_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn runtime(message: String) -> VmError {
    VmError::Runtime {
        message: message.into(),
    }
}

fn resource(resource: &'static str) -> VmError {
    VmError::Resource { resource }
}

fn internal_collection_error() -> VmError {
    runtime("collection traversal stack invariant violated".to_owned())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn nested_array(depth: usize) -> Value {
        let mut value = Value::Null;
        for _ in 0..depth {
            value = Value::array([value]);
        }
        value
    }

    #[test]
    fn nested_containment_equality_is_charged_and_depth_bounded() {
        let input = nested_array(8);
        let needle = nested_array(8);
        let calls = Cell::new(0);
        let charge = || {
            calls.set(calls.get() + 1);
            Ok(())
        };
        let error = contains(
            &input,
            &needle,
            VmLimits {
                call_stack: 4,
                ..VmLimits::default()
            },
            &charge,
        )
        .expect_err("nested equality should hit the call-depth limit");
        assert_eq!(error, resource("call-stack"));
        assert!(calls.get() > 4);
    }

    #[test]
    fn nested_inside_equality_is_depth_bounded() {
        let input = nested_array(8);
        let container = nested_array(8);
        let charge = || Ok(());
        let error = inside(
            &input,
            &container,
            VmLimits {
                call_stack: 4,
                ..VmLimits::default()
            },
            &charge,
        )
        .expect_err("nested inside equality should hit the call-depth limit");
        assert_eq!(error, resource("call-stack"));
    }

    #[test]
    fn nested_index_equality_stops_at_the_work_boundary() {
        let input = Value::array([nested_array(8)]);
        let needle = nested_array(8);
        let remaining = Cell::new(4usize);
        let charge = || {
            if remaining.get() == 0 {
                Err(resource("vm-steps"))
            } else {
                remaining.set(remaining.get() - 1);
                Ok(())
            }
        };
        let error = index(&input, &needle, false, VmLimits::default(), &charge)
            .expect_err("nested index equality must charge every comparison");
        assert_eq!(error, resource("vm-steps"));
    }

    #[test]
    fn nested_mixed_object_array_equality_preserves_result_ownership() {
        let nested = || {
            Value::array([Value::object(crate::Object::from_iter([(
                "inner".into(),
                Value::array([Value::Number(Number::from_runtime_f64(1.0))]),
            )]))])
        };
        let input = Value::object(crate::Object::from_iter([("outer".into(), nested())]));
        let needle = Value::object(crate::Object::from_iter([("outer".into(), nested())]));
        let charge = || Ok(());
        assert_eq!(
            contains(&input, &needle, VmLimits::default(), &charge),
            Ok(Value::Bool(true))
        );
    }

    #[test]
    fn array_indices_bound_retained_positions_before_growth() {
        let input = Value::array(
            std::iter::repeat_with(|| Value::Number(Number::from_runtime_f64(1.0)))
                .take(4)
                .collect::<Vec<_>>(),
        );
        let needle = Value::Number(Number::from_runtime_f64(1.0));
        let charge = || Ok(());
        let error = indices(
            &input,
            &needle,
            VmLimits {
                output_bytes: 4,
                ..VmLimits::default()
            },
            &charge,
        )
        .expect_err("retained index positions must obey the output budget");
        assert_eq!(error, resource("output-bytes"));
    }

    #[test]
    fn string_indices_bound_retained_positions_before_growth() {
        let input = Value::string("aaaa");
        let needle = Value::string("a");
        let charge = || Ok(());
        let error = indices(
            &input,
            &needle,
            VmLimits {
                output_bytes: 4,
                ..VmLimits::default()
            },
            &charge,
        )
        .expect_err("retained string index positions must obey the output budget");
        assert_eq!(error, resource("output-bytes"));
    }

    #[test]
    fn empty_indices_obey_the_container_lower_bound() {
        let input = Value::string("abc");
        let needle = Value::string("z");
        let charge = || Ok(());
        assert_eq!(
            indices(
                &input,
                &needle,
                VmLimits {
                    output_bytes: 1,
                    ..VmLimits::default()
                },
                &charge,
            ),
            Err(resource("output-bytes"))
        );
        assert_eq!(
            indices(
                &input,
                &needle,
                VmLimits {
                    output_bytes: 2,
                    ..VmLimits::default()
                },
                &charge,
            )
            .unwrap(),
            Value::array(Vec::new())
        );
    }

    #[test]
    fn scalar_index_does_not_retain_all_matching_positions() {
        let input = Value::array(
            std::iter::repeat_with(|| Value::Number(Number::from_runtime_f64(1.0)))
                .take(4)
                .collect::<Vec<_>>(),
        );
        let needle = Value::Number(Number::from_runtime_f64(1.0));
        let charge = || Ok(());
        assert_eq!(
            index(
                &input,
                &needle,
                false,
                VmLimits {
                    output_bytes: 0,
                    ..VmLimits::default()
                },
                &charge,
            )
            .unwrap(),
            Value::Number(Number::from_runtime_f64(0.0))
        );
        assert_eq!(
            index(
                &input,
                &needle,
                true,
                VmLimits {
                    output_bytes: 0,
                    ..VmLimits::default()
                },
                &charge,
            )
            .unwrap(),
            Value::Number(Number::from_runtime_f64(3.0))
        );
    }
}
