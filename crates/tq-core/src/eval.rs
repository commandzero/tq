//! Bounded evaluator for source-mapped expression bytecode.

use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use indexmap::IndexMap;

use crate::{
    Bytecode, Number, Object, Path, PathComponent, Value, VmError, VmLimits, VmObservations,
    ast::{AssignmentOperator, BinaryOperator, UnaryOperator},
    bytecode::{KeyOperand, Operation},
};

type Environment = BTreeMap<Arc<str>, Value>;
type Outcomes = Vec<Result<Value, VmError>>;

pub(crate) fn evaluate(
    bytecode: &Bytecode,
    input: &Value,
    variables: &Environment,
    limits: VmLimits,
    observations: &mut VmObservations,
    cancellation: Option<&AtomicBool>,
) -> Outcomes {
    let mut evaluator = Evaluator {
        bytecode,
        limits,
        observations,
        cancellation,
    };
    evaluator.node(bytecode.root(), input, variables, 0)
}

struct Evaluator<'a> {
    bytecode: &'a Bytecode,
    limits: VmLimits,
    observations: &'a mut VmObservations,
    cancellation: Option<&'a AtomicBool>,
}

impl Evaluator<'_> {
    #[allow(
        clippy::too_many_lines,
        reason = "the bytecode operation dispatch is intentionally exhaustive"
    )]
    fn node(
        &mut self,
        node: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        if self
            .cancellation
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            return one_error(VmError::Interrupted);
        }
        if depth >= self.limits.call_stack {
            return one_error(resource("call-stack"));
        }
        if self.observations.steps >= self.limits.steps {
            return one_error(resource("vm-steps"));
        }
        self.observations.steps += 1;
        self.observations.call_stack_high_water =
            self.observations.call_stack_high_water.max(depth + 1);
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
                Ok(name) => environment.get(name).cloned().map_or_else(
                    || one_error(runtime(format!("variable ${name} has no value"))),
                    |value| vec![Ok(value)],
                ),
                Err(error) => one_error(error),
            },
            Operation::Empty => Vec::new(),
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
                            for index in self.node(*index, &base, environment, depth + 1) {
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
                    let starts = self.bound(*start, &base, environment, depth + 1);
                    let ends = self.bound(*end, &base, environment, depth + 1);
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
            Operation::Bind { value, name, body } => {
                let name = match self.string(*name) {
                    Ok(name) => Arc::clone(name),
                    Err(error) => return one_error(error),
                };
                let mut output = Vec::new();
                for value in self.node(*value, input, environment, depth + 1) {
                    match value {
                        Ok(value) => {
                            let mut nested = environment.clone();
                            nested.insert(Arc::clone(&name), value);
                            output.extend(self.node(*body, input, &nested, depth + 1));
                        }
                        Err(error) => output.push(Err(error)),
                    }
                }
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
                                    &Value::string(catch_value(&error)),
                                    environment,
                                    depth + 1,
                                ));
                            }
                        }
                    }
                }
                output
            }
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

    fn bound(
        &mut self,
        node: Option<u32>,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Vec<Result<Option<i64>, VmError>> {
        match node {
            None => vec![Ok(None)],
            Some(node) => self
                .node(node, input, environment, depth)
                .into_iter()
                .map(|value| {
                    value.and_then(|value| match value {
                        Value::Null => Ok(None),
                        Value::Number(number) => number.exact_index().map(Some).ok_or_else(|| {
                            runtime("slice bound must be an exact integer".to_owned())
                        }),
                        value => Err(type_error("slice", &value)),
                    })
                })
                .collect(),
        }
    }

    fn binary(
        &mut self,
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

    #[allow(
        clippy::too_many_lines,
        reason = "the versioned built-in registry is executed in one exhaustive dispatch"
    )]
    fn call(
        &mut self,
        name: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        match name {
            "empty" => Vec::new(),
            "type" => vec![Ok(Value::string(type_name(input)))],
            "length" => vec![length(input)],
            "utf8bytelength" => match input {
                Value::String(value) => vec![number_usize(value.len())],
                value => one_error(type_error("utf8bytelength", value)),
            },
            "keys" | "keys_unsorted" => vec![keys(input, name == "keys")],
            "has" => self.argument_values(arguments, input, environment, depth, 0, |key| {
                has(input, key)
            }),
            "in" => self.argument_values(arguments, input, environment, depth, 0, |container| {
                has(container, input)
            }),
            "arrays" => selector(input, matches!(input, Value::Array(_))),
            "booleans" => selector(input, matches!(input, Value::Bool(_))),
            "iterables" => selector(input, matches!(input, Value::Array(_) | Value::Object(_))),
            "nulls" => selector(input, matches!(input, Value::Null)),
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
            "tonumber" => match input {
                Value::Number(_) => vec![Ok(input.clone())],
                Value::String(value) => {
                    Number::parse(value).map(Value::Number).map(Ok).map_or_else(
                        |error| one_error(runtime(error.to_string())),
                        |value| vec![value],
                    )
                }
                value => one_error(type_error("tonumber", value)),
            },
            "tostring" => vec![Ok(match input {
                Value::String(_) => input.clone(),
                _ => Value::string(input.to_string()),
            })],
            "range" => self.range(arguments, input, environment, depth),
            "add" => fold_values(input, None, binary_add),
            "min" => extrema(input, false),
            "max" => extrema(input, true),
            "sort" => sort_values(input),
            "sort_by" => self.sort_by(arguments, input, environment, depth, false),
            "unique" => unique_values(input),
            "unique_by" => self.sort_by(arguments, input, environment, depth, true),
            "reverse" => reverse(input),
            "flatten" => {
                let requested = if arguments.is_empty() {
                    None
                } else {
                    match first_value(self.node(arguments[0], input, environment, depth)) {
                        Ok(Value::Number(number)) => number.exact_index(),
                        Ok(value) => return one_error(type_error("flatten", &value)),
                        Err(error) => return one_error(error),
                    }
                };
                match flatten(input, requested) {
                    Ok(value) => vec![Ok(value)],
                    Err(error) => one_error(error),
                }
            }
            "error" => {
                let message = if let Some(argument) = arguments.first() {
                    match first_value(self.node(*argument, input, environment, depth)) {
                        Ok(Value::String(value)) => value,
                        Ok(value) => Arc::from(value.to_string()),
                        Err(error) => return one_error(error),
                    }
                } else {
                    Arc::from(input.to_string())
                };
                one_error(VmError::Runtime { message })
            }
            _ => one_error(VmError::Unsupported {
                operation: format!("builtin {name}").into(),
            }),
        }
    }

    fn argument_values(
        &mut self,
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

    fn map(
        &mut self,
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
                    match collect_values(self.node(*argument, value, environment, depth)) {
                        Ok(values) => mapped.extend(values),
                        Err(error) => return one_error(error),
                    }
                }
                vec![Ok(Value::array(mapped))]
            }
            Value::Object(object) if values_only => {
                let mut mapped = Object::new();
                for (key, value) in object.iter() {
                    match first_value(self.node(*argument, value, environment, depth)) {
                        Ok(value) => {
                            mapped.insert(Arc::clone(key), value);
                        }
                        Err(error) => return one_error(error),
                    }
                }
                vec![Ok(Value::object(mapped))]
            }
            value => one_error(type_error("map", value)),
        }
    }

    fn range(
        &mut self,
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
        &mut self,
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
            match first_value(self.node(*argument, value, environment, depth)) {
                Ok(key) => keyed.push((key, value.clone())),
                Err(error) => return one_error(error),
            }
        }
        keyed.sort_by(|left, right| left.0.cmp(&right.0));
        if unique {
            keyed.dedup_by(|left, right| left.0 == right.0);
        }
        vec![Ok(Value::array(
            keyed
                .into_iter()
                .map(|(_, value)| value)
                .collect::<Vec<_>>(),
        ))]
    }

    fn assignment(
        &mut self,
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
        let mut documents = vec![input.clone()];
        for path in paths {
            let mut next = Vec::new();
            for document in documents {
                let old = path.get(&document).cloned().unwrap_or(Value::Null);
                let expression_input = if operator == AssignmentOperator::Set {
                    input
                } else {
                    &old
                };
                let replacements = self.node(value_node, expression_input, environment, depth);
                for replacement in replacements {
                    match replacement {
                        Ok(replacement) => {
                            let replacement = match update_value(operator, &old, &replacement) {
                                Ok(value) => value,
                                Err(error) => return one_error(error),
                            };
                            match replace_or_create(&document, path.components(), replacement) {
                                Ok(value) => next.push(value),
                                Err(error) => return one_error(error),
                            }
                        }
                        Err(error) => return one_error(error),
                    }
                }
            }
            documents = next;
        }
        documents.into_iter().map(Ok).collect()
    }

    fn paths(
        &mut self,
        node: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Result<Vec<Path>, VmError> {
        let operation = self
            .bytecode
            .instructions()
            .get(node as usize)
            .ok_or_else(|| invalid("path instruction missing"))?
            .operation
            .clone();
        match operation {
            Operation::Identity => Ok(vec![Path::root()]),
            Operation::AccessField { base, key } => {
                let key = Arc::clone(self.string(key)?);
                let mut paths = self.paths(base, input, environment, depth + 1)?;
                for path in &mut paths {
                    let mut components = path.components().to_vec();
                    components.push(PathComponent::Key(Arc::clone(&key)));
                    *path = Path::new(components);
                }
                Ok(paths)
            }
            Operation::AccessIndex { base, index } => {
                let bases = self.paths(base, input, environment, depth + 1)?;
                let mut output = Vec::new();
                for base_path in bases {
                    let base_value = base_path.get(input).unwrap_or(&Value::Null);
                    for index in self.node(index, base_value, environment, depth + 1) {
                        let component = path_component(&index?)?;
                        let mut components = base_path.components().to_vec();
                        components.push(component);
                        output.push(Path::new(components));
                    }
                }
                Ok(output)
            }
            Operation::Iterate(base) => {
                let bases = self.paths(base, input, environment, depth + 1)?;
                let mut output = Vec::new();
                for base in bases {
                    match base.get(input) {
                        Some(Value::Array(values)) => {
                            for index in 0..values.len() {
                                let mut components = base.components().to_vec();
                                components.push(PathComponent::Index(index));
                                output.push(Path::new(components));
                            }
                        }
                        Some(Value::Object(values)) => {
                            for key in values.keys() {
                                let mut components = base.components().to_vec();
                                components.push(PathComponent::Key(Arc::clone(key)));
                                output.push(Path::new(components));
                            }
                        }
                        Some(value) => return Err(type_error("update iteration", value)),
                        None => {}
                    }
                }
                Ok(output)
            }
            Operation::Comma { left, right } => {
                let mut paths = self.paths(left, input, environment, depth + 1)?;
                paths.extend(self.paths(right, input, environment, depth + 1)?);
                Ok(paths)
            }
            Operation::Optional(child) => Ok(self
                .paths(child, input, environment, depth + 1)
                .unwrap_or_default()),
            _ => Err(runtime("assignment left side is not a path".to_owned())),
        }
    }
}

