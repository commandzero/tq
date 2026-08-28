//! Sealed typestate query lifecycle and mode-safe plans.

use std::{marker::PhantomData, sync::Arc};

use serde::Serialize;

use crate::{
    Bytecode, Diagnostic, DiagnosticClass, PathComponent, SourceFile, Span, Value,
    ast::{self, Access, Expr, ExprKind},
};

mod sealed {
    pub trait Sealed {}
}

/// Marker implemented only by tq query phases.
pub trait QueryPhase: sealed::Sealed {
    /// Stable phase name used by explain output.
    const NAME: &'static str;
}

/// Parsed but unresolved query.
#[derive(Clone, Copy, Debug)]
pub struct Parsed;
/// Names and lexical bindings resolved.
#[derive(Clone, Copy, Debug)]
pub struct Resolved;
/// Capabilities and effects analyzed.
#[derive(Clone, Copy, Debug)]
pub struct Analyzed;
/// Validated bytecode compiled.
#[derive(Clone, Copy, Debug)]
pub struct Compiled;

impl sealed::Sealed for Parsed {}
impl sealed::Sealed for Resolved {}
impl sealed::Sealed for Analyzed {}
impl sealed::Sealed for Compiled {}
impl QueryPhase for Parsed {
    const NAME: &'static str = "parsed";
}
impl QueryPhase for Resolved {
    const NAME: &'static str = "resolved";
}
impl QueryPhase for Analyzed {
    const NAME: &'static str = "analyzed";
}
impl QueryPhase for Compiled {
    const NAME: &'static str = "compiled";
}

/// Pre-input execution requirements.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "orthogonal analysis effects intentionally compose independently"
)]
pub struct Capabilities {
    /// Proven to return each input unchanged exactly once.
    pub semantic_identity: bool,
    /// Can consume event path/value records.
    pub event_stream: bool,
    /// Requires a complete subtree.
    pub subtree: bool,
    /// Requires a complete document.
    pub document: bool,
    /// Requires all input documents.
    pub whole_input: bool,
    /// Contains a blocking operator.
    pub blocking: bool,
    /// Can update paths.
    pub mutation: bool,
    /// May emit more than one result.
    pub generator: bool,
    /// May fail at runtime.
    pub possible_failure: bool,
    /// Retains one managed fold accumulator.
    pub fold_state: bool,
}

/// One analyzed capability and the syntax that caused it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct CapabilityCause {
    /// Effect being introduced.
    pub effect: Effect,
    /// Query syntax range.
    pub span: Span,
}

/// Stable analysis effect name.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Effect {
    /// Returns each input unchanged exactly once.
    SemanticIdentity,
    /// Event-stream compatible.
    EventStream,
    /// Retains a subtree.
    Subtree,
    /// Requires one document.
    Document,
    /// Requires all input documents.
    WholeInput,
    /// Blocks on complete input/collection.
    Blocking,
    /// Captures or updates paths.
    Mutation,
    /// May produce multiple values.
    Generator,
    /// May fail at runtime.
    PossibleFailure,
    /// Retains one managed fold accumulator frame.
    FoldState,
    /// Proves a static decoder path prefix.
    PathPrefix,
    /// Proves that one complete bounded subtree is sufficient.
    SubtreeComplete,
    /// Describes whether a retained value can escape as output.
    Escape,
}

/// Stable pre-input execution classification.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanKind {
    /// Query-independent structural identity transcode.
    Transcode,
    /// Decoder-event execution without retaining a complete value.
    Events,
    /// One independently bounded subtree at a time.
    Subtree,
    /// One complete input document.
    #[default]
    Document,
    /// Every input document.
    WholeInput,
    /// A complete document plus blocking operator state.
    Blocking,
}

impl std::fmt::Display for PlanKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Transcode => "transcode",
            Self::Events => "events",
            Self::Subtree => "subtree",
            Self::Document => "document",
            Self::WholeInput => "whole-input",
            Self::Blocking => "blocking-document",
        })
    }
}

