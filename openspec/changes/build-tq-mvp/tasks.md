## 1. Standalone Repository and Test Infrastructure

- [x] 1.1 Initialize `$HOME/Development/commandzero/tq` as its own Git repository and verify Git resolves `tq`, not the parent `commandzero`, as the worktree root
- [x] 1.2 Convert the root Cargo package into a workspace with initial `tq-test-support` and placeholder `tq-cli`, `tq-core`, `tq-formats`, and `tq-toon` members while keeping engine members free of implementation
- [x] 1.3 Add workspace-wide Rust edition, formatting, lint, license, minimum-supported-Rust, release-profile, and `unsafe_code` policies
- [x] 1.4 Add repository ignore rules for benchmark caches, large downloads, generated JSON/YAML/TOON payloads, reports under local investigation, build output, fuzz artifacts, and temporary spools
- [x] 1.5 Add task-runner commands for formatting, linting, unit tests, compatibility smoke/full runs, benchmark smoke/standard/large campaigns, fuzz checks, and OpenSpec validation
- [x] 1.6 Add local preflight commands that run infrastructure tests without a tq engine and document how future automation can invoke the same commands without making CI an MVP development dependency
- [x] 1.7 Document the baseline-first hard gate: tasks in section 6 and later MUST NOT begin until every task in section 5 is complete

## 2. Refreshable Benchmark Corpus

- [x] 2.1 Define the machine-readable corpus-source and snapshot-manifest schemas with versioning and schema validation tests
- [x] 2.2 Register the natural USGS `all_hour`, `all_day`, `all_week`, and `all_month` GeoJSON endpoints with provenance and refresh metadata
- [x] 2.3 Register the Microsoft Georgia building-footprint GeoJSON archive as the initial natural large source with archive member, provenance, and license references
- [x] 2.4 Write failing tests for fetch success, redirects, HTTP validators, interrupted downloads, wrong content types, archive corruption, and digest mismatch
- [x] 2.5 Implement atomic source fetching into the external cache with UTC retrieval timestamps, HTTP metadata, compressed size, and SHA-256 calculation
- [x] 2.6 Implement secure archive inspection/decompression with archive-member validation and uncompressed size/digest recording
- [x] 2.7 Write failing structural-validation tests for GeoJSON root type, feature-array presence, logical feature count, invalid JSON, and unexpected document shape
- [ ] 2.8 Implement source JSON validation and manifest population for exact bytes, document shape, feature count, and digests
- [ ] 2.9 Implement refreshed-campaign mode as the default and explicit frozen-snapshot replay mode for investigations
- [ ] 2.10 Write failing tests proving that corpus preparation never slices, repeats, pads, samples, or truncates a natural source artifact
- [ ] 2.11 Add JSON-to-YAML and JSON-to-TOON generation commands that execute outside benchmark timing, use `yaml_serde` rather than deprecated `serde_yaml`, and use library-only format dependencies
- [ ] 2.12 Implement ordered JSON-model semantic comparison for source JSON, generated YAML, and generated TOON, including numeric-fidelity diagnostics
- [ ] 2.13 Add tests that reject generated formats with changed types, values, result ordering, object ordering, or unsupported numeric loss
- [ ] 2.14 Add smoke-corpus support for checked-in `examples/` snapshots and require reports to label it separately from refreshed campaigns
- [ ] 2.15 Produce a corpus inventory command showing source, snapshot, sizes, feature counts, generated representations, digests, and validation status

## 3. Cross-Tool Compatibility Suite

