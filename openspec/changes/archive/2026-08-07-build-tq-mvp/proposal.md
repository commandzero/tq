## Why

TOON has fast Rust serialization support but no native query tool comparable to `jq` for JSON or `yq` for YAML. `tq` should establish a trustworthy, benchmark-driven Rust implementation whose compatibility contract, memory behavior, and performance claims are executable before feature development begins.

## What Changes

- Establish `tq` as a standalone Rust CLI and reusable query-engine library targeting a JSON-shaped data model, TOON v3 as its native format, JSON and YAML input, and jq 1.8-compatible core filter semantics.
- Build the cross-tool compatibility suite first, execute it against the reference `jq` and `yq` implementations, and preserve their observed results as versioned baselines before implementing matching `tq` behavior.
- Build the performance harness first and record `jq` and `yq` baselines for startup, throughput, latency, peak memory, and output correctness across naturally sized JSON, YAML, and TOON representations.
- Use refreshable, provenance-tracked benchmark data. The initial corpus uses the natural USGS earthquake feed files already in `examples/` and adds a naturally large public dataset without slicing, duplication, or synthetic resizing.
- Implement a multi-result jq-style filter runtime with compilation, explicit backtracking, structural sharing, deterministic object ordering, and typed lifecycle boundaries.
- Implement a jq-like CLI with stdin/files, raw and structured output, variables, defined exit statuses, diagnostics, and explicit TOON stream framing.
- Add bounded-memory TOON event decoding and encoding primitives. General filters may materialize one document, while explicit streaming execution remains depth-bounded and reports unsupported buffering requirements honestly.
- Add resource limits, plan explanations, conformance checks, fuzz/property testing, and benchmark regression thresholds.
- Defer complex jq parity—including modules/imports, the complete standard library, advanced regex/date/platform behavior, automatic streaming of arbitrary filters, source-format preservation, and byte-for-byte diagnostic parity—to later changes.

## Capabilities

### New Capabilities

- `benchmark-corpus`: Refreshable natural-size JSON/YAML/TOON datasets with provenance, checksums, semantic equivalence validation, and small-through-large execution tiers.
- `cross-tool-compatibility`: A data-driven compatibility suite that runs equivalent jq-like cases against `jq`, `yq`, and eventually `tq`, preserving outputs, errors, and exit behavior.
- `performance-benchmarks`: Reproducible JSON, YAML, and TOON end-to-end benchmarks for `jq`, `yq`, and `tq`, including same-format comparisons, native-format comparisons, correctness gates, machine manifests, startup latency, throughput, time to first result, and peak memory.
- `toon-stream-io`: Incremental TOON input events, strict validation, canonical output, multi-result framing, and bounded-memory spooling where TOON array headers require future length or schema knowledge.
- `jq-core-language`: The MVP jq-compatible grammar and semantics for values, paths, iteration, composition, construction, selection, operators, variables, core functions, and controlled failures.
- `query-runtime`: A compiled, generator-oriented runtime with backtracking, structural sharing, path-aware updates, typestate compilation phases, deterministic evaluation, and explicit execution capabilities.
- `tq-cli`: The user-facing `tq FILTER [FILE...]` interface, TOON/JSON/YAML input, stdin/file processing, variables, raw/structured output, format options, help, versioning, diagnostics, and jq-aligned exit statuses.
- `resource-governance`: Memory and execution classifications, `--explain`, nesting/output/step limits, broken-pipe handling, cancellation, and observable fallback or spill behavior.

### Modified Capabilities

None. This repository has no existing OpenSpec capabilities.

## Impact

- Introduces a Rust workspace or equivalent crate separation for CLI, compiler/runtime, multi-format input, TOON I/O, and test/benchmark support.
- Depends on a library-only `toon-format` integration and may require new streaming decode/event APIs in the local `toon-rust` project.
- Uses the actively maintained `yaml_serde` crate from the YAML organization as the preferred YAML parser instead of the deprecated `serde_yaml` crate.
- Adds local reference integration with `../jq` and `../yq`, configurable installed binaries, and self-describing local benchmark campaigns; CI and dedicated release-runner automation are deferred.
- Adds benchmark data manifests and cache directories; large datasets and generated YAML/TOON equivalents are fetched or generated locally and are not committed to Git.
- Establishes jq 1.8.x, yq 4.x, and TOON v3 as explicitly versioned compatibility inputs rather than unqualified evergreen claims.
- Future compatibility expansions will be proposed as separate changes so the MVP can ship with a precise, tested support matrix.
