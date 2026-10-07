//! Heap-backed expression continuations. A nested query suspends its caller
//! rather than adding another Rust call frame.

use super::{
    Access, AssignmentOperator, BinaryOperator, BindingPattern, Diagnostic, ExprKind,
    FunctionParameter, ParameterKind, Parser, Span, TokenKind, UnaryOperator, Value,
    filter_terminator, format, format_call, joined,
    owned::{Definition, Expr, InterpolationSegment, ObjectEntry, ObjectKey, boxed},
};
use std::sync::Arc;

#[derive(Clone, Copy)]
enum Level {
    Pipe,
    ObjectValue,
    FoldGenerator,
    Comma,
    Assignment,
    Binding,
    Alternative,
    Or,
    And,
    Comparison,
    Addition,
    Multiplication,
    Unary,
    Postfix,
    Primary,
}

impl Level {
    fn operand(self) -> Self {
        match self {
            Self::Pipe => Self::Comma,
            Self::ObjectValue | Self::Comma => Self::Assignment,
            Self::FoldGenerator | Self::Binding => Self::Alternative,
            Self::Assignment => Self::Binding,
            Self::Alternative => Self::Or,
            Self::Or => Self::And,
            Self::And => Self::Comparison,
            Self::Comparison => Self::Addition,
            Self::Addition => Self::Multiplication,
            Self::Multiplication => Self::Unary,
            Self::Unary => Self::Postfix,
            Self::Postfix => Self::Primary,
            Self::Primary => unreachable!("primary has no operand level"),
        }
    }
}

enum Operation {
    Pipe,
    Comma,
    Binary(BinaryOperator),
    Assignment(AssignmentOperator),
}

struct Template {
    open: Span,
    segments: Vec<InterpolationSegment>,
    format: Option<(Arc<str>, Span)>,
}

enum EntryKey {
    Field(Arc<str>, Span),
    Variable(Arc<str>, Span),
    Computed(Expr, Span),
}

enum Context {
    If(Span),
    Try(Span),
}

struct Fold {
    open: Span,
    foreach: bool,
    generator: Expr,
    pattern: BindingPattern,
}

enum Directive {
    Include(Arc<str>),
    Import { path: Arc<str>, alias: Arc<str> },
    Module,
}

enum Work {
    Template(Template),
    TemplateExpression(Template),
    Context(Context),
    IfCondition {
        open: Span,
        branches: Vec<(Expr, Expr)>,
    },
    IfBody {
        open: Span,
        branches: Vec<(Expr, Expr)>,
        condition: Expr,
    },
    IfEnd {
        open: Span,
        branches: Vec<(Expr, Expr)>,
    },
    Try(Span),
    Catch {
        open: Span,
        expression: Expr,
    },
    FoldGenerator {
        open: Span,
        foreach: bool,
    },
    FoldInitial(Fold),
    FoldUpdate {
        fold: Fold,
        initial: Expr,
    },
    FoldExtract {
        fold: Fold,
        initial: Expr,
        update: Expr,
    },
    Label {
        open: Span,
        name: Arc<str>,
    },
    DefinitionBody {
        open: Span,
        name: Arc<str>,
        parameters: Vec<FunctionParameter>,
    },
    DefinitionFollowing {
        open: Span,
        definition: Definition,
    },
    DirectiveMetadata {
        open: Span,
        directive: Directive,
    },
    DirectiveFollowing {
        open: Span,
        directive: Directive,
        metadata: Option<Expr>,
    },
    Parse(Level),
    Chain(Level),
    Combine {
        level: Level,
        left: Expr,
        operation: Operation,
    },
    Binding,
    Bind {
        value: Expr,
        patterns: Vec<BindingPattern>,
    },
    Unary(UnaryOperator, Span),
    Postfix,
    Group(Span),
    Array(Span),
    Call {
        name: Arc<str>,
        open: Span,
        arguments: Vec<Expr>,
    },
    Object {
        open: Span,
        entries: Vec<ObjectEntry>,
    },
    ObjectKey {
        open: Span,
        entries: Vec<ObjectEntry>,
        key_span: Span,
    },
    ObjectEntry {
        open: Span,
        entries: Vec<ObjectEntry>,
        key: ObjectKey,
        key_span: Span,
    },
    Index(Expr),

    SliceEnd {
        base: Expr,
        start: Option<Expr>,
    },
}

pub(super) fn expression(parser: &mut Parser<'_>) -> Result<super::Expr, Box<Diagnostic>> {
    let mut work = vec![Work::Parse(Level::Pipe)];
    let mut values = Vec::new();
    while let Some(next) = work.pop() {
        if let Err(mut error) = step(parser, next, &mut work, &mut values) {
            for continuation in work.iter().rev() {
                if let Work::Context(context) = continuation {
                    error = match context {
                        Context::If(open) => parser.with_if_context(error, *open),
                        Context::Try(open) => Parser::with_context(error, *open, "try expression"),
                    };
                }
            }
            return Err(error);
        }
    }
    Ok(values.pop().expect("completed expression").into_ast())
}