- [ ] 3.1 Define the versioned compatibility-case schema with stable ID, classification, capability tags, fixture, query/adapters, invocation mode, expected result contract, and MVP/deferred status
- [ ] 3.2 Implement executable discovery/configuration and identity capture for jq, yq, and an optional not-yet-present tq binary
- [ ] 3.3 Implement subprocess isolation with stdin/files, timeout, stdout/stderr capture, signal/exit classification, and secret-safe command recording
- [ ] 3.4 Write normalization tests for zero results, one null, multiple ordered results, multiline structured values, raw bytes, stderr, and exit statuses
- [ ] 3.5 Implement JSON result-text normalization for jq without sorting objects or result sequences
- [ ] 3.6 Implement YAML result normalization for yq while preserving JSON-model types, result sequence, and documented YAML-specific divergence metadata
- [ ] 3.7 Implement TOON Text Sequence normalization as a tq adapter placeholder that can remain inactive until tq exists
- [ ] 3.8 Implement stable error classes for CLI usage, query parse/compile, input parse, runtime type/path, resource, timeout, signal, and unsupported capability
- [ ] 3.9 Add common compatibility cases for identity, all primitive types, arrays/objects, ordering, and zero-versus-null cardinality
- [ ] 3.10 Add navigation cases for field/computed access, missing keys, indices, negative indices, slices, iteration, optional access, and invalid index types
- [ ] 3.11 Add generator/composition cases for pipes, commas, parentheses, nested iteration, and downstream multiplicity
- [ ] 3.12 Add construction cases for arrays, objects, shorthand keys, computed keys, and duplicate-key behavior
- [ ] 3.13 Add control/operator cases for conditionals, truthiness, boolean short circuiting, alternative, comparison, ordering, arithmetic, and type errors
- [ ] 3.14 Add numeric cases covering jq-compatible ordinary values, large exact identity, exponent input, negative zero, precision boundaries, and out-of-envelope policy inputs
- [ ] 3.15 Add variable cases for `as`, scope, generator bindings, unknown variables, string arguments, JSON arguments, and TOON arguments
- [ ] 3.16 Add core built-in cases for every built-in named by the `jq-core-language` spec, with blocking/resource classification metadata
- [ ] 3.17 Add error cases for `empty`, `error`, optional suppression, `try/catch`, errors after prior output, and source-location expectations
- [ ] 3.18 Add path-update cases for `=`, `|=`, arithmetic updates, alternative update, selected multi-path updates, and invalid lvalues
- [ ] 3.19 Add CLI cases for TOON-to-YAML-to-JSON automatic faildown, ambiguous JSON-as-YAML selection, strict format overrides, bounded late failure, stdin/files, YAML multi-document input, null/raw/slurp/stream modes, raw/join output, framing, strictness, variables, option validation, and jq-aligned exit status
- [ ] 3.20 Add deferred cases/markers for functions, modules, reduce/foreach, labels, recursive descent, interpolation, regex, dates, environment, and platform I/O
- [ ] 3.21 Implement human-readable and machine-readable reports with corpus/tool identities, per-case observations, semantic diffs, coverage groups, and final status
- [ ] 3.22 Implement reviewed baseline update workflow that shows every changed observation and offers no unreviewed bulk bless operation
- [ ] 3.23 Add harness self-tests using fake executables that emit controlled values, errors, timeouts, signals, and malformed output

## 4. Performance Benchmark Harness

- [ ] 4.1 Define the versioned benchmark-case and campaign schemas, including compatibility gate, dataset selector, command adapter, execution class, sampling policy, timeout, limits, and output contract
- [ ] 4.2 Implement a correctness-gate phase that refuses to time incorrect or unnormalized tool output
- [ ] 4.3 Implement process startup/wall-time sampling with warmups and size-aware default repetitions of 30 small, 10 medium, and 3 large samples
- [ ] 4.4 Implement local benchmark-host collection of user CPU, system CPU, peak RSS, signal/exit outcome, and output bytes with explicit unavailable values on unsupported systems
- [ ] 4.5 Implement time-to-first-result measurement without changing stdout framing or tool semantics
- [ ] 4.6 Implement environment manifests for OS/kernel, architecture, CPU, memory, filesystem, power settings when observable, tool/compiler identities, corpus, limits, and exact commands
- [ ] 4.7 Add parse-and-discard benchmark cases with small verified output to isolate input parsing and required evaluation
- [ ] 4.8 Add scalar extraction, multi-result projection, selective filter, and reduction benchmark cases over the natural USGS corpus
- [ ] 4.9 Add array/object construction, path update, blocking sort, and identity decode/re-encode benchmark cases
- [ ] 4.10 Add explicit event-stream benchmark cases with time-to-first-result and peak-RSS requirements
- [ ] 4.11 Add the explicit per-workload benchmark adapter matrix for jq/JSON; yq/JSON and YAML; tq/JSON, YAML, and TOON; plus separate jq/JSON, yq/YAML, and tq/TOON native-format views using semantically validated corpus representations
- [ ] 4.12 Calculate and report wall time, dispersion, logical records/s, physical MiB/s, output bytes, peak RSS, and reference ratios without a composite winner score
- [ ] 4.13 Record incorrect, unsupported, timeout, OOM/signal, and resource-limit outcomes as first-class report rows
- [ ] 4.14 Implement machine/corpus comparability checks that visually separate non-comparable reports
- [ ] 4.15 Implement tq self-regression thresholds as configurable report gates without making jq/yq ratios universal pass/fail requirements
- [ ] 4.16 Add benchmark-harness self-tests with deterministic sleeper/output/memory helper processes

