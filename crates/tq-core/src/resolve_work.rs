//! Heap-backed resolver traversals in source child order.

use super::*;

pub(super) fn children_mut(expr: &mut Expr) -> Vec<&mut Expr> {
    children_kind_mut(&mut expr.kind)
}

#[allow(clippy::too_many_lines, reason = "exhaustive AST child positions")]
pub(super) fn children_kind_mut(kind: &mut ExprKind) -> Vec<&mut Expr> {
    let mut children: Vec<&mut Expr> = Vec::new();
    match kind {
        ExprKind::Interpolation(segments) => {
            for segment in segments {
                if let InterpolationSegment::Expression(expression) = segment {
                    children.push(expression);
                }
            }
        }
        ExprKind::Access { base, access } => {
            children.push(base);
            match access {
                Access::Index(index) => children.push(index),
                Access::Slice { start, end } => {
                    if let Some(start) = start {
                        children.push(start);
                    }
                    if let Some(end) = end {
                        children.push(end);
                    }
                }
                Access::Field(_) | Access::Iterate => {}
            }
        }
        ExprKind::Optional(expression)
        | ExprKind::Array(expression)
        | ExprKind::Unary { expression, .. } => children.push(expression),
        ExprKind::Pipe(left, right)
        | ExprKind::Comma(left, right)
        | ExprKind::Binary { left, right, .. }
        | ExprKind::Assignment {
            path: left,
            value: right,
            ..
        } => {
            children.push(left);
            children.push(right);
        }
        ExprKind::Object(entries) => {
            for entry in entries {
                if let ObjectKey::Computed(key) = &mut entry.key {
                    children.push(key);
                }
                children.push(&mut entry.value);
            }
        }
        ExprKind::Conditional {
            branches,
            alternative,
        } => {
            for (condition, body) in branches {
                children.push(condition);
                children.push(body);
            }
            children.push(alternative);
        }
        ExprKind::Bind { value, body, .. } | ExprKind::BindAlternatives { value, body, .. } => {
            children.push(value);
            children.push(body);
        }
        ExprKind::Reduce {
            generator,
            initial,
            update,
            ..
        } => {
            children.push(generator);
            children.push(initial);
            children.push(update);
        }
        ExprKind::Foreach {
            generator,
            initial,
            update,
            extract,
            ..
        } => {
            children.push(generator);
            children.push(initial);
            children.push(update);
            if let Some(extract) = extract {
                children.push(extract);
            }
        }
        ExprKind::Define { definition, body } => {
            children.push(&mut definition.body);
            children.push(body);
        }
        ExprKind::Include { metadata, body, .. } | ExprKind::Import { metadata, body, .. } => {
            if let Some(metadata) = metadata {
                children.push(metadata);
            }
            children.push(body);
        }
        ExprKind::Module { metadata, body } => {
            children.push(metadata);
            children.push(body);
        }
        ExprKind::Call { arguments, .. } => {
            for argument in arguments {
                children.push(argument);
            }
        }
        ExprKind::TryCatch { expression, catch } => {
            children.push(expression);
            if let Some(catch) = catch {
                children.push(catch);
            }
        }
        ExprKind::Label { body, .. } => children.push(body),
        ExprKind::Identity
        | ExprKind::Literal(_)
        | ExprKind::Variable(_)
        | ExprKind::Empty
        | ExprKind::RecursiveDescent
        | ExprKind::Break { .. } => {}
    }
    children
}