/// Input syntax admitted by a structural transcode proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TranscodeInput {
    /// JSON streamed in encounter order, with duplicate names rejected late.
    Json,
    /// Strict TOON with duplicate rejection.
    Toon,
}

/// Decoder duplicate behavior carried by a structural transcode plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TranscodeDuplicatePolicy {
    /// A repeated object path rejects the current document.
    Reject,
}

/// Output publication commitment carried by a transcode plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TranscodeCommitment {
    /// Record-separator framing permits completed records to publish directly.
    DirectSequence,
    /// Bytes remain private until exactly one successful result is known.
    AtomicUnframed,
}

/// Finite resource proof attached before structural decoding begins.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct TranscodeLimits {
    /// Aggregate preparation memory.
    pub maximum_memory_bytes: u64,
    /// Aggregate temporary spool bytes.
    pub maximum_spool_bytes: u64,
    /// Aggregate rendered output bytes.
    pub maximum_output_bytes: u64,
    /// Maximum structural nesting.
    pub maximum_depth: usize,
    /// Maximum decoded bytes in one scalar or key token.
    pub maximum_token_bytes: usize,
}

/// Complete output-aware proof required to bypass jq bytecode execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct TranscodeProof {
    /// Proven source syntax and decoder semantics.
    pub input: TranscodeInput,
    /// Decoder duplicate-name behavior.
    pub duplicate_policy: TranscodeDuplicatePolicy,
    /// Whether structural failure may follow emitted events.
    pub late_errors: bool,
    /// Canonical TOON writer is selected.
    pub canonical_toon_writer: bool,
    /// Collision-prone key folding is disabled.
    pub key_folding_disabled: bool,
    /// Publication behavior selected for the output framing.
    pub commitment: TranscodeCommitment,
    /// Configured finite resource bounds.
    pub limits: TranscodeLimits,
}

/// Proof attached to an automatic bounded-retention plan.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StreamProof {
    /// Static path whose direct children are independently processed.
    pub required_path_prefix: Vec<PathComponent>,
    /// Static path projected from each selected child, when proven.
    pub projected_path: Option<Vec<PathComponent>>,
    /// Whether each selected child must be complete before evaluation.
    pub subtree_complete: bool,
    /// Whether the selected value may escape as a result.
    pub value_escapes: bool,
    /// Syntax admitting the bounded plan.
    pub cause: Span,
    /// Stable HIR evaluated for each selected child.
    pub item_hir: String,
}

/// Complete pre-input analysis report.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Analysis {
    /// Combined effect flags.
    pub capabilities: Capabilities,
    /// First-class syntax causes in source order.
    pub causes: Vec<CapabilityCause>,
    /// Selected execution class before input consumption.
    pub selected_plan: PlanKind,
    /// Automatic streaming proof, when one was established.
    pub stream_proof: Option<StreamProof>,
    /// Stable reason decoder-backed automatic planning was not selected.
    pub stream_rejection: Option<String>,
    /// Output-aware structural transcode proof, when selected by the host.
    pub transcode_proof: Option<TranscodeProof>,
    /// Stable reason structural identity transcode was not selected.
    pub transcode_rejection: Option<String>,
}

/// One canonical module admitted during pre-input resolution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ModuleInfo {
    /// Import path used by the query or another module.
    pub name: String,
    /// Canonical filesystem path.
    pub canonical_path: String,
    /// Lowercase SHA-256 content digest.
    pub sha256: String,
    /// Constant module metadata.
    pub metadata: Value,
}

#[derive(Clone, Debug)]
struct QueryInner {
    source: SourceFile,
    ast: Arc<Expr>,
    analysis: Analysis,
    modules: Vec<ModuleInfo>,
}

/// Query in one sealed compilation phase.
#[derive(Clone, Debug)]
pub struct Query<P: QueryPhase> {
    inner: Arc<QueryInner>,
    phase: PhantomData<P>,
}