## 5. jq and yq Baseline Gate

- [ ] 5.1 Build or locate the intended jq 1.8.x reference binary and record path, version, digest, and relevant numeric/regex build features
- [ ] 5.2 Build or locate the intended yq 4.x reference binary and record path, version, digest, and format configuration
- [ ] 5.3 Refresh the complete standard USGS corpus, generate and validate JSON, YAML, and TOON representations, and archive the campaign manifests
- [ ] 5.4 Run the full jq/yq-applicable compatibility suite before any tq language implementation and preserve the raw plus normalized observations
- [ ] 5.5 Review and classify every jq/yq difference as common agreement, jq-target divergence, CLI adaptation, unsupported, or deferred
- [ ] 5.6 Resolve or explicitly accept every unexplained reference crash, timeout, malformed result, and normalization failure
- [ ] 5.7 Review numeric baseline observations and record the digit, exponent, expansion, index, overflow, division-by-zero, and non-finite-result limits for the chosen jq decimal-literal hybrid
- [ ] 5.8 Freeze the reviewed jq/yq MVP compatibility baseline and generate the initial coverage report
- [ ] 5.9 Run the jq/yq standard JSON and YAML performance matrix only after correctness gates pass and archive the environment/corpus/report artifacts
- [ ] 5.10 Refresh and prepare the complete natural large corpus on the manifest-recorded local benchmark host without resizing it
- [ ] 5.11 Run applicable jq/yq large compatibility and performance cases and preserve success, timeout, OOM, or resource outcomes
- [ ] 5.12 Review the baseline reports and record explicit approval that the section 1.7 gate is satisfied before starting tq engine work

## 6. tq Core Value and Diagnostic Foundation

- [ ] 6.1 Create the real `tq-core`, `tq-formats`, `tq-toon`, and `tq-cli` crate APIs and ensure production crates do not depend on test-support or subprocess code
- [ ] 6.2 Add compile-fail tests demonstrating that unresolved/analyzed/compiled typestate phases and document/event plans cannot be interchanged through safe APIs
- [ ] 6.3 Implement source files, byte spans, line/column mapping, labels, stable diagnostic classes, and bounded context rendering
- [ ] 6.4 Write value-model tests for every primitive, insertion-ordered objects, shallow clones, deep equality, total ordering, and canonical display boundaries
- [ ] 6.5 Implement immediate scalar values and reference-counted immutable string/array/object nodes with shallow handle cloning
- [ ] 6.6 Implement the baseline-approved jq decimal-literal hybrid with arbitrary-precision literal preservation, lazy binary64 interpretation, jq-compatible arithmetic literal invalidation, canonical rendering, and range/resource errors
- [ ] 6.7 Add structural-sharing tests using pointer/identity observations for read-only branches and nested path updates
- [ ] 6.8 Implement path component and root-anchored path types without storing mutable references into values
- [ ] 6.9 Add property tests for ordered conversion between runtime values and serde-compatible JSON values within the supported numeric envelope

## 7. Query Lexer, Parser, Resolution, and Analysis

- [ ] 7.1 Write lexer golden tests for all MVP tokens, keywords, operators, strings, numbers, variables, comments/whitespace policy, invalid UTF-8, and source spans
- [ ] 7.2 Implement the source-spanned query lexer with explicit deferred-token recognition
- [ ] 7.3 Write precedence/associativity parser tests derived from accepted jq cases before implementing expression parsing
- [ ] 7.4 Implement the parser for identity, literals, grouping, access/index/slice/iteration, pipes, commas, and optional suffix
- [ ] 7.5 Implement parser support for array/object construction, conditionals, boolean/alternative, comparison, arithmetic, and variables/bindings
- [ ] 7.6 Implement parser support for errors/try-catch and all MVP assignment/update operators
- [ ] 7.7 Implement stable compile-time unsupported-capability diagnostics for every deferred grammar family
- [ ] 7.8 Write resolver tests for lexical scope, shadowing, unknown variables, CLI variables, and built-in name/arity resolution
- [ ] 7.9 Implement `Query<Parsed> -> Query<Resolved>` conversion and a versioned built-in registry
- [ ] 7.10 Write capability-analysis golden tests for event, subtree, document, whole-input, blocking, mutation, cardinality, and possible-failure effects
- [ ] 7.11 Implement `Query<Resolved> -> Query<Analyzed>` effect propagation with syntax-cause spans
- [ ] 7.12 Implement human-readable and machine-readable HIR/capability explanation output

