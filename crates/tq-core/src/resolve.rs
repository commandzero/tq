//! Lexical variable, user filter, module, and versioned built-in resolution.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use sha2::{Digest as _, Sha256};

use crate::{
    Analysis, Analyzed, CapabilityCause, Diagnostic, DiagnosticClass, Effect, ModuleInfo,
    OptimizerRewrite, Parsed, PlanKind, Query, Resolved, SourceId, Span, Value,
    ast::{
        Access, CallTarget, Definition, Expr, ExprKind, InterpolationSegment, ObjectKey,
        ParameterKind,
    },
    parser::parse_module_ast,
    phase::{automatic_stream_proof, hybrid_stream_proof},
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
    pub const VERSION: u32 = 2;

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
    builtin("all", 2, 2, false),
    builtin("any", 2, 2, false),
    builtin("arrays", 0, 0, false),
    builtin("ascii_downcase", 0, 0, false),
    builtin("booleans", 0, 0, false),
    builtin("capture", 1, 2, false),
    builtin("ceil", 0, 0, false),
    builtin("empty", 0, 0, false),
    builtin("env", 0, 0, false),
    builtin("error", 0, 1, false),
    builtin("explode", 0, 0, false),
    builtin("fabs", 0, 0, false),
    builtin("flatten", 0, 1, true),
    builtin("floor", 0, 0, false),
    builtin("fromdate", 0, 0, false),
    builtin("fromdateiso8601", 0, 0, false),
    builtin("fromjson", 0, 0, false),
    builtin("getpath", 1, 1, false),
    builtin("gmtime", 0, 0, false),
    builtin("group_by", 1, 1, true),
    builtin("gsub", 2, 3, false),
    builtin("has", 1, 1, false),
    builtin("in", 1, 1, false),
    builtin("input_filename", 0, 0, false),
    builtin("input_line_number", 0, 0, false),
    builtin("inputs", 0, 0, false),
    builtin("implode", 0, 0, false),
    builtin("iterables", 0, 0, false),
    builtin("keys", 0, 0, true),
    builtin("keys_unsorted", 0, 0, false),
    builtin("length", 0, 0, false),
    builtin("limit", 2, 2, false),
    builtin("localtime", 0, 0, false),
    builtin("ltrimstr", 1, 1, false),
    builtin("map", 1, 1, true),
    builtin("map_values", 1, 1, true),
    builtin("max", 0, 0, true),
    builtin("match", 1, 2, false),
    builtin("min", 0, 0, true),
    builtin("max_by", 1, 1, true),
    builtin("min_by", 1, 1, true),
    builtin("mktime", 0, 0, false),
    builtin("modulemeta", 0, 0, false),
    builtin("nulls", 0, 0, false),
    builtin("now", 0, 0, false),
    builtin("numbers", 0, 0, false),
    builtin("objects", 0, 0, false),
    builtin("path", 1, 1, false),
    builtin("paths", 0, 0, false),
    builtin("range", 1, 3, false),
    builtin("reverse", 0, 0, true),
    builtin("scan", 1, 2, false),
    builtin("scalars", 0, 0, false),
    builtin("select", 1, 1, false),
    builtin("setpath", 2, 2, false),
    builtin("sort", 0, 0, true),
    builtin("sort_by", 1, 1, true),
    builtin("split", 1, 2, false),
    builtin("splits", 1, 2, false),
    builtin("strftime", 1, 1, false),
    builtin("strflocaltime", 1, 1, false),
    builtin("strptime", 1, 1, false),
    builtin("strings", 0, 0, false),
    builtin("sub", 2, 3, false),
    builtin("test", 1, 2, false),
    builtin("todate", 0, 0, false),
    builtin("todateiso8601", 0, 0, false),
    builtin("to_entries", 0, 0, true),
    builtin("tojson", 0, 0, false),
    builtin("tostream", 0, 0, false),
    builtin("tonumber", 0, 0, false),
    builtin("tostring", 0, 0, false),
    builtin("type", 0, 0, false),
    builtin("unique", 0, 0, true),
    builtin("unique_by", 1, 1, true),
    builtin("utf8bytelength", 0, 0, false),
    builtin("values", 0, 0, false),
    builtin("with_entries", 1, 1, true),
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
#[derive(Clone, Debug)]
pub struct ResolveOptions {
    /// CLI variable names without `$`.
    pub variables: BTreeSet<Arc<str>>,
    /// Explicit canonicalizable roots used for jq module lookup.
    pub module_roots: Vec<PathBuf>,
    /// Maximum distinct module files admitted by one compilation.
    pub module_limit: usize,
    /// Maximum bytes read from one module file.
    pub module_bytes: usize,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            variables: BTreeSet::new(),
            module_roots: Vec::new(),
            module_limit: 256,
            module_bytes: 8 * 1024 * 1024,
        }
    }
}

#[derive(Clone)]
struct CachedModule {
    ast: Expr,
    info: ModuleInfo,
}

struct ModuleLoader {
    roots: Vec<PathBuf>,
    cache: BTreeMap<PathBuf, CachedModule>,
    stack: Vec<PathBuf>,
    module_limit: usize,
    module_bytes: usize,
    next_source: u32,
}

