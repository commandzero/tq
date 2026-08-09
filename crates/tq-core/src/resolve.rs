//! Lexical variable and versioned built-in resolution.

use std::{collections::BTreeSet, sync::Arc};

use crate::{
    Analysis, Analyzed, CapabilityCause, Diagnostic, DiagnosticClass, Effect, Parsed, PlanKind,
    Query, Resolved, Span,
    ast::{Access, Expr, ExprKind, ObjectKey},
    phase::automatic_stream_proof,
};

/// One versioned built-in signature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Builtin {
    /// Stable function name.
    pub name: &'static str,
    /// Minimum filter arguments.
    pub minimum_arity: usize,
    /// Maximum filter arguments.
    pub maximum_arity: usize,
    /// Whether evaluation blocks on a complete collection.
    pub blocking: bool,
}

/// Immutable MVP built-in registry.
#[derive(Clone, Copy, Debug, Default)]
pub struct BuiltinRegistry;

impl BuiltinRegistry {
    /// Registry semantic version.
    pub const VERSION: u32 = 1;

    /// Returns the signature for a supported built-in.
    #[must_use]
    pub fn get(self, name: &str) -> Option<Builtin> {
        BUILTINS
            .iter()
            .copied()
            .find(|builtin| builtin.name == name)
    }

    /// Ordered registry entries.
    #[must_use]
    pub const fn all(self) -> &'static [Builtin] {
        BUILTINS
    }
}

const BUILTINS: &[Builtin] = &[
    builtin("add", 0, 0, true),
    builtin("arrays", 0, 0, false),
    builtin("booleans", 0, 0, false),
    builtin("empty", 0, 0, false),
    builtin("error", 0, 1, false),
    builtin("flatten", 0, 1, true),
    builtin("has", 1, 1, false),
    builtin("in", 1, 1, false),
    builtin("iterables", 0, 0, false),
    builtin("keys", 0, 0, true),
    builtin("keys_unsorted", 0, 0, false),
    builtin("length", 0, 0, false),
    builtin("map", 1, 1, true),
    builtin("map_values", 1, 1, true),
    builtin("max", 0, 0, true),
    builtin("min", 0, 0, true),
    builtin("nulls", 0, 0, false),
    builtin("numbers", 0, 0, false),
    builtin("objects", 0, 0, false),
    builtin("range", 1, 3, false),
    builtin("reverse", 0, 0, true),
    builtin("scalars", 0, 0, false),
    builtin("select", 1, 1, false),
    builtin("sort", 0, 0, true),
    builtin("sort_by", 1, 1, true),
    builtin("strings", 0, 0, false),
    builtin("tonumber", 0, 0, false),
    builtin("tostring", 0, 0, false),
    builtin("type", 0, 0, false),
    builtin("unique", 0, 0, true),
    builtin("unique_by", 1, 1, true),
    builtin("utf8bytelength", 0, 0, false),
    builtin("values", 0, 0, false),
];

const fn builtin(
    name: &'static str,
    minimum_arity: usize,
    maximum_arity: usize,
    blocking: bool,
) -> Builtin {
    Builtin {
        name,
        minimum_arity,
        maximum_arity,
        blocking,
    }
}

/// External variables made available during resolution.
#[derive(Clone, Debug, Default)]
pub struct ResolveOptions {
    /// CLI variable names without `$`.
    pub variables: BTreeSet<Arc<str>>,
}

/// CLI/input effects applied during capability analysis.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AnalysisContext {
    /// Analyze against explicit jq-style event input.
    pub event_input: bool,
    /// Slurp or another mode requires every input document.
    pub whole_input: bool,
    /// Permit ordinary jq-shaped queries to use decoder-backed plans.
    pub automatic_streaming: bool,
}

/// Resolves variables and built-ins, yielding no resolved value on error.
///
/// # Errors
///
/// Returns a source-spanned unknown-variable, unknown-built-in, arity, or
/// deferred-capability diagnostic.
pub fn resolve(
    query: Query<Parsed>,
    options: &ResolveOptions,
) -> Result<Query<Resolved>, Box<Diagnostic>> {
    let mut scopes = vec![options.variables.clone()];
    resolve_expr(query.ast(), &mut scopes, BuiltinRegistry)?;
    Ok(query.into_resolved())
}