fn catch_value(error: &VmError) -> String {
    match error {
        VmError::Runtime { message } => message.to_string(),
        _ => error.to_string(),
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

fn one_error(error: VmError) -> Outcomes {
    vec![Err(error)]
}

fn runtime(message: String) -> VmError {
    VmError::Runtime {
        message: message.into(),
    }
}

fn invalid(message: &'static str) -> VmError {
    VmError::InvalidProgram { message }
}

fn resource(resource: &'static str) -> VmError {
    VmError::Resource { resource }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn type_error(operation: &str, value: &Value) -> VmError {
    runtime(format!(
        "{operation} cannot be applied to {}",
        type_name(value)
    ))
}

fn access_field(value: &Value, key: &Arc<str>) -> Result<Value, VmError> {
    match value {
        Value::Object(object) => Ok(object.get(key).cloned().unwrap_or(Value::Null)),
        Value::Null => Ok(Value::Null),
        value => Err(type_error("field access", value)),
    }
}

fn access_index(value: &Value, index: &Value) -> Result<Value, VmError> {
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

fn normalize_index(index: i64, length: usize) -> Option<usize> {
    if index >= 0 {
        usize::try_from(index).ok().filter(|index| *index < length)
    } else {
        let signed_length = i64::try_from(length).ok()?;
        usize::try_from(signed_length + index)
            .ok()
            .filter(|index| *index < length)
    }
}

fn slice(value: &Value, start: Option<i64>, end: Option<i64>) -> Result<Value, VmError> {
    match value {
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

fn slice_bounds(length: usize, start: Option<i64>, end: Option<i64>) -> (usize, usize) {
    let length_i64 = i64::try_from(length).unwrap_or(i64::MAX);
    let normalize = |value: i64| {
        let normalized = if value < 0 {
            (length_i64 + value).clamp(0, length_i64)
        } else {
            value.clamp(0, length_i64)
        };
        usize::try_from(normalized).unwrap_or(length)
    };
    let start = start.map_or(0, normalize);
    let end = end.map_or(length, normalize);
    (start.min(end), end)
}

fn computed_key(value: Value) -> Result<Arc<str>, VmError> {
    match value {
        Value::String(value) => Ok(value),
        value => Err(type_error("computed object key", &value)),
    }
}

fn unary(operator: UnaryOperator, value: &Value) -> Result<Value, VmError> {
    match operator {
        UnaryOperator::Not => Ok(Value::Bool(!value.is_truthy())),
        UnaryOperator::Negate => match value {
            Value::Number(value) => Number::from_f64(-value.as_f64())
                .map(Value::Number)
                .map_err(|error| runtime(error.to_string())),
            value => Err(type_error("negation", value)),
        },
    }
}

fn binary_value(operator: BinaryOperator, left: &Value, right: &Value) -> Result<Value, VmError> {
    match operator {
        BinaryOperator::Equal => Ok(Value::Bool(left == right)),
        BinaryOperator::NotEqual => Ok(Value::Bool(left != right)),
        BinaryOperator::Less => Ok(Value::Bool(left < right)),
        BinaryOperator::LessEqual => Ok(Value::Bool(left <= right)),
        BinaryOperator::Greater => Ok(Value::Bool(left > right)),
        BinaryOperator::GreaterEqual => Ok(Value::Bool(left >= right)),
        BinaryOperator::Add => binary_add(left, right),
        BinaryOperator::Subtract => binary_subtract(left, right),
        BinaryOperator::Multiply => numeric(left, right, Number::multiply, "multiply"),
        BinaryOperator::Divide => numeric(left, right, Number::divide, "divide"),
        BinaryOperator::Remainder => match (left, right) {
            (Value::Number(left), Value::Number(right)) if right.as_f64() != 0.0 => {
                Number::from_f64(left.as_f64() % right.as_f64())
                    .map(Value::Number)
                    .map_err(|error| runtime(error.to_string()))
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

fn binary_add(left: &Value, right: &Value) -> Result<Value, VmError> {
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

fn binary_subtract(left: &Value, right: &Value) -> Result<Value, VmError> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left
            .subtract(right)
            .map(Value::Number)
            .map_err(|error| runtime(error.to_string())),
        (Value::Array(left), Value::Array(right)) => Ok(Value::array(
            left.iter()
                .filter(|value| !right.contains(value))
                .cloned()
                .collect::<Vec<_>>(),
        )),
        _ => Err(runtime("subtraction requires numbers or arrays".to_owned())),
    }
}

fn numeric(
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

fn number_value(number: f64) -> Outcomes {
    Number::from_f64(number)
        .map(Value::Number)
        .map(Ok)
        .map_or_else(
            |error| one_error(runtime(error.to_string())),
            |value| vec![value],
        )
}

fn number_usize(number: usize) -> Result<Value, VmError> {
    Number::parse(&number.to_string())
        .map(Value::Number)
        .map_err(|error| runtime(error.to_string()))
}

fn length(value: &Value) -> Result<Value, VmError> {
    let length = match value {
        Value::Null => 0.0,
        Value::Number(number) => number.as_f64().abs(),
        Value::String(value) => return number_usize(value.chars().count()),
        Value::Array(value) => return number_usize(value.len()),
        Value::Object(value) => return number_usize(value.len()),
        Value::Bool(_) => return Err(type_error("length", value)),
    };
    Number::from_f64(length)
        .map(Value::Number)
        .map_err(|error| runtime(error.to_string()))
}

fn keys(value: &Value, sorted: bool) -> Result<Value, VmError> {
    let mut keys = match value {
        Value::Array(values) => (0..values.len())
            .map(|index| Number::parse(&index.to_string()).map(Value::Number))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| runtime(error.to_string()))?,
        Value::Object(values) => values
            .keys()
            .map(|key| Value::String(Arc::clone(key)))
            .collect(),
        value => return Err(type_error("keys", value)),
    };
    if sorted {
        keys.sort();
    }
    Ok(Value::array(keys))
}

fn has(container: &Value, key: &Value) -> Result<Value, VmError> {
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

fn selector(input: &Value, selected: bool) -> Outcomes {
    if selected {
        vec![Ok(input.clone())]
    } else {
        Vec::new()
    }
}

fn fold_values(
    input: &Value,
    initial: Option<Value>,
    apply: fn(&Value, &Value) -> Result<Value, VmError>,
) -> Outcomes {
    let values = match input {
        Value::Array(values) => values.iter().collect::<Vec<_>>(),
        Value::Object(values) => values.values().collect::<Vec<_>>(),
        value => return one_error(type_error("add", value)),
    };
    let mut iterator = values.into_iter();
    let mut result = if let Some(initial) = initial {
        initial
    } else if let Some(first) = iterator.next() {
        first.clone()
    } else {
        return vec![Ok(Value::Null)];
    };
    for value in iterator {
        match apply(&result, value) {
            Ok(next) => result = next,
            Err(error) => return one_error(error),
        }
    }
    vec![Ok(result)]
}

fn extrema(input: &Value, maximum: bool) -> Outcomes {
    let Value::Array(values) = input else {
        return one_error(type_error(if maximum { "max" } else { "min" }, input));
    };
    let selected = if maximum {
        values.iter().max()
    } else {
        values.iter().min()
    };
    vec![Ok(selected.cloned().unwrap_or(Value::Null))]
}

fn sort_values(input: &Value) -> Outcomes {
    let Value::Array(values) = input else {
        return one_error(type_error("sort", input));
    };
    let mut values = values.to_vec();
    values.sort();
    vec![Ok(Value::array(values))]
}

fn unique_values(input: &Value) -> Outcomes {
    let Value::Array(values) = input else {
        return one_error(type_error("unique", input));
    };
    let mut values = values.to_vec();
    values.sort();
    values.dedup();
    vec![Ok(Value::array(values))]
}

fn reverse(input: &Value) -> Outcomes {
    match input {
        Value::Array(values) => vec![Ok(Value::array(
            values.iter().rev().cloned().collect::<Vec<_>>(),
        ))],
        Value::String(value) => vec![Ok(Value::string(value.chars().rev().collect::<String>()))],
        value => one_error(type_error("reverse", value)),
    }
}

fn flatten(input: &Value, depth: Option<i64>) -> Result<Value, VmError> {
    let Value::Array(values) = input else {
        return Err(type_error("flatten", input));
    };
    if depth.is_some_and(|depth| depth < 0) {
        return Err(runtime("flatten depth cannot be negative".to_owned()));
    }
    let mut output = Vec::new();
    let mut stack = values
        .iter()
        .rev()
        .cloned()
        .map(|value| (value, depth))
        .collect::<Vec<_>>();
    while let Some((value, depth)) = stack.pop() {
        match value {
            Value::Array(values) if depth != Some(0) => {
                let next_depth = depth.map(|depth| depth - 1);
                stack.extend(
                    values
                        .iter()
                        .rev()
                        .cloned()
                        .map(|value| (value, next_depth)),
                );
            }
            value => output.push(value),
        }
    }
    Ok(Value::array(output))
}

fn path_component(value: &Value) -> Result<PathComponent, VmError> {
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

fn update_value(
    operator: AssignmentOperator,
    old: &Value,
    replacement: &Value,
) -> Result<Value, VmError> {
    match operator {
        AssignmentOperator::Set | AssignmentOperator::Update => Ok(replacement.clone()),
        AssignmentOperator::Add => binary_add(old, replacement),
        AssignmentOperator::Subtract => binary_subtract(old, replacement),
        AssignmentOperator::Multiply => numeric(old, replacement, Number::multiply, "multiply"),
        AssignmentOperator::Divide => numeric(old, replacement, Number::divide, "divide"),
        AssignmentOperator::Alternative => Ok(if old.is_truthy() {
            old.clone()
        } else {
            replacement.clone()
        }),
    }
}

fn replace_or_create(
    root: &Value,
    components: &[PathComponent],
    replacement: Value,
) -> Result<Value, VmError> {
    let Some((component, tail)) = components.split_first() else {
        return Ok(replacement);
    };
    match component {
        PathComponent::Key(key) => {
            let mut object = match root {
                Value::Object(object) => object.as_ref().clone(),
                Value::Null => IndexMap::new(),
                value => return Err(type_error("object assignment", value)),
            };
            let child = object.get(key).unwrap_or(&Value::Null);
            let value = replace_or_create(child, tail, replacement)?;
            object.insert(Arc::clone(key), value);
            Ok(Value::object(object))
        }
        PathComponent::Index(index) => {
            let mut values = match root {
                Value::Array(values) => values.to_vec(),
                Value::Null => Vec::new(),
                value => return Err(type_error("array assignment", value)),
            };
            while values.len() <= *index {
                values.push(Value::Null);
            }
            let value = replace_or_create(&values[*index], tail, replacement)?;
            values[*index] = value;
            Ok(Value::array(values))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::Arc,
    };

    use super::Value;
    use crate::{ResolveOptions, Vm, VmLimits, analyze, parse, resolve};

    fn run(query: &str, input: &str) -> Vec<Result<Value, String>> {
        run_with_variables(query, input, BTreeMap::new())
    }

    fn run_with_variables(
        query: &str,
        input: &str,
        variables: BTreeMap<Arc<str>, Value>,
    ) -> Vec<Result<Value, String>> {
        let query = parse(query).unwrap();
        let query = resolve(
            query,
            &ResolveOptions {
                variables: variables.keys().cloned().collect::<BTreeSet<_>>(),
            },
        )
        .unwrap();
        let program = analyze(query).compile().unwrap();
        let input: Value = serde_json::from_str(input).unwrap();
        let mut vm = Vm::new_with_variables(&program, input, VmLimits::default(), variables);
        let mut values = Vec::new();
        loop {
            match vm.next_result() {
                Ok(Some(value)) => values.push(Ok(value)),
                Ok(None) => break,
                Err(error) => {
                    values.push(Err(error.to_string()));
                    break;
                }
            }
        }
        values
    }

    fn json(values: Vec<Result<Value, String>>) -> Vec<String> {
        values
            .into_iter()
            .map(|value| {
                value.map_or_else(|error| format!("error:{error}"), |value| value.to_string())
            })
            .collect()
    }

    #[test]
    fn wave_one_navigation_and_generators_compose() {
        assert_eq!(json(run(".a.b", r#"{"a":{"b":2}}"#)), ["2"]);
        assert_eq!(json(run(".[-1]", "[1,2,3]")), ["3"]);
        assert_eq!(json(run(".[1:3]", "[0,1,2,3]")), ["[1,2]"]);
        assert_eq!(json(run(".[] | .x", r#"[{"x":1},{"x":2}]"#)), ["1", "2"]);
        assert_eq!(json(run(".a, .missing?", r#"{"a":1}"#)), ["1", "null"]);
        assert!(run("empty", "null").is_empty());
    }

    #[test]
    fn wave_two_construction_control_and_operators_compose() {
        assert_eq!(json(run("[.[] | . * 2]", "[1,2,3]")), ["[2,4,6]"]);
        assert_eq!(
            json(run(
                "{name, ok: (.age >= 18)}",
                r#"{"name":"Ada","age":30}"#
            )),
            [r#"{"name":"Ada","ok":true}"#]
        );
        assert_eq!(
            json(run("if . then \"yes\" else \"no\" end", "0")),
            [r#""yes""#]
        );
        assert_eq!(json(run("null // false // 7", "null")), ["7"]);
        assert_eq!(json(run(r#""a" + "b""#, "null")), [r#""ab""#]);
        assert_eq!(
            json(run(r#"{"a":1}+{"a":2,"b":3}"#, "null")),
            [r#"{"a":2,"b":3}"#]
        );
    }

    #[test]
    fn wave_three_variables_builtins_errors_and_updates_compose() {
        let variables = BTreeMap::from([(
            Arc::from("n"),
            Value::from_json(serde_json::json!(3)).unwrap(),
        )]);
        assert_eq!(json(run_with_variables("$n + 1", "null", variables)), ["4"]);
        assert_eq!(
            json(run(
                "map(.x) | sort | unique",
                r#"[{"x":2},{"x":1},{"x":2}]"#
            )),
            ["[1,2]"]
        );
        assert_eq!(json(run("type, length", r"[1,2]")), [r#""array""#, "2"]);
        assert_eq!(
            json(run("try error(\"boom\") catch .", "null")),
            [r#""boom""#]
        );
        let error_after = json(run("1, error(\"later\")", "null"));
        assert_eq!(error_after[0], "1");
        assert!(error_after[1].starts_with("error:runtime error: later"));
        assert_eq!(json(run(".a.b = 2", "{}")), [r#"{"a":{"b":2}}"#]);
        assert_eq!(
            json(run("(.a, .b) += 10", r#"{"a":1,"b":2,"c":[3]}"#)),
            [r#"{"a":11,"b":12,"c":[3]}"#]
        );
    }
}