#[allow(clippy::too_many_lines, reason = "exhaustive AST child positions")]
pub(super) fn children(expr: &Expr) -> Vec<&Expr> {
    let mut children: Vec<&Expr> = Vec::new();
    match &expr.kind {
        ExprKind::Interpolation(segments) => {
            for segment in segments {
                if let InterpolationSegment::Expression(expression) = segment {
                    children.push(expression);
                }
            }
        }
        ExprKind::Access { base, access } => {
            children.push(base);
            match access {
                Access::Index(index) => children.push(index),
                Access::Slice { start, end } => {
                    if let Some(start) = start {
                        children.push(start);
                    }
                    if let Some(end) = end {
                        children.push(end);
                    }
                }
                Access::Field(_) | Access::Iterate => {}
            }
        }
        ExprKind::Optional(expression)
        | ExprKind::Array(expression)
        | ExprKind::Unary { expression, .. } => children.push(expression),
        ExprKind::Pipe(left, right)
        | ExprKind::Comma(left, right)
        | ExprKind::Binary { left, right, .. }
        | ExprKind::Assignment {
            path: left,
            value: right,
            ..
        } => {
            children.push(left);
            children.push(right);
        }
        ExprKind::Object(entries) => {
            for entry in entries {
                if let ObjectKey::Computed(key) = &entry.key {
                    children.push(key);
                }
                children.push(&entry.value);
            }
        }
        ExprKind::Conditional {
            branches,
            alternative,
        } => {
            for (condition, body) in branches {
                children.push(condition);
                children.push(body);
            }
            children.push(alternative);
        }
        ExprKind::Bind { value, body, .. } | ExprKind::BindAlternatives { value, body, .. } => {
            children.push(value);
            children.push(body);
        }
        ExprKind::Reduce {
            generator,
            initial,
            update,
            ..
        } => {
            children.push(generator);
            children.push(initial);
            children.push(update);
        }
        ExprKind::Foreach {
            generator,
            initial,
            update,
            extract,
            ..
        } => {
            children.push(generator);
            children.push(initial);
            children.push(update);
            if let Some(extract) = extract {
                children.push(extract);
            }
        }
        ExprKind::Define { definition, body } => {
            children.push(&definition.body);
            children.push(body);
        }
        ExprKind::Include { metadata, body, .. } | ExprKind::Import { metadata, body, .. } => {
            if let Some(metadata) = metadata {
                children.push(metadata);
            }
            children.push(body);
        }
        ExprKind::Module { metadata, body } => {
            children.push(metadata);
            children.push(body);
        }
        ExprKind::Call { arguments, .. } => {
            for argument in arguments {
                children.push(argument);
            }
        }
        ExprKind::TryCatch { expression, catch } => {
            children.push(expression);
            if let Some(catch) = catch {
                children.push(catch);
            }
        }
        ExprKind::Label { body, .. } => children.push(body),
        ExprKind::Identity
        | ExprKind::Literal(_)
        | ExprKind::Variable(_)
        | ExprKind::Empty
        | ExprKind::RecursiveDescent
        | ExprKind::Break { .. } => {}
    }
    children
}

pub(super) fn take(expr: &mut Expr) -> Expr {
    std::mem::replace(expr, Expr::new(ExprKind::Empty, expr.span))
}

pub(super) fn visit_mut(expr: &mut Expr, mut visit: impl FnMut(&mut Expr)) {
    let mut pending = vec![expr];
    while let Some(node) = pending.pop() {
        visit(node);
        pending.extend(children_mut(node).into_iter().rev());
    }
}

pub(super) fn postorder(expr: &Expr, mut visit: impl FnMut(&Expr)) {
    let mut pending = vec![(expr, false)];
    while let Some((node, ready)) = pending.pop() {
        if ready {
            visit(node);
        } else {
            pending.push((node, true));
            pending.extend(children(node).into_iter().rev().map(|child| (child, false)));
        }
    }
}

/// Owned postorder keeps parent and child borrows disjoint without raw pointers.
pub(super) fn map(expr: Expr, mut visit: impl FnMut(&mut Expr)) -> Expr {
    let mut pending = vec![(expr, false)];
    let mut output: Vec<Expr> = Vec::new();
    while let Some((mut node, ready)) = pending.pop() {
        if ready {
            let slots = children_mut(&mut node);
            let offset = output.len() - slots.len();
            for (slot, child) in slots.into_iter().zip(output.drain(offset..)) {
                *slot = child;
            }
            visit(&mut node);
            output.push(node);
        } else {
            let children = children_mut(&mut node)
                .into_iter()
                .map(take)
                .collect::<Vec<_>>();
            pending.push((node, true));
            pending.extend(children.into_iter().rev().map(|child| (child, false)));
        }
    }
    output.pop().expect("mapped root")
}

/// Cached modules must be copied without invoking recursive AST Clone.
pub(super) fn clone_expr(expr: &Expr) -> Expr {
    let mut pending = vec![(expr, false)];
    let mut output: Vec<Expr> = Vec::new();
    while let Some((node, ready)) = pending.pop() {
        if ready {
            let mut copy = shallow_clone(node);
            let slots = children_mut(&mut copy);
            let offset = output.len() - slots.len();
            for (slot, child) in slots.into_iter().zip(output.drain(offset..)) {
                *slot = child;
            }
            output.push(copy);
        } else {
            pending.push((node, true));
            pending.extend(children(node).into_iter().rev().map(|child| (child, false)));
        }
    }
    output.pop().expect("cloned root")
}