impl ModuleLoader {
    fn new(options: &ResolveOptions) -> Result<Self, Box<Diagnostic>> {
        let roots = options
            .module_roots
            .iter()
            .map(|root| {
                fs::canonicalize(root).map_err(|error| {
                    module_error(
                        "TQ-MODULE-ROOT-001",
                        format!(
                            "module root '{}' cannot be canonicalized: {error}",
                            root.display()
                        ),
                        Span::new(SourceId::new(0), 0, 0),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            roots,
            cache: BTreeMap::new(),
            stack: Vec::new(),
            module_limit: options.module_limit,
            module_bytes: options.module_bytes,
            next_source: 1,
        })
    }

    fn module_info(&self) -> Vec<ModuleInfo> {
        self.cache
            .values()
            .map(|module| module.info.clone())
            .collect()
    }

    fn expand(&mut self, mut expr: Expr) -> Result<Expr, Box<Diagnostic>> {
        match expr.kind {
            ExprKind::Include {
                path,
                metadata,
                body,
            } => {
                validate_metadata(metadata.as_deref(), expr.span)?;
                let body = self.expand(*body)?;
                let module = self.load(&path, expr.span)?;
                splice_module(module.ast, body, expr.span)
            }
            ExprKind::Import {
                path,
                alias,
                metadata,
                body,
            } => {
                validate_metadata(metadata.as_deref(), expr.span)?;
                let body = self.expand(*body)?;
                let mut module = self.load(&path, expr.span)?.ast;
                qualify_module(&mut module, &alias);
                splice_module(module, body, expr.span)
            }
            ExprKind::Module { metadata, body } => {
                validate_metadata(Some(&metadata), expr.span)?;
                self.expand(*body)
            }
            _ => {
                self.expand_children(&mut expr)?;
                Ok(expr)
            }
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "module expansion exhaustively preserves every AST child position"
    )]
    fn expand_children(&mut self, expr: &mut Expr) -> Result<(), Box<Diagnostic>> {
        match &mut expr.kind {
            ExprKind::Interpolation(segments) => {
                for segment in segments {
                    if let InterpolationSegment::Expression(expression) = segment {
                        *expression = self.expand(expression.clone())?;
                    }
                }
            }
            ExprKind::Access { base, access } => {
                **base = self.expand((**base).clone())?;
                match access {
                    Access::Index(index) => **index = self.expand((**index).clone())?,
                    Access::Slice { start, end } => {
                        if let Some(start) = start {
                            **start = self.expand((**start).clone())?;
                        }
                        if let Some(end) = end {
                            **end = self.expand((**end).clone())?;
                        }
                    }
                    Access::Field(_) | Access::Iterate => {}
                }
            }
            ExprKind::Optional(expression)
            | ExprKind::Array(expression)
            | ExprKind::Unary { expression, .. } => {
                **expression = self.expand((**expression).clone())?;
            }
            ExprKind::Pipe(left, right)
            | ExprKind::Comma(left, right)
            | ExprKind::Binary { left, right, .. }
            | ExprKind::Assignment {
                path: left,
                value: right,
                ..
            } => {
                **left = self.expand((**left).clone())?;
                **right = self.expand((**right).clone())?;
            }
            ExprKind::Object(entries) => {
                for entry in entries {
                    if let ObjectKey::Computed(key) = &mut entry.key {
                        *key = self.expand(key.clone())?;
                    }
                    entry.value = self.expand(entry.value.clone())?;
                }
            }
            ExprKind::Conditional {
                branches,
                alternative,
            } => {
                for (condition, body) in branches {
                    *condition = self.expand(condition.clone())?;
                    *body = self.expand(body.clone())?;
                }
                **alternative = self.expand((**alternative).clone())?;
            }
            ExprKind::Bind { value, body, .. } => {
                **value = self.expand((**value).clone())?;
                **body = self.expand((**body).clone())?;
            }
            ExprKind::Reduce {
                generator,
                initial,
                update,
                ..
            } => {
                **generator = self.expand((**generator).clone())?;
                **initial = self.expand((**initial).clone())?;
                **update = self.expand((**update).clone())?;
            }
            ExprKind::Foreach {
                generator,
                initial,
                update,
                extract,
                ..
            } => {
                **generator = self.expand((**generator).clone())?;
                **initial = self.expand((**initial).clone())?;
                **update = self.expand((**update).clone())?;
                **extract = self.expand((**extract).clone())?;
            }
            ExprKind::Define { definition, body } => {
                definition.body = self.expand(definition.body.clone())?;
                **body = self.expand((**body).clone())?;
            }
            ExprKind::Call { arguments, .. } => {
                for argument in arguments {
                    *argument = self.expand(argument.clone())?;
                }
            }
            ExprKind::TryCatch { expression, catch } => {
                **expression = self.expand((**expression).clone())?;
                if let Some(catch) = catch {
                    **catch = self.expand((**catch).clone())?;
                }
            }
            ExprKind::Identity
            | ExprKind::Literal(_)
            | ExprKind::Variable(_)
            | ExprKind::Empty
            | ExprKind::RecursiveDescent => {}
            ExprKind::Include { .. } | ExprKind::Import { .. } | ExprKind::Module { .. } => {
                unreachable!("module wrapper handled before child expansion")
            }
        }
        Ok(())
    }

    fn load(&mut self, requested: &str, span: Span) -> Result<CachedModule, Box<Diagnostic>> {
        let canonical = self.resolve_path(requested, span)?;
        if let Some(position) = self.stack.iter().position(|path| path == &canonical) {
            let mut cycle = self.stack[position..]
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>();
            cycle.push(canonical.display().to_string());
            return Err(module_error(
                "TQ-MODULE-CYCLE-001",
                format!("cyclic module import: {}", cycle.join(" -> ")),
                span,
            ));
        }
        if let Some(module) = self.cache.get(&canonical) {
            return Ok(module.clone());
        }
        if self.cache.len().saturating_add(self.stack.len()) >= self.module_limit {
            return Err(module_error(
                "TQ-RESOURCE-MODULES-001",
                "module count limit exceeded".to_owned(),
                span,
            ));
        }
        let bytes = fs::read(&canonical).map_err(|error| {
            module_error(
                "TQ-MODULE-READ-001",
                format!("failed to read module '{}': {error}", canonical.display()),
                span,
            )
        })?;
        if bytes.len() > self.module_bytes {
            return Err(module_error(
                "TQ-RESOURCE-MODULE-BYTES-001",
                format!("module '{}' exceeds the byte limit", canonical.display()),
                span,
            ));
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| {
            module_error(
                "TQ-MODULE-UTF8-001",
                format!("module '{}' is not valid UTF-8", canonical.display()),
                span,
            )
        })?;
        let source_id = SourceId::new(self.next_source);
        self.next_source = self.next_source.saturating_add(1);
        let parsed = parse_module_ast(&canonical.display().to_string(), text, source_id)?;
        let (metadata, parsed) = module_metadata(parsed)?;
        let metadata = enrich_module_metadata(metadata, &parsed);
        self.stack.push(canonical.clone());
        let expanded = self.expand(parsed);
        self.stack.pop();
        let ast = expanded?;
        let info = ModuleInfo {
            name: requested.to_owned(),
            canonical_path: canonical.display().to_string(),
            sha256: hex_digest(&bytes),
            metadata,
        };
        let module = CachedModule { ast, info };
        self.cache.insert(canonical, module.clone());
        Ok(module)
    }

    fn resolve_path(&self, requested: &str, span: Span) -> Result<PathBuf, Box<Diagnostic>> {
        let requested_path = Path::new(requested);
        if requested_path.is_absolute()
            || requested_path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(module_error(
                "TQ-MODULE-CONFINEMENT-001",
                format!("module path {requested:?} escapes configured roots"),
                span,
            ));
        }
        if self.roots.is_empty() {
            return Err(module_error(
                "TQ-MODULE-ROOT-001",
                format!("module {requested:?} requires an explicit module root"),
                span,
            ));
        }
        let leaf = requested_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(requested);
        for root in &self.roots {
            let direct = root.join(format!("{requested}.jq"));
            let nested = root.join(requested).join(format!("{leaf}.jq"));
            for candidate in [direct, nested] {
                let Ok(canonical) = fs::canonicalize(&candidate) else {
                    continue;
                };
                if !canonical.starts_with(root) {
                    return Err(module_error(
                        "TQ-MODULE-CONFINEMENT-001",
                        format!(
                            "module path {requested:?} resolves outside root '{}'",
                            root.display()
                        ),
                        span,
                    ));
                }
                if canonical.is_file() {
                    return Ok(canonical);
                }
            }
        }
        Err(module_error(
            "TQ-MODULE-NOT-FOUND-001",
            format!("module {requested:?} was not found in configured roots"),
            span,
        ))
    }
}

fn module_metadata(expr: Expr) -> Result<(Value, Expr), Box<Diagnostic>> {
    if let ExprKind::Module { metadata, body } = expr.kind {
        let metadata = constant_value(&metadata).ok_or_else(|| {
            module_error(
                "TQ-MODULE-METADATA-001",
                "module metadata must be a constant expression".to_owned(),
                metadata.span,
            )
        })?;
        Ok((metadata, *body))
    } else {
        Ok((Value::object(crate::Object::new()), expr))
    }
}

fn enrich_module_metadata(metadata: Value, expr: &Expr) -> Value {
    let mut metadata = match metadata {
        Value::Object(values) => values.as_ref().clone(),
        value => return value,
    };
    let mut definitions = Vec::new();
    collect_definition_names(expr, &mut definitions);
    let mut dependencies = Vec::new();
    collect_module_dependencies(expr, &mut dependencies);
    metadata.insert(Arc::from("deps"), Value::array(dependencies));
    metadata.insert(
        Arc::from("defs"),
        Value::array(
            definitions
                .into_iter()
                .map(|(name, arity)| Value::string(format!("{name}/{arity}")))
                .collect::<Vec<_>>(),
        ),
    );
    Value::object(metadata)
}

fn collect_module_dependencies(expr: &Expr, dependencies: &mut Vec<Value>) {
    match &expr.kind {
        ExprKind::Import {
            path, alias, body, ..
        } => {
            dependencies.push(Value::object(crate::Object::from_iter([
                (Arc::from("as"), Value::string(alias.as_ref())),
                (Arc::from("is_data"), Value::Bool(false)),
                (Arc::from("relpath"), Value::string(path.as_ref())),
            ])));
            collect_module_dependencies(body, dependencies);
        }
        ExprKind::Include { path, body, .. } => {
            dependencies.push(Value::object(crate::Object::from_iter([
                (Arc::from("is_data"), Value::Bool(false)),
                (Arc::from("relpath"), Value::string(path.as_ref())),
            ])));
            collect_module_dependencies(body, dependencies);
        }
        ExprKind::Define { body, .. } | ExprKind::Module { body, .. } => {
            collect_module_dependencies(body, dependencies);
        }
        _ => {}
    }
}

fn collect_definition_names(expr: &Expr, definitions: &mut Vec<(Arc<str>, usize)>) {
    match &expr.kind {
        ExprKind::Define { definition, body } => {
            definitions.push((Arc::clone(&definition.name), definition.parameters.len()));
            collect_definition_names(body, definitions);
        }
        ExprKind::Include { body, .. }
        | ExprKind::Import { body, .. }
        | ExprKind::Module { body, .. } => collect_definition_names(body, definitions),
        _ => {}
    }
}

fn validate_metadata(metadata: Option<&Expr>, span: Span) -> Result<(), Box<Diagnostic>> {
    if metadata.is_some_and(|metadata| constant_value(metadata).is_none()) {
        return Err(module_error(
            "TQ-MODULE-METADATA-001",
            "module metadata must be a constant expression".to_owned(),
            span,
        ));
    }
    Ok(())
}

fn constant_value(expr: &Expr) -> Option<Value> {
    match &expr.kind {
        ExprKind::Literal(value) => Some(value.clone()),
        ExprKind::Array(body) => {
            let mut values = Vec::new();
            constant_sequence(body, &mut values)?;
            Some(Value::array(values))
        }
        ExprKind::Object(entries) => {
            let mut object = crate::Object::new();
            for entry in entries {
                let ObjectKey::Static(key) = &entry.key else {
                    return None;
                };
                object.insert(Arc::clone(key), constant_value(&entry.value)?);
            }
            Some(Value::object(object))
        }
        _ => None,
    }
}

fn constant_sequence(expr: &Expr, output: &mut Vec<Value>) -> Option<()> {
    if let ExprKind::Comma(left, right) = &expr.kind {
        constant_sequence(left, output)?;
        constant_sequence(right, output)
    } else if matches!(expr.kind, ExprKind::Empty) {
        Some(())
    } else {
        output.push(constant_value(expr)?);
        Some(())
    }
}

fn splice_module(module: Expr, body: Expr, span: Span) -> Result<Expr, Box<Diagnostic>> {
    match module.kind {
        ExprKind::Define {
            definition,
            body: module_body,
        } => Ok(Expr::new(
            ExprKind::Define {
                definition,
                body: Box::new(splice_module(*module_body, body, span)?),
            },
            span,
        )),
        ExprKind::Empty => Ok(body),
        _ => Err(module_error(
            "TQ-MODULE-CONTENT-001",
            "module files may contain metadata, imports, includes, and definitions".to_owned(),
            module.span,
        )),
    }
}

fn qualify_module(expr: &mut Expr, alias: &str) {
    let mut definitions = BTreeSet::new();
    collect_definitions(expr, &mut definitions);
    qualify_expr(expr, alias, &definitions);
}

fn collect_definitions(expr: &Expr, definitions: &mut BTreeSet<(Arc<str>, usize)>) {
    if let ExprKind::Define { definition, body } = &expr.kind {
        definitions.insert((Arc::clone(&definition.name), definition.parameters.len()));
        collect_definitions(body, definitions);
    }
}

fn qualify_expr(expr: &mut Expr, alias: &str, definitions: &BTreeSet<(Arc<str>, usize)>) {
    match &mut expr.kind {
        ExprKind::Define { definition, body } => {
            let old = Arc::clone(&definition.name);
            definition.name = Arc::from(format!("{alias}::{old}"));
            qualify_expr(&mut definition.body, alias, definitions);
            qualify_expr(body, alias, definitions);
        }
        ExprKind::Call {
            name, arguments, ..
        } => {
            if definitions.contains(&(Arc::clone(name), arguments.len())) {
                *name = Arc::from(format!("{alias}::{name}"));
            }
            for argument in arguments {
                qualify_expr(argument, alias, definitions);
            }
        }
        _ => walk_expr_mut(expr, |child| qualify_expr(child, alias, definitions)),
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the shared AST walker keeps module qualification exhaustive"
)]
fn walk_expr_mut(expr: &mut Expr, mut visit: impl FnMut(&mut Expr)) {
    match &mut expr.kind {
        ExprKind::Interpolation(segments) => {
            for segment in segments {
                if let InterpolationSegment::Expression(expression) = segment {
                    visit(expression);
                }
            }
        }
        ExprKind::Access { base, access } => {
            visit(base);
            match access {
                Access::Index(index) => visit(index),
                Access::Slice { start, end } => {
                    if let Some(start) = start {
                        visit(start);
                    }
                    if let Some(end) = end {
                        visit(end);
                    }
                }
                Access::Field(_) | Access::Iterate => {}
            }
        }
        ExprKind::Optional(expression)
        | ExprKind::Array(expression)
        | ExprKind::Unary { expression, .. } => visit(expression),
        ExprKind::Pipe(left, right)
        | ExprKind::Comma(left, right)
        | ExprKind::Binary { left, right, .. }
        | ExprKind::Assignment {
            path: left,
            value: right,
            ..
        } => {
            visit(left);
            visit(right);
        }
        ExprKind::Object(entries) => {
            for entry in entries {
                if let ObjectKey::Computed(key) = &mut entry.key {
                    visit(key);
                }
                visit(&mut entry.value);
            }
        }
        ExprKind::Conditional {
            branches,
            alternative,
        } => {
            for (condition, body) in branches {
                visit(condition);
                visit(body);
            }
            visit(alternative);
        }
        ExprKind::Bind { value, body, .. } => {
            visit(value);
            visit(body);
        }
        ExprKind::Reduce {
            generator,
            initial,
            update,
            ..
        } => {
            visit(generator);
            visit(initial);
            visit(update);
        }
        ExprKind::Foreach {
            generator,
            initial,
            update,
            extract,
            ..
        } => {
            visit(generator);
            visit(initial);
            visit(update);
            visit(extract);
        }
        ExprKind::Define { definition, body } => {
            visit(&mut definition.body);
            visit(body);
        }
        ExprKind::Include { metadata, body, .. } | ExprKind::Import { metadata, body, .. } => {
            if let Some(metadata) = metadata {
                visit(metadata);
            }
            visit(body);
        }
        ExprKind::Module { metadata, body } => {
            visit(metadata);
            visit(body);
        }
        ExprKind::Call { arguments, .. } => {
            for argument in arguments {
                visit(argument);
            }
        }
        ExprKind::TryCatch { expression, catch } => {
            visit(expression);
            if let Some(catch) = catch {
                visit(catch);
            }
        }
        ExprKind::Identity
        | ExprKind::Literal(_)
        | ExprKind::Variable(_)
        | ExprKind::Empty
        | ExprKind::RecursiveDescent => {}
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn module_error(code: &str, message: String, span: Span) -> Box<Diagnostic> {
    Box::new(Diagnostic::new(code, DiagnosticClass::Compile, &message).at(span, message))
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
    mut query: Query<Parsed>,
    options: &ResolveOptions,
) -> Result<Query<Resolved>, Box<Diagnostic>> {
    let mut loader = ModuleLoader::new(options)?;
    let expanded = loader.expand(query.ast().clone())?;
    *query.ast_mut() = expanded;
    query.set_modules(loader.module_info());

    Resolver::new(options).resolve_expr(query.ast_mut())?;
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
pub fn analyze_with_context(
    mut query: Query<Resolved>,
    context: AnalysisContext,
) -> Query<Analyzed> {
    let mut analysis = Analysis::default();
    optimize_resolved(query.ast_mut(), &mut analysis.optimizer_rewrites);
    if matches!(query.ast().kind, ExprKind::Identity) {
        add_effect(&mut analysis, Effect::SemanticIdentity, query.ast().span);
    }
    analyze_expr(query.ast(), &mut analysis);
    if context.whole_input || analysis.capabilities.whole_input {
        add_effect(&mut analysis, Effect::WholeInput, query.ast().span);
        add_effect(&mut analysis, Effect::Document, query.ast().span);
        analysis.selected_plan = PlanKind::WholeInput;
        analysis.stream_rejection = Some("whole-input mode requires all documents".to_owned());
    } else if context.event_input && event_effects_compatible(&analysis) {
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
        if context.automatic_streaming
            && !analysis.capabilities.mutation
            && let Some(proof) = hybrid_stream_proof(query.ast())
        {
            add_effect(&mut analysis, Effect::PathPrefix, proof.producer.cause);
            if proof.producer.subtree_complete {
                add_effect(&mut analysis, Effect::SubtreeComplete, proof.producer.cause);
                add_effect(&mut analysis, Effect::Subtree, proof.producer.cause);
            } else {
                add_effect(&mut analysis, Effect::EventStream, proof.producer.cause);
            }
            if proof.producer.value_escapes {
                add_effect(&mut analysis, Effect::Escape, proof.producer.cause);
            }
            analysis.selected_plan = PlanKind::HybridBlocking;
            analysis.stream_proof = Some(proof.producer.clone());
            analysis.hybrid_proof = Some(proof);
        } else {
            add_effect(&mut analysis, Effect::Document, query.ast().span);
            analysis.selected_plan = PlanKind::Blocking;
            analysis.stream_rejection = Some(
                if context.automatic_streaming {
                    "blocking query dependency is not statically partitionable into a sound streaming producer and suffix"
                } else {
                    "blocking operator requires complete input state"
                }
                .to_owned(),
            );
        }
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

fn optimize_resolved(expr: &mut Expr, rewrites: &mut Vec<OptimizerRewrite>) {
    walk_expr_mut(expr, |child| optimize_resolved(child, rewrites));
    let replacement = match &expr.kind {
        ExprKind::Pipe(left, length) if builtin_call(length, "length", 0) => match &left.kind {
            ExprKind::Pipe(array, sort)
                if matches!(array.kind, ExprKind::Array(_)) && builtin_call(sort, "sort", 0) =>
            {
                Some(((**array).clone(), sort.span, (**length).clone()))
            }
            _ => None,
        },
        _ => None,
    };
    if let Some((array, span, length)) = replacement {
        expr.kind = ExprKind::Pipe(Box::new(array), Box::new(length));
        rewrites.push(OptimizerRewrite {
            name: "array-sort-before-length",
            span,
        });
    }
}

fn builtin_call(expr: &Expr, expected: &str, arity: usize) -> bool {
    matches!(
        &expr.kind,
        ExprKind::Call {
            name,
            arguments,
            target: Some(CallTarget::Builtin),
        } if &**name == expected && arguments.len() == arity
    )
}

struct Resolver {
    variables: Vec<BTreeMap<Arc<str>, Arc<str>>>,
    functions: Vec<BTreeMap<(Arc<str>, usize), CallTarget>>,
    registry: BuiltinRegistry,
    next_variable: u32,
    next_function: u32,
}

impl Resolver {
    fn new(options: &ResolveOptions) -> Self {
        Self {
            variables: vec![
                options
                    .variables
                    .iter()
                    .map(|name| (Arc::clone(name), Arc::clone(name)))
                    .collect(),
            ],
            functions: vec![BTreeMap::new()],
            registry: BuiltinRegistry,
            next_variable: 0,
            next_function: 0,
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "resolution exhaustively walks syntax and rewrites stable symbols"
    )]
    fn resolve_expr(&mut self, expr: &mut Expr) -> Result<(), Box<Diagnostic>> {
        match &mut expr.kind {
            ExprKind::Variable(name) => {
                let Some(runtime) = self
                    .variables
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(name))
                else {
                    return Err(error(
                        "TQ-RESOLVE-VARIABLE-001",
                        format!("unknown variable ${name}"),
                        expr.span,
                    ));
                };
                *name = Arc::clone(runtime);
            }
            ExprKind::Interpolation(segments) => {
                for segment in segments {
                    if let InterpolationSegment::Expression(expression) = segment {
                        self.resolve_expr(expression)?;
                    }
                }
            }
            ExprKind::Access { base, access } => {
                self.resolve_expr(base)?;
                match access {
                    Access::Index(index) => self.resolve_expr(index)?,
                    Access::Slice { start, end } => {
                        if let Some(start) = start {
                            self.resolve_expr(start)?;
                        }
                        if let Some(end) = end {
                            self.resolve_expr(end)?;
                        }
                    }
                    Access::Field(_) | Access::Iterate => {}
                }
            }
            ExprKind::Optional(expression)
            | ExprKind::Array(expression)
            | ExprKind::Unary { expression, .. } => self.resolve_expr(expression)?,
            ExprKind::Pipe(left, right)
            | ExprKind::Comma(left, right)
            | ExprKind::Binary { left, right, .. }
            | ExprKind::Assignment {
                path: left,
                value: right,
                ..
            } => {
                self.resolve_expr(left)?;
                self.resolve_expr(right)?;
            }
            ExprKind::Object(entries) => {
                for entry in entries {
                    if let ObjectKey::Computed(key) = &mut entry.key {
                        self.resolve_expr(key)?;
                    }
                    self.resolve_expr(&mut entry.value)?;
                }
            }
            ExprKind::Conditional {
                branches,
                alternative,
            } => {
                for (condition, body) in branches {
                    self.resolve_expr(condition)?;
                    self.resolve_expr(body)?;
                }
                self.resolve_expr(alternative)?;
            }
            ExprKind::Bind { value, name, body } => {
                self.resolve_expr(value)?;
                let source_name = Arc::clone(name);
                let runtime = self.runtime_variable(&source_name);
                *name = Arc::clone(&runtime);
                self.variables
                    .push(BTreeMap::from([(source_name, runtime)]));
                let result = self.resolve_expr(body);
                self.variables.pop();
                result?;
            }
            ExprKind::Reduce {
                generator,
                name,
                initial,
                update,
            } => {
                self.resolve_expr(generator)?;
                self.resolve_expr(initial)?;
                let source_name = Arc::clone(name);
                let runtime = self.runtime_variable(&source_name);
                *name = Arc::clone(&runtime);
                self.variables
                    .push(BTreeMap::from([(source_name, runtime)]));
                let result = self.resolve_expr(update);
                self.variables.pop();
                result?;
            }
            ExprKind::Foreach {
                generator,
                name,
                initial,
                update,
                extract,
            } => {
                self.resolve_expr(generator)?;
                self.resolve_expr(initial)?;
                let source_name = Arc::clone(name);
                let runtime = self.runtime_variable(&source_name);
                *name = Arc::clone(&runtime);
                self.variables
                    .push(BTreeMap::from([(source_name, runtime)]));
                let result = self
                    .resolve_expr(update)
                    .and_then(|()| self.resolve_expr(extract));
                self.variables.pop();
                result?;
            }
            ExprKind::Define { definition, body } => {
                self.resolve_definition(definition, body)?;
            }
            ExprKind::Call {
                name,
                arguments,
                target,
            } => {
                for argument in &mut *arguments {
                    self.resolve_expr(argument)?;
                }
                let arity = arguments.len();
                if let Some(resolved) = self
                    .functions
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(&(Arc::clone(name), arity)).copied())
                {
                    *target = Some(resolved);
                } else if self
                    .functions
                    .iter()
                    .rev()
                    .any(|scope| scope.keys().any(|(candidate, _)| candidate == name))
                {
                    return Err(error(
                        "TQ-RESOLVE-ARITY-001",
                        format!("invalid arity {arity} for user filter {name}"),
                        expr.span,
                    ));
                } else if let Some(capability) = deferred_builtin(name) {
                    return Err(error(
                        &format!("TQ-CAP-{}", capability.to_ascii_uppercase()),
                        format!("jq capability {capability:?} is deferred"),
                        expr.span,
                    ));
                } else if let Some(builtin) = self.registry.get(name) {
                    if !(builtin.minimum_arity..=builtin.maximum_arity).contains(&arity) {
                        return Err(error(
                            "TQ-RESOLVE-ARITY-001",
                            format!("invalid arity {arity} for built-in {name}"),
                            expr.span,
                        ));
                    }
                    *target = Some(CallTarget::Builtin);
                } else {
                    return Err(error(
                        "TQ-RESOLVE-BUILTIN-001",
                        format!("unknown filter {name}/{arity}"),
                        expr.span,
                    ));
                }
            }
            ExprKind::TryCatch { expression, catch } => {
                self.resolve_expr(expression)?;
                if let Some(catch) = catch {
                    self.resolve_expr(catch)?;
                }
            }
            ExprKind::Include { .. } | ExprKind::Import { .. } | ExprKind::Module { .. } => {
                return Err(error(
                    "TQ-MODULE-INTERNAL-001",
                    "module directive remained after expansion".to_owned(),
                    expr.span,
                ));
            }
            ExprKind::Identity
            | ExprKind::Literal(_)
            | ExprKind::Empty
            | ExprKind::RecursiveDescent => {}
        }
        Ok(())
    }

    fn resolve_definition(
        &mut self,
        definition: &mut Definition,
        body: &mut Expr,
    ) -> Result<(), Box<Diagnostic>> {
        let symbol = self.next_function;
        self.next_function = self.next_function.checked_add(1).ok_or_else(|| {
            error(
                "TQ-RESOURCE-FUNCTIONS-001",
                "user filter symbol limit exceeded".to_owned(),
                definition.span,
            )
        })?;
        definition.symbol = Some(symbol);
        let signature = (Arc::clone(&definition.name), definition.parameters.len());
        self.functions
            .push(BTreeMap::from([(signature, CallTarget::User(symbol))]));

        let mut parameter_variables = BTreeMap::new();
        let mut parameter_filters = BTreeMap::new();
        let mut seen_parameters = BTreeSet::new();
        for (index, parameter) in definition.parameters.iter_mut().enumerate() {
            if !seen_parameters.insert(Arc::clone(&parameter.name)) {
                return Err(error(
                    "TQ-RESOLVE-PARAMETER-001",
                    format!("duplicate parameter {}", parameter.name),
                    parameter.span,
                ));
            }
            match parameter.kind {
                ParameterKind::Value => {
                    let runtime = self.runtime_variable(&parameter.name);
                    parameter_variables.insert(Arc::clone(&parameter.name), Arc::clone(&runtime));
                    parameter.runtime_name = Some(runtime);
                }
                ParameterKind::Filter => {
                    parameter_filters.insert(
                        (Arc::clone(&parameter.name), 0),
                        CallTarget::Parameter {
                            function: symbol,
                            index: u32::try_from(index).unwrap_or(u32::MAX),
                        },
                    );
                }
            }
        }
        self.variables.push(parameter_variables);
        self.functions.push(parameter_filters);
        let definition_result = self.resolve_expr(&mut definition.body);
        self.functions.pop();
        self.variables.pop();
        definition_result?;

        let body_result = self.resolve_expr(body);
        self.functions.pop();
        body_result
    }

    fn runtime_variable(&mut self, name: &str) -> Arc<str> {
        let symbol = self.next_variable;
        self.next_variable = self.next_variable.saturating_add(1);
        Arc::from(format!("@{symbol}:{name}"))
    }
}

fn deferred_builtin(name: &str) -> Option<&'static str> {
    match name {
        "nan" => Some("nonfinite-result"),
        "recurse" | "walk" => Some("recursive-builtins"),
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
        ExprKind::RecursiveDescent => {
            add_effect(analysis, Effect::Generator, expr.span);
            add_effect(analysis, Effect::Subtree, expr.span);
        }
        ExprKind::Interpolation(segments) => {
            for segment in segments {
                if let InterpolationSegment::Expression(expression) = segment {
                    analyze_expr(expression, analysis);
                }
            }
            add_effect(analysis, Effect::Generator, expr.span);
        }
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
        ExprKind::Reduce {
            generator,
            initial,
            update,
            ..
        } => {
            analyze_expr(generator, analysis);
            analyze_expr(initial, analysis);
            analyze_expr(update, analysis);
            add_effect(analysis, Effect::FoldState, expr.span);
            add_effect(analysis, Effect::Subtree, expr.span);
            add_effect(analysis, Effect::Blocking, expr.span);
        }
        ExprKind::Foreach {
            generator,
            initial,
            update,
            extract,
            ..
        } => {
            analyze_expr(generator, analysis);
            analyze_expr(initial, analysis);
            analyze_expr(update, analysis);
            analyze_expr(extract, analysis);
            add_effect(analysis, Effect::FoldState, expr.span);
            add_effect(analysis, Effect::Subtree, expr.span);
            add_effect(analysis, Effect::Generator, expr.span);
        }
        ExprKind::Define { definition, body } => {
            analyze_expr(&definition.body, analysis);
            analyze_expr(body, analysis);
            add_effect(analysis, Effect::Generator, expr.span);
            add_effect(analysis, Effect::PossibleFailure, expr.span);
        }
        ExprKind::Call {
            name, arguments, ..
        } => {
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
                    | "inputs"
                    | "paths"
                    | "tostream"
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
            if &**name == "inputs" {
                add_effect(analysis, Effect::WholeInput, expr.span);
            }
            if matches!(
                &**name,
                "all"
                    | "any"
                    | "ascii_downcase"
                    | "ceil"
                    | "explode"
                    | "fabs"
                    | "floor"
                    | "fromjson"
                    | "getpath"
                    | "group_by"
                    | "implode"
                    | "limit"
                    | "ltrimstr"
                    | "max_by"
                    | "min_by"
                    | "path"
                    | "setpath"
                    | "to_entries"
                    | "tojson"
                    | "tostream"
                    | "with_entries"
                    | "error"
                    | "tonumber"
                    | "length"
                    | "has"
                    | "in"
            ) {
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
        ExprKind::Include { .. } | ExprKind::Import { .. } | ExprKind::Module { .. } => {
            add_effect(analysis, Effect::Document, expr.span);
            add_effect(analysis, Effect::PossibleFailure, expr.span);
        }
    }
}

fn add_effect(analysis: &mut Analysis, effect: Effect, span: Span) {
    match effect {
        Effect::SemanticIdentity => analysis.capabilities.semantic_identity = true,
        Effect::EventStream => analysis.capabilities.event_stream = true,
        Effect::Subtree => analysis.capabilities.subtree = true,
        Effect::Document => analysis.capabilities.document = true,
        Effect::WholeInput => analysis.capabilities.whole_input = true,
        Effect::Blocking => analysis.capabilities.blocking = true,
        Effect::Mutation => analysis.capabilities.mutation = true,
        Effect::Generator => analysis.capabilities.generator = true,
        Effect::PossibleFailure => analysis.capabilities.possible_failure = true,
        Effect::FoldState => analysis.capabilities.fold_state = true,
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

fn event_effects_compatible(analysis: &Analysis) -> bool {
    if analysis.capabilities.document
        || analysis.capabilities.whole_input
        || analysis.capabilities.mutation
    {
        return false;
    }
    analysis.causes.iter().all(|cause| {
        !matches!(cause.effect, Effect::Subtree | Effect::Blocking)
            || analysis
                .causes
                .iter()
                .any(|fold| fold.effect == Effect::FoldState && fold.span == cause.span)
    })
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
            ..ResolveOptions::default()
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
    fn registry_is_versioned_and_checks_arity_and_new_builtin_names() {
        assert_eq!(BuiltinRegistry::VERSION, 2);
        assert!(BuiltinRegistry.get("sort_by").unwrap().blocking);
        assert_eq!(
            resolve(parse("range()").unwrap(), &ResolveOptions::default())
                .unwrap_err()
                .code,
            "TQ-RESOLVE-ARITY-001"
        );
        resolve(parse("test(\"a\")").unwrap(), &ResolveOptions::default()).unwrap();
        resolve(
            parse("fromdateiso8601").unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap();
        resolve(parse("env").unwrap(), &ResolveOptions::default()).unwrap();
        resolve(parse("input_filename").unwrap(), &ResolveOptions::default()).unwrap();
        for query in [
            "to_entries",
            "with_entries(.)",
            "group_by(.)",
            "min_by(.)",
            "max_by(.)",
            "limit(1; .)",
            "paths",
            "getpath([])",
            "setpath([]; 1)",
            "path(.)",
            "tostream",
            "tojson",
            "fromjson",
            "inputs",
            "any(.[]; .)",
            "all(.[]; .)",
            "ltrimstr(\"x\")",
            "ascii_downcase",
            "explode",
            "implode",
            "floor",
            "ceil",
            "fabs",
        ] {
            resolve(parse(query).unwrap(), &ResolveOptions::default()).unwrap();
        }
        resolve(parse("sort_by(.a,.b)").unwrap(), &ResolveOptions::default()).unwrap();
        resolve(
            parse("def pair(f; g): [f, g]; pair(1,2; 3,4)").unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap();
    }

    #[test]
    fn every_deferred_grammar_family_has_a_stable_capability_code() {
        let parse_cases = [
            ("label $x | .", "TQ-CAP-LABELS"),
            ("break $x", "TQ-CAP-BREAK"),
            ("@text \"x\"", "TQ-CAP-FORMAT-STRINGS"),
        ];
        for (query, code) in parse_cases {
            assert_eq!(parse(query).unwrap_err().code, code, "{query}");
        }
        let resolve_cases = [
            ("nan", "TQ-CAP-NONFINITE-RESULT"),
            ("recurse", "TQ-CAP-RECURSIVE-BUILTINS"),
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
    fn semantic_identity_proof_is_narrow_and_survives_compilation() {
        let identity = analyze(resolve(parse("(.)").unwrap(), &ResolveOptions::default()).unwrap());
        assert!(identity.capabilities().semantic_identity);
        assert!(identity.compile().unwrap().capabilities().semantic_identity);

        for query in [". | .", ".a", "empty", "if true then . else . end"] {
            let analyzed =
                analyze(resolve(parse(query).unwrap(), &ResolveOptions::default()).unwrap());
            assert!(
                !analyzed.capabilities().semantic_identity,
                "unexpected semantic-identity proof for {query}"
            );
        }
    }

    #[test]
    fn fold_resolution_and_analysis_cover_scope_cardinality_failure_and_retention() {
        let reduce = analyze(
            resolve(
                parse("reduce .[] as $x (0; . + $x)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        );
        assert!(reduce.capabilities().blocking);
        assert!(reduce.capabilities().subtree);
        assert!(reduce.capabilities().generator);
        assert!(reduce.capabilities().possible_failure);
        assert!(reduce.capabilities().fold_state);
        assert_eq!(reduce.analysis().selected_plan, crate::PlanKind::Blocking);

        let foreach = analyze(
            resolve(
                parse("foreach .[] as $x (0; . + $x; error(\"late\"))").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        );
        assert!(!foreach.capabilities().blocking);
        assert!(foreach.capabilities().subtree);
        assert!(foreach.capabilities().generator);
        assert!(foreach.capabilities().possible_failure);
        assert!(foreach.capabilities().fold_state);
        assert_eq!(foreach.analysis().selected_plan, crate::PlanKind::Document);

        assert_eq!(
            resolve(
                parse("reduce .[] as $x ($x; .)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap_err()
            .code,
            "TQ-RESOLVE-VARIABLE-001"
        );
    }

    #[test]
    fn recursive_and_interpolation_analysis_cover_cardinality_scope_and_retention() {
        let recursive =
            analyze(resolve(parse(".. | scalars").unwrap(), &ResolveOptions::default()).unwrap());
        assert!(recursive.capabilities().generator);
        assert!(recursive.capabilities().subtree);
        assert_eq!(
            recursive.analysis().selected_plan,
            crate::PlanKind::Document
        );

        let interpolation = analyze(
            resolve(
                parse("\"x=\\(.a, .b)\"").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        );
        assert!(interpolation.capabilities().generator);
        assert!(interpolation.capabilities().possible_failure);
        assert!(!interpolation.capabilities().blocking);
        assert_eq!(
            interpolation.analysis().selected_plan,
            crate::PlanKind::Document
        );

        assert_eq!(
            resolve(
                parse("\"x=\\($missing)\"").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap_err()
            .code,
            "TQ-RESOLVE-VARIABLE-001"
        );
    }

    #[test]
    fn explicit_events_admit_only_folds_with_event_compatible_bodies() {
        let compatible = analyze_with_context(
            resolve(
                parse("reduce .[] as $x (0; . + $x)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
            AnalysisContext {
                event_input: true,
                whole_input: false,
                automatic_streaming: false,
            },
        );
        assert_eq!(compatible.analysis().selected_plan, crate::PlanKind::Events);
        assert!(!compatible.capabilities().document);

        let incompatible = analyze_with_context(
            resolve(
                parse("reduce .[] as $x ([0]; . + [$x])").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
            AnalysisContext {
                event_input: true,
                whole_input: false,
                automatic_streaming: false,
            },
        );
        assert_eq!(
            incompatible.analysis().selected_plan,
            crate::PlanKind::Blocking
        );
        assert!(incompatible.capabilities().document);
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
    fn optimizer_removes_only_proven_array_sort_before_length() {
        let optimized = analyze(
            resolve(
                parse("[.features[].properties.release] | sort | length").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        );
        assert_eq!(optimized.analysis().optimizer_rewrites.len(), 1);
        assert_eq!(
            optimized.analysis().optimizer_rewrites[0].name,
            "array-sort-before-length"
        );
        assert!(!optimized.hir().contains("call(sort)"));
        assert_eq!(
            optimized.analysis().selected_plan,
            crate::PlanKind::HybridBlocking
        );

        for query in [
            ". | sort | length",
            "[.] | sort_by(.) | length",
            "def sort: .; [.] | sort | length",
            "[.] | (sort, .) | length",
        ] {
            let analyzed =
                analyze(resolve(parse(query).unwrap(), &ResolveOptions::default()).unwrap());
            assert!(
                analyzed.analysis().optimizer_rewrites.is_empty(),
                "unexpected rewrite for {query}"
            );
        }
    }

    #[test]
    fn hybrid_analysis_is_narrow_and_records_its_split() {
        let analyzed = analyze(
            resolve(
                parse("[.features[].properties.release] | sort").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        );
        assert_eq!(
            analyzed.analysis().selected_plan,
            crate::PlanKind::HybridBlocking
        );
        let proof = analyzed.analysis().hybrid_proof.as_ref().unwrap();
        assert_eq!(proof.producer.required_path_prefix.len(), 1);
        assert_eq!(proof.producer.projected_path.as_ref().unwrap().len(), 2);
        assert_eq!(proof.preparation, crate::HybridPreparation::StableSortRuns);

        for query in [
            "[.features[] | .[.key]] | sort",
            "[.features[] | .properties.release] |= sort",
            "reduce .features[] as $item ([]; . + [$item])",
        ] {
            let analyzed =
                analyze(resolve(parse(query).unwrap(), &ResolveOptions::default()).unwrap());
            assert_ne!(
                analyzed.analysis().selected_plan,
                crate::PlanKind::HybridBlocking,
                "unexpected hybrid plan for {query}"
            );
        }
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
