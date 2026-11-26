//! Bounded evaluator for source-mapped expression bytecode.

use std::{
    cell::Cell,
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::{
    Bytecode, InputCursor, Number, Object, Value, VmError, VmLimits, VmObservations, collection,
    vm::EffectState,
};

mod admission;
mod builtin_effects;
mod direct;
mod generator;
mod iterate;
mod managed_paths;
mod managed_runtime;
mod managed_work;
mod ordered;
mod origins;
mod path;
mod path_builtin;
mod scalar;
mod schedule;
mod sql;
mod structured_paths;
mod unwind;
mod value_ops;

pub(crate) use self::admission::{managed_execution, user_execution_gap};
use self::builtin_effects::builtin_signatures;
use self::direct::Evaluator;
use self::managed_work::UserFrame;
use self::ordered::extrema;
pub use self::ordered::{
    StableSortPipeline, StableSortPipelineObservations, parallel_worker_count, stable_sort_values,
};
use self::origins::OriginToken;
use self::structured_paths::{compare_paths_for_deletion, path_component, path_value};
use self::value_ops::{
    absolute_value, access_index, binary_add, fromjson, has, jq_compare, length, math_argument,
    math_error, math_result_value, normalize_index, number_usize, numeric_predicate,
    runtime_number, slice_bounds, type_error, type_name,
};

pub(crate) const AMBIENT_ENVIRONMENT: &str = "__tq_ambient_environment";
pub(crate) const AMBIENT_PLATFORM: &str = "__tq_ambient_platform";
pub(crate) const INPUT_FILENAME: &str = "__tq_input_filename";
pub(crate) const INPUT_LINE_NUMBER: &str = "__tq_input_line_number";

type Environment = BTreeMap<Arc<str>, Value>;
type OriginEnvironment = BTreeMap<Arc<str>, OriginToken>;
type Outcomes = Vec<Result<Value, VmError>>;
type UserFrames = Arc<[UserFrame]>;

fn evaluation_interrupted(stop: &AtomicBool, cancellation: Option<&AtomicBool>) -> bool {
    stop.load(Ordering::Relaxed) || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
}

fn charge_managed_step(
    observations: &mut VmObservations,
    limits: VmLimits,
    cancellation: Option<&AtomicBool>,
    stop: &AtomicBool,
) -> Result<(), VmError> {
    if evaluation_interrupted(stop, cancellation) {
        return Err(VmError::Interrupted);
    }
    if observations.steps >= limits.steps {
        return Err(resource("vm-steps"));
    }
    observations.steps += 1;
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "stream evaluation keeps execution limits, observations, cancellation, and the shared input cursor explicit"
)]
pub(crate) fn evaluate_stream(
    bytecode: &Bytecode,
    input: &Value,
    variables: &Arc<Environment>,
    limits: VmLimits,
    cancellation: Option<&AtomicBool>,
    stop: &AtomicBool,
    input_cursor: Option<&InputCursor>,
    effects: &Arc<EffectState>,
    mut emit: impl FnMut(Result<Value, VmError>, VmObservations) -> bool,
) -> VmObservations {
    if bytecode.managed_tree_execution() {
        return managed_runtime::evaluate(
            Evaluation {
                bytecode,
                limits,
                cancellation,
                stop,
                input_cursor,
                effects,
            },
            input,
            variables,
            emit,
        );
    }
    let evaluator = Evaluator {
        bytecode,
        limits,
        observations: Cell::new(VmObservations::default()),
        cancellation,
        stop,
        input_cursor,
        effects: Arc::clone(effects),
    };
    evaluator.emit_node(bytecode.root(), input, variables, 0, &mut |result| {
        let mut observations = evaluator.observations.get();
        if result.is_ok() {
            observations.results += 1;
            evaluator.observations.set(observations);
        }
        emit(result, observations)
    });
    evaluator.observations.get()
}

#[derive(Clone, Copy)]
struct Evaluation<'a> {
    bytecode: &'a Bytecode,
    limits: VmLimits,
    cancellation: Option<&'a AtomicBool>,
    stop: &'a AtomicBool,
    input_cursor: Option<&'a InputCursor>,
    effects: &'a Arc<EffectState>,
}