fn empty(expr: &Expr) -> Expr {
    Expr::new(ExprKind::Empty, expr.span)
}

fn shallow_access(access: &Access) -> Access {
    match access {
        Access::Field(name) => Access::Field(Arc::clone(name)),
        Access::Index(index) => Access::Index(Box::new(empty(index))),
        Access::Slice { start, end } => Access::Slice {
            start: start.as_deref().map(empty).map(Box::new),
            end: end.as_deref().map(empty).map(Box::new),
        },
        Access::Iterate => Access::Iterate,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "exhaustive shallow AST reconstruction"
)]
fn shallow_clone(expr: &Expr) -> Expr {
    let kind = match &expr.kind {
        ExprKind::Identity => ExprKind::Identity,
        ExprKind::Literal(value) => ExprKind::Literal(value.clone()),
        ExprKind::Variable(value) => ExprKind::Variable(value.clone()),
        ExprKind::Empty => ExprKind::Empty,
        ExprKind::RecursiveDescent => ExprKind::RecursiveDescent,
        ExprKind::Label { name, symbol, body } => ExprKind::Label {
            name: name.clone(),
            symbol: *symbol,
            body: Box::new(empty(body)),
        },
        ExprKind::Break { name, symbol } => ExprKind::Break {
            name: name.clone(),
            symbol: *symbol,
        },
        ExprKind::Interpolation(segments) => ExprKind::Interpolation(
            segments
                .iter()
                .map(|segment| match segment {
                    InterpolationSegment::Expression(child) => {
                        InterpolationSegment::Expression(empty(child))
                    }
                    literal @ InterpolationSegment::Literal { .. } => literal.clone(),
                })
                .collect(),
        ),
        ExprKind::Access { base, access } => ExprKind::Access {
            base: Box::new(empty(base)),
            access: shallow_access(access),
        },
        ExprKind::Optional(child) => ExprKind::Optional(Box::new(empty(child))),
        ExprKind::Pipe(a, b) => ExprKind::Pipe(Box::new(empty(a)), Box::new(empty(b))),
        ExprKind::Comma(a, b) => ExprKind::Comma(Box::new(empty(a)), Box::new(empty(b))),
        ExprKind::Array(child) => ExprKind::Array(Box::new(empty(child))),
        ExprKind::Object(entries) => ExprKind::Object(
            entries
                .iter()
                .map(|entry| crate::ast::ObjectEntry {
                    key: match &entry.key {
                        ObjectKey::Static(name) => ObjectKey::Static(Arc::clone(name)),
                        ObjectKey::Computed(child) => ObjectKey::Computed(empty(child)),
                    },
                    value: empty(&entry.value),
                    span: entry.span,
                })
                .collect(),
        ),
        ExprKind::Unary {
            operator,
            expression,
        } => ExprKind::Unary {
            operator: *operator,
            expression: Box::new(empty(expression)),
        },
        ExprKind::Binary {
            operator,
            left,
            right,
        } => ExprKind::Binary {
            operator: *operator,
            left: Box::new(empty(left)),
            right: Box::new(empty(right)),
        },
        ExprKind::Conditional {
            branches,
            alternative,
        } => ExprKind::Conditional {
            branches: branches.iter().map(|(a, b)| (empty(a), empty(b))).collect(),
            alternative: Box::new(empty(alternative)),
        },
        ExprKind::Bind {
            value,
            pattern,
            body,
        } => ExprKind::Bind {
            value: Box::new(empty(value)),
            pattern: pattern.clone(),
            body: Box::new(empty(body)),
        },
        ExprKind::BindAlternatives {
            value,
            patterns,
            body,
        } => ExprKind::BindAlternatives {
            value: Box::new(empty(value)),
            patterns: patterns.clone(),
            body: Box::new(empty(body)),
        },
        ExprKind::Reduce {
            generator,
            pattern,
            initial,
            update,
        } => ExprKind::Reduce {
            generator: Box::new(empty(generator)),
            pattern: pattern.clone(),
            initial: Box::new(empty(initial)),
            update: Box::new(empty(update)),
        },
        ExprKind::Foreach {
            generator,
            pattern,
            initial,
            update,
            extract,
        } => ExprKind::Foreach {
            generator: Box::new(empty(generator)),
            pattern: pattern.clone(),
            initial: Box::new(empty(initial)),
            update: Box::new(empty(update)),
            extract: extract.as_deref().map(empty).map(Box::new),
        },
        ExprKind::Define { definition, body } => ExprKind::Define {
            definition: Box::new(Definition {
                name: Arc::clone(&definition.name),
                parameters: definition.parameters.clone(),
                body: empty(&definition.body),
                span: definition.span,
                symbol: definition.symbol,
            }),
            body: Box::new(empty(body)),
        },
        ExprKind::Include {
            path,
            metadata,
            body,
        } => ExprKind::Include {
            path: path.clone(),
            metadata: metadata.as_deref().map(empty).map(Box::new),
            body: Box::new(empty(body)),
        },
        ExprKind::Import {
            path,
            alias,
            metadata,
            body,
        } => ExprKind::Import {
            path: path.clone(),
            alias: alias.clone(),
            metadata: metadata.as_deref().map(empty).map(Box::new),
            body: Box::new(empty(body)),
        },
        ExprKind::Module { metadata, body } => ExprKind::Module {
            metadata: Box::new(empty(metadata)),
            body: Box::new(empty(body)),
        },
        ExprKind::Call {
            name,
            arguments,
            target,
        } => ExprKind::Call {
            name: name.clone(),
            arguments: arguments.iter().map(empty).collect(),
            target: *target,
        },
        ExprKind::TryCatch { expression, catch } => ExprKind::TryCatch {
            expression: Box::new(empty(expression)),
            catch: catch.as_deref().map(empty).map(Box::new),
        },
        ExprKind::Assignment {
            operator,
            path,
            value,
        } => ExprKind::Assignment {
            operator: *operator,
            path: Box::new(empty(path)),
            value: Box::new(empty(value)),
        },
    };
    Expr::new(kind, expr.span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloning_and_owned_mapping_preserve_every_child_and_span() {
        for source in [
            "., empty, .., $x, [1, (2, 3)]",
            r#"@json "before\(.a[1:2]?)after\(.[:3], .[4:])""#,
            "{a: -1, (.key): .value} | .[] + 2",
            "if .a then .b elif .c then .d else .e end",
            "0 as [$x, {a: $y}] | $x",
            "0 as [$x] ?// {a: $x} | $x",
            "reduce .[] as $x (0; . + $x)",
            "foreach .[] as $x (0; . + $x; .)",
            "def f($x; g): g | $x; f(1; .)",
            "label $x | try (break $x) catch 0",
            ".a += 1",
            r#"include "m" {search: ["."]}; ."#,
            r#"import "m" as m {search: ["."]}; m::f"#,
            r#"module {search: ["."]}; ."#,
        ] {
            let parsed = crate::parse(source).expect("child-position fixture parses");
            let expected = format!("{:?}", parsed.ast());
            let cloned = clone_expr(parsed.ast());
            assert_eq!(format!("{cloned:?}"), expected, "{source}");
            let mapped = map(cloned, |_| {});
            assert_eq!(format!("{mapped:?}"), expected, "{source}");
        }
    }

    #[test]
    fn startup_location_replacement_preserves_deep_leaf_span_and_value() {
        let startup = format!("def f:\n{}$__loc__{};", "[".repeat(1024), "]".repeat(1024));
        let parsed = crate::parse_with_startup("query.jq", b".", "startup.jq", startup.as_bytes())
            .expect("startup parses");
        let ExprKind::Define { definition, .. } = &parsed.ast().kind else {
            panic!("startup definition retained");
        };
        let mut leaf = &definition.body;
        for _ in 0..1024 {
            let ExprKind::Array(body) = &leaf.kind else {
                panic!("array shape retained")
            };
            leaf = body;
        }
        assert_eq!(leaf.span.source, SourceId::new(1));
        assert_eq!(
            usize::try_from(leaf.span.start).unwrap(),
            startup.find("$__loc__").unwrap()
        );
        assert_eq!(leaf.span.end - leaf.span.start, 8);
        let ExprKind::Literal(value) = &leaf.kind else {
            panic!("location replaced")
        };
        assert_eq!(
            value,
            &Value::from_json(serde_json::json!({"file": "startup.jq", "line": 2})).unwrap()
        );
    }
}
