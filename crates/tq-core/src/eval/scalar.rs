//! Scalar built-ins that do not need bytecode, effects, or a live input cursor.
//!
//! The managed evaluator uses this seam when all filter arguments have already
//! produced concrete values.  Operations that can branch, suspend, invoke a
//! filter argument, or consume input stay in the evaluator dispatcher.

use std::{cell::RefCell, cmp::Ordering, sync::Arc};

use crate::{
    Number, Object, Path, PathComponent, Value, VmError, VmLimits, collection, format, math,
    parse_jq_number, stdlib, string_compat,
};

use super::{
    absolute_value, builtin_signatures, compare_paths_for_deletion, extrema, fromjson, has,
    jq_compare, length, math_argument, math_error, math_result_value, number_usize,
    numeric_predicate, path_component, resource, runtime, runtime_number, stable_sort_values,
    type_error, type_name,
};

const MATH_UNARY: &[&str] = &[
    "acos",
    "acosh",
    "asin",
    "asinh",
    "atan",
    "atanh",
    "cbrt",
    "ceil",
    "cos",
    "cosh",
    "erf",
    "erfc",
    "exp",
    "exp10",
    "exp2",
    "expm1",
    "fabs",
    "floor",
    "frexp",
    "gamma",
    "j0",
    "j1",
    "lgamma",
    "log",
    "log10",
    "log1p",
    "log2",
    "logb",
    "modf",
    "nearbyint",
    "rint",
    "round",
    "significand",
    "sin",
    "sinh",
    "sqrt",
    "tan",
    "tanh",
    "tgamma",
    "trunc",
    "y0",
    "y1",
];

const MATH_BINARY: &[&str] = &[
    "atan2",
    "copysign",
    "drem",
    "fdim",
    "fmax",
    "fmin",
    "fmod",
    "hypot",
    "jn",
    "ldexp",
    "nextafter",
    "nexttoward",
    "pow",
    "remainder",
    "scalb",
    "scalbln",
    "yn",
];

/// Returns whether a call is a concrete scalar operation.
///
/// `arity` is the number of explicit, already-evaluated filter arguments.
/// Zero-arity math functions consume `input`; binary and ternary functions
/// consume their explicit argument values.
pub(super) fn supports(name: &str, arity: usize) -> bool {
    if format::is_supported(name) || name == "@urid" {
        return arity == 0;
    }
    if MATH_UNARY.contains(&name) {
        return arity == 0;
    }
    if MATH_BINARY.contains(&name) {
        return arity == 2;
    }
    if name == "fma" {
        return arity == 3;
    }
    match name {
        "type"
        | "length"
        | "abs"
        | "utf8bytelength"
        | "keys"
        | "keys_unsorted"
        | "builtins"
        | "not"
        | "arrays"
        | "booleans"
        | "finites"
        | "iterables"
        | "nulls"
        | "normals"
        | "numbers"
        | "objects"
        | "scalars"
        | "strings"
        | "values"
        | "nan"
        | "infinite"
        | "have_decnum"
        | "have_literal_numbers"
        | "isnan"
        | "isinfinite"
        | "isfinite"
        | "isnormal"
        | "to_entries"
        | "from_entries"
        | "tonumber"
        | "tostring"
        | "tojson"
        | "fromjson"
        | "trim"
        | "ltrim"
        | "rtrim"
        | "toboolean"
        | "ascii_upcase"
        | "ascii_downcase"
        | "explode"
        | "implode"
        | "min"
        | "max"
        | "sort"
        | "reverse"
        | "unique"
        | "add"
        | "fromdate"
        | "fromdateiso8601"
        | "todate"
        | "todateiso8601"
        | "gmtime"
        | "mktime"
        | "transpose" => arity == 0,
        "flatten" => arity <= 1,
        "contains" | "inside" | "indices" | "index" | "rindex" | "has" | "in" | "ltrimstr"
        | "rtrimstr" | "trimstr" | "startswith" | "endswith" | "join" | "strptime" | "strftime"
        | "getpath" | "delpaths" | "bsearch" => arity == 1,
        "setpath" => arity == 2,
        _ => false,
    }
}

/// Returns whether an ambient scalar builtin can be dispatched after its
/// filter arguments have been materialized.  Input-cursor ambient builtins
/// remain in the evaluator dispatcher because they need source context.
pub(super) fn supports_ambient(name: &str, arity: usize) -> bool {
    match name {
        "env" | "now" | "localtime" => arity == 0,
        "strflocaltime" => arity == 1,
        _ => false,
    }
}

