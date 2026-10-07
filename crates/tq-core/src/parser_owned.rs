//! Parser-local ownership: abandoning a completed subtree must not recurse
//! through `Expr`'s normal drop glue. Successful parses transfer the unchanged
//! AST to the caller, whose ownership and destruction policy remain unchanged.

use std::{ops::Deref, sync::Arc};

use crate::{Span, ast};

pub(super) struct Expr(Option<ast::Expr>);

impl Expr {
    pub(super) fn new(kind: ast::ExprKind, span: Span) -> Self {
        Self::from(ast::Expr::new(kind, span))
    }

    pub(super) fn into_ast(mut self) -> ast::Expr {
        self.0
            .take()
            .expect("parser owns expression until transfer")
    }

    pub(super) fn ast_mut(&mut self) -> &mut ast::Expr {
        self.0
            .as_mut()
            .expect("parser owns expression until transfer")
    }
}

impl From<ast::Expr> for Expr {
    fn from(expression: ast::Expr) -> Self {
        Self(Some(expression))
    }
}

impl Deref for Expr {
    type Target = ast::Expr;

    fn deref(&self) -> &Self::Target {
        self.0
            .as_ref()
            .expect("parser owns expression until transfer")
    }
}

impl Drop for Expr {
    fn drop(&mut self) {
        if let Some(expression) = self.0.take() {
            dispose(expression);
        }
    }
}

pub(super) struct Definition {
    pub(super) name: Arc<str>,
    pub(super) parameters: Vec<ast::FunctionParameter>,
    pub(super) body: Expr,
    pub(super) span: Span,
    pub(super) symbol: Option<u32>,
}

impl Definition {
    pub(super) fn into_ast(self) -> ast::Definition {
        ast::Definition {
            name: self.name,
            parameters: self.parameters,
            body: self.body.into_ast(),
            span: self.span,
            symbol: self.symbol,
        }
    }
}

pub(super) enum ObjectKey {
    Static(Arc<str>),
    Computed(Expr),
}

impl ObjectKey {
    pub(super) fn into_ast(self) -> ast::ObjectKey {
        match self {
            Self::Static(name) => ast::ObjectKey::Static(name),
            Self::Computed(expression) => ast::ObjectKey::Computed(expression.into_ast()),
        }
    }
}

pub(super) struct ObjectEntry {
    pub(super) key: ObjectKey,
    pub(super) value: Expr,
    pub(super) span: Span,
}

impl ObjectEntry {
    pub(super) fn into_ast(self) -> ast::ObjectEntry {
        ast::ObjectEntry {
            key: self.key.into_ast(),
            value: self.value.into_ast(),
            span: self.span,
        }
    }
}

pub(super) enum InterpolationSegment {
    Literal { value: Arc<str>, span: Span },
    Expression(Expr),
}

impl InterpolationSegment {
    pub(super) fn into_ast(self) -> ast::InterpolationSegment {
        match self {
            Self::Literal { value, span } => ast::InterpolationSegment::Literal { value, span },
            Self::Expression(expression) => {
                ast::InterpolationSegment::Expression(expression.into_ast())
            }
        }
    }
}

pub(super) fn boxed(expression: Expr) -> Box<ast::Expr> {
    Box::new(expression.into_ast())
}

#[allow(
    clippy::too_many_lines,
    reason = "exhaustively dismantles every AST child before normal drop glue runs"
)]
fn dispose(expression: ast::Expr) {
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match expression.kind {
            ast::ExprKind::Identity
            | ast::ExprKind::Literal(_)
            | ast::ExprKind::Variable(_)
            | ast::ExprKind::Empty
            | ast::ExprKind::RecursiveDescent
            | ast::ExprKind::Break { .. } => {}
            ast::ExprKind::Label { body, .. }
            | ast::ExprKind::Optional(body)
            | ast::ExprKind::Array(body)
            | ast::ExprKind::Unary {
                expression: body, ..
            } => pending.push(*body),
            ast::ExprKind::Pipe(left, right)
            | ast::ExprKind::Comma(left, right)
            | ast::ExprKind::Binary { left, right, .. }
            | ast::ExprKind::Assignment {
                path: left,
                value: right,
                ..
            }
            | ast::ExprKind::Bind {
                value: left,
                body: right,
                ..
            }
            | ast::ExprKind::BindAlternatives {
                value: left,
                body: right,
                ..
            } => {
                pending.push(*left);
                pending.push(*right);
            }
            ast::ExprKind::Interpolation(segments) => {
                for segment in segments {
                    if let ast::InterpolationSegment::Expression(expression) = segment {
                        pending.push(expression);
                    }
                }
            }
            ast::ExprKind::Access { base, access } => {
                pending.push(*base);
                match access {
                    ast::Access::Index(index) => pending.push(*index),
                    ast::Access::Slice { start, end } => {
                        pending.extend(start.map(|expression| *expression));
                        pending.extend(end.map(|expression| *expression));
                    }
                    ast::Access::Field(_) | ast::Access::Iterate => {}
                }
            }
            ast::ExprKind::Object(entries) => {
                for entry in entries {
                    if let ast::ObjectKey::Computed(key) = entry.key {
                        pending.push(key);
                    }
                    pending.push(entry.value);
                }
            }
            ast::ExprKind::Conditional {
                branches,
                alternative,
            } => {
                for (condition, body) in branches {
                    pending.push(condition);
                    pending.push(body);
                }
                pending.push(*alternative);
            }
            ast::ExprKind::Reduce {
                generator,
                initial,
                update,
                ..
            } => {
                pending.push(*generator);
                pending.push(*initial);
                pending.push(*update);
            }
            ast::ExprKind::Foreach {
                generator,
                initial,
                update,
                extract,
                ..
            } => {
                pending.push(*generator);
                pending.push(*initial);
                pending.push(*update);
                pending.extend(extract.map(|expression| *expression));
            }
            ast::ExprKind::Define { definition, body } => {
                pending.push(definition.body);
                pending.push(*body);
            }
            ast::ExprKind::Include { metadata, body, .. }
            | ast::ExprKind::Import { metadata, body, .. } => {
                pending.extend(metadata.map(|expression| *expression));
                pending.push(*body);
            }
            ast::ExprKind::Module { metadata, body } => {
                pending.push(*metadata);
                pending.push(*body);
            }
            ast::ExprKind::Call { arguments, .. } => pending.extend(arguments),
            ast::ExprKind::TryCatch { expression, catch } => {
                pending.push(*expression);
                pending.extend(catch.map(|expression| *expression));
            }
        }
    }
}
