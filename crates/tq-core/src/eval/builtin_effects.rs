//! Shared predefined values, diagnostic effects, and module metadata.

use super::ordered::{extrema, fold_values, reverse, sort_values, unique_values};
use super::value_ops::{
    binary_add, from_entries, keys, length, number_usize, to_entries, type_error, type_name,
};
use super::{invalid, resource, runtime};
use crate::vm::EffectState;
use crate::{BuiltinRegistry, Bytecode, Value, VmError, format, parse_jq_number, string_compat};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) fn generator_builtin(
    name: &str,
    input: &Value,
    output_limit: usize,
) -> Result<Option<Value>, VmError> {
    let selected = match name {
        "builtins" => return Ok(Some(builtin_signatures())),
        "not" => return Ok(Some(Value::Bool(!input.is_truthy()))),
        "add" => {
            return fold_values(input, None, binary_add)
                .into_iter()
                .next()
                .transpose();
        }
        "arrays" => matches!(input, Value::Array(_)),
        "booleans" => matches!(input, Value::Bool(_)),
        "finites" => matches!(input, Value::Number(number) if !number.as_f64().is_infinite()),
        "iterables" => matches!(input, Value::Array(_) | Value::Object(_)),
        "nulls" => matches!(input, Value::Null),
        "normals" => matches!(input, Value::Number(number) if number.as_f64().is_normal()),
        "numbers" => matches!(input, Value::Number(_)),
        "objects" => matches!(input, Value::Object(_)),
        "scalars" => !matches!(input, Value::Array(_) | Value::Object(_)),
        "strings" => matches!(input, Value::String(_)),
        "values" => !matches!(input, Value::Null),
        "keys" | "keys_unsorted" => return keys(input, name == "keys").map(Some),
        "to_entries" => return to_entries(input).map(Some),
        "from_entries" => match input {
            Value::Array(entries) => return from_entries(entries, "from_entries").map(Some),
            value => return Err(type_error("from_entries", value)),
        },
        "length" => return length(input).map(Some),
        "max" => return extrema(input, true).into_iter().next().transpose(),
        "min" => return extrema(input, false).into_iter().next().transpose(),
        "reverse" => return reverse(input).into_iter().next().transpose(),
        "sort" => return sort_values(input).into_iter().next().transpose(),
        "tonumber" => match input {
            Value::Number(_) => return Ok(Some(input.clone())),
            Value::String(value) => {
                return parse_jq_number(value)
                    .map(Value::Number)
                    .map(Some)
                    .map_err(|error| runtime(error.to_string()));
            }
            value => return Err(type_error("tonumber", value)),
        },
        "tostring" => {
            return format::text(input, output_limit)
                .map(Value::String)
                .map(Some);
        }
        "@urid" => return string_compat::urid(input, output_limit).map(Some),
        name if name.starts_with('@') => return format::apply(name, input, output_limit).map(Some),
        "type" => return Ok(Some(Value::string(type_name(input)))),
        "unique" => return unique_values(input).into_iter().next().transpose(),
        "utf8bytelength" => match input {
            Value::String(value) => return Ok(Some(number_usize(value.len()))),
            value => return Err(type_error("utf8bytelength", value)),
        },
        "trim" => return string_compat::trim(input).map(Some),
        "ltrim" => return string_compat::ltrim(input).map(Some),
        "rtrim" => return string_compat::rtrim(input).map(Some),
        "toboolean" => return string_compat::toboolean(input).map(Some),
        "ascii_upcase" => return string_compat::ascii_upcase(input).map(Some),
        "ascii_downcase" => return string_compat::ascii_downcase(input).map(Some),
        _ => return Err(invalid("generator built-in left admitted subset")),
    };
    Ok(selected.then(|| input.clone()))
}

pub(super) fn append_effect(
    effects: &Arc<EffectState>,
    bytes: &[u8],
    limit: usize,
) -> Result<(), VmError> {
    effects.append(bytes, limit)
}

pub(super) fn debug_effect(value: &Value, output_limit: usize) -> Result<Vec<u8>, VmError> {
    const PREFIX: &str = "[\"DEBUG:\",";
    const SUFFIX: &str = "]\n";
    let overhead = PREFIX.len().saturating_add(SUFFIX.len());
    if output_limit < overhead {
        return Err(resource("output-bytes"));
    }
    let Value::String(encoded) = format::apply("@json", value, output_limit - overhead)? else {
        return Err(invalid("JSON debug encoding did not produce a string"));
    };
    let total = overhead.saturating_add(encoded.len());
    let mut output = String::new();
    output
        .try_reserve_exact(total)
        .map_err(|_| resource("output-bytes"))?;
    output.push_str(PREFIX);
    output.push_str(&encoded);
    output.push_str(SUFFIX);
    Ok(output.into_bytes())
}

pub(super) fn halt_error_text(value: &Value, output_limit: usize) -> Result<Arc<str>, VmError> {
    if matches!(value, Value::Null) {
        return Ok(Arc::from(""));
    }
    let text = format::text(value, output_limit)?;
    if matches!(value, Value::String(_)) {
        return Ok(text);
    }
    if text.len() >= output_limit {
        return Err(resource("output-bytes"));
    }
    Ok(Arc::from(format!("{text}\n")))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "jq truncates numeric halt status modulo 256"
)]
pub(super) fn halt_status(value: &Value) -> Result<u8, VmError> {
    let Value::Number(number) = value else {
        return Err(type_error("halt_error", value));
    };
    let value = number.as_f64();
    if value.is_nan() || value <= 0.0 {
        return Ok(0);
    }
    if value.is_infinite() {
        return Ok(u8::MAX);
    }
    let status = value.trunc() % 256.0;
    Ok(status as u8)
}

pub(super) fn builtin_signatures() -> Value {
    let signatures = BuiltinRegistry
        .all()
        .iter()
        .flat_map(|builtin| {
            let name = builtin.name;
            (builtin.minimum_arity..=builtin.maximum_arity)
                .map(move |arity| Value::string(format!("{name}/{arity}")))
        })
        .collect::<Vec<_>>();
    Value::array(signatures)
}

pub(super) fn module_metadata(
    bytecode: &Bytecode,
    input: &Value,
    cancellation: Option<&AtomicBool>,
    stop: &AtomicBool,
) -> Result<Value, VmError> {
    if stop.load(Ordering::Relaxed) || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
    {
        return Err(VmError::Interrupted);
    }
    let Value::String(requested) = input else {
        return Err(type_error("modulemeta", input));
    };
    if let Some(module) = bytecode
        .modules()
        .iter()
        .find(|module| module.name == requested.as_ref())
    {
        return Ok(module.metadata.clone());
    }
    let metadata = bytecode
        .load_dynamic_module_metadata(requested)
        .map(|module| module.metadata)
        .map_err(|error| runtime(error.to_string()))?;
    if stop.load(Ordering::Relaxed) || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
    {
        return Err(VmError::Interrupted);
    }
    Ok(metadata)
}