/// Evaluates an ambient builtin without reading a cursor or writing effects.
/// The dispatcher supplies the reserved capability bindings in `environment`;
/// missing or false platform admission remains a deterministic runtime error.
pub(super) fn evaluate_ambient(
    name: &str,
    input: &Value,
    arguments: &[Value],
    environment: &super::Environment,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Option<Value>, VmError> {
    if !supports_ambient(name, arguments.len()) {
        return Err(super::invalid(
            "unsupported ambient scalar builtin or arity",
        ));
    }
    charge()?;
    let value = match name {
        "env" => super::ambient_environment(environment, "env")?,
        "now" => stdlib::now(super::ambient_platform(environment))?,
        "localtime" => stdlib::localtime(input, super::ambient_platform(environment))?,
        "strflocaltime" => {
            let Some(Value::String(format)) = arguments.first() else {
                return Err(type_error(
                    "strflocaltime",
                    arguments.first().unwrap_or(&Value::Null),
                ));
            };
            stdlib::strflocaltime(
                input,
                format,
                super::ambient_platform(environment),
                limits.output_bytes,
            )?
        }
        _ => return Err(super::invalid("unsupported ambient scalar builtin")),
    };
    Ok(Some(value))
}

/// Evaluates one already-materialized scalar call.
///
/// `Ok(None)` means that an admitted selector intentionally emitted no value.
/// Unsupported operations and wrong arities are invalid dispatcher state; the
/// managed dispatcher must check [`supports`] before calling this seam.
#[allow(
    clippy::too_many_lines,
    reason = "scalar dispatch keeps the bounded builtin contract in one table"
)]
pub(super) fn evaluate(
    name: &str,
    input: &Value,
    arguments: &[Value],
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Option<Value>, VmError> {
    if !supports(name, arguments.len()) {
        return Err(super::invalid("unsupported scalar builtin or arity"));
    }
    charge()?;

    match name {
        "type" => Ok(Some(Value::string(type_name(input)))),
        "length" => length(input).map(Some),
        "abs" => absolute_value(input).map(Some),
        "utf8bytelength" => match input {
            Value::String(value) => number_usize(value.len()).map(Some),
            value => Err(type_error("utf8bytelength", value)),
        },
        "keys" => bounded_keys(input, true, limits, charge).map(Some),
        "keys_unsorted" => bounded_keys(input, false, limits, charge).map(Some),
        "builtins" => Ok(Some(builtin_signatures())),
        "not" => Ok(Some(Value::Bool(!input.is_truthy()))),
        "arrays" => selected(input, matches!(input, Value::Array(_))),
        "booleans" => selected(input, matches!(input, Value::Bool(_))),
        "finites" => selected(
            input,
            matches!(input, Value::Number(number) if !number.as_f64().is_infinite()),
        ),
        "iterables" => selected(input, matches!(input, Value::Array(_) | Value::Object(_))),
        "nulls" => selected(input, matches!(input, Value::Null)),
        "normals" => selected(
            input,
            matches!(input, Value::Number(number) if number.as_f64().is_normal()),
        ),
        "numbers" => selected(input, matches!(input, Value::Number(_))),
        "objects" => selected(input, matches!(input, Value::Object(_))),
        "scalars" => selected(input, !matches!(input, Value::Array(_) | Value::Object(_))),
        "strings" => selected(input, matches!(input, Value::String(_))),
        "values" => selected(input, !matches!(input, Value::Null)),
        "nan" => Ok(Some(runtime_number(f64::NAN))),
        "infinite" => Ok(Some(runtime_number(f64::INFINITY))),
        "have_decnum" | "have_literal_numbers" => Ok(Some(Value::Bool(true))),
        "isnan" | "isinfinite" | "isfinite" | "isnormal" => {
            numeric_predicate(name, input).map(Some)
        }
        "contains" => unary_argument(name, arguments, |needle| {
            collection_contains(charge, input, needle)
        }),
        "inside" => unary_argument(name, arguments, |container| {
            collection_inside(charge, input, container)
        }),
        "indices" => unary_argument(name, arguments, |needle| {
            collection_indices(charge, input, needle)
        }),
        "index" => unary_argument(name, arguments, |needle| {
            collection_index(charge, input, needle, false)
        }),
        "rindex" => unary_argument(name, arguments, |needle| {
            collection_index(charge, input, needle, true)
        }),
        "has" => unary_argument(name, arguments, |key| has(input, key)),
        "in" => unary_argument(name, arguments, |container| has(container, input)),
        "ltrimstr" => unary_argument(name, arguments, |prefix| {
            string_compat::ltrimstr(input, prefix)
        }),
        "rtrimstr" => unary_argument(name, arguments, |suffix| {
            string_compat::rtrimstr(input, suffix)
        }),
        "trimstr" => unary_argument(name, arguments, |affix| {
            string_compat::trimstr(input, affix)
        }),
        "startswith" => unary_argument(name, arguments, |prefix| {
            string_compat::startswith(input, prefix)
        }),
        "endswith" => unary_argument(name, arguments, |suffix| {
            string_compat::endswith(input, suffix)
        }),
        "join" => unary_argument(name, arguments, |separator| {
            string_compat::join(input, separator, limits.output_bytes)
        }),
        "getpath" => unary_argument(name, arguments, |path_value| {
            bounded_getpath_resolution(input, path_value, limits, charge)
                .map(|resolution| resolution.value)
        }),
        "setpath" => {
            let path = bounded_setpath_path(
                input,
                arguments.first().expect("setpath arity checked"),
                limits,
                charge,
            )?;
            super::path::replace_or_create_bounded(
                input,
                &path,
                arguments[1].clone(),
                limits,
                charge,
            )
            .map(Some)
        }
        "delpaths" => bounded_delpaths(input, &arguments[0], limits, charge).map(Some),
        "transpose" => transpose_value(input, limits, charge).map(Some),
        "bsearch" => unary_argument(name, arguments, |needle| {
            bsearch_value(input, needle, limits, charge)
        }),
        "strptime" => unary_argument(name, arguments, |format| match format {
            Value::String(format) => stdlib::strptime(input, format),
            value => Err(type_error(name, value)),
        }),
        "strftime" => unary_argument(name, arguments, |format| match format {
            Value::String(format) => stdlib::strftime(input, format, limits.output_bytes),
            value => Err(type_error(name, value)),
        }),
        "to_entries" => bounded_to_entries(input, limits, charge).map(Some),
        "from_entries" => match input {
            Value::Array(entries) => bounded_from_entries(entries, name, limits, charge).map(Some),
            value => Err(type_error(name, value)),
        },
        "tonumber" => tonumber(input),
        "tostring" => format::text(input, limits.output_bytes)
            .map(Value::String)
            .map(Some),
        "tojson" => format::bounded_json(input, limits.output_bytes)
            .map(Value::string)
            .map(Some),
        "fromjson" => fromjson(input, limits, charge).map(Some),
        "@urid" => string_compat::urid(input, limits.output_bytes).map(Some),
        name if format::is_supported(name) => {
            format::apply(name, input, limits.output_bytes).map(Some)
        }
        "trim" => string_compat::trim(input).map(Some),
        "ltrim" => string_compat::ltrim(input).map(Some),
        "rtrim" => string_compat::rtrim(input).map(Some),
        "toboolean" => string_compat::toboolean(input).map(Some),
        "ascii_upcase" => string_compat::ascii_upcase(input).map(Some),
        "ascii_downcase" => string_compat::ascii_downcase(input).map(Some),
        "explode" => bounded_explode(input, limits, charge).map(Some),
        "implode" => bounded_implode(input, limits, charge).map(Some),
        "min" => {
            charge_array_items(input, charge)?;
            single(extrema(input, false))
        }
        "max" => {
            charge_array_items(input, charge)?;
            single(extrema(input, true))
        }
        "sort" => bounded_sort(input, limits, charge).map(Some),
        "reverse" => bounded_reverse(input, limits, charge).map(Some),
        "unique" => bounded_unique(input, limits, charge).map(Some),
        "add" => bounded_add(input, limits, charge).map(Some),
        "flatten" => {
            let depth = match arguments.first() {
                None => None,
                Some(Value::Number(number)) => match number.exact_index() {
                    Some(depth) => Some(depth),
                    None if number.as_f64().is_sign_negative() => {
                        return Err(runtime("flatten depth must not be negative".to_owned()));
                    }
                    None => None,
                },
                Some(value) => return Err(type_error(name, value)),
            };
            bounded_flatten(input, depth, limits, charge).map(Some)
        }
        "fromdate" | "fromdateiso8601" => stdlib::fromdate_iso8601(input).map(Some),
        "todate" | "todateiso8601" => stdlib::todate_iso8601(input, limits.output_bytes).map(Some),
        "gmtime" => stdlib::gmtime(input).map(Some),
        "mktime" => stdlib::mktime(input).map(Some),
        name if MATH_UNARY.contains(&name) || MATH_BINARY.contains(&name) || name == "fma" => {
            evaluate_math(name, input, arguments, charge)
        }
        _ => Err(super::invalid("unsupported scalar builtin")),
    }
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "the Result shape matches the scalar dispatcher arms"
)]
fn selected(input: &Value, selected: bool) -> Result<Option<Value>, VmError> {
    Ok(selected.then(|| input.clone()))
}

