//! Recursive-descent jq MVP parser with explicit precedence.

use std::sync::Arc;

use crate::{
    Diagnostic, DiagnosticClass, Number, Parsed, Query, SourceFile, SourceId, Span, Value,
    ast::{
        Access, AssignmentOperator, BinaryOperator, Definition, Expr, ExprKind, FunctionParameter,
        InterpolationSegment, ObjectEntry, ObjectKey, ParameterKind, UnaryOperator,
    },
    lexer::{Token, TokenKind, lex, validate_utf8},
};

/// Parses a UTF-8 jq filter into the parsed typestate phase.
///
/// # Errors
///
/// Returns a source-spanned lexical, syntax, numeric, or deferred-capability diagnostic.
pub fn parse(source: &str) -> Result<Query<Parsed>, Box<Diagnostic>> {
    parse_named("<query>", source)
}

/// Parses named UTF-8 query bytes, rejecting invalid UTF-8 without panicking.
///
/// # Errors
///
/// Returns a source-spanned lexical, syntax, numeric, or deferred-capability diagnostic.
pub fn parse_bytes(name: &str, bytes: &[u8]) -> Result<Query<Parsed>, Box<Diagnostic>> {
    parse_named(name, validate_utf8(bytes, name)?)
}

fn parse_named(name: &str, text: &str) -> Result<Query<Parsed>, Box<Diagnostic>> {
    let source = SourceFile::new(SourceId::new(0), name, text);
    let ast = parse_source(&source)?;
    Ok(Query::from_ast(source, ast))
}

pub(crate) fn parse_module_ast(
    name: &str,
    text: &str,
    source_id: SourceId,
) -> Result<Expr, Box<Diagnostic>> {
    parse_source(&SourceFile::new(source_id, name, text))
}

fn parse_source(source: &SourceFile) -> Result<Expr, Box<Diagnostic>> {
    let tokens = lex(source)?;
    Parser {
        source,
        tokens,
        index: 0,
    }
    .complete()
}

struct Parser<'a> {
    source: &'a SourceFile,
    tokens: Vec<Token>,
    index: usize,
}