/// Analyzes execution effects before input consumption.
#[must_use]
pub fn analyze(query: Query<Resolved>) -> Query<Analyzed> {
    analyze_with_context(
        query,
        AnalysisContext {
            automatic_streaming: true,
            ..AnalysisContext::default()
        },
    )
}

/// Analyzes execution effects with input-mode context.
#[must_use]
pub fn analyze_with_context(query: Query<Resolved>, context: AnalysisContext) -> Query<Analyzed> {
    let mut analysis = Analysis::default();
    analyze_expr(query.ast(), &mut analysis);
    if context.whole_input {
        add_effect(&mut analysis, Effect::WholeInput, query.ast().span);
        add_effect(&mut analysis, Effect::Document, query.ast().span);
        analysis.selected_plan = PlanKind::WholeInput;
        analysis.stream_rejection = Some("whole-input mode requires all documents".to_owned());
    } else if context.event_input
        && !analysis.capabilities.subtree
        && !analysis.capabilities.blocking
        && !analysis.capabilities.mutation
    {
        add_effect(&mut analysis, Effect::EventStream, query.ast().span);
        analysis.selected_plan = PlanKind::Events;
    } else if context.event_input {
        add_effect(&mut analysis, Effect::Document, query.ast().span);
        analysis.selected_plan = if analysis.capabilities.blocking {
            PlanKind::Blocking
        } else {
            PlanKind::Document
        };
        analysis.stream_rejection = Some(
            "explicit event input was rejected by document, mutation, or blocking effects"
                .to_owned(),
        );
    } else if analysis.capabilities.blocking {
        add_effect(&mut analysis, Effect::Document, query.ast().span);
        analysis.selected_plan = PlanKind::Blocking;
        analysis.stream_rejection =
            Some("blocking operator requires complete input state".to_owned());
    } else if analysis.capabilities.mutation {
        add_effect(&mut analysis, Effect::Document, query.ast().span);
        analysis.selected_plan = PlanKind::Document;
        analysis.stream_rejection = Some("mutation requires a complete document".to_owned());
    } else if context.automatic_streaming {
        if let Some((proof, plan)) = automatic_stream_proof(query.ast()) {
            add_effect(&mut analysis, Effect::PathPrefix, proof.cause);
            if proof.subtree_complete {
                add_effect(&mut analysis, Effect::SubtreeComplete, proof.cause);
                add_effect(&mut analysis, Effect::Subtree, proof.cause);
            } else {
                add_effect(&mut analysis, Effect::EventStream, proof.cause);
            }
            if proof.value_escapes {
                add_effect(&mut analysis, Effect::Escape, proof.cause);
            }
            analysis.selected_plan = plan;
            analysis.stream_proof = Some(proof);
        } else {
            add_effect(&mut analysis, Effect::Document, query.ast().span);
            analysis.stream_rejection =
                Some("query lacks a sound static-prefix iteration proof".to_owned());
        }
    } else {
        add_effect(&mut analysis, Effect::Document, query.ast().span);
        analysis.stream_rejection = Some(
            "the selected input or CLI mode does not expose automatic decoder events".to_owned(),
        );
    }
    query.with_analysis(analysis)
}