impl Query<Parsed> {
    pub(crate) fn from_ast(source: SourceFile, ast: Expr) -> Self {
        Self {
            inner: Arc::new(QueryInner {
                source,
                ast: Arc::new(ast),
                analysis: Analysis::default(),
                modules: Vec::new(),
            }),
            phase: PhantomData,
        }
    }

    pub(crate) fn into_resolved(self) -> Query<Resolved> {
        self.change_phase()
    }

    pub(crate) fn ast_mut(&mut self) -> &mut Expr {
        let inner = Arc::make_mut(&mut self.inner);
        Arc::make_mut(&mut inner.ast)
    }

    pub(crate) fn set_modules(&mut self, modules: Vec<ModuleInfo>) {
        Arc::make_mut(&mut self.inner).modules = modules;
    }
}

impl Query<Resolved> {
    pub(crate) fn with_analysis(mut self, analysis: Analysis) -> Query<Analyzed> {
        Arc::make_mut(&mut self.inner).analysis = analysis;
        self.change_phase()
    }
}

impl<P: QueryPhase> Query<P> {
    /// Query source.
    #[must_use]
    pub fn source(&self) -> &SourceFile {
        &self.inner.source
    }

    /// Capabilities known at this phase.
    #[must_use]
    pub fn capabilities(&self) -> Capabilities {
        self.inner.analysis.capabilities
    }

    /// Stable source-spanned HIR explanation.
    #[must_use]
    pub fn hir(&self) -> String {
        ast::display(&self.inner.ast)
    }

    /// Complete capability analysis known at this phase.
    #[must_use]
    pub fn analysis(&self) -> &Analysis {
        &self.inner.analysis
    }

    /// Canonical modules loaded before input consumption.
    #[must_use]
    pub fn modules(&self) -> &[ModuleInfo] {
        &self.inner.modules
    }

    /// Human-readable HIR and capability explanation.
    #[must_use]
    pub fn explain(&self) -> String {
        let mut output = format!(
            "phase: {}\nspan: {}..{}\nhir: {}\n",
            P::NAME,
            self.inner.ast.span.start,
            self.inner.ast.span.end,
            self.hir()
        );
        if self.inner.analysis.causes.is_empty() {
            output.push_str("effects: none\n");
        } else {
            output.push_str("effects:\n");
            for cause in &self.inner.analysis.causes {
                use std::fmt::Write as _;
                writeln!(
                    output,
                    "- {:?} at {}..{}",
                    cause.effect, cause.span.start, cause.span.end
                )
                .expect("writing to String cannot fail");
            }
        }
        if !self.inner.modules.is_empty() {
            output.push_str("modules:\n");
            for module in &self.inner.modules {
                use std::fmt::Write as _;
                writeln!(
                    output,
                    "- {} {} sha256={}",
                    module.name, module.canonical_path, module.sha256
                )
                .expect("writing to String cannot fail");
            }
        }
        output
    }

    /// Machine-readable HIR and capability explanation.
    #[must_use]
    pub fn explain_json(&self) -> serde_json::Value {
        serde_json::json!({
            "schema_version": 1,
            "phase": P::NAME,
            "span": self.inner.ast.span,
            "hir": self.hir(),
            "analysis": self.analysis(),
            "modules": self.modules(),
        })
    }

    pub(crate) fn ast(&self) -> &Expr {
        &self.inner.ast
    }

    fn change_phase<N: QueryPhase>(self) -> Query<N> {
        Query {
            inner: self.inner,
            phase: PhantomData,
        }
    }
}

/// Immutable compiled program constructible only through analyzed compilation.
#[derive(Clone, Debug)]
pub struct Program<P: QueryPhase> {
    inner: Arc<QueryInner>,
    bytecode: Arc<Bytecode>,
    phase: PhantomData<P>,
}

impl Query<Analyzed> {
    /// Compiles analyzed HIR and mandates bytecode validation.
    ///
    /// # Errors
    ///
    /// Returns a source-spanned compile/resource diagnostic when lowering or
    /// mandatory validation fails.
    pub fn compile(self) -> Result<Program<Compiled>, Box<Diagnostic>> {
        let bytecode = Bytecode::compile(&self.inner.ast, &self.inner.modules)?;
        Ok(Program {
            inner: self.inner,
            bytecode: Arc::new(bytecode),
            phase: PhantomData,
        })
    }
}

