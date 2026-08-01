//! Sealed typestate query lifecycle and mode-safe plans.

use std::{marker::PhantomData, sync::Arc};

use serde::Serialize;

use crate::{
    Bytecode, Diagnostic, DiagnosticClass, SourceFile, Span, Value,
    ast::{self, Expr},
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
}

/// Complete pre-input analysis report.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Analysis {
    /// Combined effect flags.
    pub capabilities: Capabilities,
    /// First-class syntax causes in source order.
    pub causes: Vec<CapabilityCause>,
}

#[derive(Clone, Debug)]
struct QueryInner {
    source: SourceFile,
    ast: Arc<Expr>,
    analysis: Analysis,
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
            }),
            phase: PhantomData,
        }
    }

    pub(crate) fn into_resolved(self) -> Query<Resolved> {
        self.change_phase()
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
        let bytecode = Bytecode::compile(&self.inner.ast)?;
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

    /// Converts a compatible program to a document plan.
    #[must_use]
    pub fn document_plan(self) -> Plan<Compiled, Document> {
        Plan {
            program: self,
            mode: PhantomData,
        }
    }

    /// Converts a compatible program to an event plan.
    ///
    /// # Errors
    ///
    /// Returns a pre-input capability diagnostic for document-only programs.
    pub fn event_plan(self) -> Result<Plan<Compiled, Event>, Box<Diagnostic>> {
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
            mode: PhantomData,
        })
    }
}

/// Document execution marker.
#[derive(Clone, Copy, Debug)]
pub struct Document;
/// Event execution marker.
#[derive(Clone, Copy, Debug)]
pub struct Event;

/// Mode-safe compiled execution plan.
#[derive(Clone, Debug)]
pub struct Plan<P: QueryPhase, M> {
    program: Program<P>,
    mode: PhantomData<M>,
}

/// Temporary document executor proving phase/mode constraints. The bytecode VM
/// replaces identity behavior in the compiler slice.
#[must_use]
pub fn execute_document(plan: &Plan<Compiled, Document>, input: Value) -> Vec<Value> {
    let _ = &plan.program;
    vec![input]
}

#[cfg(test)]
mod tests {
    use crate::{ResolveOptions, parse, resolve};

    use super::{Analysis, Capabilities};

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
            })
            .compile()
            .unwrap();
        assert!(program.event_plan().is_err());
    }
}