#[allow(
    clippy::too_many_lines,
    reason = "resolution exhaustively walks every syntax form in one auditable dispatch"
)]
fn resolve_expr(
    expr: &Expr,
    scopes: &mut Vec<BTreeSet<Arc<str>>>,
    registry: BuiltinRegistry,
) -> Result<(), Box<Diagnostic>> {
    match &expr.kind {
        ExprKind::Variable(name) => {
            if !scopes.iter().rev().any(|scope| scope.contains(name)) {
                return Err(error(
                    "TQ-RESOLVE-VARIABLE-001",
                    format!("unknown variable ${name}"),
                    expr.span,
                ));
            }
        }
        ExprKind::Access { base, access } => {
            resolve_expr(base, scopes, registry)?;
            match access {
                Access::Index(index) => resolve_expr(index, scopes, registry)?,
                Access::Slice { start, end } => {
                    if let Some(start) = start {
                        resolve_expr(start, scopes, registry)?;
                    }
                    if let Some(end) = end {
                        resolve_expr(end, scopes, registry)?;
                    }
                }
                Access::Field(_) | Access::Iterate => {}
            }
        }
        ExprKind::Optional(expression)
        | ExprKind::Array(expression)
        | ExprKind::Unary { expression, .. }
        | ExprKind::TryCatch {
            expression,
            catch: None,
        } => resolve_expr(expression, scopes, registry)?,
        ExprKind::Pipe(left, right)
        | ExprKind::Comma(left, right)
        | ExprKind::Binary { left, right, .. }
        | ExprKind::Assignment {
            path: left,
            value: right,
            ..
        } => {
            resolve_expr(left, scopes, registry)?;
            resolve_expr(right, scopes, registry)?;
        }
        ExprKind::Object(entries) => {
            for entry in entries {
                if let ObjectKey::Computed(key) = &entry.key {
                    resolve_expr(key, scopes, registry)?;
                }
                resolve_expr(&entry.value, scopes, registry)?;
            }
        }
        ExprKind::Conditional {
            branches,
            alternative,
        } => {
            for (condition, body) in branches {
                resolve_expr(condition, scopes, registry)?;
                resolve_expr(body, scopes, registry)?;
            }
            resolve_expr(alternative, scopes, registry)?;
        }
        ExprKind::Bind { value, name, body } => {
            resolve_expr(value, scopes, registry)?;
            scopes.push(BTreeSet::from([name.clone()]));
            let result = resolve_expr(body, scopes, registry);
            scopes.pop();
            result?;
        }
        ExprKind::Call { name, arguments } => {
            if let Some(capability) = deferred_builtin(name) {
                return Err(error(
                    &format!("TQ-CAP-{}", capability.to_ascii_uppercase()),
                    format!("jq capability {capability:?} is deferred"),
                    expr.span,
                ));
            }
            let Some(builtin) = registry.get(name) else {
                return Err(error(
                    "TQ-RESOLVE-BUILTIN-001",
                    format!("unknown built-in {name}/{arity}", arity = arguments.len()),
                    expr.span,
                ));
            };
            if !(builtin.minimum_arity..=builtin.maximum_arity).contains(&arguments.len()) {
                return Err(error(
                    "TQ-RESOLVE-ARITY-001",
                    format!("invalid arity {} for built-in {name}", arguments.len()),
                    expr.span,
                ));
            }
            for argument in arguments {
                resolve_expr(argument, scopes, registry)?;
            }
        }
        ExprKind::TryCatch {
            expression,
            catch: Some(catch),
        } => {
            resolve_expr(expression, scopes, registry)?;
            resolve_expr(catch, scopes, registry)?;
        }
        ExprKind::Identity | ExprKind::Literal(_) | ExprKind::Empty => {}
    }
    Ok(())
}

