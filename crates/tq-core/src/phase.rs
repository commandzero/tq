//! Sealed typestate query lifecycle and mode-safe plans.

use std::{marker::PhantomData, sync::Arc};

use crate::{Diagnostic, DiagnosticClass, SourceFile, SourceId, Value};

mod sealed {
    pub trait Sealed {}
}

/// Marker implemented only by tq query phases.
pub trait QueryPhase: sealed::Sealed {}

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
impl QueryPhase for Parsed {}
impl QueryPhase for Resolved {}
impl QueryPhase for Analyzed {}
impl QueryPhase for Compiled {}

/// Pre-input execution requirements.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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

#[derive(Clone, Debug)]
struct QueryInner {
    source: SourceFile,
    capabilities: Capabilities,
}

/// Query in one sealed compilation phase.
#[derive(Clone, Debug)]
pub struct Query<P: QueryPhase> {
    inner: Arc<QueryInner>,
    phase: PhantomData<P>,
}

impl Query<Parsed> {
    /// Creates a parsed-phase query container. The parser will replace this
    /// convenience constructor with a fallible source-spanned transition.
    #[must_use]
    pub fn from_source(source: impl Into<Arc<str>>) -> Self {
        Self {
            inner: Arc::new(QueryInner {
                source: SourceFile::new(SourceId::new(0), "<query>", source),
                capabilities: Capabilities::default(),
            }),
            phase: PhantomData,
        }
    }

    /// Resolves lexical names. The resolver module enriches this transition.
    #[must_use]
    pub fn resolve(self) -> Query<Resolved> {
        self.change_phase()
    }
}

impl Query<Resolved> {
    /// Attaches analyzed execution capabilities.
    #[must_use]
    pub fn analyze(mut self, capabilities: Capabilities) -> Query<Analyzed> {
        Arc::make_mut(&mut self.inner).capabilities = capabilities;
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
        self.inner.capabilities
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
    phase: PhantomData<P>,
}

impl Query<Analyzed> {
    /// Compiles the analyzed query into a validated program placeholder.
    #[must_use]
    pub fn compile(self) -> Program<Compiled> {
        Program {
            inner: self.inner,
            phase: PhantomData,
        }
    }
}

impl Program<Compiled> {
    /// Pre-input capability metadata.
    #[must_use]
    pub fn capabilities(&self) -> Capabilities {
        self.inner.capabilities
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
    use super::{Capabilities, Query};

    #[test]
    fn document_requirement_rejects_event_plan_before_input() {
        let program = Query::from_source("sort")
            .resolve()
            .analyze(Capabilities {
                document: true,
                blocking: true,
                ..Capabilities::default()
            })
            .compile();
        assert!(program.event_plan().is_err());
    }
}