fn value(values: &mut Vec<Expr>) -> Expr {
    values
        .pop()
        .expect("continuation receives its parsed operand")
}

fn descend(work: &mut Vec<Work>, continuation: Work, level: Level) {
    work.push(continuation);
    work.push(Work::Parse(level));
}

fn operation(level: Level, token: &TokenKind) -> Option<Operation> {
    use BinaryOperator as B;
    use TokenKind as T;
    let binary = match (level, token) {
        (Level::Pipe | Level::ObjectValue | Level::FoldGenerator, T::Pipe) => {
            return Some(Operation::Pipe);
        }
        (Level::Comma, T::Comma) => return Some(Operation::Comma),
        (Level::Assignment, token) => {
            return match token {
                T::Assign => Some(Operation::Assignment(AssignmentOperator::Set)),
                T::Update => Some(Operation::Assignment(AssignmentOperator::Update)),
                T::AddUpdate => Some(Operation::Assignment(AssignmentOperator::Add)),
                T::SubtractUpdate => Some(Operation::Assignment(AssignmentOperator::Subtract)),
                T::MultiplyUpdate => Some(Operation::Assignment(AssignmentOperator::Multiply)),
                T::DivideUpdate => Some(Operation::Assignment(AssignmentOperator::Divide)),
                T::AlternativeUpdate => {
                    Some(Operation::Assignment(AssignmentOperator::Alternative))
                }
                _ => None,
            };
        }
        (Level::Alternative, T::Alternative) => B::Alternative,
        (Level::Or, T::Or) => B::Or,
        (Level::And, T::And) => B::And,
        (Level::Comparison, T::Equal) => B::Equal,
        (Level::Comparison, T::NotEqual) => B::NotEqual,
        (Level::Comparison, T::Less) => B::Less,
        (Level::Comparison, T::LessEqual) => B::LessEqual,
        (Level::Comparison, T::Greater) => B::Greater,
        (Level::Comparison, T::GreaterEqual) => B::GreaterEqual,
        (Level::Addition, T::Plus) => B::Add,
        (Level::Addition, T::Minus) => B::Subtract,
        (Level::Multiplication, T::Star) => B::Multiply,
        (Level::Multiplication, T::Slash) => B::Divide,
        (Level::Multiplication, T::Percent) => B::Remainder,
        _ => return None,
    };
    Some(Operation::Binary(binary))
}

fn finish_call(
    parser: &mut Parser<'_>,
    name: Arc<str>,
    open: Span,
    arguments: Vec<Expr>,
    values: &mut Vec<Expr>,
) -> Result<(), Box<Diagnostic>> {
    let close = parser.expect(
        |kind| matches!(kind, TokenKind::RightParen),
        "')' after function arguments",
    )?;
    values.push(Expr::new(
        ExprKind::Call {
            name,
            arguments: arguments.into_iter().map(Expr::into_ast).collect(),
            target: None,
        },
        joined(open, close.span),
    ));
    Ok(())
}

fn push_access(values: &mut Vec<Expr>, base: Expr, access: Access, end: Span) {
    let span = joined(base.span, end);
    values.push(Expr::new(
        ExprKind::Access {
            base: Box::new(base.into_ast()),
            access,
        },
        span,
    ));
}

fn slice_end(
    parser: &mut Parser<'_>,
    work: &mut Vec<Work>,
    values: &mut Vec<Expr>,
    base: Expr,
    start: Option<Expr>,
) {
    if let Some(close) = parser.take(|kind| matches!(kind, TokenKind::RightBracket)) {
        push_access(
            values,
            base,
            Access::Slice {
                start: start.map(boxed),
                end: None,
            },
            close.span,
        );
    } else {
        descend(work, Work::SliceEnd { base, start }, Level::Pipe);
    }
}

fn finish_object(
    parser: &mut Parser<'_>,
    values: &mut Vec<Expr>,
    open: Span,
    entries: Vec<ObjectEntry>,
) -> Result<(), Box<Diagnostic>> {
    let close = parser.expect(
        |kind| matches!(kind, TokenKind::RightBrace),
        "'}' after object constructor",
    )?;
    values.push(Expr::new(
        ExprKind::Object(entries.into_iter().map(ObjectEntry::into_ast).collect()),
        joined(open, close.span),
    ));
    Ok(())
}