fn deferred_builtin(name: &str) -> Option<&'static str> {
    match name {
        "env" => Some("environment"),
        "fromdateiso8601" => Some("dates"),
        "input_filename" => Some("platform-io"),
        "test" => Some("regex"),
        "nan" => Some("nonfinite-result"),
        _ => None,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "effect propagation exhaustively maps every syntax form"
)]
fn analyze_expr(expr: &Expr, analysis: &mut Analysis) {
    match &expr.kind {
        ExprKind::Identity | ExprKind::Literal(_) | ExprKind::Variable(_) => {}
        ExprKind::Empty => add_effect(analysis, Effect::Generator, expr.span),
        ExprKind::Access { base, access } => {
            analyze_expr(base, analysis);
            match access {
                Access::Index(index) => analyze_expr(index, analysis),
                Access::Slice { start, end } => {
                    if let Some(start) = start {
                        analyze_expr(start, analysis);
                    }
                    if let Some(end) = end {
                        analyze_expr(end, analysis);
                    }
                }
                Access::Iterate => add_effect(analysis, Effect::Generator, expr.span),
                Access::Field(_) => {}
            }
            add_effect(analysis, Effect::PossibleFailure, expr.span);
        }
        ExprKind::Optional(expression) | ExprKind::Unary { expression, .. } => {
            analyze_expr(expression, analysis);
        }
        ExprKind::Pipe(left, right) | ExprKind::Binary { left, right, .. } => {
            analyze_expr(left, analysis);
            analyze_expr(right, analysis);
            if matches!(expr.kind, ExprKind::Binary { .. }) {
                add_effect(analysis, Effect::PossibleFailure, expr.span);
            }
        }
        ExprKind::Comma(left, right) => {
            analyze_expr(left, analysis);
            analyze_expr(right, analysis);
            add_effect(analysis, Effect::Generator, expr.span);
        }
        ExprKind::Array(expression) => {
            analyze_expr(expression, analysis);
            add_effect(analysis, Effect::Subtree, expr.span);
            add_effect(analysis, Effect::Blocking, expr.span);
        }
        ExprKind::Object(entries) => {
            for entry in entries {
                if let ObjectKey::Computed(key) = &entry.key {
                    analyze_expr(key, analysis);
                }
                analyze_expr(&entry.value, analysis);
            }
            add_effect(analysis, Effect::Subtree, expr.span);
        }
        ExprKind::Conditional {
            branches,
            alternative,
        } => {
            for (condition, body) in branches {
                analyze_expr(condition, analysis);
                analyze_expr(body, analysis);
            }
            analyze_expr(alternative, analysis);
        }
        ExprKind::Bind { value, body, .. } => {
            analyze_expr(value, analysis);
            analyze_expr(body, analysis);
            add_effect(analysis, Effect::Generator, expr.span);
        }
        ExprKind::Call { name, arguments } => {
            for argument in arguments {
                analyze_expr(argument, analysis);
            }
            if BuiltinRegistry
                .get(name)
                .is_some_and(|builtin| builtin.blocking)
            {
                add_effect(analysis, Effect::Blocking, expr.span);
            }
            if matches!(
                &**name,
                "range"
                    | "select"
                    | "values"
                    | "scalars"
                    | "arrays"
                    | "objects"
                    | "iterables"
                    | "booleans"
                    | "numbers"
                    | "strings"
                    | "nulls"
            ) {
                add_effect(analysis, Effect::Generator, expr.span);
            }
            if matches!(&**name, "error" | "tonumber" | "length" | "has" | "in") {
                add_effect(analysis, Effect::PossibleFailure, expr.span);
            }
        }
        ExprKind::TryCatch { expression, catch } => {
            analyze_expr(expression, analysis);
            if let Some(catch) = catch {
                analyze_expr(catch, analysis);
            }
        }
        ExprKind::Assignment { path, value, .. } => {
            analyze_expr(path, analysis);
            analyze_expr(value, analysis);
            add_effect(analysis, Effect::Mutation, expr.span);
            add_effect(analysis, Effect::Document, expr.span);
            add_effect(analysis, Effect::PossibleFailure, expr.span);
        }
    }
}

fn add_effect(analysis: &mut Analysis, effect: Effect, span: Span) {
    match effect {
        Effect::EventStream => analysis.capabilities.event_stream = true,
        Effect::Subtree => analysis.capabilities.subtree = true,
        Effect::Document => analysis.capabilities.document = true,
        Effect::WholeInput => analysis.capabilities.whole_input = true,
        Effect::Blocking => analysis.capabilities.blocking = true,
        Effect::Mutation => analysis.capabilities.mutation = true,
        Effect::Generator => analysis.capabilities.generator = true,
        Effect::PossibleFailure => analysis.capabilities.possible_failure = true,
        Effect::PathPrefix | Effect::SubtreeComplete | Effect::Escape => {}
    }
    if !analysis
        .causes
        .iter()
        .any(|cause| cause.effect == effect && cause.span == span)
    {
        analysis.causes.push(CapabilityCause { effect, span });
    }
}

