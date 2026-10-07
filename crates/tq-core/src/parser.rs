//! jq MVP parser with explicit precedence and heap-backed expression continuations.

#[path = "parser_machine.rs"]
mod machine;
#[path = "parser_owned.rs"]
mod owned;

use std::{collections::BTreeMap, sync::Arc};

use crate::{
    Diagnostic, DiagnosticClass, Label, Number, Parsed, Query, SourceFile, SourceId, Span, Value,
    ast::{
        Access, AssignmentOperator, BinaryOperator, BindingPattern, BindingPatternEntry, Expr,
        ExprKind, FunctionParameter, ParameterKind, UnaryOperator,
    },
    format,
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

/// Parses a query together with a jq startup file while retaining source spans.
///
/// Startup files contribute definitions before the command-line filter. Their
/// source identity and line offsets remain independent so `__loc__` reports
/// the startup path for startup definitions and the top-level query for the
/// command-line filter.
///
/// # Errors
///
/// Returns a source-spanned UTF-8, lexical, syntax, numeric, or deferred-
/// capability diagnostic from either source.
pub fn parse_with_startup(
    name: &str,
    bytes: &[u8],
    startup_name: &str,
    startup_bytes: &[u8],
) -> Result<Query<Parsed>, Box<Diagnostic>> {
    let text = validate_utf8(bytes, name)?;
    let startup_text = validate_utf8(startup_bytes, startup_name)?;
    let source = SourceFile::new(SourceId::new(0), name, text);
    let startup_source = SourceFile::new(SourceId::new(1), startup_name, startup_text);
    let query = owned::Expr::from(parse_source(&source)?);
    let mut startup = owned::Expr::from(parse_source(&startup_source)?);
    crate::resolve::replace_location_variables(startup.ast_mut(), startup_name, startup_text);
    let ast = prepend_startup(startup, query)?;
    Ok(Query::from_ast_with_sources(
        source,
        ast,
        BTreeMap::from([(startup_source.id(), startup_source)]),
    ))
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

fn prepend_startup(
    mut startup: owned::Expr,
    mut query: owned::Expr,
) -> Result<Expr, Box<Diagnostic>> {
    let mut definitions = Vec::new();
    loop {
        match &startup.kind {
            ExprKind::Define { .. } => {
                let ExprKind::Define { definition, body } = startup.into_ast().kind else {
                    unreachable!("startup definition checked before ownership transfer")
                };
                definitions.push(owned::Definition {
                    name: definition.name,
                    parameters: definition.parameters,
                    body: owned::Expr::from(definition.body),
                    span: definition.span,
                    symbol: definition.symbol,
                });
                startup = owned::Expr::from(*body);
            }
            ExprKind::Empty => break,
            _ => {
                return Err(Box::new(
                    Diagnostic::new(
                        "TQ-STARTUP-CONTENT-001",
                        DiagnosticClass::Compile,
                        "startup file may contain only definitions",
                    )
                    .at(
                        startup.span,
                        "startup file may contain only definitions".to_owned(),
                    ),
                ));
            }
        }
    }
    for definition in definitions.into_iter().rev() {
        // A wrapper cannot span both sources; children retain their identities.
        let span = definition.span;
        query = owned::Expr::new(
            ExprKind::Define {
                definition: Box::new(definition.into_ast()),
                body: Box::new(query.into_ast()),
            },
            span,
        );
    }
    Ok(query.into_ast())
}

struct Parser<'a> {
    source: &'a SourceFile,
    tokens: Vec<Token>,
    index: usize,
}

const MAX_BINDING_PATTERN_DEPTH: usize = 256;

impl Parser<'_> {
    fn complete(mut self) -> Result<Expr, Box<Diagnostic>> {
        let expression = owned::Expr::from(self.expression()?);
        if !matches!(self.current().kind, TokenKind::EndOfInput) {
            return Err(self.unexpected("end of query"));
        }
        Ok(expression.into_ast())
    }

    fn expression(&mut self) -> Result<Expr, Box<Diagnostic>> {
        machine::expression(self)
    }

    fn atom(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Dot => {
                let identity = Expr::new(ExprKind::Identity, token.span);
                match self.current().kind.clone() {
                    TokenKind::Identifier(name) if name.contains("::") => {
                        let field = self.advance().clone();
                        Err(self.error_at(
                            "TQ-PARSE-FIELD-001",
                            "field names cannot use namespace separators",
                            field.span,
                        ))
                    }
                    TokenKind::Identifier(name) | TokenKind::String(name) => {
                        let field = self.advance().span;
                        Ok(Expr::new(
                            ExprKind::Access {
                                base: Box::new(identity),
                                access: Access::Field(name),
                            },
                            joined(token.span, field),
                        ))
                    }
                    _ => Ok(identity),
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
            TokenKind::Break => self.break_expression(token.span),
            _ => Err(self.error_at(
                "TQ-PARSE-EXPRESSION-001",
                "expected filter expression",
                token.span,
            )),
        }
    }

    fn binding_pattern(&mut self) -> Result<BindingPattern, Box<Diagnostic>> {
        self.binding_pattern_at_depth(0)
    }

    fn binding_pattern_at_depth(
        &mut self,
        depth: usize,
    ) -> Result<BindingPattern, Box<Diagnostic>> {
        if depth >= MAX_BINDING_PATTERN_DEPTH {
            return Err(self.error_at(
                "TQ-RESOURCE-BIND-PATTERN-001",
                "binding pattern nesting limit exceeded",
                self.current().span,
            ));
        }
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Variable(name) => Ok(BindingPattern::Variable(name)),
            TokenKind::LeftBracket => {
                let mut patterns = Vec::new();
                while !matches!(self.current().kind, TokenKind::RightBracket) {
                    patterns.push(self.binding_pattern_at_depth(depth + 1)?);
                    if self.take(|kind| matches!(kind, TokenKind::Comma)).is_none() {
                        break;
                    }
                }
                self.expect(
                    |kind| matches!(kind, TokenKind::RightBracket),
                    "']' after array binding pattern",
                )?;
                Ok(BindingPattern::Array(patterns))
            }
            TokenKind::LeftBrace => {
                let mut entries = Vec::new();
                while !matches!(self.current().kind, TokenKind::RightBrace) {
                    let key = self.advance().clone();
                    let (key, pattern) = match key.kind {
                        TokenKind::Variable(name) => {
                            let pattern = BindingPattern::Variable(Arc::clone(&name));
                            (name, pattern)
                        }
                        TokenKind::Identifier(name) | TokenKind::String(name) => {
                            self.expect(
                                |kind| matches!(kind, TokenKind::Colon),
                                "':' after object binding key",
                            )?;
                            (name, self.binding_pattern_at_depth(depth + 1)?)
                        }
                        _ => {
                            return Err(self.error_at(
                                "TQ-PARSE-BIND-OBJECT-001",
                                "expected object binding key",
                                key.span,
                            ));
                        }
                    };
                    entries.push(BindingPatternEntry { key, pattern });
                    if self.take(|kind| matches!(kind, TokenKind::Comma)).is_none() {
                        break;
                    }
                }
                self.expect(
                    |kind| matches!(kind, TokenKind::RightBrace),
                    "'}' after object binding pattern",
                )?;
                Ok(BindingPattern::Object(entries))
            }
            _ => Err(self.error_at(
                "TQ-PARSE-BIND-001",
                "expected variable, array, or object binding pattern",
                token.span,
            )),
        }
    }

    fn break_expression(&mut self, open: Span) -> Result<Expr, Box<Diagnostic>> {
        let variable = self.advance().clone();
        let TokenKind::Variable(name) = variable.kind else {
            return Err(self.error_at(
                "TQ-PARSE-BREAK-001",
                "expected label variable after 'break'",
                variable.span,
            ));
        };
        Ok(Expr::new(
            ExprKind::Break { name, symbol: None },
            joined(open, variable.span),
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

    fn error_at(&self, code: &str, message: &str, span: Span) -> Box<Diagnostic> {
        let context = self.source.render_context(span, 160);
        let label = if context.is_empty() {
            message.to_owned()
        } else {
            format!("{message}; near {context:?}")
        };
        Box::new(Diagnostic::new(code, DiagnosticClass::Compile, message).at(span, label))
    }

    fn with_context(mut diagnostic: Box<Diagnostic>, span: Span, message: &str) -> Box<Diagnostic> {
        diagnostic.labels.push(Label {
            span,
            message: message.to_owned(),
            primary: false,
        });
        diagnostic
    }

    fn with_if_context(&self, diagnostic: Box<Diagnostic>, open: Span) -> Box<Diagnostic> {
        let has_expression_after_opener = diagnostic
            .labels
            .iter()
            .find(|label| label.primary)
            .is_some_and(|label| label.span.start > open.end);
        if !has_expression_after_opener {
            return diagnostic;
        }
        let end = self.current().span.start.max(open.end);
        Self::with_context(
            diagnostic,
            Span::new(open.source, open.start, end),
            "unterminated if expression",
        )
    }
}

const fn joined(left: Span, right: Span) -> Span {
    Span::new(left.source, left.start, right.end)
}

fn format_call(name: Arc<str>, span: Span) -> Expr {
    Expr::new(
        ExprKind::Call {
            name,
            arguments: Vec::new(),
            target: None,
        },
        span,
    )
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
    use super::{parse, parse_bytes, parse_with_startup};
    use crate::{ResolveOptions, SourceId, analyze, resolve};

    #[test]
    fn startup_and_query_sources_survive_query_phase_transitions() {
        let parsed = parse_with_startup("query.jq", b".\n", "startup.jq", b"def startup: .;\n")
            .expect("startup and query parse");
        assert_eq!(parsed.source().id(), SourceId::new(0));
        assert_eq!(parsed.source().text(), ".\n");
        assert_eq!(
            parsed
                .source_by_id(SourceId::new(1))
                .expect("startup source")
                .text(),
            "def startup: .;\n"
        );

        let resolved = resolve(parsed, &ResolveOptions::default()).expect("resolve startup");
        assert_eq!(resolved.source().name(), "query.jq");
        assert_eq!(
            resolved
                .source_by_id(SourceId::new(1))
                .expect("startup source after resolve")
                .name(),
            "startup.jq"
        );
        let analyzed = analyze(resolved);
        assert_eq!(
            analyzed
                .source_by_id(SourceId::new(1))
                .expect("startup source after analysis")
                .text(),
            "def startup: .;\n"
        );
    }

    #[test]
    fn startup_resolution_diagnostics_keep_their_startup_source_id() {
        let parsed = parse_with_startup("query.jq", b".", "startup.jq", b"def broken: $missing;\n")
            .expect("startup parse");
        let retained = parsed.clone();
        let error =
            resolve(parsed, &ResolveOptions::default()).expect_err("unknown startup variable");
        let label = error.labels.first().expect("diagnostic source label");
        assert_eq!(label.span.source, SourceId::new(1));
        let source = retained
            .source_by_id(label.span.source)
            .expect("startup diagnostic source");
        assert_eq!(source.name(), "startup.jq");
        assert!(source.render_context(label.span, 80).contains("$missing"));
    }

    #[test]
    fn precedence_and_associativity_are_stable() {
        assert_eq!(
            parse(".a, .b | .c // 1 + 2 * 3").unwrap().hir(),
            "pipe(comma(access(., field:a), access(., field:b)), alternative(access(., field:c), add(1, multiply(2, 3))))"
        );
        assert_eq!(
            parse(".a = .b = 1").unwrap().hir(),
            "set(access(., field:a), set(access(., field:b), 1))"
        );
    }

    #[test]
    fn comma_generator_is_one_function_argument() {
        assert_eq!(
            parse("sort_by(.a,.b)").unwrap().hir(),
            "call(sort_by, comma(access(., field:a), access(., field:b)))"
        );
        assert_eq!(
            parse("sort_by((.a,.b))").unwrap().hir(),
            "call(sort_by, comma(access(., field:a), access(., field:b)))"
        );
        assert_eq!(
            parse("def pair(f; g): [f, g]; pair(1,2; 3,4)")
                .unwrap()
                .hir(),
            "def(pair; f; g => array(comma(call(f), call(g))); call(pair, comma(1, 2), comma(3, 4)))"
        );
        assert_eq!(
            parse("sort_by(.a,.b; )").unwrap_err().code,
            "TQ-PARSE-EXPRESSION-001"
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
    fn parses_lexical_labels_and_breaks_with_stable_spans() {
        let query = parse("label $outer | 1, break $outer, 2").unwrap();
        assert_eq!(
            query.hir(),
            "label($outer; comma(comma(1, break($outer)), 2))"
        );
        assert_eq!(query.source().text().len() as u64, 33);

        for malformed in ["label", "label x | .", "label $x .", "break", "break x"] {
            assert!(parse(malformed).is_err(), "{malformed}");
        }
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
    fn lowers_standalone_and_template_formats_to_calls() {
        assert_eq!(parse("@base64").unwrap().hir(), "call(@base64)");
        assert_eq!(
            parse(r#"@uri "https://x.test?q=\(.q)""#).unwrap().hir(),
            "interpolate(\"https://x.test?q=\", pipe(access(., field:q), call(@uri)), \"\")"
        );
        assert_eq!(
            parse(r#"@uri "literal ?&""#).unwrap().hir(),
            r#""literal ?&""#
        );
        assert!(parse(r#"@uri "\(empty)""#).is_ok());
        assert!(parse(r#"@uri "\(1,2)""#).is_ok());
        assert_eq!(
            parse(r#"@unknown "literal""#).unwrap_err().code,
            "TQ-RESOLVE-FORMAT-001"
        );
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
        assert_eq!(
            parse("foreach .[] as $x (0; . + $x)").unwrap().hir(),
            "foreach(access(., iterate) as $x; init: 0; update: add(., $x))"
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