fn object_next(
    parser: &mut Parser<'_>,
    work: &mut Vec<Work>,
    values: &mut Vec<Expr>,
    open: Span,
    entries: Vec<ObjectEntry>,
) -> Result<(), Box<Diagnostic>> {
    if matches!(parser.current().kind, TokenKind::RightBrace) {
        return finish_object(parser, values, open, entries);
    }
    let token = parser.advance().clone();
    let variable = matches!(token.kind, TokenKind::Variable(_));
    match token.kind {
        TokenKind::Identifier(name) | TokenKind::String(name) | TokenKind::Variable(name) => {
            object_value(
                parser,
                work,
                values,
                open,
                entries,
                if variable {
                    EntryKey::Variable(name, token.span)
                } else {
                    EntryKey::Field(name, token.span)
                },
            )?;
        }
        TokenKind::LeftParen => descend(
            work,
            Work::ObjectKey {
                open,
                entries,
                key_span: token.span,
            },
            Level::Pipe,
        ),
        _ => return Err(parser.error_at("TQ-PARSE-OBJECT-001", "expected object key", token.span)),
    }
    Ok(())
}

fn object_value(
    parser: &mut Parser<'_>,
    work: &mut Vec<Work>,
    values: &mut Vec<Expr>,
    open: Span,
    entries: Vec<ObjectEntry>,
    entry_key: EntryKey,
) -> Result<(), Box<Diagnostic>> {
    let (mut key, key_span, shorthand, variable) = match entry_key {
        EntryKey::Field(name, span) => (
            ObjectKey::Static(Arc::clone(&name)),
            span,
            Some(name),
            false,
        ),
        EntryKey::Variable(name, span) => {
            (ObjectKey::Static(Arc::clone(&name)), span, Some(name), true)
        }
        EntryKey::Computed(expression, span) => {
            (ObjectKey::Computed(expression), span, None, false)
        }
    };
    if parser
        .take(|kind| matches!(kind, TokenKind::Colon))
        .is_some()
    {
        if variable {
            let ObjectKey::Static(name) = &key else {
                unreachable!("variable key is static before colon")
            };
            key = ObjectKey::Computed(Expr::new(ExprKind::Variable(Arc::clone(name)), key_span));
        }
        if matches!(parser.current().kind, TokenKind::RightBrace) {
            return Err(Parser::with_context(
                parser.unexpected("object value"),
                parser.current().span,
                "object value",
            ));
        }
        descend(
            work,
            Work::ObjectEntry {
                open,
                entries,
                key,
                key_span,
            },
            Level::ObjectValue,
        );
    } else if let Some(name) = shorthand {
        let kind = if variable {
            ExprKind::Variable(name)
        } else {
            ExprKind::Access {
                base: Box::new(Expr::new(ExprKind::Identity, key_span).into_ast()),
                access: Access::Field(name),
            }
        };
        values.push(Expr::new(kind, key_span));
        work.push(Work::ObjectEntry {
            open,
            entries,
            key,
            key_span,
        });
    } else {
        return Err(parser.unexpected("':' after computed object key"));
    }
    Ok(())
}

fn finish_fold(
    parser: &mut Parser<'_>,
    values: &mut Vec<Expr>,
    fold: Fold,
    initial: Expr,
    update: Expr,
    extract: Option<Expr>,
) -> Result<(), Box<Diagnostic>> {
    let close = parser.expect(
        |kind| matches!(kind, TokenKind::RightParen),
        "')' after fold body",
    )?;
    let kind = if fold.foreach {
        ExprKind::Foreach {
            generator: Box::new(fold.generator.into_ast()),
            pattern: fold.pattern,
            initial: Box::new(initial.into_ast()),
            update: Box::new(update.into_ast()),
            extract: extract.map(boxed),
        }
    } else {
        ExprKind::Reduce {
            generator: Box::new(fold.generator.into_ast()),
            pattern: fold.pattern,
            initial: Box::new(initial.into_ast()),
            update: Box::new(update.into_ast()),
        }
    };
    values.push(Expr::new(kind, joined(fold.open, close.span)));
    Ok(())
}

fn following(
    parser: &Parser<'_>,
    work: &mut Vec<Work>,
    values: &mut Vec<Expr>,
    continuation: Work,
    empty_span: Span,
) {
    if matches!(parser.current().kind, TokenKind::EndOfInput) {
        values.push(Expr::new(ExprKind::Empty, empty_span));
        work.push(continuation);
    } else {
        descend(work, continuation, Level::Pipe);
    }
}

