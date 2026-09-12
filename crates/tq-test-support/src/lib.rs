//! Shared compatibility, corpus, and benchmark test support.

/// Refreshable corpus acquisition and integrity primitives.
pub mod corpus;

/// Cross-tool executable, process, normalization, and reporting support.
pub mod compatibility;

/// Correctness-gated performance measurement and reporting.
pub mod benchmark;

/// Repository-owned TOON fixture storage and typed decoding.
pub mod fixture_data;