## 8. Bytecode Compiler and VM Kernel

- [ ] 8.1 Define the minimal instruction set, operand encoding, constant pool, source map, stack effects, forks, paths, calls, errors, and returns
- [ ] 8.2 Write bytecode validation tests for invalid jumps, constants, stack effects, fork targets, call arity, instruction boundaries, and source maps
- [ ] 8.3 Implement `Query<Analyzed> -> Program<Compiled>` and mandatory bytecode validation
- [ ] 8.4 Implement stable bytecode disassembly with instruction offsets, operands, stack effects, capability metadata, and source spans
- [ ] 8.5 Write VM-kernel tests for load, duplicate, pop, branch, jump, return, error, fork, backtrack, and zero-result completion
- [ ] 8.6 Implement bounded explicit value, call-frame, path, and fork stacks with high-water observations
- [ ] 8.7 Implement pull-based `next_result` execution and cleanup when a caller stops before exhausting results
- [ ] 8.8 Implement deterministic error unwinding and optional/try catch points without native-stack recursion
- [ ] 8.9 Add malformed-bytecode and arbitrary-program fuzz targets that require validation failure or panic-free bounded execution

## 9. Incremental TOON Decoder and DOM Builder

- [ ] 9.1 Import or reference official TOON conformance fixtures and write event-contract golden tests before implementing the decoder
- [ ] 9.2 Define source-spanned document/object/key/array/scalar event types, active delimiter/schema state, and event-consumer interfaces
- [ ] 9.3 Implement incremental buffered byte reading, UTF-8 validation, source positioning, configurable token/line limits, and bounded lookahead
- [ ] 9.4 Implement strict root and primitive/object field decoding into events
- [ ] 9.5 Implement strict inline primitive array and nested array decoding with incremental declared-count validation
- [ ] 9.6 Implement strict tabular array decoding with ordered fields, delimiter scope, row-width validation, and count validation
- [ ] 9.7 Implement expanded list-item object/array decoding and the remaining TOON v3 root/nesting forms
- [ ] 9.8 Add hostile tests for excessive depth, huge declared counts, huge tokens, invalid escapes, indentation, delimiters, truncation, and invalid UTF-8
- [ ] 9.9 Implement the ordered DOM builder as an event consumer using `tq-core` values
- [ ] 9.10 Add event-to-DOM versus established decoder differential tests across all conformance fixtures
- [ ] 9.11 Add decoder fuzz and property targets and promote every found crash into a regression fixture
- [ ] 9.12 Stabilize a query-independent `tq-toon` event boundary, document the later `toon-rust` extraction seam, and keep upstreaming out of the MVP critical path

## 10. Canonical Writers, Framing, Spooling, and Format Adapters

- [ ] 10.1 Write canonical primitive/object/array output goldens covering quoting, numbers, key order, indentation, delimiters, and no document trailing newline
- [ ] 10.2 Implement canonical standalone TOON writing for known runtime values
- [ ] 10.3 Write unknown-length array tests for memory-threshold transition, correct count/header, tabular schema agreement/fallback, and cleanup
- [ ] 10.4 Implement secure memory-then-disk array preparation with configurable spool directory/limits and observable spool status
- [ ] 10.5 Implement safe key folding with explicit whole-document/materialization classification
- [ ] 10.6 Write TOON Text Sequence tests for zero, one, many, multiline, error-after-output, and malformed input records
- [ ] 10.7 Implement RS-prefix/LF-suffix sequence writing and explicit unframed exactly-one-result writing
- [ ] 10.8 Implement explicit TOON Text Sequence input as multiple ordered documents
- [ ] 10.9 Add ordered, arbitrary-precision-literal-aware JSON input/output adapters without routing TOON through lossy conversions
- [ ] 10.10 Add canonical writer round-trip properties and JSON/TOON ordered semantic equivalence tests
- [ ] 10.11 Spike `yaml_serde` reader, multi-document, ordering, duplicate-key, tag/alias, scalar-resolution, and hybrid-number fidelity behavior before accepting the YAML adapter design
- [ ] 10.12 Implement the `yaml_serde` YAML-to-runtime adapter with string-key/profile validation, one-document-at-a-time execution, explicit unsupported-value diagnostics, and no silent numeric loss
- [ ] 10.13 Implement shared `tq-formats` document sources and differential tests proving equivalent JSON, YAML, and TOON inputs produce the same ordered runtime values
- [ ] 10.14 Implement bounded per-source format probing and replay with TOON-to-YAML-to-JSON faildown, an observable commitment point, combined rejection diagnostics, and strict single-parser overrides