fn definition_header(parser: &mut Parser<'_>, work: &mut Vec<Work>) -> Result<(), Box<Diagnostic>> {
    let open = parser.advance().span;
    let name = parser.advance().clone();
    let TokenKind::Identifier(name_value) = name.kind else {
        return Err(Parser::with_context(
            parser.error_at(
                "TQ-PARSE-DEF-001",
                "expected filter name after 'def'",
                name.span,
            ),
            open,
            "definition context",
        ));
    };
    let mut parameters = Vec::new();
    if parser
        .take(|kind| matches!(kind, TokenKind::LeftParen))
        .is_some()
    {
        if !matches!(parser.current().kind, TokenKind::RightParen) {
            loop {
                let parameter = parser.advance().clone();
                let (name, kind) = match parameter.kind {
                    TokenKind::Identifier(name) => (name, ParameterKind::Filter),
                    TokenKind::Variable(name) => (name, ParameterKind::Value),
                    _ => {
                        return Err(parser.error_at(
                            "TQ-PARSE-DEF-PARAMETER-001",
                            "expected filter or value parameter",
                            parameter.span,
                        ));
                    }
                };
                parameters.push(FunctionParameter {
                    name,
                    kind,
                    span: parameter.span,
                    runtime_name: None,
                });
                if parser
                    .take(|kind| matches!(kind, TokenKind::Semicolon))
                    .is_none()
                {
                    break;
                }
            }
        }
        parser.expect(
            |kind| matches!(kind, TokenKind::RightParen),
            "')' after definition parameters",
        )?;
    }
    parser.expect(
        |kind| matches!(kind, TokenKind::Colon),
        "':' before definition body",
    )?;
    descend(
        work,
        Work::DefinitionBody {
            open,
            name: name_value,
            parameters,
        },
        Level::Pipe,
    );
    Ok(())
}

fn directive_header(
    parser: &mut Parser<'_>,
    work: &mut Vec<Work>,
    values: &mut Vec<Expr>,
) -> Result<(), Box<Diagnostic>> {
    let token = parser.advance().clone();
    let directive = match token.kind {
        TokenKind::Include => Directive::Include(parser.module_path("after 'include'")?),
        TokenKind::Import => {
            let path = parser.module_path("after 'import'")?;
            parser.expect(
                |kind| matches!(kind, TokenKind::As),
                "'as' after import path",
            )?;
            let alias = parser.advance().clone();
            let alias = match alias.kind {
                TokenKind::Identifier(alias) => alias,
                TokenKind::Variable(alias) => Arc::from(format!("${alias}")),
                _ => {
                    return Err(parser.error_at(
                        "TQ-PARSE-IMPORT-001",
                        "expected module alias after 'as'",
                        alias.span,
                    ));
                }
            };
            Directive::Import { path, alias }
        }
        TokenKind::Module => Directive::Module,
        _ => unreachable!("directive opener"),
    };
    if !matches!(directive, Directive::Module)
        && matches!(parser.current().kind, TokenKind::Semicolon)
    {
        return directive_following(parser, work, values, token.span, directive, None);
    }
    descend(
        work,
        Work::DirectiveMetadata {
            open: token.span,
            directive,
        },
        Level::Pipe,
    );
    Ok(())
}