impl Program<Compiled> {
    /// Pre-input capability metadata.
    #[must_use]
    pub fn capabilities(&self) -> Capabilities {
        self.inner.analysis.capabilities
    }

    /// Validated immutable bytecode.
    #[must_use]
    pub fn bytecode(&self) -> &Bytecode {
        &self.bytecode
    }

    pub(crate) fn bytecode_arc(&self) -> Arc<Bytecode> {
        Arc::clone(&self.bytecode)
    }

    /// Stable source-annotated disassembly.
    #[must_use]
    pub fn disassemble(&self) -> String {
        format!(
            "capabilities={:?}\n{}",
            self.capabilities(),
            self.bytecode.disassemble()
        )
    }

    /// Selects the analyzed execution plan before input consumption.
    ///
    /// # Errors
    ///
    /// Returns a compile diagnostic if an analyzed automatic lowering cannot
    /// be reconstructed or validated.
    pub fn automatic_plan(self) -> Result<AutomaticPlan, Box<Diagnostic>> {
        match self.inner.analysis.selected_plan {
            PlanKind::Transcode => Err(plan_error(
                "TQ-CAP-TRANSCODE-002",
                "transcode selection requires an output-aware proof",
            )),
            PlanKind::Events | PlanKind::Subtree => {
                let lowering = automatic_lowering(&self.inner.ast).ok_or_else(|| {
                    plan_error(
                        "TQ-CAP-AUTO-001",
                        "automatic stream proof could not be lowered",
                    )
                })?;
                let item_bytecode = Arc::new(Bytecode::compile(&lowering.item, &[])?);
                let base_bytecode = Arc::new(Bytecode::compile(&lowering.base, &[])?);
                let execution = Some(AutomaticExecution {
                    prefix: lowering.prefix,
                    projection: lowering.projection,
                    item_bytecode,
                    base_bytecode,
                    scalar_events_only: self.inner.analysis.selected_plan == PlanKind::Events,
                });
                let plan = Plan {
                    program: self,
                    automatic: execution,
                    transcode: None,
                    mode: PhantomData,
                };
                Ok(
                    if plan.program.inner.analysis.selected_plan == PlanKind::Events {
                        AutomaticPlan::Events(plan)
                    } else {
                        AutomaticPlan::Subtree(Plan {
                            program: plan.program,
                            automatic: plan.automatic,
                            transcode: None,
                            mode: PhantomData,
                        })
                    },
                )
            }
            PlanKind::WholeInput => self.whole_input_plan().map(AutomaticPlan::WholeInput),
            PlanKind::Blocking => self.blocking_plan().map(AutomaticPlan::Blocking),
            PlanKind::Document => Ok(AutomaticPlan::Document(self.document_plan())),
        }
    }

    /// Converts a compatible program to a document plan.
    #[must_use]
    pub fn document_plan(self) -> Plan<Compiled, Document> {
        Plan {
            program: self,
            automatic: None,
            transcode: None,
            mode: PhantomData,
        }
    }

    /// Converts a compatible program to an event plan.
    ///
    /// # Errors
    ///
    /// Returns a pre-input capability diagnostic for document-only programs.
    pub fn event_plan(self) -> Result<Plan<Compiled, Events>, Box<Diagnostic>> {
        let capabilities = self.capabilities();
        if capabilities.document || capabilities.whole_input || capabilities.mutation {
            return Err(Box::new(Diagnostic::new(
                "TQ-CAP-EVENT-001",
                DiagnosticClass::Compile,
                "query requires document values and cannot run in event mode",
            )));
        }
        Ok(Plan {
            program: self,
            automatic: None,
            transcode: None,
            mode: PhantomData,
        })
    }