fn limit_count(value: &Value) -> Result<usize, VmError> {
    let Value::Number(number) = value else {
        return Err(type_error("limit", value));
    };
    let count = number.exact_index().or_else(|| {
        let value = number.as_f64();
        (value.is_finite() && value >= 0.0)
            .then(|| Number::from_runtime_f64(value.trunc()))
            .and_then(|number| number.exact_index())
    });
    match count {
        Some(count) if count >= 0 => Ok(usize::try_from(count).unwrap_or(usize::MAX)),
        _ => Err(runtime(
            "limit count must be a non-negative number".to_owned(),
        )),
    }
}

fn catch_value(error: &VmError) -> Value {
    match error {
        VmError::Raised { value, .. } => value.clone(),
        VmError::Runtime { message } | VmError::CapabilityDenied { message } => {
            Value::string(Arc::clone(message))
        }
        VmError::Break { .. } => {
            let mut sentinel = Object::new();
            sentinel.insert(
                Arc::from("__jq"),
                Value::Number(Number::parse("0").expect("zero is a valid number")),
            );
            Value::object(sentinel)
        }
        _ => Value::string(error.to_string()),
    }
}

fn error_message(value: &Value) -> Arc<str> {
    match value {
        Value::String(message) => Arc::clone(message),
        value => value.to_string().into(),
    }
}

fn raised(value: Value) -> VmError {
    VmError::Raised {
        message: error_message(&value),
        value,
    }
}

fn is_catchable_error(error: &VmError) -> bool {
    matches!(
        error,
        VmError::Input { .. }
            | VmError::RecoverableInput { .. }
            | VmError::Runtime { .. }
            | VmError::CapabilityDenied { .. }
            | VmError::Raised { .. }
            | VmError::NumericRange { .. }
            | VmError::Break { .. }
    )
}

fn runtime(message: String) -> VmError {
    VmError::Runtime {
        message: message.into(),
    }
}

fn capability_denied(message: String) -> VmError {
    VmError::CapabilityDenied {
        message: message.into(),
    }
}

fn invalid(message: &'static str) -> VmError {
    VmError::InvalidProgram { message }
}

fn resource(resource: &'static str) -> VmError {
    VmError::Resource { resource }
}

fn ambient_platform(environment: &Environment) -> bool {
    matches!(environment.get(AMBIENT_PLATFORM), Some(Value::Bool(true)))
}

fn ambient_environment(environment: &Environment, operation: &str) -> Result<Value, VmError> {
    match environment.get(AMBIENT_ENVIRONMENT) {
        Some(Value::Object(values)) => Ok(Value::Object(Arc::clone(values))),
        _ => Err(capability_denied(format!(
            "{operation} requires environment access permitted by capability policy"
        ))),
    }
}

pub(crate) fn variable_value(environment: &Environment, name: &str) -> Result<Value, VmError> {
    if name == AMBIENT_ENVIRONMENT {
        ambient_environment(environment, "$ENV")
    } else {
        environment
            .get(name)
            .cloned()
            .ok_or_else(|| runtime(format!("variable ${name} has no value")))
    }
}

fn ambient_value(
    environment: &Environment,
    key: &str,
    operation: &str,
    current: Option<Value>,
) -> Result<Value, VmError> {
    if !ambient_platform(environment) {
        return Err(capability_denied(format!(
            "{operation} requires platform access permitted by capability policy"
        )));
    }
    current
        .or_else(|| environment.get(key).cloned())
        .ok_or_else(|| {
            runtime(format!(
                "{operation} metadata is unavailable for this input mode"
            ))
        })
}

fn input_context_value(
    environment: &Environment,
    key: &str,
    operation: &str,
) -> Result<Value, VmError> {
    environment.get(key).cloned().ok_or_else(|| {
        runtime(format!(
            "{operation} metadata is unavailable for this input mode"
        ))
    })
}

#[cfg(test)]
mod tests;
