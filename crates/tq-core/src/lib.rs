//! Core query language types and execution machinery for `tq`.
//!
//! The public API makes compilation and execution phases explicit. A parsed
//! query cannot be executed, and an event plan cannot be passed to a document
//! executor:
//!
//! ```compile_fail
//! use tq_core::{Parsed, Query, execute_document};
//!
//! let parsed = Query::<Parsed>::from_source(".");
//! execute_document(&parsed, tq_core::Value::Null);
//! ```
//!
//! ```compile_fail
//! use tq_core::{Compiled, Event, Plan, execute_document};
//!
//! let event_plan: Plan<Compiled, Event> = todo!();
//! execute_document(&event_plan, tq_core::Value::Null);
//! ```

mod diagnostic;
mod number;
mod path;
mod phase;
mod value;

pub use diagnostic::{
    Diagnostic, DiagnosticClass, Label, SourceFile, SourceId, SourcePosition, Span,
};
pub use number::{Number, NumberError, NumberLimits};
pub use path::{Path, PathComponent, PathError};
pub use phase::{
    Analyzed, Capabilities, Compiled, Document, Event, Parsed, Plan, Program, Query, Resolved,
    execute_document,
};
pub use value::{Object, Value, ValueKind};