fn directive_following(
    parser: &mut Parser<'_>,
    work: &mut Vec<Work>,
    values: &mut Vec<Expr>,
    open: Span,
    directive: Directive,
    metadata: Option<Expr>,
) -> Result<(), Box<Diagnostic>> {
    let expected = match directive {
        Directive::Include(_) => "';' after include directive",
        Directive::Import { .. } => "';' after import directive",
        Directive::Module => "';' after module metadata",
    };
    let semicolon = parser.expect(|kind| matches!(kind, TokenKind::Semicolon), expected)?;
    following(
        parser,
        work,
        values,
        Work::DirectiveFollowing {
            open,
            directive,
            metadata,
        },
        semicolon.span,
    );
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    reason = "primary forms dispatch their delimiter-specific continuations"
)]
fn primary(
    parser: &mut Parser<'_>,
    work: &mut Vec<Work>,
    values: &mut Vec<Expr>,
) -> Result<(), Box<Diagnostic>> {
    match parser.current().kind.clone() {
        TokenKind::LeftParen => {
            let open = parser.advance().span;
            descend(work, Work::Group(open), Level::Pipe);
        }
        TokenKind::LeftBracket => {
            let open = parser.advance().span;
            if matches!(parser.current().kind, TokenKind::RightBracket) {
                values.push(Expr::new(ExprKind::Empty, parser.current().span));
                work.push(Work::Array(open));
            } else {
                descend(work, Work::Array(open), Level::Pipe);
            }
        }
        TokenKind::LeftBrace => {
            let open = parser.advance().span;
            work.push(Work::Object {
                open,
                entries: Vec::new(),
            });
        }
        TokenKind::Identifier(name) => {
            let open = parser.advance().span;
            if &*name == "empty" && !matches!(parser.current().kind, TokenKind::LeftParen) {
                values.push(Expr::new(ExprKind::Empty, open));
            } else if parser
                .take(|kind| matches!(kind, TokenKind::LeftParen))
                .is_some()
            {
                if matches!(parser.current().kind, TokenKind::RightParen) {
                    finish_call(parser, name, open, Vec::new(), values)?;
                } else {
                    descend(
                        work,
                        Work::Call {
                            name,
                            open,
                            arguments: Vec::new(),
                        },
                        Level::Pipe,
                    );
                }
            } else {
                values.push(Expr::new(
                    ExprKind::Call {
                        name,
                        arguments: Vec::new(),
                        target: None,
                    },
                    open,
                ));
            }
        }
        TokenKind::StringStart => {
            let open = parser.advance().span;
            work.push(Work::Template(Template {
                open,
                segments: Vec::new(),
                format: None,
            }));
        }
        TokenKind::Format(name) => {
            let span = parser.advance().span;
            if !format::is_supported(&name) {
                return Err(parser.error_at(
                    "TQ-RESOLVE-FORMAT-001",
                    &format!("unknown jq format {name:?}"),
                    span,
                ));
            }
            match parser.current().kind.clone() {
                TokenKind::String(value) => {
                    let end = parser.advance().span;
                    values.push(Expr::new(
                        ExprKind::Literal(Value::string(value)),
                        joined(span, end),
                    ));
                }
                TokenKind::StringStart => {
                    let open = parser.advance().span;
                    work.push(Work::Template(Template {
                        open,
                        segments: Vec::new(),
                        format: Some((name, span)),
                    }));
                }
                _ => values.push(format_call(name, span).into()),
            }
        }
        TokenKind::If => {
            let open = parser.advance().span;
            work.push(Work::Context(Context::If(open)));
            descend(
                work,
                Work::IfCondition {
                    open,
                    branches: Vec::new(),
                },
                Level::Pipe,
            );
        }
        TokenKind::Try => {
            let open = parser.advance().span;
            work.push(Work::Try(open));
            work.push(Work::Context(Context::Try(open)));
            work.push(Work::Parse(Level::Assignment));
        }
        TokenKind::Reduce | TokenKind::Foreach => {
            let token = parser.advance().clone();
            descend(
                work,
                Work::FoldGenerator {
                    open: token.span,
                    foreach: matches!(token.kind, TokenKind::Foreach),
                },
                Level::FoldGenerator,
            );
        }
        TokenKind::Label => {
            let open = parser.advance().span;
            let variable = parser.advance().clone();
            let TokenKind::Variable(name) = variable.kind else {
                return Err(Parser::with_context(
                    parser.error_at(
                        "TQ-PARSE-LABEL-001",
                        "expected label variable after 'label'",
                        variable.span,
                    ),
                    open,
                    "label context",
                ));
            };
            parser.expect(
                |kind| matches!(kind, TokenKind::Pipe),
                "'|' after label variable",
            )?;
            descend(work, Work::Label { open, name }, Level::Pipe);
        }
        TokenKind::Def => definition_header(parser, work)?,
        TokenKind::Include | TokenKind::Import | TokenKind::Module => {
            directive_header(parser, work, values)?;
        }
        _ => values.push(parser.atom()?.into()),
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    reason = "exhaustive transitions retain the grammar and diagnostic unwind order in one table"
)]
fn step(
    parser: &mut Parser<'_>,
    next: Work,
    work: &mut Vec<Work>,
    values: &mut Vec<Expr>,
) -> Result<(), Box<Diagnostic>> {
    match next {
        Work::Template(mut template) => loop {
            let token = parser.advance().clone();
            match token.kind {
                TokenKind::StringFragment(value) => {
                    template.segments.push(InterpolationSegment::Literal {
                        value,
                        span: token.span,
                    });
                }
                TokenKind::InterpolationStart => {
                    if matches!(parser.current().kind, TokenKind::InterpolationEnd) {
                        return Err(parser.error_at(
                            "TQ-PARSE-INTERPOLATION-001",
                            "expected filter expression in string interpolation",
                            parser.current().span,
                        ));
                    }
                    descend(work, Work::TemplateExpression(template), Level::Pipe);
                    break;
                }
                TokenKind::StringEnd => {
                    let start = template
                        .format
                        .as_ref()
                        .map_or(template.open, |(_, span)| *span);
                    values.push(Expr::new(
                        ExprKind::Interpolation(
                            template
                                .segments
                                .into_iter()
                                .map(InterpolationSegment::into_ast)
                                .collect(),
                        ),
                        joined(start, token.span),
                    ));
                    break;
                }
                _ => {
                    return Err(parser.error_at(
                        "TQ-PARSE-INTERPOLATION-001",
                        "expected interpolation segment or closing quote",
                        token.span,
                    ));
                }
            }
        },
        Work::TemplateExpression(mut template) => {
            let mut expression = value(values);
            parser.expect(
                |kind| matches!(kind, TokenKind::InterpolationEnd),
                "')' after string interpolation",
            )?;
            if let Some((name, span)) = &template.format {
                let end = expression.span;
                expression = Expr::new(
                    ExprKind::Pipe(
                        Box::new(expression.into_ast()),
                        Box::new(format_call(Arc::clone(name), *span)),
                    ),
                    joined(*span, end),
                );
            }
            template
                .segments
                .push(InterpolationSegment::Expression(expression));
            work.push(Work::Template(template));
        }
        Work::Context(_) => {}
        Work::IfCondition { open, branches } => {
            let condition = value(values);
            parser
                .expect(|kind| matches!(kind, TokenKind::Then), "'then'")
                .map_err(|error| Parser::with_context(error, condition.span, "if condition"))?;
            descend(
                work,
                Work::IfBody {
                    open,
                    branches,
                    condition,
                },
                Level::Pipe,
            );
        }
        Work::IfBody {
            open,
            mut branches,
            condition,
        } => {
            branches.push((condition, value(values)));
            if parser
                .take(|kind| matches!(kind, TokenKind::Elif))
                .is_some()
            {
                descend(work, Work::IfCondition { open, branches }, Level::Pipe);
            } else if parser
                .take(|kind| matches!(kind, TokenKind::Else))
                .is_some()
            {
                descend(work, Work::IfEnd { open, branches }, Level::Pipe);
            } else {
                values.push(Expr::new(ExprKind::Identity, parser.current().span));
                work.push(Work::IfEnd { open, branches });
            }
        }
        Work::IfEnd { open, branches } => {
            let alternative = value(values);
            let end = parser.expect(|kind| matches!(kind, TokenKind::End), "'end'")?;
            values.push(Expr::new(
                ExprKind::Conditional {
                    branches: branches
                        .into_iter()
                        .map(|(condition, body)| (condition.into_ast(), body.into_ast()))
                        .collect(),
                    alternative: Box::new(alternative.into_ast()),
                },
                joined(open, end.span),
            ));
        }
        Work::Try(open) => {
            let expression = value(values);
            if parser
                .take(|kind| matches!(kind, TokenKind::Catch))
                .is_some()
            {
                descend(work, Work::Catch { open, expression }, Level::Assignment);
            } else {
                let end = expression.span;
                values.push(Expr::new(
                    ExprKind::TryCatch {
                        expression: Box::new(expression.into_ast()),
                        catch: None,
                    },
                    joined(open, end),
                ));
            }
        }
        Work::Catch { open, expression } => {
            let catch = value(values);
            let end = catch.span;
            values.push(Expr::new(
                ExprKind::TryCatch {
                    expression: Box::new(expression.into_ast()),
                    catch: Some(Box::new(catch.into_ast())),
                },
                joined(open, end),
            ));
        }
        Work::FoldGenerator { open, foreach } => {
            let generator = value(values);
            parser.expect(
                |kind| matches!(kind, TokenKind::As),
                "'as' after fold generator",
            )?;
            let pattern = parser.binding_pattern()?;
            parser.expect(
                |kind| matches!(kind, TokenKind::LeftParen),
                "'(' before fold initializer",
            )?;
            descend(
                work,
                Work::FoldInitial(Fold {
                    open,
                    foreach,
                    generator,
                    pattern,
                }),
                Level::Pipe,
            );
        }
        Work::FoldInitial(fold) => {
            let initial = value(values);
            parser.expect(
                |kind| matches!(kind, TokenKind::Semicolon),
                "';' after fold initializer",
            )?;
            descend(work, Work::FoldUpdate { fold, initial }, Level::Pipe);
        }
        Work::FoldUpdate { fold, initial } => {
            let update = value(values);
            if fold.foreach && !matches!(parser.current().kind, TokenKind::RightParen) {
                parser.expect(
                    |kind| matches!(kind, TokenKind::Semicolon),
                    "';' after foreach update",
                )?;
                descend(
                    work,
                    Work::FoldExtract {
                        fold,
                        initial,
                        update,
                    },
                    Level::Pipe,
                );
            } else {
                finish_fold(parser, values, fold, initial, update, None)?;
            }
        }
        Work::FoldExtract {
            fold,
            initial,
            update,
        } => {
            let extract = Some(value(values));
            finish_fold(parser, values, fold, initial, update, extract)?;
        }
        Work::Label { open, name } => {
            let body = value(values);
            let span = joined(open, body.span);
            values.push(Expr::new(
                ExprKind::Label {
                    name,
                    symbol: None,
                    body: Box::new(body.into_ast()),
                },
                span,
            ));
        }
        Work::DefinitionBody {
            open,
            name,
            parameters,
        } => {
            let body = value(values);
            let semicolon = parser.expect(
                |kind| matches!(kind, TokenKind::Semicolon),
                "';' after definition body",
            )?;
            let definition = Definition {
                name,
                parameters,
                span: joined(open, body.span),
                body,
                symbol: None,
            };
            following(
                parser,
                work,
                values,
                Work::DefinitionFollowing { open, definition },
                semicolon.span,
            );
        }
        Work::DefinitionFollowing { open, definition } => {
            let body = value(values);
            let span = joined(open, body.span);
            values.push(Expr::new(
                ExprKind::Define {
                    definition: Box::new(definition.into_ast()),
                    body: Box::new(body.into_ast()),
                },
                span,
            ));
        }
        Work::DirectiveMetadata { open, directive } => {
            let metadata = Some(value(values));
            directive_following(parser, work, values, open, directive, metadata)?;
        }
        Work::DirectiveFollowing {
            open,
            directive,
            metadata,
        } => {
            let body = value(values);
            let span = joined(open, body.span);
            let kind = match directive {
                Directive::Include(path) => ExprKind::Include {
                    path,
                    metadata: metadata.map(boxed),
                    body: Box::new(body.into_ast()),
                },
                Directive::Import { path, alias } => ExprKind::Import {
                    path,
                    alias,
                    metadata: metadata.map(boxed),
                    body: Box::new(body.into_ast()),
                },
                Directive::Module => ExprKind::Module {
                    metadata: boxed(metadata.expect("module always parses metadata")),
                    body: Box::new(body.into_ast()),
                },
            };
            values.push(Expr::new(kind, span));
        }
        Work::Parse(Level::Primary) => primary(parser, work, values)?,
        Work::Parse(Level::Unary) => {
            let operator = match parser.current().kind {
                TokenKind::Not => Some(UnaryOperator::Not),
                TokenKind::Minus => Some(UnaryOperator::Negate),
                _ => None,
            };
            if let Some(operator) = operator {
                let open = parser.advance().span;
                work.push(Work::Unary(operator, open));
                if operator == UnaryOperator::Not && filter_terminator(&parser.current().kind) {
                    values.push(Expr::new(ExprKind::Identity, open));
                } else {
                    work.push(Work::Parse(Level::Unary));
                }
            } else {
                work.push(Work::Parse(Level::Postfix));
            }
        }
        Work::Parse(Level::Postfix) => descend(work, Work::Postfix, Level::Primary),
        Work::Parse(Level::Binding) => descend(work, Work::Binding, Level::Alternative),
        Work::Parse(level) => descend(work, Work::Chain(level), level.operand()),
        Work::Chain(level) => {
            if let Some(operation) = operation(level, &parser.current().kind) {
                parser.advance();
                let left = value(values);
                let right_level = if matches!(level, Level::Assignment) {
                    level
                } else {
                    level.operand()
                };
                descend(
                    work,
                    Work::Combine {
                        level,
                        left,
                        operation,
                    },
                    right_level,
                );
            }
        }
        Work::Combine {
            level,
            left,
            operation,
        } => {
            let right = value(values);
            let span = joined(left.span, right.span);
            let kind = match operation {
                Operation::Pipe => {
                    ExprKind::Pipe(Box::new(left.into_ast()), Box::new(right.into_ast()))
                }
                Operation::Comma => {
                    ExprKind::Comma(Box::new(left.into_ast()), Box::new(right.into_ast()))
                }
                Operation::Binary(operator) => ExprKind::Binary {
                    operator,
                    left: Box::new(left.into_ast()),
                    right: Box::new(right.into_ast()),
                },
                Operation::Assignment(operator) => ExprKind::Assignment {
                    operator,
                    path: Box::new(left.into_ast()),
                    value: Box::new(right.into_ast()),
                },
            };
            values.push(Expr::new(kind, span));
            if !matches!(level, Level::Assignment) {
                work.push(Work::Chain(level));
            }
        }
        Work::Binding => {
            if parser.take(|kind| matches!(kind, TokenKind::As)).is_some() {
                let mut patterns = vec![parser.binding_pattern()?];
                while parser
                    .take(|kind| matches!(kind, TokenKind::Question))
                    .is_some()
                {
                    parser.expect(
                        |kind| matches!(kind, TokenKind::Alternative),
                        "'//' after '?' in destructuring alternative",
                    )?;
                    patterns.push(parser.binding_pattern()?);
                }
                parser.expect(
                    |kind| matches!(kind, TokenKind::Pipe),
                    "'|' after binding pattern",
                )?;
                let value = value(values);
                descend(work, Work::Bind { value, patterns }, Level::Pipe);
            }
        }
        Work::Bind {
            value: input,
            mut patterns,
        } => {
            let body = value(values);
            let span = joined(input.span, body.span);
            let kind = if patterns.len() == 1 {
                ExprKind::Bind {
                    value: Box::new(input.into_ast()),
                    pattern: patterns.pop().expect("one pattern"),
                    body: Box::new(body.into_ast()),
                }
            } else {
                ExprKind::BindAlternatives {
                    value: Box::new(input.into_ast()),
                    patterns,
                    body: Box::new(body.into_ast()),
                }
            };
            values.push(Expr::new(kind, span));
        }
        Work::Unary(operator, open) => {
            let expression = value(values);
            let span = joined(open, expression.span);
            values.push(Expr::new(
                ExprKind::Unary {
                    operator,
                    expression: Box::new(expression.into_ast()),
                },
                span,
            ));
        }
        Work::Group(open) => {
            let expression = value(values);
            let close = parser.expect(
                |kind| matches!(kind, TokenKind::RightParen),
                "')' after grouped expression",
            )?;
            values.push(Expr::new(
                expression.into_ast().kind,
                joined(open, close.span),
            ));
        }
        Work::Array(open) => {
            let body = value(values);
            let close = parser.expect(
                |kind| matches!(kind, TokenKind::RightBracket),
                "']' after array constructor",
            )?;
            values.push(Expr::new(
                ExprKind::Array(Box::new(body.into_ast())),
                joined(open, close.span),
            ));
        }
        Work::Call {
            name,
            open,
            mut arguments,
        } => {
            arguments.push(value(values));
            if parser
                .take(|kind| matches!(kind, TokenKind::Semicolon))
                .is_some()
            {
                descend(
                    work,
                    Work::Call {
                        name,
                        open,
                        arguments,
                    },
                    Level::Pipe,
                );
            } else {
                finish_call(parser, name, open, arguments, values)?;
            }
        }
        Work::Object { open, entries } => object_next(parser, work, values, open, entries)?,
        Work::ObjectKey {
            open,
            entries,
            key_span,
        } => {
            let key = value(values);
            parser.expect(
                |kind| matches!(kind, TokenKind::RightParen),
                "')' after computed object key",
            )?;
            object_value(
                parser,
                work,
                values,
                open,
                entries,
                EntryKey::Computed(key, key_span),
            )?;
        }
        Work::ObjectEntry {
            open,
            mut entries,
            key,
            key_span,
        } => {
            let value = value(values);
            let span = joined(key_span, value.span);
            entries.push(ObjectEntry { key, value, span });
            if parser
                .take(|kind| matches!(kind, TokenKind::Comma))
                .is_some()
            {
                work.push(Work::Object { open, entries });
            } else {
                finish_object(parser, values, open, entries)?;
            }
        }
        Work::Index(base) => {
            let start = value(values);
            if parser
                .take(|kind| matches!(kind, TokenKind::Colon))
                .is_some()
            {
                slice_end(parser, work, values, base, Some(start));
            } else {
                let close = parser.expect(
                    |kind| matches!(kind, TokenKind::RightBracket),
                    "']' after index or slice",
                )?;
                push_access(values, base, Access::Index(boxed(start)), close.span);
            }
        }

        Work::SliceEnd { base, start } => {
            let end = Some(value(values));
            let close = parser.expect(
                |kind| matches!(kind, TokenKind::RightBracket),
                "']' after index or slice",
            )?;
            push_access(
                values,
                base,
                Access::Slice {
                    start: start.map(boxed),
                    end: end.map(boxed),
                },
                close.span,
            );
        }
        Work::Postfix => {
            let mut expression = value(values);
            loop {
                if let Some(question) = parser.take(|kind| matches!(kind, TokenKind::Question)) {
                    let span = joined(expression.span, question.span);
                    expression =
                        Expr::new(ExprKind::Optional(Box::new(expression.into_ast())), span);
                } else if parser.take(|kind| matches!(kind, TokenKind::Dot)).is_some() {
                    let field = parser.advance().clone();
                    let name = match field.kind {
                        TokenKind::Identifier(name) if !name.contains("::") => name,
                        TokenKind::String(name) => name,
                        TokenKind::Identifier(_) => {
                            return Err(parser.error_at(
                                "TQ-PARSE-FIELD-001",
                                "field names cannot use namespace separators",
                                field.span,
                            ));
                        }
                        _ => {
                            return Err(parser.error_at(
                                "TQ-PARSE-FIELD-001",
                                "expected field name after '.'",
                                field.span,
                            ));
                        }
                    };
                    let span = joined(expression.span, field.span);
                    expression = Expr::new(
                        ExprKind::Access {
                            base: Box::new(expression.into_ast()),
                            access: Access::Field(name),
                        },
                        span,
                    );
                } else if parser
                    .take(|kind| matches!(kind, TokenKind::LeftBracket))
                    .is_some()
                {
                    work.push(Work::Postfix);
                    if let Some(close) = parser.take(|kind| matches!(kind, TokenKind::RightBracket))
                    {
                        push_access(values, expression, Access::Iterate, close.span);
                    } else if parser
                        .take(|kind| matches!(kind, TokenKind::Colon))
                        .is_some()
                    {
                        slice_end(parser, work, values, expression, None);
                    } else {
                        descend(work, Work::Index(expression), Level::Pipe);
                    }
                    return Ok(());
                } else {
                    break;
                }
            }
            values.push(expression);
        }
    }
    Ok(())
}