fn unary_argument(
    _name: &str,
    arguments: &[Value],
    apply: impl FnOnce(&Value) -> Result<Value, VmError>,
) -> Result<Option<Value>, VmError> {
    let Some(argument) = arguments.first() else {
        return Err(super::invalid("builtin argument missing"));
    };
    apply(argument).map(Some)
}

fn bounded_delpaths(
    input: &Value,
    paths_value: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let Value::Array(path_values) = paths_value else {
        return Err(type_error("delpaths", paths_value));
    };

    let mut paths = Vec::new();
    for path_value in path_values.iter() {
        let path = bounded_delete_path(input, path_value, limits, charge)?;
        paths.try_reserve(1).map_err(|_| resource("path-stack"))?;
        paths.push(Path::new(path));
    }
    paths.sort_by(compare_paths_for_deletion);
    paths.dedup_by(|left, right| left == right);

    let mut deleted = input.clone();
    for path in paths {
        deleted = super::path::delete_path_bounded(&deleted, path.components(), limits, charge)?;
    }
    Ok(deleted)
}

pub(super) fn bounded_path(
    value: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Vec<PathComponent>, VmError> {
    let Value::Array(components) = value else {
        return Err(type_error("path", value));
    };
    if components.len() > limits.path_stack {
        return Err(resource("path-stack"));
    }
    charge()?;
    let mut path = Vec::new();
    path.try_reserve_exact(components.len())
        .map_err(|_| resource("path-stack"))?;
    for component in components.iter() {
        charge()?;
        path.push(path_component(component)?);
    }
    Ok(path)
}

#[derive(Clone, Copy)]
enum PathMode {
    Set,
    Delete,
}

/// The result of a scalar `getpath` read.
///
/// A path can be absent even when the read is a valid `null`: negative
/// indices outside an array and non-finite indices are not assignable jq
/// paths.  Keeping that distinction explicit prevents a synthetic sentinel
/// index from escaping into origin reconstruction and attempting a huge
/// allocation.  Ordinary missing keys and positive out-of-range indices keep
/// their concrete path, because jq can use those paths for a later update.
#[derive(Clone, Debug)]
pub(super) struct GetPathResolution {
    pub(super) value: Value,
    pub(super) path: Option<Path>,
}

pub(super) fn bounded_getpath_resolution(
    root: &Value,
    value: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<GetPathResolution, VmError> {
    let Value::Array(raw_components) = value else {
        return Err(type_error("path", value));
    };
    if raw_components.len() > limits.path_stack {
        return Err(resource("path-stack"));
    }
    charge()?;
    let mut components = Vec::new();
    components
        .try_reserve_exact(raw_components.len())
        .map_err(|_| resource("path-stack"))?;
    let null = Value::Null;
    let mut target = root;
    let mut assignable = true;

    for raw_component in raw_components.iter() {
        charge()?;
        if !assignable {
            validate_missing_component(raw_component)?;
            target = &null;
            continue;
        }
        let Some(component) = read_target_component(target, raw_component)? else {
            assignable = false;
            target = &null;
            continue;
        };
        let (next, _missing) = target_child(target, &component, &null);
        components.push(component);
        target = next;
    }

    let path = assignable.then(|| Path::new(components));
    let value = match path.as_ref() {
        Some(path) => super::path::getpath_bounded(root, path.components(), limits, charge)?,
        None => Value::Null,
    };
    Ok(GetPathResolution { value, path })
}

fn bounded_setpath_path(
    root: &Value,
    value: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Vec<PathComponent>, VmError> {
    bounded_target_path(root, value, PathMode::Set, limits, charge)
}

fn bounded_delete_path(
    root: &Value,
    value: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Vec<PathComponent>, VmError> {
    bounded_target_path(root, value, PathMode::Delete, limits, charge)
}

fn bounded_target_path(
    root: &Value,
    value: &Value,
    mode: PathMode,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Vec<PathComponent>, VmError> {
    let Value::Array(raw_components) = value else {
        return Err(type_error("path", value));
    };
    if raw_components.len() > limits.path_stack {
        return Err(resource("path-stack"));
    }
    charge()?;
    let mut components = Vec::new();
    components
        .try_reserve_exact(raw_components.len())
        .map_err(|_| resource("path-stack"))?;
    let null = Value::Null;
    let mut target = root;
    for raw_component in raw_components.iter() {
        charge()?;
        let (component, missing) = target_component(target, raw_component, mode)?;
        components.push(component.clone());
        let (next, next_missing) = target_child(target, &component, &null);
        target = next;
        if matches!(mode, PathMode::Delete) && (missing || next_missing) {
            break;
        }
    }
    Ok(components)
}

fn target_component(
    target: &Value,
    value: &Value,
    mode: PathMode,
) -> Result<(PathComponent, bool), VmError> {
    match value {
        Value::String(key) => Ok((PathComponent::Key(Arc::clone(key)), false)),
        Value::Number(number) => {
            let Some(index) = truncated_path_index(number) else {
                return if matches!(mode, PathMode::Set) {
                    Err(runtime("path index is not finite or in range".to_owned()))
                } else {
                    Ok((PathComponent::Index(usize::MAX), true))
                };
            };
            if index >= 0 {
                return Ok((
                    PathComponent::Index(usize::try_from(index).unwrap_or(usize::MAX)),
                    false,
                ));
            }
            match target {
                Value::Array(values) => match super::normalize_index(index, values.len()) {
                    Some(index) => Ok((PathComponent::Index(index), false)),
                    None if matches!(mode, PathMode::Set) => {
                        Err(runtime("out of bounds negative array index".to_owned()))
                    }
                    None => Ok((PathComponent::Index(usize::MAX), true)),
                },
                Value::Null if matches!(mode, PathMode::Set) => {
                    Err(runtime("out of bounds negative array index".to_owned()))
                }
                Value::Null => Ok((PathComponent::Index(usize::MAX), true)),
                _ => Ok((PathComponent::Index(usize::MAX), false)),
            }
        }
        value => Err(type_error("path component", value)),
    }
}

fn read_target_component(target: &Value, value: &Value) -> Result<Option<PathComponent>, VmError> {
    match value {
        Value::String(key) => match target {
            Value::Object(_) | Value::Null => Ok(Some(PathComponent::Key(Arc::clone(key)))),
            value => Err(type_error("index", value)),
        },
        Value::Number(number) => {
            if !matches!(target, Value::Array(_) | Value::Null) {
                return Err(type_error("index", target));
            }
            let Some(index) = truncated_path_index(number) else {
                return Ok(None);
            };
            if index >= 0 {
                return Ok(Some(PathComponent::Index(usize::try_from(index).map_err(
                    |_| runtime("path index is not finite or in range".to_owned()),
                )?)));
            }
            match target {
                Value::Array(values) => {
                    Ok(super::normalize_index(index, values.len()).map(PathComponent::Index))
                }
                Value::Null => Ok(None),
                value => Err(type_error("index", value)),
            }
        }
        value => Err(type_error("path component", value)),
    }
}

fn validate_missing_component(value: &Value) -> Result<(), VmError> {
    match value {
        Value::String(_) | Value::Number(_) => Ok(()),
        value => Err(type_error("path component", value)),
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "the finite integer range check intentionally compares against jq's f64 number"
)]
fn truncated_path_index(number: &Number) -> Option<i64> {
    let value = number.as_f64();
    if !value.is_finite() {
        return None;
    }
    let value = value.trunc();
    if value < i64::MIN as f64 || value > i64::MAX as f64 {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    Some(value as i64)
}

fn target_child<'a>(
    target: &'a Value,
    component: &PathComponent,
    null: &'a Value,
) -> (&'a Value, bool) {
    match (target, component) {
        (Value::Object(values), PathComponent::Key(key)) => {
            values.get(key).map_or((null, true), |value| (value, false))
        }
        (Value::Array(values), PathComponent::Index(index)) => values
            .get(*index)
            .map_or((null, true), |value| (value, false)),
        (Value::Null, _) => (null, true),
        _ => (target, false),
    }
}

fn tonumber(input: &Value) -> Result<Option<Value>, VmError> {
    match input {
        Value::Number(_) => Ok(Some(input.clone())),
        Value::String(value) => parse_jq_number(value)
            .map(Value::Number)
            .map(Some)
            .map_err(|error| runtime(error.to_string())),
        value => Err(type_error("tonumber", value)),
    }
}

fn evaluate_math(
    name: &str,
    input: &Value,
    arguments: &[Value],
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Option<Value>, VmError> {
    let values = if arguments.is_empty() {
        vec![math_argument(name, input)?]
    } else {
        arguments
            .iter()
            .map(|value| math_argument(name, value))
            .collect::<Result<Vec<_>, _>>()?
    };
    if matches!(name, "jn" | "yn")
        && let Some(order) = values.first().copied()
        && order.is_finite()
        && order.abs() <= 1_024.0
    {
        let mut remaining = order.abs().trunc();
        while remaining >= 1.0 {
            charge()?;
            remaining -= 1.0;
        }
    }
    math::evaluate(name, &values)
        .map(math_result_value)
        .map(Some)
        .map_err(|error| math_error(name, &error))
}

/// Transposes an array of arrays with jq's null padding for ragged rows.
///
/// The evaluator's legacy method should delegate here after it has evaluated
/// any filter arguments; keeping the value operation in one place prevents
/// optimized and ordinary execution from drifting.
pub(super) fn transpose_value(
    input: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let Value::Array(rows) = input else {
        return Err(type_error("transpose", input));
    };
    if rows.len() > limits.value_stack {
        return Err(resource("value-stack"));
    }
    let mut row_values = Vec::new();
    if row_values.try_reserve(rows.len()).is_err() {
        return Err(resource("value-stack"));
    }
    let mut width = 0usize;
    for row in rows.iter() {
        charge()?;
        let Value::Array(values) = row else {
            return Err(type_error("transpose", row));
        };
        if values.len() > limits.value_stack {
            return Err(resource("value-stack"));
        }
        width = width.max(values.len());
        row_values.push(values.as_ref());
    }
    if width > limits.value_stack {
        return Err(resource("value-stack"));
    }
    let mut columns = Vec::new();
    if columns.try_reserve(width).is_err() {
        return Err(resource("value-stack"));
    }
    for index in 0..width {
        let mut column = Vec::new();
        if column.try_reserve(row_values.len()).is_err() {
            return Err(resource("value-stack"));
        }
        for row in &row_values {
            charge()?;
            column.push(row.get(index).cloned().unwrap_or(Value::Null));
        }
        columns.push(Value::array(column));
    }
    Ok(Value::array(columns))
}

/// Performs jq's sorted-array binary search and insertion-position encoding.
pub(super) fn bsearch_value(
    input: &Value,
    needle: &Value,
    _limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let Value::Array(values) = input else {
        return Err(type_error("bsearch", input));
    };
    let mut low = 0usize;
    let mut high = values.len();
    let mut found = None;
    while low < high {
        charge()?;
        let middle = low + (high - low) / 2;
        match jq_compare(&values[middle], needle) {
            Ordering::Less => low = middle.saturating_add(1),
            Ordering::Greater => high = middle,
            Ordering::Equal => {
                found = Some(middle);
                break;
            }
        }
    }
    let index = found.map_or_else(
        || {
            i64::try_from(low)
                .unwrap_or(i64::MAX)
                .saturating_neg()
                .saturating_sub(1)
        },
        |index| i64::try_from(index).unwrap_or(i64::MAX),
    );
    Number::parse(&index.to_string())
        .map(Value::Number)
        .map_err(|error| runtime(error.to_string()))
}

fn single(outcomes: Vec<Result<Value, VmError>>) -> Result<Option<Value>, VmError> {
    outcomes.into_iter().next().transpose()
}

pub(super) fn bounded_keys(
    input: &Value,
    sorted: bool,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let count = match input {
        Value::Array(values) => values.len(),
        Value::Object(values) => values.len(),
        value => return Err(type_error("keys", value)),
    };
    ensure_container_lower_bound(count, limits.output_bytes)?;
    let mut output = Vec::new();
    output
        .try_reserve(count)
        .map_err(|_| resource("output-bytes"))?;
    match input {
        Value::Array(values) => {
            for index in 0..values.len() {
                charge()?;
                output.push(number_usize(index)?);
            }
        }
        Value::Object(values) => {
            for key in values.keys() {
                charge()?;
                output.push(Value::String(Arc::clone(key)));
            }
        }
        _ => unreachable!("input kind checked above"),
    }
    if sorted {
        stable_sort_values(&mut output);
    }
    Ok(Value::array(output))
}

pub(super) fn bounded_to_entries(
    input: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let count = match input {
        Value::Array(values) => values.len(),
        Value::Object(values) => values.len(),
        value => return Err(type_error("to_entries", value)),
    };
    ensure_container_lower_bound(count, limits.output_bytes)?;
    let mut entries = Vec::new();
    entries
        .try_reserve(count)
        .map_err(|_| resource("output-bytes"))?;
    match input {
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                charge()?;
                let mut entry = Object::new();
                entry.insert(Arc::from("key"), number_usize(index)?);
                entry.insert(Arc::from("value"), value.clone());
                entries.push(Value::object(entry));
            }
        }
        Value::Object(values) => {
            for (key, value) in values.iter() {
                charge()?;
                let mut entry = Object::new();
                entry.insert(Arc::from("key"), Value::String(Arc::clone(key)));
                entry.insert(Arc::from("value"), value.clone());
                entries.push(Value::object(entry));
            }
        }
        _ => unreachable!("input kind checked above"),
    }
    Ok(Value::array(entries))
}

pub(super) fn bounded_explode(
    input: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let Value::String(input) = input else {
        return Err(type_error("explode", input));
    };
    let count = input.chars().count();
    ensure_container_lower_bound(count, limits.output_bytes)?;
    let mut output = Vec::new();
    output
        .try_reserve(count)
        .map_err(|_| resource("output-bytes"))?;
    for character in input.chars() {
        charge()?;
        output.push(number_usize(character as usize)?);
    }
    Ok(Value::array(output))
}

pub(super) fn bounded_implode(
    input: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let Value::Array(input) = input else {
        return Err(type_error("implode", input));
    };
    ensure_string_lower_bound(input.len(), limits.output_bytes)?;
    let mut output = String::new();
    output
        .try_reserve(input.len())
        .map_err(|_| resource("output-bytes"))?;
    for value in input.iter() {
        charge()?;
        let Value::Number(number) = value else {
            return Err(type_error("implode", value));
        };
        let codepoint = number
            .exact_index()
            .and_then(|value| u32::try_from(value).ok())
            .and_then(char::from_u32)
            .unwrap_or(char::REPLACEMENT_CHARACTER);
        let mut encoded = [0_u8; 4];
        let width = codepoint.encode_utf8(&mut encoded).len();
        let next = output
            .len()
            .checked_add(width)
            .ok_or_else(|| resource("output-bytes"))?;
        ensure_string_lower_bound(next, limits.output_bytes)?;
        output
            .try_reserve(width)
            .map_err(|_| resource("output-bytes"))?;
        output.push(codepoint);
    }
    Ok(Value::string(output))
}

pub(super) fn bounded_sort(
    input: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let Value::Array(values) = input else {
        return Err(type_error("sort", input));
    };
    ensure_container_lower_bound(values.len(), limits.output_bytes)?;
    charge_input_count(values.len(), charge)?;
    let mut output = Vec::new();
    output
        .try_reserve(values.len())
        .map_err(|_| resource("output-bytes"))?;
    output.extend(values.iter().cloned());
    stable_sort_values(&mut output);
    Ok(Value::array(output))
}

pub(super) fn bounded_reverse(
    input: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    match input {
        Value::Array(values) => {
            ensure_container_lower_bound(values.len(), limits.output_bytes)?;
            let mut output = Vec::new();
            output
                .try_reserve(values.len())
                .map_err(|_| resource("output-bytes"))?;
            for value in values.iter().rev() {
                charge()?;
                output.push(value.clone());
            }
            Ok(Value::array(output))
        }
        Value::String(value) => {
            ensure_string_lower_bound(value.len(), limits.output_bytes)?;
            let mut output = String::new();
            output
                .try_reserve(value.len())
                .map_err(|_| resource("output-bytes"))?;
            for character in value.chars().rev() {
                charge()?;
                output.push(character);
            }
            Ok(Value::string(output))
        }
        value => Err(type_error("reverse", value)),
    }
}

pub(super) fn bounded_binary_add(
    left: &Value,
    right: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    match (left, right) {
        (Value::Null, value) | (value, Value::Null) => Ok(value.clone()),
        (Value::Number(_), Value::Number(_)) => super::binary_add(left, right),
        (Value::String(left), Value::String(right)) => {
            let bytes = left
                .len()
                .checked_add(right.len())
                .ok_or_else(|| resource("output-bytes"))?;
            ensure_string_lower_bound(bytes, limits.output_bytes)?;
            let mut output = String::new();
            output
                .try_reserve(bytes)
                .map_err(|_| resource("output-bytes"))?;
            for value in [left.as_ref(), right.as_ref()] {
                for character in value.chars() {
                    charge()?;
                    output.push(character);
                }
            }
            Ok(Value::string(output))
        }
        (Value::Array(left), Value::Array(right)) => {
            let length = left
                .len()
                .checked_add(right.len())
                .ok_or_else(|| resource("output-bytes"))?;
            ensure_container_lower_bound(length, limits.output_bytes)?;
            let mut output = Vec::new();
            output
                .try_reserve(length)
                .map_err(|_| resource("output-bytes"))?;
            for value in left.iter().chain(right.iter()) {
                charge()?;
                output.push(value.clone());
            }
            Ok(Value::array(output))
        }
        (Value::Object(left), Value::Object(right)) => {
            let mut additional = 0usize;
            for key in right.keys() {
                charge()?;
                if !left.contains_key(key) {
                    additional = additional
                        .checked_add(1)
                        .ok_or_else(|| resource("output-bytes"))?;
                }
            }
            let length = left
                .len()
                .checked_add(additional)
                .ok_or_else(|| resource("output-bytes"))?;
            ensure_container_lower_bound(length, limits.output_bytes)?;
            let mut output = Object::new();
            output
                .try_reserve(length)
                .map_err(|_| resource("output-bytes"))?;
            for (key, value) in left.iter().chain(right.iter()) {
                charge()?;
                output.insert(Arc::clone(key), value.clone());
            }
            Ok(Value::object(output))
        }
        _ => Err(runtime(format!(
            "cannot add {} and {}",
            type_name(left),
            type_name(right)
        ))),
    }
}

pub(super) fn bounded_add(
    input: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    match input {
        Value::Array(values) => bounded_add_values(values.iter(), limits, charge),
        Value::Object(values) => bounded_add_values(values.values(), limits, charge),
        value => Err(type_error("add", value)),
    }
}

fn bounded_add_values<'a>(
    mut values: impl Iterator<Item = &'a Value>,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let Some(first) = values.next() else {
        return Ok(Value::Null);
    };
    let mut result = first.clone();
    for value in values {
        charge()?;
        result = bounded_binary_add(&result, value, limits, charge)?;
    }
    Ok(result)
}

pub(super) fn bounded_unique(
    input: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let Value::Array(values) = input else {
        return Err(type_error("unique", input));
    };
    let mut sorted = Vec::new();
    sorted
        .try_reserve(values.len())
        .map_err(|_| resource("output-bytes"))?;
    for value in values.iter() {
        charge()?;
        sorted.push(value.clone());
    }
    stable_sort_values(&mut sorted);
    sorted.dedup_by(|left, right| collection::jq_equal(left, right));
    ensure_container_lower_bound(sorted.len(), limits.output_bytes)?;
    Ok(Value::array(sorted))
}

pub(super) fn bounded_from_entries(
    entries: &[Value],
    operation: &str,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let mut object = Object::new();
    for entry in entries {
        charge()?;
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
        if !object.contains_key(&key) {
            let entries = object
                .len()
                .checked_add(1)
                .ok_or_else(|| resource("output-bytes"))?;
            ensure_container_lower_bound(entries, limits.output_bytes)?;
            object
                .try_reserve(1)
                .map_err(|_| resource("output-bytes"))?;
        }
        object.insert(key, value);
    }
    Ok(Value::object(object))
}

fn charge_array_items(
    input: &Value,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<(), VmError> {
    let Value::Array(values) = input else {
        return Ok(());
    };
    charge_input_count(values.len(), charge)
}

fn charge_input_count(
    count: usize,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<(), VmError> {
    for _ in 0..count {
        charge()?;
    }
    Ok(())
}

fn ensure_container_lower_bound(entries: usize, output_limit: usize) -> Result<(), VmError> {
    let lower_bound = entries
        .checked_add(2)
        .ok_or_else(|| resource("output-bytes"))?;
    if lower_bound > output_limit {
        return Err(resource("output-bytes"));
    }
    Ok(())
}

fn ensure_string_lower_bound(bytes: usize, output_limit: usize) -> Result<(), VmError> {
    if bytes > output_limit {
        return Err(resource("output-bytes"));
    }
    Ok(())
}

struct FlattenFrame {
    values: Arc<[Value]>,
    index: usize,
    depth: Option<i64>,
}

/// Flattens with an explicit depth-bounded frame stack and fallible output
/// growth. Flat large arrays use one frame; only active nested containers
/// consume the VM's work-stack limit.
pub(super) fn bounded_flatten(
    input: &Value,
    depth: Option<i64>,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let Value::Array(values) = input else {
        return Err(type_error("flatten", input));
    };
    if depth.is_some_and(|depth| depth < 0) {
        return Err(runtime("flatten depth cannot be negative".to_owned()));
    }
    if limits.value_stack == 0 {
        return Err(resource("value-stack"));
    }
    let mut frames = Vec::new();
    frames.try_reserve(1).map_err(|_| resource("value-stack"))?;
    frames.push(FlattenFrame {
        values: Arc::clone(values),
        index: 0,
        depth,
    });
    let mut output = Vec::new();
    while let Some(frame) = frames.last_mut() {
        if frame.index == frame.values.len() {
            frames.pop();
            continue;
        }
        let value = frame.values[frame.index].clone();
        frame.index += 1;
        let frame_depth = frame.depth;
        charge()?;
        if let Value::Array(values) = &value
            && frame_depth != Some(0)
        {
            if frames.len() >= limits.value_stack {
                return Err(resource("value-stack"));
            }
            frames.try_reserve(1).map_err(|_| resource("value-stack"))?;
            frames.push(FlattenFrame {
                values: Arc::clone(values),
                index: 0,
                depth: frame_depth.map(|depth| depth - 1),
            });
            continue;
        }
        let output_len = output
            .len()
            .checked_add(1)
            .ok_or_else(|| resource("output-bytes"))?;
        ensure_container_lower_bound(output_len, limits.output_bytes)?;
        output
            .try_reserve(1)
            .map_err(|_| resource("output-bytes"))?;
        output.push(value);
    }
    Ok(Value::array(output))
}

fn make_checkpoint<'a, F>(charge: &'a mut F) -> impl Fn() -> Result<(), VmError> + 'a
where
    F: FnMut() -> Result<(), VmError> + 'a,
{
    let charge = RefCell::new(charge);
    move || (charge.borrow_mut())()
}

fn collection_contains(
    charge: &mut impl FnMut() -> Result<(), VmError>,
    input: &Value,
    needle: &Value,
) -> Result<Value, VmError> {
    let checkpoint = make_checkpoint(charge);
    collection::contains(input, needle, &checkpoint)
}

fn collection_inside(
    charge: &mut impl FnMut() -> Result<(), VmError>,
    input: &Value,
    container: &Value,
) -> Result<Value, VmError> {
    let checkpoint = make_checkpoint(charge);
    collection::inside(input, container, &checkpoint)
}

fn collection_indices(
    charge: &mut impl FnMut() -> Result<(), VmError>,
    input: &Value,
    needle: &Value,
) -> Result<Value, VmError> {
    let checkpoint = make_checkpoint(charge);
    collection::indices(input, needle, &checkpoint)
}

fn collection_index(
    charge: &mut impl FnMut() -> Result<(), VmError>,
    input: &Value,
    needle: &Value,
    reverse: bool,
) -> Result<Value, VmError> {
    let checkpoint = make_checkpoint(charge);
    collection::index(input, needle, reverse, &checkpoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(value: &str) -> Value {
        Value::Number(Number::parse(value).expect("test number"))
    }

    fn limits() -> VmLimits {
        VmLimits::default()
    }

    #[test]
    fn supports_scalar_math_and_rejects_filter_builtins() {
        assert!(supports("acos", 0));
        assert!(supports("pow", 2));
        assert!(supports("fma", 3));
        assert!(supports("contains", 1));
        assert!(supports("transpose", 0));
        assert!(supports("bsearch", 1));
        assert!(supports("@json", 0));
        assert!(!supports("map", 1));
        assert!(!supports("recurse", 0));
        assert!(!supports("unknown", 0));
    }

    #[test]
    fn evaluates_math_predicates_and_literal_helpers() {
        let mut charged = 0;
        let mut charge = || {
            charged += 1;
            Ok(())
        };
        assert_eq!(
            evaluate(
                "pow",
                &number("0"),
                &[number("2"), number("3")],
                limits(),
                &mut charge
            )
            .unwrap(),
            Some(number("8"))
        );
        assert_eq!(
            evaluate("isfinite", &Value::Null, &[], limits(), &mut charge).unwrap(),
            Some(Value::Bool(false))
        );
        assert_eq!(
            evaluate(
                "trim",
                &Value::string("  text "),
                &[],
                limits(),
                &mut charge,
            )
            .unwrap(),
            Some(Value::string("text"))
        );
        let rows = Value::array(vec![
            Value::array(vec![number("1")]),
            Value::array(vec![number("2"), number("3")]),
        ]);
        assert_eq!(
            evaluate("transpose", &rows, &[], limits(), &mut charge).unwrap(),
            Some(Value::array(vec![
                Value::array(vec![number("1"), number("2")]),
                Value::array(vec![Value::Null, number("3")]),
            ]))
        );
        let sorted = Value::array(vec![number("1"), number("3")]);
        assert_eq!(
            evaluate("bsearch", &sorted, &[number("2")], limits(), &mut charge,)
                .unwrap()
                .map(|value| value.to_string()),
            Some("-2".to_owned())
        );
        assert!(charged >= 3);
    }

    #[test]
    fn rejects_unadmitted_calls_but_preserves_empty_selectors() {
        let mut charge = || Ok(());
        assert_eq!(
            evaluate("pow", &number("1"), &[number("2")], limits(), &mut charge),
            Err(VmError::InvalidProgram {
                message: "unsupported scalar builtin or arity",
            })
        );
        assert_eq!(
            evaluate("arrays", &Value::Null, &[], limits(), &mut charge).unwrap(),
            None
        );
    }

    #[test]
    fn collection_calls_use_the_bounded_charge_callback() {
        let mut remaining = 1usize;
        let mut charge = || {
            if remaining == 0 {
                Err(VmError::Resource {
                    resource: "vm-steps",
                })
            } else {
                remaining -= 1;
                Ok(())
            }
        };
        let result = evaluate(
            "contains",
            &Value::array(vec![number("1"), number("2")]),
            &[number("2")],
            limits(),
            &mut charge,
        );
        assert_eq!(
            result,
            Err(VmError::Resource {
                resource: "vm-steps"
            })
        );
    }

    #[test]
    fn transpose_and_bsearch_stop_at_the_charge_boundary() {
        let rows = Value::array(vec![Value::array(vec![number("1")])]);
        let mut remaining = 1usize;
        let mut charge = || {
            if remaining == 0 {
                Err(VmError::Resource {
                    resource: "vm-steps",
                })
            } else {
                remaining -= 1;
                Ok(())
            }
        };
        assert_eq!(
            evaluate("transpose", &rows, &[], limits(), &mut charge),
            Err(VmError::Resource {
                resource: "vm-steps"
            })
        );

        let sorted = Value::array(vec![number("1"), number("3")]);
        let mut remaining = 1usize;
        let mut charge = || {
            if remaining == 0 {
                Err(VmError::Resource {
                    resource: "vm-steps",
                })
            } else {
                remaining -= 1;
                Ok(())
            }
        };
        assert_eq!(
            evaluate("bsearch", &sorted, &[number("2")], limits(), &mut charge),
            Err(VmError::Resource {
                resource: "vm-steps"
            })
        );
    }

    #[test]
    fn getpath_resolution_keeps_nonassignable_missing_paths_explicit() {
        let input = Value::array(vec![number("1")]);
        let mut charge = || Ok(());
        let missing_negative = Value::array(vec![number("-5")]);
        let missing_nonfinite =
            Value::array(vec![Value::Number(Number::from_runtime_f64(f64::NAN))]);
        let object_input = Value::object(Object::new());
        let ordinary_missing = Value::array(vec![Value::string("missing")]);
        let ordinary_array_index = Value::array(vec![number("5")]);

        let negative = bounded_getpath_resolution(&input, &missing_negative, limits(), &mut charge)
            .expect("negative missing read is valid");
        assert_eq!(negative.value, Value::Null);
        assert!(negative.path.is_none());

        let nonfinite =
            bounded_getpath_resolution(&input, &missing_nonfinite, limits(), &mut charge)
                .expect("nonfinite missing read is valid");
        assert_eq!(nonfinite.value, Value::Null);
        assert!(nonfinite.path.is_none());

        let ordinary =
            bounded_getpath_resolution(&object_input, &ordinary_missing, limits(), &mut charge)
                .expect("ordinary missing key read is valid");
        assert_eq!(ordinary.value, Value::Null);
        assert!(ordinary.path.is_some());

        let ordinary_index =
            bounded_getpath_resolution(&input, &ordinary_array_index, limits(), &mut charge)
                .expect("ordinary missing array index read is valid");
        assert_eq!(ordinary_index.value, Value::Null);
        assert!(ordinary_index.path.is_some());
    }

    #[test]
    fn getpath_missing_still_validates_later_components() {
        let input = Value::array(vec![number("1")]);
        let path = Value::array(vec![number("-5"), Value::Bool(true)]);
        let mut charge = || Ok(());
        assert!(matches!(
            bounded_getpath_resolution(&input, &path, limits(), &mut charge),
            Err(VmError::Runtime { message }) if message.contains("path component")
        ));
    }
}