## 11. CLI Skeleton and Execution Plumbing

- [ ] 11.1 Write CLI parsing tests for `tq [OPTIONS] FILTER [FILE...]`, `-f`, stdin `-`, conflicting inputs, help, version, and unsupported options
- [ ] 11.2 Implement command parsing and stable usage/unsupported/compile/input/runtime/resource exit-status categories
- [ ] 11.3 Implement ordered stdin/file document sources, one-document-at-a-time lifecycle, and source identity propagation
- [ ] 11.4 Implement default best-effort TOON-to-YAML-to-JSON input detection, strict `--input-format` overrides, TOON/JSON output selection, TOON output defaults, and incompatible-option validation
- [ ] 11.5 Implement stdout-only data, stderr-only diagnostics/explain/trace, and report-file behavior
- [ ] 11.6 Implement structured sequence output, unframed output, raw output, join output, and JSON result-text output
- [ ] 11.7 Implement null input, raw input, slurp, stream, stream-errors, and TOON sequence-input plumbing
- [ ] 11.8 Implement `--arg`, `--argjson`, and `--argtoon` parsing and resolved-variable injection
- [ ] 11.9 Implement strictness and TOON/JSON/YAML option validation with explicit input/output format compatibility
- [ ] 11.10 Implement `-e/--exit-status` last-result and no-result tracking using the accepted jq baseline statuses
- [ ] 11.11 Implement help/version output with tq, TOON target, jq compatibility target, and build revision
- [ ] 11.12 Implement `tq compatibility` from the shared compatibility manifest rather than a hand-maintained feature list

## 12. Progressive Language Wave 1: Identity, Navigation, and Generators

- [ ] 12.1 Enable tq in identity/value-type compatibility cases and confirm they fail before implementation
- [ ] 12.2 Compile and execute identity plus primitive constants through the bytecode VM
- [ ] 12.3 Re-run identity/value cases, fix semantic/cardinality differences, and enable their tq support markers
- [ ] 12.4 Enable field/computed/index/slice navigation compatibility cases and confirm expected failures
- [ ] 12.5 Compile and execute field/computed/index/slice access with missing, negative, out-of-range, optional, and type-error behavior
- [ ] 12.6 Re-run navigation cases and promote only passing cases into the tq MVP matrix
- [ ] 12.7 Enable iteration/pipe/comma/parentheses cases and confirm expected failures
- [ ] 12.8 Implement fork/backtrack-based array/object iteration, pipes, commas, parentheses, and `empty`
- [ ] 12.9 Re-run generator cases and add cardinality/order regressions for every observed difference
- [ ] 12.10 Admit the passing Wave 1 correctness-gated tq benchmarks and record the first tq performance baseline without optimization

## 13. Progressive Language Wave 2: Construction, Control, and Operators

- [ ] 13.1 Enable array/object construction compatibility cases and confirm expected failures
- [ ] 13.2 Implement array collection and ordered object construction with explicit, shorthand, computed, and duplicate keys
- [ ] 13.3 Re-run construction cases and promote only passing cases
- [ ] 13.4 Enable conditional, truthiness, boolean, and alternative cases and confirm expected failures
- [ ] 13.5 Implement conditionals, jq truthiness, short-circuit boolean operators, unary not, and alternative semantics
- [ ] 13.6 Enable comparison, total ordering, equality, and arithmetic cases and confirm expected failures
- [ ] 13.7 Implement deep equality, baseline-approved total ordering, arithmetic, overloaded addition, and type errors
- [ ] 13.8 Re-run all Wave 2 cases including numeric edge cases and add regression fixtures for every baseline-sensitive decision
- [ ] 13.9 Admit passing Wave 2 construction/operator/blocking benchmarks and record tq baselines

## 14. Progressive Language Wave 3: Variables, Built-ins, Errors, and Updates

