//! jq-compatible collection containment and index operations.

use std::sync::Arc;

use crate::{Number, Value, VmError};

pub(crate) fn contains(
    input: &Value,
    needle: &Value,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    charge()?;
    let result = match (input, needle) {
        (Value::String(input), Value::String(needle)) => string_contains(input, needle, charge)?,
        (Value::Array(_), Value::Array(_))
        | (Value::Object(_), Value::Object(_))
        | (Value::Number(_), Value::Number(_)) => contains_value(input, needle, charge)?,
        (Value::Null, Value::Null) => true,
        (Value::Bool(left), Value::Bool(right)) if left == right => true,
        _ => return Err(containment_error(input, needle)),
    };
    Ok(Value::Bool(result))
}

pub(crate) fn inside(
    input: &Value,
    container: &Value,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    contains(container, input, charge)
}

pub(crate) fn indices(
    input: &Value,
    needle: &Value,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    charge()?;
    let positions = match (input, needle) {
        (Value::String(input), Value::String(needle)) => string_indices(input, needle, charge)?,
        (Value::Array(input), Value::Array(needle)) => array_indices(input, needle, charge)?,
        (Value::Array(input), needle) => {
            array_indices(input, std::slice::from_ref(needle), charge)?
        }
        _ => return Err(index_error("indices", input, needle)),
    };
    positions_value(positions)
}

pub(crate) fn index(
    input: &Value,
    needle: &Value,
    reverse: bool,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let positions = indices(input, needle, charge)?;
    let Value::Array(positions) = positions else {
        unreachable!("indices always returns an array");
    };
    let selected = if reverse {
        positions.last()
    } else {
        positions.first()
    };
    Ok(selected.cloned().unwrap_or(Value::Null))
}

fn contains_value(
    input: &Value,
    needle: &Value,
    charge: &impl Fn() -> Result<(), VmError>,
) -> Result<bool, VmError> {
    charge()?;
    match (input, needle) {
        (Value::String(input), Value::String(needle)) => string_contains(input, needle, charge),
        (Value::Array(input), Value::Array(needle)) => {
            for needle in needle.iter() {
                charge()?;
                let mut found = false;
                for input in input.iter() {
                    charge()?;
                    if contains_value(input, needle, charge)? {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (Value::Object(input), Value::Object(needle)) => {
            for (key, needle) in needle.iter() {
                charge()?;
                let Some(input) = input.get(key) else {
                    return Ok(false);
                };
                if !contains_value(input, needle, charge)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        _ => Ok(jq_equal(input, needle)),
    }
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
            positions.push(index);
        }
    }
    Ok(positions)
}

fn array_indices(
    input: &[Value],
    needle: &[Value],
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
            if !jq_equal(input, needle) {
                matches = false;
                break;
            }
        }
        if matches {
            positions.push(index);
        }
    }
    Ok(positions)
}

fn positions_value(positions: Vec<usize>) -> Result<Value, VmError> {
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