    /// Converts a subtree-requiring program to its explicit typed plan.
    ///
    /// # Errors
    ///
    /// Returns a pre-input diagnostic when analysis did not require a subtree
    /// or when the query instead requires all input documents.
    pub fn subtree_plan(self) -> Result<Plan<Compiled, Subtree>, Box<Diagnostic>> {
        let capabilities = self.capabilities();
        if !capabilities.subtree || capabilities.whole_input {
            return Err(plan_error(
                "TQ-CAP-SUBTREE-001",
                "query does not admit the requested subtree plan",
            ));
        }
        Ok(Plan {
            program: self,
            automatic: None,
            transcode: None,
            mode: PhantomData,
        })
    }

    /// Converts a whole-input program to its explicit typed plan.
    ///
    /// # Errors
    ///
    /// Returns a pre-input diagnostic unless analysis requires all documents.
    pub fn whole_input_plan(self) -> Result<Plan<Compiled, WholeInput>, Box<Diagnostic>> {
        if !self.capabilities().whole_input {
            return Err(plan_error(
                "TQ-CAP-WHOLE-001",
                "query does not require a whole-input plan",
            ));
        }
        Ok(Plan {
            program: self,
            automatic: None,
            transcode: None,
            mode: PhantomData,
        })
    }

    /// Converts a blocking program to a typed blocking document plan.
    ///
    /// # Errors
    ///
    /// Returns a pre-input diagnostic unless analysis contains a blocking
    /// operator.
    pub fn blocking_plan(self) -> Result<Plan<Compiled, Blocking<Document>>, Box<Diagnostic>> {
        if !self.capabilities().blocking {
            return Err(plan_error(
                "TQ-CAP-BLOCKING-001",
                "query does not require a blocking plan",
            ));
        }
        Ok(Plan {
            program: self,
            automatic: None,
            transcode: None,
            mode: PhantomData,
        })
    }

    /// Constructs an output-aware identity transcode plan.
    ///
    /// # Errors
    ///
    /// Returns a pre-input diagnostic unless identity, decoder, writer, framing,
    /// and finite resource properties have all been proved.
    pub fn transcode_plan(
        self,
        proof: TranscodeProof,
    ) -> Result<Plan<Compiled, Transcode>, Box<Diagnostic>> {
        if !self.capabilities().semantic_identity {
            return Err(plan_error(
                "TQ-CAP-TRANSCODE-001",
                "query is not proven to emit its input unchanged exactly once",
            ));
        }
        if !proof.canonical_toon_writer || !proof.key_folding_disabled {
            return Err(plan_error(
                "TQ-CAP-TRANSCODE-003",
                "writer options do not admit canonical structural transcode",
            ));
        }
        let expected_policy = TranscodeDuplicatePolicy::Reject;
        if proof.duplicate_policy != expected_policy {
            return Err(plan_error(
                "TQ-CAP-TRANSCODE-004",
                "decoder or resource proof is inconsistent",
            ));
        }
        Ok(Plan {
            program: self,
            automatic: None,
            transcode: Some(proof),
            mode: PhantomData,
        })
    }
}

fn plan_error(code: &'static str, message: &'static str) -> Box<Diagnostic> {
    Box::new(Diagnostic::new(code, DiagnosticClass::Compile, message))
}

/// Document execution marker.
#[derive(Clone, Copy, Debug)]
pub struct Document;
/// Event-stream execution marker.
#[derive(Clone, Copy, Debug)]
pub struct Events;
/// Complete-subtree execution marker.
#[derive(Clone, Copy, Debug)]
pub struct Subtree;
/// All-documents execution marker.
#[derive(Clone, Copy, Debug)]
pub struct WholeInput;
/// Query-independent structural identity transcode marker.
#[derive(Clone, Copy, Debug)]
pub struct Transcode;
/// Blocking execution wrapper around another input mode.
#[derive(Clone, Copy, Debug)]
pub struct Blocking<M>(PhantomData<M>);

/// Mode-safe compiled execution plan.
#[derive(Clone, Debug)]
pub struct Plan<P: QueryPhase, M> {
    program: Program<P>,
    automatic: Option<AutomaticExecution>,
    transcode: Option<TranscodeProof>,
    mode: PhantomData<M>,
}