fn error(code: &str, message: String, span: Span) -> Box<Diagnostic> {
    Box::new(Diagnostic::new(code, DiagnosticClass::Compile, &message).at(span, message))
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, sync::Arc};

    use crate::parse;

    use super::{
        AnalysisContext, BuiltinRegistry, ResolveOptions, analyze, analyze_with_context, resolve,
    };

    #[test]
    fn resolves_scope_shadowing_cli_variables_and_unknowns() {
        let options = ResolveOptions {
            variables: BTreeSet::from([Arc::from("cli")]),
        };
        resolve(
            parse("1 as $x | (2 as $x | $x), $x, $cli").unwrap(),
            &options,
        )
        .unwrap();
        let error = resolve(parse("$missing").unwrap(), &ResolveOptions::default()).unwrap_err();
        assert_eq!(error.code, "TQ-RESOLVE-VARIABLE-001");
    }

    #[test]
    fn registry_is_versioned_and_checks_arity_and_deferred_names() {
        assert_eq!(BuiltinRegistry::VERSION, 1);
        assert!(BuiltinRegistry.get("sort_by").unwrap().blocking);
        assert_eq!(
            resolve(parse("range()").unwrap(), &ResolveOptions::default())
                .unwrap_err()
                .code,
            "TQ-RESOLVE-ARITY-001"
        );
        assert_eq!(
            resolve(parse("test(\"a\")").unwrap(), &ResolveOptions::default())
                .unwrap_err()
                .code,
            "TQ-CAP-REGEX"
        );
    }

    #[test]
    fn every_deferred_grammar_family_has_a_stable_capability_code() {
        let parse_cases = [
            ("def f: .; f", "TQ-CAP-FUNCTION"),
            ("module {}; .", "TQ-CAP-MODULES"),
            ("import \"x\" as x; .", "TQ-CAP-IMPORT"),
            ("include \"x\"; .", "TQ-CAP-INCLUDE"),
            ("reduce .[] as $x (0; .)", "TQ-CAP-REDUCE"),
            ("foreach .[] as $x (0; .; .)", "TQ-CAP-FOREACH"),
            ("label $x | .", "TQ-CAP-LABELS"),
            ("break $x", "TQ-CAP-BREAK"),
            ("..", "TQ-CAP-RECURSIVE-DESCENT"),
            ("\"x=\\(.)\"", "TQ-CAP-INTERPOLATION"),
        ];
        for (query, code) in parse_cases {
            assert_eq!(parse(query).unwrap_err().code, code, "{query}");
        }
        let resolve_cases = [
            ("test(\"a\")", "TQ-CAP-REGEX"),
            ("fromdateiso8601", "TQ-CAP-DATES"),
            ("env", "TQ-CAP-ENVIRONMENT"),
            ("input_filename", "TQ-CAP-PLATFORM-IO"),
            ("nan", "TQ-CAP-NONFINITE-RESULT"),
        ];
        for (query, code) in resolve_cases {
            let error = resolve(parse(query).unwrap(), &ResolveOptions::default()).unwrap_err();
            assert_eq!(error.code, code, "{query}");
        }
    }

    #[test]
    fn analysis_reports_blocking_mutation_generator_and_failure_causes() {
        let resolved = resolve(
            parse("(.items[] | sort_by(.n)) |= . + 1").unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap();
        let analyzed = analyze(resolved);
        let capabilities = analyzed.capabilities();
        assert!(capabilities.document);
        assert!(capabilities.blocking);
        assert!(capabilities.mutation);
        assert!(capabilities.generator);
        assert!(capabilities.possible_failure);
    }

    #[test]
    fn human_and_machine_explanations_include_hir_effects_and_spans() {
        let analyzed =
            analyze(resolve(parse(".[] | sort").unwrap(), &ResolveOptions::default()).unwrap());
        let human = analyzed.explain();
        assert!(human.contains("phase: analyzed"));
        assert!(human.contains("Blocking"));
        let machine = analyzed.explain_json();
        assert_eq!(machine["schema_version"], 1);
        assert_eq!(machine["phase"], "analyzed");
        assert_eq!(machine["span"]["start"], 0);
        assert_eq!(machine["analysis"]["capabilities"]["blocking"], true);
    }

    #[test]
    fn context_covers_event_and_whole_input_effects() {
        let event = analyze_with_context(
            resolve(parse(".").unwrap(), &ResolveOptions::default()).unwrap(),
            AnalysisContext {
                event_input: true,
                whole_input: false,
                automatic_streaming: false,
            },
        );
        assert!(event.capabilities().event_stream);
        assert!(!event.capabilities().document);

        let slurped = analyze_with_context(
            resolve(parse(".").unwrap(), &ResolveOptions::default()).unwrap(),
            AnalysisContext {
                event_input: false,
                whole_input: true,
                automatic_streaming: false,
            },
        );
        assert!(slurped.capabilities().whole_input);
        assert!(slurped.capabilities().document);
    }
}
