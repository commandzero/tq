//! Check whether a bytecode program can use the managed evaluator.

use super::scalar;
use crate::Bytecode;
use crate::bytecode::{InterpolationOperand, KeyOperand, Operation};
use std::sync::Arc;

fn managed_subset_gap(bytecode: &Bytecode) -> Option<crate::Span> {
    let mut pending = vec![bytecode.root()];
    let mut seen = vec![false; bytecode.instructions().len()];
    while let Some(node) = pending.pop() {
        let Some(instruction) = bytecode.instructions().get(node as usize) else {
            return bytecode
                .instructions()
                .first()
                .map(|instruction| instruction.span);
        };
        if std::mem::replace(&mut seen[node as usize], true) {
            continue;
        }
        if !managed_operation_supported(bytecode, &instruction.operation) {
            return Some(instruction.span);
        }
        let Some(children) = operation_children(bytecode, &instruction.operation) else {
            return Some(instruction.span);
        };
        pending.extend(children);
    }
    None
}

pub(crate) fn user_execution_gap(bytecode: &Bytecode) -> Option<crate::Span> {
    reachable_user_call(bytecode).then(|| managed_subset_gap(bytecode))?
}

pub(crate) fn managed_execution(bytecode: &Bytecode) -> bool {
    managed_subset_gap(bytecode).is_none()
}

fn reachable_user_call(bytecode: &Bytecode) -> bool {
    let mut pending = vec![bytecode.root()];
    let mut seen = vec![false; bytecode.instructions().len()];
    while let Some(node) = pending.pop() {
        let Some(instruction) = bytecode.instructions().get(node as usize) else {
            continue;
        };
        if std::mem::replace(&mut seen[node as usize], true) {
            continue;
        }
        if matches!(instruction.operation, Operation::UserCall { .. }) {
            return true;
        }
        if let Some(children) = operation_children(bytecode, &instruction.operation) {
            pending.extend(children);
        }
    }
    false
}

fn managed_operation_supported(bytecode: &Bytecode, operation: &Operation) -> bool {
    match operation {
        Operation::Identity
        | Operation::Literal(_)
        | Operation::Variable(_)
        | Operation::Empty
        | Operation::RecursiveDescent
        | Operation::Label { .. }
        | Operation::Break(_)
        | Operation::ParameterCall { .. }
        | Operation::AccessField { .. }
        | Operation::Iterate(_)
        | Operation::Optional(_)
        | Operation::Array(_)
        | Operation::Object(_)
        | Operation::Unary { .. }
        | Operation::AccessIndex { .. }
        | Operation::Slice { .. }
        | Operation::Pipe { .. }
        | Operation::Comma { .. }
        | Operation::Binary { .. }
        | Operation::Conditional { .. }
        | Operation::Bind { .. }
        | Operation::BindAlternatives { .. }
        | Operation::Reduce { .. }
        | Operation::Foreach { .. }
        | Operation::Assignment { .. }
        | Operation::UserCall { .. }
        | Operation::TryCatch { .. }
        | Operation::Interpolation(_) => true,
        Operation::Call { name, arguments } => bytecode
            .string(*name)
            .is_some_and(|name| managed_call_supported(name, arguments.len())),
        _ => false,
    }
}