#[derive(Clone, Debug)]
struct AutomaticExecution {
    prefix: Vec<PathComponent>,
    projection: Option<Vec<PathComponent>>,
    item_bytecode: Arc<Bytecode>,
    base_bytecode: Arc<Bytecode>,
    scalar_events_only: bool,
}

/// One of the typed plans selected by automatic analysis.
#[derive(Clone, Debug)]
pub enum AutomaticPlan {
    /// Decoder-event plan.
    Events(Plan<Compiled, Events>),
    /// Independently bounded subtree plan.
    Subtree(Plan<Compiled, Subtree>),
    /// Eager document plan.
    Document(Plan<Compiled, Document>),
    /// All-input plan.
    WholeInput(Plan<Compiled, WholeInput>),
    /// Blocking document plan.
    Blocking(Plan<Compiled, Blocking<Document>>),
}

impl<M> Plan<Compiled, M> {
    /// Compiled program admitted by this typed plan.
    #[must_use]
    pub fn program(&self) -> &Program<Compiled> {
        &self.program
    }

    /// Static decoder path prefix used by an automatic bounded plan.
    #[must_use]
    pub fn automatic_prefix(&self) -> Option<&[PathComponent]> {
        self.automatic.as_ref().map(|plan| plan.prefix.as_slice())
    }

    /// Static path projected from each selected value without running the VM.
    #[must_use]
    pub fn automatic_projection(&self) -> Option<&[PathComponent]> {
        self.automatic
            .as_ref()
            .and_then(|plan| plan.projection.as_deref())
    }

    pub(crate) fn automatic_item_bytecode(&self) -> Option<Arc<Bytecode>> {
        self.automatic
            .as_ref()
            .map(|plan| Arc::clone(&plan.item_bytecode))
    }

    pub(crate) fn automatic_base_bytecode(&self) -> Option<Arc<Bytecode>> {
        self.automatic
            .as_ref()
            .map(|plan| Arc::clone(&plan.base_bytecode))
    }

    /// Whether this automatic event plan only admits scalar child values.
    #[must_use]
    pub fn scalar_events_only(&self) -> bool {
        self.automatic
            .as_ref()
            .is_some_and(|plan| plan.scalar_events_only)
    }

    /// Output-aware proof attached to a structural transcode plan.
    #[must_use]
    pub const fn transcode_proof(&self) -> Option<&TranscodeProof> {
        self.transcode.as_ref()
    }
}

#[derive(Clone)]
struct AutomaticLowering {
    prefix: Vec<PathComponent>,
    projection: Option<Vec<PathComponent>>,
    item: Expr,
    base: Expr,
    scalar_events_only: bool,
}

pub(crate) fn automatic_stream_proof(expr: &Expr) -> Option<(StreamProof, PlanKind)> {
    let lowering = automatic_lowering(expr)?;
    let kind = if lowering.scalar_events_only {
        PlanKind::Events
    } else {
        PlanKind::Subtree
    };
    Some((
        StreamProof {
            required_path_prefix: lowering.prefix,
            projected_path: lowering.projection,
            subtree_complete: kind == PlanKind::Subtree,
            value_escapes: kind == PlanKind::Subtree,
            cause: expr.span,
            item_hir: ast::display(&lowering.item),
        },
        kind,
    ))
}

