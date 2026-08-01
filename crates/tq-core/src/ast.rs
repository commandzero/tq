//! Internal source-spanned syntax tree.

use std::sync::Arc;

use crate::{Span, Value};

#[derive(Clone, Debug)]
pub(crate) struct Expr {
    pub(crate) kind: ExprKind,
    pub(crate) span: Span,
}

impl Expr {
    pub(crate) const fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ExprKind {
    Identity,
    Literal(Value),
    Variable(Arc<str>),
    Empty,
    Access {
        base: Box<Expr>,
        access: Access,
    },
    Optional(Box<Expr>),
    Pipe(Box<Expr>, Box<Expr>),
    Comma(Box<Expr>, Box<Expr>),
    Array(Box<Expr>),
    Object(Vec<ObjectEntry>),
    Unary {
        operator: UnaryOperator,
        expression: Box<Expr>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Conditional {
        branches: Vec<(Expr, Expr)>,
        alternative: Box<Expr>,
    },
    Bind {
        value: Box<Expr>,
        name: Arc<str>,
        body: Box<Expr>,
    },
    Call {
        name: Arc<str>,
        arguments: Vec<Expr>,
    },
    TryCatch {
        expression: Box<Expr>,
        catch: Option<Box<Expr>>,
    },
    Assignment {
        operator: AssignmentOperator,
        path: Box<Expr>,
        value: Box<Expr>,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum Access {
    Field(Arc<str>),
    Index(Box<Expr>),
    Slice {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
    },
    Iterate,
}

#[derive(Clone, Debug)]
pub(crate) struct ObjectEntry {
    pub(crate) key: ObjectKey,
    pub(crate) value: Expr,
    #[allow(
        dead_code,
        reason = "retained for source-local object-entry diagnostics"
    )]
    pub(crate) span: Span,
}

#[derive(Clone, Debug)]
pub(crate) enum ObjectKey {
    Static(Arc<str>),
    Computed(Expr),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnaryOperator {
    Not,
    Negate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BinaryOperator {
    Alternative,
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AssignmentOperator {
    Set,
    Update,
    Add,
    Subtract,
    Multiply,
    Divide,
    Alternative,
}

pub(crate) fn display(expr: &Expr) -> String {
    let mut output = String::new();
    render(expr, &mut output);
    output
}

#[allow(
    clippy::too_many_lines,
    reason = "stable HIR rendering is a direct exhaustive syntax mapping"
)]
fn render(expr: &Expr, output: &mut String) {
    match &expr.kind {
        ExprKind::Identity => output.push('.'),
        ExprKind::Literal(value) => output.push_str(&value.to_string()),
        ExprKind::Variable(name) => {
            output.push('$');
            output.push_str(name);
        }
        ExprKind::Empty => output.push_str("empty"),
        ExprKind::Access { base, access } => {
            output.push_str("access(");
            render(base, output);
            output.push_str(", ");
            match access {
                Access::Field(name) => {
                    output.push_str("field:");
                    output.push_str(name);
                }
                Access::Index(index) => {
                    output.push_str("index:");
                    render(index, output);
                }
                Access::Slice { start, end } => {
                    output.push_str("slice:");
                    render_optional(start.as_deref(), output);
                    output.push(':');
                    render_optional(end.as_deref(), output);
                }
                Access::Iterate => output.push_str("iterate"),
            }
            output.push(')');
        }
        ExprKind::Optional(expression) => {
            output.push_str("optional(");
            render(expression, output);
            output.push(')');
        }
        ExprKind::Pipe(left, right) => render_binary("pipe", left, right, output),
        ExprKind::Comma(left, right) => render_binary("comma", left, right, output),
        ExprKind::Array(expression) => {
            output.push_str("array(");
            render(expression, output);
            output.push(')');
        }
        ExprKind::Object(entries) => {
            output.push_str("object(");
            for (index, entry) in entries.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                match &entry.key {
                    ObjectKey::Static(key) => output.push_str(key),
                    ObjectKey::Computed(key) => {
                        output.push('(');
                        render(key, output);
                        output.push(')');
                    }
                }
                output.push(':');
                render(&entry.value, output);
            }
            output.push(')');
        }
        ExprKind::Unary {
            operator,
            expression,
        } => {
            output.push_str(match operator {
                UnaryOperator::Not => "not(",
                UnaryOperator::Negate => "negate(",
            });
            render(expression, output);
            output.push(')');
        }
        ExprKind::Binary {
            operator,
            left,
            right,
        } => render_binary(binary_name(*operator), left, right, output),
        ExprKind::Conditional {
            branches,
            alternative,
        } => {
            output.push_str("if(");
            for (index, (condition, body)) in branches.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                render(condition, output);
                output.push_str(" => ");
                render(body, output);
            }
            output.push_str(", else => ");
            render(alternative, output);
            output.push(')');
        }
        ExprKind::Bind { value, name, body } => {
            output.push_str("bind(");
            render(value, output);
            output.push_str(" as $");
            output.push_str(name);
            output.push_str(" => ");
            render(body, output);
            output.push(')');
        }
        ExprKind::Call { name, arguments } => {
            output.push_str("call(");
            output.push_str(name);
            for argument in arguments {
                output.push_str(", ");
                render(argument, output);
            }
            output.push(')');
        }
        ExprKind::TryCatch { expression, catch } => {
            output.push_str("try(");
            render(expression, output);
            if let Some(catch) = catch {
                output.push_str(", catch => ");
                render(catch, output);
            }
            output.push(')');
        }
        ExprKind::Assignment {
            operator,
            path,
            value,
        } => render_binary(assignment_name(*operator), path, value, output),
    }
}

fn render_optional(expr: Option<&Expr>, output: &mut String) {
    if let Some(expr) = expr {
        render(expr, output);
    } else {
        output.push('_');
    }
}

fn render_binary(name: &str, left: &Expr, right: &Expr, output: &mut String) {
    output.push_str(name);
    output.push('(');
    render(left, output);
    output.push_str(", ");
    render(right, output);
    output.push(')');
}

const fn binary_name(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Alternative => "alternative",
        BinaryOperator::Or => "or",
        BinaryOperator::And => "and",
        BinaryOperator::Equal => "equal",
        BinaryOperator::NotEqual => "not-equal",
        BinaryOperator::Less => "less",
        BinaryOperator::LessEqual => "less-equal",
        BinaryOperator::Greater => "greater",
        BinaryOperator::GreaterEqual => "greater-equal",
        BinaryOperator::Add => "add",
        BinaryOperator::Subtract => "subtract",
        BinaryOperator::Multiply => "multiply",
        BinaryOperator::Divide => "divide",
        BinaryOperator::Remainder => "remainder",
    }
}

const fn assignment_name(operator: AssignmentOperator) -> &'static str {
    match operator {
        AssignmentOperator::Set => "set",
        AssignmentOperator::Update => "update",
        AssignmentOperator::Add => "update-add",
        AssignmentOperator::Subtract => "update-subtract",
        AssignmentOperator::Multiply => "update-multiply",
        AssignmentOperator::Divide => "update-divide",
        AssignmentOperator::Alternative => "update-alternative",
    }
}
