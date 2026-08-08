//! Core query language types and execution machinery for `tq`.
//!
//! The public API makes compilation and execution phases explicit. A parsed
//! query cannot be executed, and an event plan cannot be passed to a document
//! executor:
//!
//! ```compile_fail
//! use tq_core::{Parsed, Query, Value, Vm, VmLimits, parse};
//!
//! let parsed: Query<Parsed> = parse(".").unwrap();
//! Vm::new(&parsed, Value::Null, VmLimits::default());
//! ```
//!
//! ```compile_fail
//! use tq_core::{Compiled, Events, Plan, Value, Vm, VmLimits};
//!
//! let event_plan: Plan<Compiled, Events> = todo!();
//! Vm::new(&event_plan, Value::Null, VmLimits::default());
//! ```

mod ast;
mod bytecode;
mod diagnostic;
mod eval;
mod lexer;
mod number;
mod parser;
mod path;
mod phase;
mod resolve;
mod value;
mod vm;

pub use bytecode::{Bytecode, BytecodeError};
pub use diagnostic::{
    Diagnostic, DiagnosticClass, Label, SourceFile, SourceId, SourcePosition, Span,
};
pub use number::{Number, NumberError, NumberLimits};
pub use parser::{parse, parse_bytes};
pub use path::{Path, PathComponent, PathError};
pub use phase::{
    Analysis, Analyzed, Blocking, Capabilities, CapabilityCause, Compiled, Document, Effect,
    Events, Parsed, Plan, Program, Query, Resolved, Subtree, WholeInput,
};
pub use resolve::{
    AnalysisContext, Builtin, BuiltinRegistry, ResolveOptions, analyze, analyze_with_context,
    resolve,
};
pub use value::{Object, Value, ValueKind};
pub use vm::{Vm, VmError, VmLimits, VmObservations};