fn automatic_lowering(expr: &Expr) -> Option<AutomaticLowering> {
    let mut stages = Vec::new();
    flatten_pipe(expr, &mut stages);
    let first = stages.first()?;
    let mut accesses = Vec::new();
    navigation(first, &mut accesses)?;
    let iterate = accesses
        .iter()
        .position(|(access, _)| matches!(access, Access::Iterate))?;
    if accesses[iterate + 1..]
        .iter()
        .any(|(access, _)| matches!(access, Access::Iterate))
    {
        return None;
    }
    let prefix = accesses[..iterate]
        .iter()
        .map(|(access, _)| static_component(access))
        .collect::<Option<Vec<_>>>()?;
    let projection = projection_path(&accesses[iterate + 1..], &stages[1..]);
    let identity_span = accesses[iterate].1;
    let mut item = Expr::new(ExprKind::Identity, identity_span);
    for (access, span) in &accesses[iterate + 1..] {
        item = Expr::new(
            ExprKind::Access {
                base: Box::new(item),
                access: access.clone(),
            },
            *span,
        );
    }
    for stage in &stages[1..] {
        let span = Span::new(item.span.source, item.span.start, stage.span.end);
        item = Expr::new(
            ExprKind::Pipe(Box::new(item), Box::new((*stage).clone())),
            span,
        );
    }
    let iterate_expr = Expr::new(
        ExprKind::Access {
            base: Box::new(Expr::new(ExprKind::Identity, identity_span)),
            access: Access::Iterate,
        },
        identity_span,
    );
    let base = Expr::new(
        ExprKind::Pipe(Box::new(iterate_expr), Box::new(item.clone())),
        expr.span,
    );
    Some(AutomaticLowering {
        prefix,
        projection,
        scalar_events_only: scalar_event_filter(&item),
        item,
        base,
    })
}

fn projection_path(
    first_tail: &[(Access, Span)],
    remaining_stages: &[&Expr],
) -> Option<Vec<PathComponent>> {
    let mut projection = first_tail
        .iter()
        .map(|(access, _)| static_component(access))
        .collect::<Option<Vec<_>>>()?;
    for stage in remaining_stages {
        let mut accesses = Vec::new();
        navigation(stage, &mut accesses)?;
        projection.extend(
            accesses
                .iter()
                .map(|(access, _)| static_component(access))
                .collect::<Option<Vec<_>>>()?,
        );
    }
    (!projection.is_empty()).then_some(projection)
}

fn flatten_pipe<'a>(expr: &'a Expr, stages: &mut Vec<&'a Expr>) {
    if let ExprKind::Pipe(left, right) = &expr.kind {
        flatten_pipe(left, stages);
        flatten_pipe(right, stages);
    } else {
        stages.push(expr);
    }
}

fn navigation(expr: &Expr, accesses: &mut Vec<(Access, Span)>) -> Option<()> {
    match &expr.kind {
        ExprKind::Identity => Some(()),
        ExprKind::Access { base, access } => {
            navigation(base, accesses)?;
            accesses.push((access.clone(), expr.span));
            Some(())
        }
        _ => None,
    }
}

fn static_component(access: &Access) -> Option<PathComponent> {
    match access {
        Access::Field(key) => Some(PathComponent::Key(Arc::clone(key))),
        Access::Index(index) => match &index.kind {
            ExprKind::Literal(Value::Number(number)) => number
                .to_string()
                .parse::<usize>()
                .ok()
                .map(PathComponent::Index),
            ExprKind::Literal(Value::String(key)) => Some(PathComponent::Key(Arc::clone(key))),
            _ => None,
        },
        Access::Iterate | Access::Slice { .. } => None,
    }
}

fn scalar_event_filter(expr: &Expr) -> bool {
    let filter = match &expr.kind {
        ExprKind::Pipe(left, right) if matches!(left.kind, ExprKind::Identity) => right.as_ref(),
        _ => expr,
    };
    matches!(
        &filter.kind,
        ExprKind::Call {
            name, arguments, ..
        }
            if arguments.is_empty()
                && matches!(&**name, "scalars" | "booleans" | "numbers" | "strings" | "nulls")
    )
}

#[cfg(test)]
mod tests {
    use crate::{AnalysisContext, ResolveOptions, analyze_with_context, parse, resolve};

    use super::{
        Analysis, AutomaticPlan, Capabilities, PlanKind, TranscodeCommitment,
        TranscodeDuplicatePolicy, TranscodeInput, TranscodeLimits, TranscodeProof,
    };

