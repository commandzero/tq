## 1. Align the implementation base

- [x] 1.1 Integrate the completed `add-streaming-transcode` implementation and archived specs into this worktree without losing the current Rayon changes
- [x] 1.2 Resolve dependency and source conflicts, then run formatting, workspace checks, and the existing test suite to establish a clean baseline
- [x] 1.3 Add planner and benchmark fixtures for an order-sensitive projected sort and for `[.features[].properties.release] | sort | length`

## 2. Add optimizer and typed planning support

- [x] 2.1 Add a resolved-HIR rewrite pass with source-spanned rewrite observations
- [x] 2.2 Implement the proven-array built-in `sort | length` rewrite and negative cases for unknown inputs, `sort_by`, user functions, errors, and multi-result expressions
- [x] 2.3 Define the hybrid producer, collection boundary, blocking suffix proof, and `hybrid-streaming-blocking` plan kind in the typed phase model
- [x] 2.4 Decompose the initial array-constructor-over-streamable-generator query shape before blocking-document fallback
- [x] 2.5 Compile and validate separate producer and suffix bytecode while preserving existing event, subtree, document, whole-input, and transcode plans
- [x] 2.6 Add analysis tests for eligible static projections and pre-input fallback for dynamic paths, mutation, cross-item dependencies, slurp, YAML, and explicit event input

## 3. Execute hybrid plans from structural events

- [x] 3.1 Refactor the automatic projection and capture rules so a structural-event consumer can reuse them without jq `[path, value]` wrapper allocation
- [x] 3.2 Implement JSON and TOON container tracking, static-prefix selection, per-item closure, projected-member absence, and proven subtree capture
- [x] 3.3 Enforce duplicate-key, depth, token, input-byte, result, VM-step, and cancellation policies across selected and discarded subtrees
- [x] 3.4 Add the CLI hybrid executor that evaluates producer bytecode in encounter order and invokes suffix bytecode once after successful decoder completion
- [x] 3.5 Preserve multi-document ordering, late syntax-error behavior, and the rule that a blocking suffix publishes no result before producer completion
- [x] 3.6 Differential-test hybrid and forced document execution for empty inputs, missing members, mixed values, equal-comparing objects, multiple documents, duplicate keys, malformed late input, limits, and cancellation

## 4. Add bounded parallel preparation

- [x] 4.1 Add finite configuration and defaults for batch values, in-flight batch count, and in-flight estimated bytes
- [x] 4.2 Implement owned result batching with backpressure, ordered collection, cooperative cancellation, and high-water observations
- [x] 4.3 Implement stable Rayon sort-run preparation with ordered batch ordinals and the existing serial-versus-parallel thresholds
- [x] 4.4 Implement deterministic stable run merge and hand the completed array to the remaining suffix bytecode
- [x] 4.5 Keep a generic collect-only path for suffixes without an incremental preparation proof
- [x] 4.6 Test queue saturation, cancellation with workers active, worker-count limits, threshold behavior, stable equal values across batches, and deterministic repeated output

## 5. Expose plan and resource behavior

- [x] 5.1 Extend human and JSON explain output with the hybrid producer proof, collection boundary, blocking cause, fallback reason, and optimizer rewrites
- [x] 5.2 Extend execution reports with root-materialization status, decoder depth, batch and byte high-water marks, retained result estimates, sort runs, worker count, and final resource outcome
- [x] 5.3 Document that hybrid plans retain cardinality-proportional projected and blocking state and do not satisfy fixed-memory event guarantees
- [x] 5.4 Add report-schema and CLI tests for hybrid execution, dead-sort elimination, fallback, interruption, and resource-limit failures

## 6. Validate performance and compatibility

- [x] 6.1 Add an optimizer-resistant projected-sort benchmark whose correctness digest depends on sorted content
- [x] 6.2 Make the harness reject or relabel a blocking benchmark when machine-readable explain output says the measured operator was removed
- [x] 6.3 Run jq, the document-plan tq baseline, single-thread hybrid tq, and multi-thread hybrid tq against the largest catalogue input with identical correctness gates
- [x] 6.4 Capture wall, user, system, and total CPU time, peak RSS, worker count, exact commands, corpus identity, plan classification, and correctness digests
- [x] 6.5 Store the benchmark report and raw samples under `~/Development/commandzero/tq-benchmarks` and compare hybrid wall time and memory with the accepted document baseline
- [x] 6.6 Run workspace formatting, checks, clippy, unit tests, compatibility tests, OpenSpec strict validation, and the relevant benchmark smoke tests

## 7. Remove discarded-subtree construction

- [x] 7.1 Add a validation-only JSON seed for statically rejected subtrees that avoids structural events, jq values, paths, and exact-number construction while preserving decoder limits and errors
- [x] 7.2 Differential-test selected decoding against the structural path for values, malformed input, late errors, depth, token, numeric-envelope, missing members, and duplicate keys
- [x] 7.3 Benchmark commit `5f9c8fa` and the fast-discard implementation on the largest catalogue input with identical correctness, worker, timing, CPU, and memory controls

## 8. Remove avoidable identity-transcode construction

- [x] 8.1 Add default lightweight key, string, and numeric-literal callbacks to the structural consumer contract and preserve owned-event compatibility
- [x] 8.2 Route JSON identity transcode through canonical token text without constructing intermediate `Number`, `Scalar`, `Event`, or `Value` instances for eligible scalars
- [x] 8.3 Store and publish scalar-array replay records without rebuilding tq values, while preserving limits, cancellation, output bytes, and TOON quoting
- [x] 8.4 Differential-test lightweight and forced-document output, run workspace validation, and benchmark the accepted transcode cases against commit `983879e`

## 9. Resolve verification findings

- [x] 9.1 Enforce the VM-step limit across the complete hybrid invocation and add a multi-document reset regression
- [x] 9.2 Record truthful hybrid decoder, retention, blocking, worker, and final resource observations on success and failure
- [x] 9.3 Make selected-subtree discard and in-flight sort preparation observe cooperative cancellation without adding hot-loop synchronization
- [x] 9.4 Apply identical correctness gates to jq, forced-document tq, single-thread hybrid tq, and multi-thread hybrid tq benchmark rows
- [x] 9.5 Align the stable-order design and task wording with the implemented stable batch-order merge
- [x] 9.6 Make compatibility smoke distinguish expected semantic differences from malformed harness output and repair the incomplete-frame normalization drift
- [x] 9.7 Run formatting, checks, clippy, unit and compatibility tests, strict OpenSpec validation, benchmark smoke, and focused performance comparisons