fn managed_call_supported(name: &str, arity: usize) -> bool {
    if regex_builtin_shape(name, arity).is_some() || supports_builtin_call(name, arity) {
        return true;
    }
    match name {
        "IN" | "INDEX" | "nth" | "skip" => (1..=2).contains(&arity),
        "JOIN" => (2..=4).contains(&arity),
        "any" | "all" | "recurse" => arity <= 2,
        "error" | "first" | "last" | "add" | "paths" | "debug" | "halt_error" => arity <= 1,
        "while" | "until" | "limit" => arity == 2,
        "map" | "map_values" | "select" | "sort_by" | "unique_by" | "group_by" | "min_by"
        | "max_by" | "with_entries" | "has" | "in" | "isempty" | "fromstream"
        | "truncate_stream" | "repeat" | "walk" | "path" | "pick" | "del" => arity == 1,
        "arrays" | "booleans" | "empty" | "keys" | "keys_unsorted" | "length" | "iterables"
        | "max" | "min" | "modulemeta" | "nulls" | "numbers" | "objects" | "scalars"
        | "strings" | "to_entries" | "from_entries" | "tonumber" | "values" | "reverse"
        | "sort" | "tostring" | "type" | "unique" | "utf8bytelength" | "@base64" | "@base64d"
        | "@csv" | "@html" | "@json" | "@sh" | "@text" | "@tsv" | "@uri" | "input" | "inputs"
        | "input_filename" | "input_line_number" | "stderr" | "halt" => arity == 0,
        _ => false,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the child-edge table exhaustively covers bytecode operations"
)]
fn operation_children(bytecode: &Bytecode, operation: &Operation) -> Option<Vec<u32>> {
    let mut children = Vec::new();
    match operation {
        Operation::AccessField { base, .. }
        | Operation::Iterate(base)
        | Operation::Optional(base)
        | Operation::Array(base)
        | Operation::Unary { child: base, .. } => children.push(*base),
        Operation::Label { body, .. } => children.push(*body),
        Operation::AccessIndex { base, index } => children.extend([*base, *index]),
        Operation::Slice { base, start, end } => {
            children.push(*base);
            children.extend([*start, *end].into_iter().flatten());
        }
        Operation::Pipe { left, right }
        | Operation::Comma { left, right }
        | Operation::Binary { left, right, .. } => children.extend([*left, *right]),
        Operation::Object(entries) => {
            for entry in entries {
                if let KeyOperand::Computed(key) = entry.key {
                    children.push(key);
                }
                children.push(entry.value);
            }
        }
        Operation::Conditional {
            branches,
            alternative,
        } => {
            for (condition, body) in branches {
                children.extend([*condition, *body]);
            }
            children.push(*alternative);
        }
        Operation::Bind { value, body, .. } | Operation::BindAlternatives { value, body, .. } => {
            children.extend([*value, *body]);
        }
        Operation::UserCall { symbol, arguments } => {
            children.extend(arguments.iter().copied());
            children.push(bytecode.functions().get(*symbol as usize)?.body);
        }
        Operation::Reduce {
            generator,
            initial,
            update,
            ..
        } => children.extend([*generator, *initial, *update]),
        Operation::Foreach {
            generator,
            initial,
            update,
            extract,
            ..
        } => {
            children.extend([*generator, *initial, *update]);
            children.extend(extract.iter().copied());
        }
        Operation::Call { arguments, .. } => children.extend(arguments.iter().copied()),
        Operation::TryCatch { expression, catch } => {
            children.push(*expression);
            children.extend(catch.iter().copied());
        }
        Operation::Interpolation(segments) => {
            children.extend(segments.iter().filter_map(|segment| match segment {
                InterpolationOperand::Literal(_) => None,
                InterpolationOperand::Expression(expression) => Some(*expression),
            }));
        }
        Operation::Assignment { path, value, .. } => children.extend([*path, *value]),
        Operation::LoadInput
        | Operation::LoadConstant(_)
        | Operation::Duplicate
        | Operation::Pop
        | Operation::Jump(_)
        | Operation::Branch { .. }
        | Operation::Fork(_)
        | Operation::Backtrack
        | Operation::Return
        | Operation::Raise(_)
        | Operation::Catch(_)
        | Operation::EndCatch
        | Operation::Identity
        | Operation::Literal(_)
        | Operation::Variable(_)
        | Operation::Empty
        | Operation::RecursiveDescent
        | Operation::Break(_)
        | Operation::ParameterCall { .. } => {}
    }
    Some(children)
}

pub(super) fn supports_builtin_call(name: &str, arity: usize) -> bool {
    scalar::supports(name, arity)
        || scalar::supports_ambient(name, arity)
        || (name == "range" && (1..=3).contains(&arity))
        || (name == "combinations" && arity <= 1)
        || (name == "tostream" && arity == 0)
}

pub(super) fn builtin_argument_order(name: &str, arity: usize) -> Arc<[usize]> {
    let order = if arity > 1 && scalar::supports(name, arity) {
        (0..arity).rev().collect::<Vec<_>>()
    } else {
        (0..arity).collect::<Vec<_>>()
    };
    order.into()
}

pub(super) fn regex_builtin_shape(
    name: &str,
    arity: usize,
) -> Option<(Arc<[usize]>, Option<usize>)> {
    match name {
        "test" | "match" | "capture" | "scan" | "split" | "splits" if arity == 1 => {
            Some((Arc::from([0]), None))
        }
        "test" | "match" | "capture" if arity == 2 => Some((Arc::from([1, 0]), None)),
        "scan" | "split" | "splits" if arity == 2 => Some((Arc::from([0, 1]), None)),
        "sub" | "gsub" if arity == 2 => Some((Arc::from([0]), Some(1))),
        "sub" | "gsub" if arity == 3 => Some((Arc::from([0, 2]), Some(1))),
        _ => None,
    }
}
