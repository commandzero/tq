//! Shared value operations for the managed and direct evaluators.

use super::direct::one_error;
use super::ordered::stable_sort_values;
use super::origins::OriginToken;
use super::{Outcomes, invalid, resource, runtime};
use crate::ast::{BinaryOperator, UnaryOperator};
use crate::{
    JsonInput, JsonInputError, JsonInputOptions, Number, Object, PathComponent, Value, VmError,
    VmLimits, collection, math, string_compat,
};
use std::io;
use std::sync::Arc;

pub(super) fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub(super) fn type_error(operation: &str, value: &Value) -> VmError {
    runtime(format!(
        "{operation} cannot be applied to {}",
        type_name(value)
    ))
}
pub(super) fn access_field(value: &Value, key: &Arc<str>) -> Result<Value, VmError> {
    match value {
        Value::Object(object) => Ok(object.get(key).cloned().unwrap_or(Value::Null)),
        Value::Null => Ok(Value::Null),
        value => Err(type_error("field access", value)),
    }
}

pub(super) fn access_index(value: &Value, index: &Value) -> Result<Value, VmError> {
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

pub(super) fn nth_index(value: &Value, index: &Value) -> Result<Value, VmError> {
    match index {
        Value::Number(number) => {
            let index = Value::Number(Number::from_runtime_f64(number.as_f64().trunc()));
            access_index(value, &index)
        }
        index => access_index(value, index),
    }
}

pub(super) fn index_origin(
    value: &Value,
    index: f64,
    origin: Option<&OriginToken>,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Option<OriginToken>, VmError> {
    let Value::Array(values) = value else {
        return Ok(None);
    };
    if !index.is_finite() {
        return Ok(None);
    }
    let index = index.trunc();
    #[allow(clippy::cast_possible_truncation)]
    let index = index as i128;
    let Some(index) = i64::try_from(index).ok() else {
        return Ok(None);
    };
    let Some(index) = normalize_index(index, values.len()) else {
        return Ok(None);
    };
    origin
        .map(|origin| origin.child_bounded(PathComponent::Index(index), limits, charge))
        .transpose()
}

pub(super) fn normalize_index(index: i64, length: usize) -> Option<usize> {
    if index >= 0 {
        usize::try_from(index).ok().filter(|index| *index < length)
    } else {
        let signed_length = i64::try_from(length).ok()?;
        usize::try_from(signed_length + index)
            .ok()
            .filter(|index| *index < length)
    }
}

pub(super) fn slice_bound_value(value: &Value) -> Result<Option<f64>, VmError> {
    match value {
        Value::Null => Ok(None),
        Value::Number(number) => Ok(Some(number.as_f64())),
        value => Err(type_error("slice", value)),
    }
}

pub(super) fn slice(value: &Value, start: Option<f64>, end: Option<f64>) -> Result<Value, VmError> {
    match value {
        Value::Null => Ok(Value::Null),
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

#[allow(clippy::cast_precision_loss)]
pub(super) fn slice_bounds(length: usize, start: Option<f64>, end: Option<f64>) -> (usize, usize) {
    let normalize = |value: f64, is_end: bool| {
        let rounded = if value.is_nan() {
            if is_end {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            }
        } else if value.is_infinite() {
            value
        } else if is_end {
            value.ceil()
        } else {
            value.floor()
        };
        if rounded < 0.0 || (is_end && value < 0.0 && rounded == 0.0) {
            let magnitude = -rounded;
            if magnitude >= length as f64 {
                0
            } else {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    length.saturating_sub(magnitude as usize)
                }
            }
        } else if rounded >= length as f64 || rounded.is_infinite() {
            length
        } else {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                rounded as usize
            }
        }
    };
    let start = start.map_or(0, |value| normalize(value, false));
    let end = end.map_or(length, |value| normalize(value, true));
    (start.min(end), end)
}

pub(super) fn computed_key(value: Value) -> Result<Arc<str>, VmError> {
    match value {
        Value::String(value) => Ok(value),
        value => Err(type_error("computed object key", &value)),
    }
}

pub(super) fn unary(operator: UnaryOperator, value: &Value) -> Result<Value, VmError> {
    match operator {
        UnaryOperator::Not => Ok(Value::Bool(!value.is_truthy())),
        UnaryOperator::Negate => match value {
            Value::Number(value) => Ok(Value::Number(value.negate())),
            value => Err(type_error("negation", value)),
        },
    }
}

pub(super) fn binary_value(
    operator: BinaryOperator,
    left: &Value,
    right: &Value,
) -> Result<Value, VmError> {
    match operator {
        BinaryOperator::Equal => Ok(Value::Bool(collection::jq_equal(left, right))),
        BinaryOperator::NotEqual => Ok(Value::Bool(!collection::jq_equal(left, right))),
        BinaryOperator::Less => Ok(Value::Bool(jq_compare(left, right).is_lt())),
        BinaryOperator::LessEqual => Ok(Value::Bool(!jq_compare(left, right).is_gt())),
        BinaryOperator::Greater => Ok(Value::Bool(jq_compare(left, right).is_gt())),
        BinaryOperator::GreaterEqual => Ok(Value::Bool(!jq_compare(left, right).is_lt())),
        BinaryOperator::Add => binary_add(left, right),
        BinaryOperator::Subtract => binary_subtract(left, right),
        BinaryOperator::Multiply => binary_multiply(left, right),
        BinaryOperator::Divide => match (left, right) {
            (Value::String(_), Value::String(_)) => string_compat::divide(left, right),
            _ => numeric(left, right, Number::divide, "divide"),
        },
        BinaryOperator::Remainder => match (left, right) {
            (Value::Number(left), Value::Number(right)) if right.as_f64() != 0.0 => {
                match (
                    remainder_operand(left.as_f64()),
                    remainder_operand(right.as_f64()),
                ) {
                    (Some(left), Some(right)) if right != 0 => {
                        Ok(runtime_number(integer_remainder(left, right)))
                    }
                    (Some(_), Some(0)) => Err(runtime("cannot divide by zero".to_owned())),
                    _ => Ok(runtime_number(f64::NAN)),
                }
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

#[allow(clippy::cast_possible_truncation)]
fn remainder_operand(value: f64) -> Option<i64> {
    if value.is_nan() {
        None
    } else if value >= 9_223_372_036_854_775_808.0 {
        Some(i64::MAX)
    } else if value <= -9_223_372_036_854_775_808.0 {
        Some(i64::MIN)
    } else {
        Some(value.trunc() as i64)
    }
}

#[allow(clippy::cast_precision_loss)]
fn integer_remainder(left: i64, right: i64) -> f64 {
    let remainder = if left == i64::MIN && right == -1 {
        0
    } else {
        left % right
    };
    remainder as f64
}

pub(super) fn jq_compare(left: &Value, right: &Value) -> std::cmp::Ordering {
    let kind = jq_kind_rank(left).cmp(&jq_kind_rank(right));
    if kind != std::cmp::Ordering::Equal {
        return kind;
    }
    match (left, right) {
        (Value::Bool(left), Value::Bool(right)) => left.cmp(right),
        (Value::Number(left), Value::Number(right)) => jq_number_compare(left, right),
        (Value::String(left), Value::String(right)) => left.cmp(right),
        (Value::Array(left), Value::Array(right)) => left
            .iter()
            .zip(right.iter())
            .map(|(left, right)| jq_compare(left, right))
            .find(|ordering| *ordering != std::cmp::Ordering::Equal)
            .unwrap_or_else(|| left.len().cmp(&right.len())),
        (Value::Object(left), Value::Object(right)) => jq_object_compare(left, right),
        _ => std::cmp::Ordering::Equal,
    }
}

fn jq_number_compare(left: &Number, right: &Number) -> std::cmp::Ordering {
    let left_value = left.as_f64();
    let right_value = right.as_f64();
    match (left_value.is_nan(), right_value.is_nan()) {
        (true, true) => std::cmp::Ordering::Equal,
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (false, false) if left_value.is_finite() && right_value.is_finite() => left.cmp(right),
        (false, false) => left_value.total_cmp(&right_value),
    }
}

fn jq_kind_rank(value: &Value) -> u8 {
    match value {
        Value::Null => 0,
        Value::Bool(false) => 1,
        Value::Bool(true) => 2,
        Value::Number(_) => 3,
        Value::String(_) => 4,
        Value::Array(_) => 5,
        Value::Object(_) => 6,
    }
}

fn jq_object_compare(left: &Object, right: &Object) -> std::cmp::Ordering {
    let mut left_keys = left.keys().collect::<Vec<_>>();
    let mut right_keys = right.keys().collect::<Vec<_>>();
    left_keys.sort_unstable();
    right_keys.sort_unstable();
    let key_order = left_keys
        .iter()
        .zip(right_keys.iter())
        .map(|(left, right)| left.cmp(right))
        .find(|ordering| *ordering != std::cmp::Ordering::Equal)
        .unwrap_or(std::cmp::Ordering::Equal);
    if key_order != std::cmp::Ordering::Equal {
        return key_order;
    }
    let length_order = left_keys.len().cmp(&right_keys.len());
    if length_order != std::cmp::Ordering::Equal {
        return length_order;
    }
    left_keys
        .iter()
        .zip(right_keys.iter())
        .map(|(left_key, right_key)| {
            jq_compare(
                left.get(left_key.as_ref()).expect("sorted key exists"),
                right.get(right_key.as_ref()).expect("sorted key exists"),
            )
        })
        .find(|ordering| *ordering != std::cmp::Ordering::Equal)
        .unwrap_or(std::cmp::Ordering::Equal)
}

pub(super) fn binary_add(left: &Value, right: &Value) -> Result<Value, VmError> {
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

pub(super) fn binary_subtract(left: &Value, right: &Value) -> Result<Value, VmError> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left
            .subtract(right)
            .map(Value::Number)
            .map_err(|error| runtime(error.to_string())),
        (Value::Array(left), Value::Array(right)) => Ok(Value::array(
            left.iter()
                .filter(|value| {
                    !right
                        .iter()
                        .any(|candidate| collection::jq_equal(candidate, value))
                })
                .cloned()
                .collect::<Vec<_>>(),
        )),
        _ => Err(runtime("subtraction requires numbers or arrays".to_owned())),
    }
}

pub(super) fn binary_multiply(left: &Value, right: &Value) -> Result<Value, VmError> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left
            .multiply(right)
            .map(Value::Number)
            .map_err(|error| runtime(error.to_string())),
        (Value::Object(left), Value::Object(right)) => {
            Ok(Value::object(merge_objects(left, right)))
        }
        _ => Err(runtime(format!(
            "cannot multiply {} and {}",
            type_name(left),
            type_name(right)
        ))),
    }
}

fn merge_objects(left: &Object, right: &Object) -> Object {
    let mut merged = left.clone();
    merge_into_object(&mut merged, right);
    merged
}

fn merge_into_object(merged: &mut Object, right: &Object) {
    for (key, right_value) in right {
        if let Some(left_value) = merged.get_mut(key) {
            match (left_value, right_value) {
                (Value::Object(left), Value::Object(right)) => {
                    merge_into_object(Arc::make_mut(left), right);
                }
                (left, right) => *left = right.clone(),
            }
        } else {
            merged.insert(Arc::clone(key), right_value.clone());
        }
    }
}

pub(super) fn numeric(
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

pub(super) fn number_value(number: f64) -> Outcomes {
    Number::from_f64(number)
        .map(Value::Number)
        .map(Ok)
        .map_or_else(
            |error| one_error(runtime(error.to_string())),
            |value| vec![value],
        )
}

pub(super) fn number_usize(number: usize) -> Value {
    Value::Number(Number::from_usize(number))
}

pub(super) fn length(value: &Value) -> Result<Value, VmError> {
    let length = match value {
        Value::Null => 0.0,
        Value::Number(number) => number.as_f64().abs(),
        Value::String(value) => return Ok(number_usize(value.chars().count())),
        Value::Array(value) => return Ok(number_usize(value.len())),
        Value::Object(value) => return Ok(number_usize(value.len())),
        Value::Bool(_) => return Err(type_error("length", value)),
    };
    Number::from_f64(length)
        .map(Value::Number)
        .map_err(|error| runtime(error.to_string()))
}

pub(super) fn absolute_value(value: &Value) -> Result<Value, VmError> {
    match value {
        Value::Number(number) if number.is_less_than_zero() => Ok(Value::Number(number.negate())),
        Value::Number(_) | Value::String(_) | Value::Array(_) | Value::Object(_) => {
            Ok(value.clone())
        }
        Value::Null | Value::Bool(_) => Err(type_error("negation", value)),
    }
}

pub(super) fn keys(value: &Value, sorted: bool) -> Result<Value, VmError> {
    let mut keys = match value {
        Value::Array(values) => (0..values.len()).map(number_usize).collect::<Vec<_>>(),
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

pub(super) fn has(container: &Value, key: &Value) -> Result<Value, VmError> {
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

pub(super) fn selector(input: &Value, selected: bool) -> Outcomes {
    if selected {
        vec![Ok(input.clone())]
    } else {
        Vec::new()
    }
}

pub(super) fn to_entries(input: &Value) -> Result<Value, VmError> {
    let entries = match input {
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                Value::object(Object::from_iter([
                    (Arc::from("key"), number_usize(index)),
                    (Arc::from("value"), value.clone()),
                ]))
            })
            .collect::<Vec<_>>(),
        Value::Object(values) => values
            .iter()
            .map(|(key, value)| {
                Value::object(Object::from_iter([
                    (Arc::from("key"), Value::String(Arc::clone(key))),
                    (Arc::from("value"), value.clone()),
                ]))
            })
            .collect(),
        value => return Err(type_error("to_entries", value)),
    };
    Ok(Value::array(entries))
}

pub(super) fn from_entries(entries: &[Value], operation: &str) -> Result<Value, VmError> {
    let mut object = Object::new();
    for entry in entries {
        let Value::Object(entry) = entry else {
            return Err(type_error(operation, entry));
        };
        let key = ["key", "Key", "name", "Name"]
            .into_iter()
            .filter_map(|name| entry.get(name))
            .find(|value| value.is_truthy())
            .ok_or_else(|| runtime("entry is missing a key".to_owned()))?;
        let value = entry
            .get("value")
            .or_else(|| entry.get("Value"))
            .cloned()
            .unwrap_or(Value::Null);
        let key = match key {
            Value::String(key) => Arc::clone(key),
            value => return Err(type_error(operation, value)),
        };
        object.insert(key, value);
    }
    Ok(Value::object(object))
}

pub(super) fn explode(input: &Value) -> Result<Value, VmError> {
    let Value::String(input) = input else {
        return Err(type_error("explode", input));
    };
    Ok(Value::array(
        input
            .chars()
            .map(|character| number_usize(character as usize))
            .collect::<Vec<_>>(),
    ))
}

pub(super) fn implode(input: &Value) -> Result<Value, VmError> {
    let Value::Array(input) = input else {
        return Err(type_error("implode", input));
    };
    let mut output = String::with_capacity(input.len());
    for value in input.iter() {
        let Value::Number(number) = value else {
            return Err(type_error("implode", value));
        };
        let codepoint = number
            .exact_index()
            .and_then(|value| u32::try_from(value).ok())
            .and_then(char::from_u32)
            .unwrap_or(char::REPLACEMENT_CHARACTER);
        output.push(codepoint);
    }
    Ok(Value::string(output))
}

pub(super) fn math_argument(operation: &str, value: &Value) -> Result<f64, VmError> {
    match value {
        Value::Number(number) => Ok(number.as_f64()),
        value => Err(type_error(operation, value)),
    }
}

pub(super) fn runtime_number(value: f64) -> Value {
    Value::Number(Number::from_runtime_f64(value))
}

pub(super) fn math_result_value(result: math::MathResult) -> Value {
    match result {
        math::MathResult::Scalar(value) => runtime_number(value),
        math::MathResult::Pair(values) => {
            Value::array(values.into_iter().map(runtime_number).collect::<Vec<_>>())
        }
    }
}

pub(super) fn regex_pattern_argument(
    name: &str,
    value: &Value,
    array_allowed: bool,
) -> Result<(Arc<str>, Option<Arc<str>>), VmError> {
    match value {
        Value::String(pattern) => Ok((Arc::clone(pattern), None)),
        Value::Array(values) if array_allowed => {
            let Some(Value::String(pattern)) = values.first() else {
                return Err(type_error(name, value));
            };
            let flags = match values.get(1) {
                None | Some(Value::Null) => Arc::from(""),
                Some(Value::String(flags)) => Arc::clone(flags),
                Some(value) => return Err(type_error(name, value)),
            };
            Ok((Arc::clone(pattern), Some(flags)))
        }
        _ => Err(type_error(name, value)),
    }
}

pub(super) fn math_error(name: &str, error: &math::MathError) -> VmError {
    let message = error.to_string();
    match error {
        math::MathError::UnknownFunction => VmError::Unsupported {
            operation: format!("math builtin {name}").into(),
        },
        math::MathError::InvalidOrder { .. } => resource("math-bessel-order"),
        math::MathError::Arity { .. } => runtime(message),
    }
}

pub(super) fn numeric_predicate(name: &str, input: &Value) -> Result<Value, VmError> {
    let Value::Number(number) = input else {
        return Ok(Value::Bool(false));
    };
    let value = number.as_f64();
    let result = match name {
        "isnan" => value.is_nan(),
        "isinfinite" => value.is_infinite(),
        // jq's decNumber predicate treats NaN as finite; only infinities
        // are non-finite in the public filter contract.
        "isfinite" => !value.is_infinite(),
        "isnormal" => value.is_normal(),
        _ => {
            return Err(VmError::Unsupported {
                operation: format!("numeric predicate {name}").into(),
            });
        }
    };
    Ok(Value::Bool(result))
}

pub(super) fn fromjson(
    input: &Value,
    limits: VmLimits,
    checkpoint: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let Value::String(input) = input else {
        return Err(type_error("fromjson", input));
    };
    if input.len() > limits.output_bytes {
        return Err(resource("output-bytes"));
    }
    let mut reader = JsonInput::new(
        io::Cursor::new(input.as_bytes()),
        JsonInputOptions {
            maximum_depth: limits.json_depth,
            maximum_token_bytes: limits.json_token_bytes,
        },
    );
    let mut checkpoint_error = None;
    let mut io_checkpoint = || match checkpoint() {
        Ok(()) => Ok(()),
        Err(error) => {
            checkpoint_error = Some(error);
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "evaluator checkpoint failed",
            ))
        }
    };
    let parsed = reader
        .next_value(&mut io_checkpoint)
        .and_then(|value| {
            value.ok_or_else(|| JsonInputError::Syntax {
                position: reader.position(),
                message: "expected JSON value".into(),
            })
        })
        .and_then(|value| reader.check_end(&mut io_checkpoint).map(|()| value));
    if let Some(error) = checkpoint_error {
        return Err(error);
    }
    let value = parsed.map_err(|error| match error {
        JsonInputError::Limit { limit, .. } => match limit {
            crate::JsonLimit::Depth => resource("depth"),
            crate::JsonLimit::TokenBytes => resource("token-bytes"),
        },
        error => runtime(format!("invalid JSON: {error}")),
    })?;
    Ok(value)
}