- [ ] 14.1 Enable lexical/CLI variable compatibility cases and confirm expected failures
- [ ] 14.2 Implement variable frames, `as` generator binding, scope/shadowing, and CLI variable loads
- [ ] 14.3 Re-run variable cases and promote only passing cases
- [ ] 14.4 Enable core built-in cases in small coherent groups and confirm expected failures before each implementation group
- [ ] 14.5 Implement type/length/UTF-8/key/membership and type-selector built-ins
- [ ] 14.6 Implement select/map/map-values/string-number conversion and range built-ins
- [ ] 14.7 Implement add/min/max/sort/sort-by/unique/unique-by/reverse/flatten with blocking analysis
- [ ] 14.8 Re-run every built-in case, verify resource classification, and promote only passing built-ins
- [ ] 14.9 Enable error/optional/try-catch cases and confirm expected failures
- [ ] 14.10 Implement scoped optional suppression, error values, try/catch, and deterministic error-after-output behavior
- [ ] 14.11 Enable path and update-operator cases and confirm expected failures
- [ ] 14.12 Implement stable lvalue path capture, path-copying assignment, relative updates, arithmetic updates, and multi-path order
- [ ] 14.13 Re-run all update/property tests and verify unaffected subtrees remain structurally shared
- [ ] 14.14 Admit passing Wave 3 built-in/error/update benchmarks and record tq baselines

## 15. Streaming Plans and Resource Governance

- [ ] 15.1 Write typed plan-selection tests for document, event, subtree, whole-input, blocking, and rejected combinations
- [ ] 15.2 Implement `Program<Compiled> -> Plan<Document>` and `Plan<Events>` conversions with pre-input incompatibility errors
- [ ] 15.3 Implement jq-compatible stream path/value event formation from TOON and JSON decoder events
- [ ] 15.4 Enable explicit stream compatibility cases for jq, yq where applicable, and tq; resolve path, empty-container, and stream-error differences
- [ ] 15.5 Implement `--explain` human and machine reports with syntax causes, input-detection policy or override, retained working set, blocking, and spool requirements
- [ ] 15.6 Write resource-limit tests for format-detection lookahead/replay, depth, token/line, VM stacks, preparation memory, spool bytes, result count, output bytes, and VM steps
- [ ] 15.7 Implement coherent CLI/library resource configuration and stable limit diagnostics with high-water observations
- [ ] 15.8 Ensure untrusted declared lengths never trigger proportional unchecked allocation and add allocation-regression tests
- [ ] 15.9 Implement interrupt cancellation, downstream broken-pipe handling, VM cleanup, and spool cleanup
- [ ] 15.10 Add partial-output tests proving prior framed results remain valid when a later runtime/resource error occurs
- [ ] 15.11 Run the opt-in large explicit-stream tq benchmark locally, record the complete host manifest, and evaluate the 128 MiB peak-RSS objective
- [ ] 15.12 Profile any failed memory objective, fix retained-input growth, and add a regression benchmark before claiming bounded streaming

## 16. Hardening, Release Gates, and Deferred Roadmap

- [ ] 16.1 Run formatting, lint, documentation, all unit/integration/property tests, OpenSpec validation, and `cargo test` under the minimum and stable Rust toolchains
- [ ] 16.2 Run query parser, TOON decoder, bytecode validator, VM, and CLI fuzz targets for the release budget and land all discovered regressions
- [ ] 16.3 Run the complete refreshed jq/yq/tq MVP compatibility campaign and require zero unexplained tq jq-target failures
- [ ] 16.4 Publish the generated compatibility matrix with supported, partial, divergent, unsupported, deferred, and untested capability counts
- [ ] 16.5 Run refreshed standard and natural-large JSON, YAML, and TOON performance campaigns with correctness gates and archive full machine/corpus/tool manifests
- [ ] 16.6 Review tq startup, throughput, time-to-first-result, peak RSS, spooling, blocking, and failure outcomes without suppressing unfavorable comparisons
- [ ] 16.7 Establish accepted tq self-regression thresholds from the first stable local baselines and enable manifest-aware local regression reporting
- [ ] 16.8 Add user documentation for jq-compatible syntax, TOON framing, raw/unframed modes, memory classifications, limits, benchmark reproduction, and known divergences
- [ ] 16.9 Add contributor documentation for compatibility-case-first development, baseline review, benchmark correctness gates, and capability promotion
- [ ] 16.10 Create separate follow-up OpenSpec proposals for user functions/modules, reduce/foreach, recursive descent/interpolation, regex/date/platform built-ins, automatic stream planning, and extended jq CLI parity
- [ ] 16.11 Verify every requirement scenario in all eight MVP capability specs maps to an automated test, report assertion, or explicitly documented manual release check
- [ ] 16.12 Mark the MVP change ready for archive only after implementation, compatibility, performance, and documentation gates all pass