impl Parser<'_> {
    fn complete(mut self) -> Result<Expr, Box<Diagnostic>> {
        if let TokenKind::Deferred(capability) = &self.current().kind {
            return Err(self.deferred(capability, self.current().span));
        }
        let expression = self.comma()?;
        if !matches!(self.current().kind, TokenKind::EndOfInput) {
            return Err(self.unexpected("end of query"));
        }
        Ok(expression)
    }

    fn comma(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let mut expression = self.assignment()?;
        while self.take(|kind| matches!(kind, TokenKind::Comma)).is_some() {
            let right = self.assignment()?;
            let span = joined(expression.span, right.span);
            expression = Expr::new(ExprKind::Comma(Box::new(expression), Box::new(right)), span);
        }
        Ok(expression)
    }

    fn assignment(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let left = self.binding()?;
        let operator = match self.current().kind {
            TokenKind::Assign => AssignmentOperator::Set,
            TokenKind::Update => AssignmentOperator::Update,
            TokenKind::AddUpdate => AssignmentOperator::Add,
            TokenKind::SubtractUpdate => AssignmentOperator::Subtract,
            TokenKind::MultiplyUpdate => AssignmentOperator::Multiply,
            TokenKind::DivideUpdate => AssignmentOperator::Divide,
            TokenKind::AlternativeUpdate => AssignmentOperator::Alternative,
            _ => return Ok(left),
        };
        self.advance();
        let right = self.assignment()?;
        let span = joined(left.span, right.span);
        Ok(Expr::new(
            ExprKind::Assignment {
                operator,
                path: Box::new(left),
                value: Box::new(right),
            },
            span,
        ))
    }

    fn binding(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let value = self.pipe()?;
        if self.take(|kind| matches!(kind, TokenKind::As)).is_none() {
            return Ok(value);
        }
        let variable = self.advance().clone();
        let TokenKind::Variable(name) = variable.kind else {
            return Err(self.error_at(
                "TQ-PARSE-BIND-001",
                "expected variable after 'as'",
                variable.span,
            ));
        };
        self.expect(
            |kind| matches!(kind, TokenKind::Pipe),
            "'|' after variable binding",
        )?;
        let body = self.comma()?;
        let span = joined(value.span, body.span);
        Ok(Expr::new(
            ExprKind::Bind {
                value: Box::new(value),
                name,
                body: Box::new(body),
            },
            span,
        ))
    }

    fn pipe(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let mut expression = self.alternative()?;
        while self.take(|kind| matches!(kind, TokenKind::Pipe)).is_some() {
            let right = self.alternative()?;
            let span = joined(expression.span, right.span);
            expression = Expr::new(ExprKind::Pipe(Box::new(expression), Box::new(right)), span);
        }
        Ok(expression)
    }

    fn alternative(&mut self) -> Result<Expr, Box<Diagnostic>> {
        self.left_associative(Self::logical_or, |kind| match kind {
            TokenKind::Alternative => Some(BinaryOperator::Alternative),
            _ => None,
        })
    }

    fn logical_or(&mut self) -> Result<Expr, Box<Diagnostic>> {
        self.left_associative(Self::logical_and, |kind| match kind {
            TokenKind::Or => Some(BinaryOperator::Or),
            _ => None,
        })
    }

    fn logical_and(&mut self) -> Result<Expr, Box<Diagnostic>> {
        self.left_associative(Self::comparison, |kind| match kind {
            TokenKind::And => Some(BinaryOperator::And),
            _ => None,
        })
    }

    fn comparison(&mut self) -> Result<Expr, Box<Diagnostic>> {
        self.left_associative(Self::addition, |kind| match kind {
            TokenKind::Equal => Some(BinaryOperator::Equal),
            TokenKind::NotEqual => Some(BinaryOperator::NotEqual),
            TokenKind::Less => Some(BinaryOperator::Less),
            TokenKind::LessEqual => Some(BinaryOperator::LessEqual),
            TokenKind::Greater => Some(BinaryOperator::Greater),
            TokenKind::GreaterEqual => Some(BinaryOperator::GreaterEqual),
            _ => None,
        })
    }

    fn addition(&mut self) -> Result<Expr, Box<Diagnostic>> {
        self.left_associative(Self::multiplication, |kind| match kind {
            TokenKind::Plus => Some(BinaryOperator::Add),
            TokenKind::Minus => Some(BinaryOperator::Subtract),
            _ => None,
        })
    }

    fn multiplication(&mut self) -> Result<Expr, Box<Diagnostic>> {
        self.left_associative(Self::unary, |kind| match kind {
            TokenKind::Star => Some(BinaryOperator::Multiply),
            TokenKind::Slash => Some(BinaryOperator::Divide),
            TokenKind::Percent => Some(BinaryOperator::Remainder),
            _ => None,
        })
    }

    fn left_associative(
        &mut self,
        operand: fn(&mut Self) -> Result<Expr, Box<Diagnostic>>,
        operator: fn(&TokenKind) -> Option<BinaryOperator>,
    ) -> Result<Expr, Box<Diagnostic>> {
        let mut expression = operand(self)?;
        while let Some(operation) = operator(&self.current().kind) {
            self.advance();
            let right = operand(self)?;
            let span = joined(expression.span, right.span);
            expression = Expr::new(
                ExprKind::Binary {
                    operator: operation,
                    left: Box::new(expression),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(expression)
    }

    fn unary(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let operator = match self.current().kind {
            TokenKind::Not => Some(UnaryOperator::Not),
            TokenKind::Minus => Some(UnaryOperator::Negate),
            _ => None,
        };
        if let Some(operator) = operator {
            let start = self.advance().span;
            let expression =
                if operator == UnaryOperator::Not && filter_terminator(&self.current().kind) {
                    Expr::new(ExprKind::Identity, start)
                } else {
                    self.unary()?
                };
            let span = joined(start, expression.span);
            return Ok(Expr::new(
                ExprKind::Unary {
                    operator,
                    expression: Box::new(expression),
                },
                span,
            ));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let mut expression = self.primary()?;
        loop {
            if let Some(question) = self.take(|kind| matches!(kind, TokenKind::Question)) {
                let span = joined(expression.span, question.span);
                expression = Expr::new(ExprKind::Optional(Box::new(expression)), span);
                continue;
            }
            if self.take(|kind| matches!(kind, TokenKind::Dot)).is_some() {
                let field = self.advance().clone();
                let TokenKind::Identifier(name) = field.kind else {
                    return Err(self.error_at(
                        "TQ-PARSE-FIELD-001",
                        "expected field name after '.'",
                        field.span,
                    ));
                };
                let span = joined(expression.span, field.span);
                expression = Expr::new(
                    ExprKind::Access {
                        base: Box::new(expression),
                        access: Access::Field(name),
                    },
                    span,
                );
                continue;
            }
            if self
                .take(|kind| matches!(kind, TokenKind::LeftBracket))
                .is_some()
            {
                expression = self.bracket_access(expression)?;
                continue;
            }
            break;
        }
        Ok(expression)
    }

    fn bracket_access(&mut self, base: Expr) -> Result<Expr, Box<Diagnostic>> {
        if let Some(close) = self.take(|kind| matches!(kind, TokenKind::RightBracket)) {
            let span = joined(base.span, close.span);
            return Ok(Expr::new(
                ExprKind::Access {
                    base: Box::new(base),
                    access: Access::Iterate,
                },
                span,
            ));
        }
        let start = if matches!(self.current().kind, TokenKind::Colon) {
            None
        } else {
            Some(Box::new(self.assignment()?))
        };
        let access = if self.take(|kind| matches!(kind, TokenKind::Colon)).is_some() {
            let end = if matches!(self.current().kind, TokenKind::RightBracket) {
                None
            } else {
                Some(Box::new(self.assignment()?))
            };
            Access::Slice { start, end }
        } else {
            Access::Index(start.ok_or_else(|| self.unexpected("index expression"))?)
        };
        let close = self.expect(
            |kind| matches!(kind, TokenKind::RightBracket),
            "']' after index or slice",
        )?;
        let span = joined(base.span, close.span);
        Ok(Expr::new(
            ExprKind::Access {
                base: Box::new(base),
                access,
            },
            span,
        ))
    }

    #[allow(
        clippy::too_many_lines,
        reason = "primary forms share delimiter handling"
    )]
    fn primary(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Dot => {
                let identity = Expr::new(ExprKind::Identity, token.span);
                if let TokenKind::Identifier(name) = self.current().kind.clone() {
                    let field = self.advance().span;
                    Ok(Expr::new(
                        ExprKind::Access {
                            base: Box::new(identity),
                            access: Access::Field(name),
                        },
                        joined(token.span, field),
                    ))
                } else {
                    Ok(identity)
                }
            }
            TokenKind::DotDot => Ok(Expr::new(ExprKind::RecursiveDescent, token.span)),
            TokenKind::True => Ok(Expr::new(ExprKind::Literal(Value::Bool(true)), token.span)),
            TokenKind::False => Ok(Expr::new(ExprKind::Literal(Value::Bool(false)), token.span)),
            TokenKind::Null => Ok(Expr::new(ExprKind::Literal(Value::Null), token.span)),
            TokenKind::String(value) => Ok(Expr::new(
                ExprKind::Literal(Value::string(value)),
                token.span,
            )),
            TokenKind::StringStart => self.interpolation(token.span),
            TokenKind::Number(value) => {
                let number = Number::parse(&value).map_err(|error| {
                    self.error_at("TQ-NUMBER-RANGE-001", &error.to_string(), token.span)
                })?;
                Ok(Expr::new(
                    ExprKind::Literal(Value::Number(number)),
                    token.span,
                ))
            }
            TokenKind::Variable(name) => Ok(Expr::new(ExprKind::Variable(name), token.span)),
            TokenKind::Identifier(name) => self.call_or_name(name, token.span),
            TokenKind::LeftParen => {
                let expression = self.comma()?;
                let close = self.expect(
                    |kind| matches!(kind, TokenKind::RightParen),
                    "')' after grouped expression",
                )?;
                Ok(Expr::new(expression.kind, joined(token.span, close.span)))
            }
            TokenKind::LeftBracket => self.array(token.span),
            TokenKind::LeftBrace => self.object(token.span),
            TokenKind::If => self.conditional(token.span),
            TokenKind::Try => self.try_catch(token.span),
            TokenKind::Reduce => self.fold(token.span, false),
            TokenKind::Foreach => self.fold(token.span, true),
            TokenKind::Def => self.definition(token.span),
            TokenKind::Include => self.include(token.span),
            TokenKind::Import => self.import(token.span),
            TokenKind::Module => self.module(token.span),
            TokenKind::Deferred(capability) => Err(self.deferred(&capability, token.span)),
            _ => Err(self.error_at(
                "TQ-PARSE-EXPRESSION-001",
                "expected filter expression",
                token.span,
            )),
        }
    }

    fn interpolation(&mut self, open: Span) -> Result<Expr, Box<Diagnostic>> {
        let mut segments = Vec::new();
        loop {
            let token = self.advance().clone();
            match token.kind {
                TokenKind::StringFragment(value) => {
                    segments.push(InterpolationSegment::Literal {
                        value,
                        span: token.span,
                    });
                }
                TokenKind::InterpolationStart => {
                    if matches!(self.current().kind, TokenKind::InterpolationEnd) {
                        return Err(self.error_at(
                            "TQ-PARSE-INTERPOLATION-001",
                            "expected filter expression in string interpolation",
                            self.current().span,
                        ));
                    }
                    let expression = self.comma()?;
                    self.expect(
                        |kind| matches!(kind, TokenKind::InterpolationEnd),
                        "')' after string interpolation",
                    )?;
                    segments.push(InterpolationSegment::Expression(expression));
                }
                TokenKind::StringEnd => {
                    return Ok(Expr::new(
                        ExprKind::Interpolation(segments),
                        joined(open, token.span),
                    ));
                }
                _ => {
                    return Err(self.error_at(
                        "TQ-PARSE-INTERPOLATION-001",
                        "expected interpolation segment or closing quote",
                        token.span,
                    ));
                }
            }
        }
    }

    fn call_or_name(&mut self, name: Arc<str>, span: Span) -> Result<Expr, Box<Diagnostic>> {
        if &*name == "empty" && !matches!(self.current().kind, TokenKind::LeftParen) {
            return Ok(Expr::new(ExprKind::Empty, span));
        }
        let mut arguments = Vec::new();
        let mut end = span;
        if self
            .take(|kind| matches!(kind, TokenKind::LeftParen))
            .is_some()
        {
            if !matches!(self.current().kind, TokenKind::RightParen) {
                loop {
                    arguments.push(self.assignment()?);
                    if self
                        .take(|kind| matches!(kind, TokenKind::Semicolon))
                        .is_none()
                    {
                        break;
                    }
                }
            }
            end = self
                .expect(
                    |kind| matches!(kind, TokenKind::RightParen),
                    "')' after function arguments",
                )?
                .span;
        }
        Ok(Expr::new(
            ExprKind::Call {
                name,
                arguments,
                target: None,
            },
            joined(span, end),
        ))
    }

    fn array(&mut self, open: Span) -> Result<Expr, Box<Diagnostic>> {
        let body = if matches!(self.current().kind, TokenKind::RightBracket) {
            Expr::new(ExprKind::Empty, self.current().span)
        } else {
            self.comma()?
        };
        let close = self.expect(
            |kind| matches!(kind, TokenKind::RightBracket),
            "']' after array constructor",
        )?;
        Ok(Expr::new(
            ExprKind::Array(Box::new(body)),
            joined(open, close.span),
        ))
    }

    fn object(&mut self, open: Span) -> Result<Expr, Box<Diagnostic>> {
        let mut entries = Vec::new();
        while !matches!(self.current().kind, TokenKind::RightBrace) {
            let key_token = self.advance().clone();
            let (key, shorthand) = match key_token.kind {
                TokenKind::Identifier(key) | TokenKind::String(key) => {
                    (ObjectKey::Static(key.clone()), Some(key))
                }
                TokenKind::LeftParen => {
                    let key = self.comma()?;
                    self.expect(
                        |kind| matches!(kind, TokenKind::RightParen),
                        "')' after computed object key",
                    )?;
                    (ObjectKey::Computed(key), None)
                }
                _ => {
                    return Err(self.error_at(
                        "TQ-PARSE-OBJECT-001",
                        "expected object key",
                        key_token.span,
                    ));
                }
            };
            let value = if self.take(|kind| matches!(kind, TokenKind::Colon)).is_some() {
                self.assignment()?
            } else if let Some(name) = shorthand {
                Expr::new(
                    ExprKind::Access {
                        base: Box::new(Expr::new(ExprKind::Identity, key_token.span)),
                        access: Access::Field(name),
                    },
                    key_token.span,
                )
            } else {
                return Err(self.unexpected("':' after computed object key"));
            };
            let span = joined(key_token.span, value.span);
            entries.push(ObjectEntry { key, value, span });
            if self.take(|kind| matches!(kind, TokenKind::Comma)).is_none() {
                break;
            }
        }
        let close = self.expect(
            |kind| matches!(kind, TokenKind::RightBrace),
            "'}' after object constructor",
        )?;
        Ok(Expr::new(
            ExprKind::Object(entries),
            joined(open, close.span),
        ))
    }

    fn conditional(&mut self, open: Span) -> Result<Expr, Box<Diagnostic>> {
        let mut branches = Vec::new();
        let condition = self.comma()?;
        self.expect(|kind| matches!(kind, TokenKind::Then), "'then'")?;
        let body = self.comma()?;
        branches.push((condition, body));
        while self.take(|kind| matches!(kind, TokenKind::Elif)).is_some() {
            let condition = self.comma()?;
            self.expect(|kind| matches!(kind, TokenKind::Then), "'then'")?;
            let body = self.comma()?;
            branches.push((condition, body));
        }
        self.expect(|kind| matches!(kind, TokenKind::Else), "'else'")?;
        let alternative = self.comma()?;
        let end = self.expect(|kind| matches!(kind, TokenKind::End), "'end'")?;
        Ok(Expr::new(
            ExprKind::Conditional {
                branches,
                alternative: Box::new(alternative),
            },
            joined(open, end.span),
        ))
    }

    fn try_catch(&mut self, open: Span) -> Result<Expr, Box<Diagnostic>> {
        let expression = self.assignment()?;
        let catch = if self.take(|kind| matches!(kind, TokenKind::Catch)).is_some() {
            Some(Box::new(self.assignment()?))
        } else {
            None
        };
        let end = catch.as_deref().map_or(expression.span, |catch| catch.span);
        Ok(Expr::new(
            ExprKind::TryCatch {
                expression: Box::new(expression),
                catch,
            },
            joined(open, end),
        ))
    }

    fn fold(&mut self, open: Span, foreach: bool) -> Result<Expr, Box<Diagnostic>> {
        let generator = self.pipe()?;
        self.expect(
            |kind| matches!(kind, TokenKind::As),
            "'as' after fold generator",
        )?;
        let variable = self.advance().clone();
        let TokenKind::Variable(name) = variable.kind else {
            return Err(self.error_at(
                "TQ-PARSE-FOLD-VARIABLE-001",
                "expected variable after 'as'",
                variable.span,
            ));
        };
        self.expect(
            |kind| matches!(kind, TokenKind::LeftParen),
            "'(' before fold initializer",
        )?;
        let initial = self.comma()?;
        self.expect(
            |kind| matches!(kind, TokenKind::Semicolon),
            "';' after fold initializer",
        )?;
        let update = self.comma()?;
        let extract = if foreach {
            self.expect(
                |kind| matches!(kind, TokenKind::Semicolon),
                "';' after foreach update",
            )?;
            Some(self.comma()?)
        } else {
            None
        };
        let close = self.expect(
            |kind| matches!(kind, TokenKind::RightParen),
            "')' after fold body",
        )?;
        let kind = if let Some(extract) = extract {
            ExprKind::Foreach {
                generator: Box::new(generator),
                name,
                initial: Box::new(initial),
                update: Box::new(update),
                extract: Box::new(extract),
            }
        } else {
            ExprKind::Reduce {
                generator: Box::new(generator),
                name,
                initial: Box::new(initial),
                update: Box::new(update),
            }
        };
        Ok(Expr::new(kind, joined(open, close.span)))
    }

    fn definition(&mut self, open: Span) -> Result<Expr, Box<Diagnostic>> {
        let name = self.advance().clone();
        let TokenKind::Identifier(name_value) = name.kind else {
            return Err(self.error_at(
                "TQ-PARSE-DEF-001",
                "expected filter name after 'def'",
                name.span,
            ));
        };
        let mut parameters = Vec::new();
        if self
            .take(|kind| matches!(kind, TokenKind::LeftParen))
            .is_some()
        {
            if !matches!(self.current().kind, TokenKind::RightParen) {
                loop {
                    let parameter = self.advance().clone();
                    let (name, kind) = match parameter.kind {
                        TokenKind::Identifier(name) => (name, ParameterKind::Filter),
                        TokenKind::Variable(name) => (name, ParameterKind::Value),
                        _ => {
                            return Err(self.error_at(
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
                    if self
                        .take(|kind| matches!(kind, TokenKind::Semicolon))
                        .is_none()
                    {
                        break;
                    }
                }
            }
            self.expect(
                |kind| matches!(kind, TokenKind::RightParen),
                "')' after definition parameters",
            )?;
        }
        self.expect(
            |kind| matches!(kind, TokenKind::Colon),
            "':' before definition body",
        )?;
        let definition_body = self.comma()?;
        let semicolon = self.expect(
            |kind| matches!(kind, TokenKind::Semicolon),
            "';' after definition body",
        )?;
        let body = self.following_filter(semicolon.span)?;
        let definition = Definition {
            name: name_value,
            parameters,
            span: joined(open, definition_body.span),
            body: definition_body,
            symbol: None,
        };
        let span = joined(open, body.span);
        Ok(Expr::new(
            ExprKind::Define {
                definition: Box::new(definition),
                body: Box::new(body),
            },
            span,
        ))
    }

    fn include(&mut self, open: Span) -> Result<Expr, Box<Diagnostic>> {
        let path = self.module_path("after 'include'")?;
        let metadata = self.optional_module_metadata()?;
        let semicolon = self.expect(
            |kind| matches!(kind, TokenKind::Semicolon),
            "';' after include directive",
        )?;
        let body = self.following_filter(semicolon.span)?;
        let span = joined(open, body.span);
        Ok(Expr::new(
            ExprKind::Include {
                path,
                metadata,
                body: Box::new(body),
            },
            span,
        ))
    }

    fn import(&mut self, open: Span) -> Result<Expr, Box<Diagnostic>> {
        let path = self.module_path("after 'import'")?;
        self.expect(
            |kind| matches!(kind, TokenKind::As),
            "'as' after import path",
        )?;
        let alias = self.advance().clone();
        let TokenKind::Identifier(alias) = alias.kind else {
            return Err(self.error_at(
                "TQ-PARSE-IMPORT-001",
                "expected module alias after 'as'",
                alias.span,
            ));
        };
        let metadata = self.optional_module_metadata()?;
        let semicolon = self.expect(
            |kind| matches!(kind, TokenKind::Semicolon),
            "';' after import directive",
        )?;
        let body = self.following_filter(semicolon.span)?;
        let span = joined(open, body.span);
        Ok(Expr::new(
            ExprKind::Import {
                path,
                alias,
                metadata,
                body: Box::new(body),
            },
            span,
        ))
    }

    fn module(&mut self, open: Span) -> Result<Expr, Box<Diagnostic>> {
        let metadata = self.assignment()?;
        let semicolon = self.expect(
            |kind| matches!(kind, TokenKind::Semicolon),
            "';' after module metadata",
        )?;
        let body = self.following_filter(semicolon.span)?;
        let span = joined(open, body.span);
        Ok(Expr::new(
            ExprKind::Module {
                metadata: Box::new(metadata),
                body: Box::new(body),
            },
            span,
        ))
    }

    fn module_path(&mut self, expected: &str) -> Result<Arc<str>, Box<Diagnostic>> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::String(path) => Ok(path),
            _ => Err(self.error_at(
                "TQ-PARSE-MODULE-PATH-001",
                &format!("expected constant module path {expected}"),
                token.span,
            )),
        }
    }

    fn optional_module_metadata(&mut self) -> Result<Option<Box<Expr>>, Box<Diagnostic>> {
        if matches!(self.current().kind, TokenKind::Semicolon) {
            Ok(None)
        } else {
            self.assignment().map(Box::new).map(Some)
        }
    }

    fn following_filter(&mut self, empty_span: Span) -> Result<Expr, Box<Diagnostic>> {
        if matches!(self.current().kind, TokenKind::EndOfInput) {
            Ok(Expr::new(ExprKind::Empty, empty_span))
        } else {
            self.comma()
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> &Token {
        let current = self.index;
        if self.index + 1 < self.tokens.len() {
            self.index += 1;
        }
        &self.tokens[current]
    }

    fn take(&mut self, predicate: impl FnOnce(&TokenKind) -> bool) -> Option<Token> {
        predicate(&self.current().kind).then(|| self.advance().clone())
    }

    fn expect(
        &mut self,
        predicate: impl FnOnce(&TokenKind) -> bool,
        expected: &str,
    ) -> Result<Token, Box<Diagnostic>> {
        if predicate(&self.current().kind) {
            Ok(self.advance().clone())
        } else {
            Err(self.unexpected(expected))
        }
    }

    fn unexpected(&self, expected: &str) -> Box<Diagnostic> {
        self.error_at(
            "TQ-PARSE-UNEXPECTED-001",
            &format!("expected {expected}"),
            self.current().span,
        )
    }

    fn deferred(&self, capability: &str, span: Span) -> Box<Diagnostic> {
        let capability = capability.replace('_', "-");
        self.error_at(
            &format!("TQ-CAP-{}", capability.to_ascii_uppercase()),
            &format!("jq capability {capability:?} is deferred"),
            span,
        )
    }

    fn error_at(&self, code: &str, message: &str, span: Span) -> Box<Diagnostic> {
        let context = self.source.render_context(span, 160);
        let label = if context.is_empty() {
            message.to_owned()
        } else {
            format!("{message}; near {context:?}")
        };
        Box::new(Diagnostic::new(code, DiagnosticClass::Compile, message).at(span, label))
    }
}

const fn joined(left: Span, right: Span) -> Span {
    Span::new(left.source, left.start, right.end)
}

const fn filter_terminator(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::RightParen
            | TokenKind::RightBracket
            | TokenKind::RightBrace
            | TokenKind::Comma
            | TokenKind::Semicolon
            | TokenKind::Pipe
            | TokenKind::Then
            | TokenKind::Elif
            | TokenKind::Else
            | TokenKind::End
            | TokenKind::Catch
            | TokenKind::InterpolationEnd
            | TokenKind::EndOfInput
    )
}

#[cfg(test)]
mod tests {
    use super::{parse, parse_bytes};

    #[test]
    fn precedence_and_associativity_are_stable() {
        assert_eq!(
            parse(".a, .b | .c // 1 + 2 * 3").unwrap().hir(),
            "comma(access(., field:a), pipe(access(., field:b), alternative(access(., field:c), add(1, multiply(2, 3)))))"
        );
        assert_eq!(
            parse(".a = .b = 1").unwrap().hir(),
            "set(access(., field:a), set(access(., field:b), 1))"
        );
    }

    #[test]
    fn parses_navigation_construction_control_variables_and_updates() {
        let cases = [
            ".[1:3]",
            ".[\"name\"]?",
            "[.items[] | .name]",
            "{id, title: .properties.title, (.key): .value}",
            "if .a then 1 elif .b then 2 else 3 end",
            ".[] as $item | $item.name",
            "try error(\"bad\") catch .",
            "(.a, .b) |= . + 10",
        ];
        for query in cases {
            parse(query).unwrap_or_else(|error| panic!("{query}: {error}"));
        }
    }

    #[test]
    fn deferred_and_invalid_inputs_have_stable_classes() {
        assert_eq!(
            parse_bytes("query", &[0xff]).unwrap_err().code,
            "TQ-LEX-UTF8-001"
        );
    }

    #[test]
    fn parses_recursive_descent_and_source_spanned_nested_interpolation() {
        assert_eq!(parse("..").unwrap().hir(), "recursive-descent");
        assert_eq!(
            parse("\"x=\\(1,2); y=\\(\"z=\\(.)\")\"").unwrap().hir(),
            "interpolate(\"x=\", comma(1, 2), \"; y=\", interpolate(\"z=\", ., \"\"), \"\")"
        );
        assert_eq!(parse("..[0]").unwrap().source().text(), "..[0]");
    }

    #[test]
    fn malformed_interpolation_has_stable_source_diagnostics() {
        assert_eq!(
            parse("\"x=\\()\"").unwrap_err().code,
            "TQ-PARSE-INTERPOLATION-001"
        );
        assert_eq!(parse("\"x=\\(1\"").unwrap_err().code, "TQ-LEX-STRING-001");
    }

    #[test]
    fn parses_source_spanned_reduce_and_foreach_forms() {
        let reduce = parse("reduce (1,2) as $x (0; . + $x)").unwrap();
        assert_eq!(
            reduce.hir(),
            "reduce(comma(1, 2) as $x; init: 0; update: add(., $x))"
        );
        assert_eq!(reduce.source().text().len() as u64, 30);

        let foreach = parse("foreach .[] as $x (0; . + $x; .)").unwrap();
        assert_eq!(
            foreach.hir(),
            "foreach(access(., iterate) as $x; init: 0; update: add(., $x); extract: .)"
        );
    }

    #[test]
    fn parses_source_spanned_definitions_and_module_directives() {
        let query = parse(
            "def twice(f): f | f; include \"shared\"; import \"math\" as m {search:\"lib\"}; twice(m::inc)",
        )
        .unwrap();
        let hir = query.hir();
        assert!(hir.starts_with("def(twice; f =>"));
        assert!(hir.contains("include(\"shared\""));
        assert!(hir.contains("import(\"math\" as m"));
        assert!(hir.ends_with("call(twice, call(m::inc)))))"));

        let module = parse("module {homepage:\"https://example.invalid\"}; def id: .;").unwrap();
        assert!(module.hir().starts_with("module(object(homepage:"));
    }

    #[test]
    fn malformed_fold_fuzz_regressions_return_diagnostics_without_panicking() {
        for query in [
            "reduce",
            "reduce .",
            "reduce . as",
            "reduce . as $x",
            "reduce . as $x (",
            "reduce . as $x (0)",
            "reduce . as $x (0;)",
            "foreach . as $x (0; .)",
            "foreach . as $x (0; .;)",
            "foreach . as $x (0; .; .",
        ] {
            assert!(parse(query).is_err(), "{query}");
        }
    }

    #[test]
    fn parses_the_complete_mvp_compatibility_query_surface() {
        let queries = [
            ".",
            ".a",
            ".[\"name\"]",
            ".[0]",
            ".[9007199254740991]",
            ".[9007199254740992]",
            ".[{}]",
            ".[1:3]",
            ".[ ]",
            ".foo?",
            ".a, .b",
            ".[] | (., . + 10)",
            ".[] as $item | $item.name",
            "[.[] | . * 2]",
            "{name, age}",
            "{(.key): .value}",
            "{a: 1, a: 2}",
            "if . then \"yes\" else \"no\" end",
            ".nickname // .name // \"unknown\"",
            "false and error(\"must not run\")",
            "true or error(\"must not run\")",
            "[0,\"\",[],{}] | map(if . then \"yes\" else \"no\" end)",
            "[null,false,0] | map(not)",
            "[(6*7)+1,10-3,8/2,10%3]",
            "[1+2,\"a\"+\"b\",[1]+[2],{\"a\":1}+{\"b\":2,\"a\":3}]",
            "1 / 0",
            "$name",
            "1 as $x | (2 as $x | $x), $x",
            "empty",
            "error(\"boom\")",
            "try error(\"boom\") catch .",
            "1, error(\"later\")",
            ".a = 2",
            ".a |= . + 2",
            ".a += 3",
            ".a -= 3",
            ".a *= 3",
            ".a /= 2",
            ".a //= 7",
            "(.a, .b) |= . + 10",
            "type",
            "length",
            "utf8bytelength",
            "keys",
            "keys_unsorted",
            "has(\"a\")",
            "in({\"a\":1})",
            "select(. % 2 == 0)",
            "map(. * 2)",
            "map_values(. + 1)",
            "values",
            "scalars",
            "arrays",
            "objects",
            "iterables",
            "booleans",
            "numbers",
            "strings",
            "nulls",
            "tostring",
            "tonumber",
            "add",
            "min",
            "max",
            "sort",
            "sort_by(.n)",
            "unique",
            "unique_by(.n)",
            "reverse",
            "flatten",
            "range(0;5;2)",
            ".. | scalars",
            "\"name=\\(.name)\"",
        ];
        for query in queries {
            parse(query).unwrap_or_else(|error| panic!("{query}: {error}"));
        }
    }
}