    fn json_transcode_proof() -> TranscodeProof {
        TranscodeProof {
            input: TranscodeInput::Json,
            duplicate_policy: TranscodeDuplicatePolicy::Reject,
            late_errors: true,
            canonical_toon_writer: true,
            key_folding_disabled: true,
            commitment: TranscodeCommitment::DirectSequence,
            limits: TranscodeLimits {
                maximum_memory_bytes: 1024,
                maximum_spool_bytes: 4096,
                maximum_output_bytes: 4096,
                maximum_depth: 16,
                maximum_token_bytes: 1024,
            },
        }
    }

    #[test]
    fn document_requirement_rejects_event_plan_before_input() {
        let resolved = resolve(parse("sort").unwrap(), &ResolveOptions::default()).unwrap();
        let program = resolved
            .with_analysis(Analysis {
                capabilities: Capabilities {
                    document: true,
                    blocking: true,
                    ..Capabilities::default()
                },
                causes: Vec::new(),
                ..Analysis::default()
            })
            .compile()
            .unwrap();
        assert!(program.event_plan().is_err());
    }

    #[test]
    fn analyzed_effects_admit_only_matching_typed_plans() {
        let event = analyze_with_context(
            resolve(parse(".").unwrap(), &ResolveOptions::default()).unwrap(),
            AnalysisContext {
                event_input: true,
                whole_input: false,
                automatic_streaming: false,
            },
        )
        .compile()
        .unwrap();
        assert!(event.event_plan().is_ok());

        let whole = analyze_with_context(
            resolve(parse(".").unwrap(), &ResolveOptions::default()).unwrap(),
            AnalysisContext {
                event_input: false,
                whole_input: true,
                automatic_streaming: false,
            },
        )
        .compile()
        .unwrap();
        assert!(whole.whole_input_plan().is_ok());

        let blocking = analyze_with_context(
            resolve(parse("sort").unwrap(), &ResolveOptions::default()).unwrap(),
            AnalysisContext::default(),
        )
        .compile()
        .unwrap();
        assert!(blocking.blocking_plan().is_ok());
    }

    #[test]
    fn automatic_analysis_selects_every_typed_retention_class() {
        let automatic = |source| {
            crate::analyze(resolve(parse(source).unwrap(), &ResolveOptions::default()).unwrap())
                .compile()
                .unwrap()
                .automatic_plan()
                .unwrap()
        };
        assert!(matches!(
            automatic(".items[] | numbers"),
            AutomaticPlan::Events(_)
        ));
        assert!(matches!(
            automatic(".items[] | select(.active) | .id"),
            AutomaticPlan::Subtree(_)
        ));
        assert!(matches!(automatic("."), AutomaticPlan::Document(_)));
        assert!(matches!(automatic("sort"), AutomaticPlan::Blocking(_)));

        let whole = analyze_with_context(
            resolve(parse(".").unwrap(), &ResolveOptions::default()).unwrap(),
            AnalysisContext {
                whole_input: true,
                ..AnalysisContext::default()
            },
        );
        assert_eq!(whole.analysis().selected_plan, PlanKind::WholeInput);
        assert!(matches!(
            whole.compile().unwrap().automatic_plan().unwrap(),
            AutomaticPlan::WholeInput(_)
        ));
    }

    #[test]
    fn typed_transcode_requires_identity_and_consistent_io_proofs() {
        let identity =
            crate::analyze(resolve(parse(".").unwrap(), &ResolveOptions::default()).unwrap())
                .compile()
                .unwrap();
        let plan = identity.transcode_plan(json_transcode_proof()).unwrap();
        assert_eq!(plan.transcode_proof(), Some(&json_transcode_proof()));

        let non_identity =
            crate::analyze(resolve(parse(".x").unwrap(), &ResolveOptions::default()).unwrap())
                .compile()
                .unwrap();
        assert!(non_identity.transcode_plan(json_transcode_proof()).is_err());

        let identity =
            crate::analyze(resolve(parse(".").unwrap(), &ResolveOptions::default()).unwrap())
                .compile()
                .unwrap();
        let mut inconsistent = json_transcode_proof();
        inconsistent.canonical_toon_writer = false;
        assert!(identity.transcode_plan(inconsistent).is_err());
    }
}
