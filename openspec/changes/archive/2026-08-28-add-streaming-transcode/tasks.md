## 1. Baselines and differential contracts

- [x] 1.1 Add a test and benchmark-only forced-document override so identity cases can compare plans without adding a public CLI option.
- [x] 1.2 Record pre-change wall, CPU, RSS, output bytes, and correctness results for the accepted object-heavy and array-heavy natural cases.
- [x] 1.3 Add byte-for-byte cross-plan fixtures covering exact numbers, ordered objects, duplicate JSON keys, empty containers, malformed late input, and all TOON delimiters.

## 2. Sink-backed writing and direct runtime-value Serde

- [x] 2.1 Extract shared TOON key, scalar, indentation, delimiter, and line-boundary rendering that targets `Write`.
- [x] 2.2 Route value, sequence, and CLI TOON output through the sink-backed writer while keeping `encode()` as a compatibility wrapper.
- [x] 2.3 Add golden and property tests proving the sink-backed writer matches existing canonical bytes and never adds a document-internal trailing newline.
- [x] 2.4 Replace `tq_core::Value` deserialization through `serde_json::Value` with a direct ordered visitor and preserve the exact number envelope.
- [x] 2.5 Replace `tq_core::Value` serialization through `to_json()` with direct variant serialization and add round-trip tests.

## 3. Shared container preparation

- [x] 3.1 Introduce a result-scoped preparation ledger with aggregate memory, spool, output-byte, and nesting limits plus high-water observations.
- [x] 3.2 Implement a private length-framed structural replay codec for keys, exact scalars, and container boundaries without JSON serialization.
- [x] 3.3 Implement secure memory-to-disk transition, configurable spool location, restrictive permissions, byte accounting, cleanup, and cancellation tests.
- [x] 3.4 Rebuild array preparation on the shared arena with count and scalar, tabular, or expanded eligibility metadata.
- [x] 3.5 Replay each prepared array element once in input order and test late tabular invalidation and nested aggregate budgeting.
- [x] 3.6 Implement in-memory JSON object normalization with first-position and last-value duplicate semantics.
- [x] 3.7 Add sorted-run spill and deterministic merge for object key indexes that exceed the shared memory budget.
- [x] 3.8 Add an atomic unframed publication buffer that shares the ledger and publishes only after successful single-result validation.

## 4. Structural transcode adapters

- [x] 4.1 Define the query-independent structural consumer and decoder capability metadata for duplicate policy, array declarations, and late errors.
- [x] 4.2 Add a JSON `DeserializeSeed` adapter that emits ordered structural events and exact numeric values without building a root value.
- [x] 4.3 Bridge the incremental TOON decoder into the same structural consumer without converting through jq path/value stream records.
- [x] 4.4 Implement the TOON transcode consumer over the sink and preparation arena for scalar, object, array, sequence, and unframed roots.
- [x] 4.5 Add event-versus-document equivalence tests for valid, malformed, depth-limited, duplicate-key, and spool-limited inputs.

## 5. Output-aware planning and CLI integration

- [x] 5.1 Record a narrow semantic-identity capability for normalized `.` during query analysis and add positive and negative proof tests.
- [x] 5.2 Add a typed transcode plan carrying the identity, decoder, writer, framing, and resource proofs.
- [x] 5.3 Select transcode after format detection for eligible JSON or TOON identity output and preserve existing plans for every rejected condition.
- [x] 5.4 Dispatch transcode across stdin and ordered files while preserving per-document result order, sequence framing, unframed cardinality, and exit status behavior.
- [x] 5.5 Add human-readable and JSON explain fields for proof, fallback cause, duplicate policy, commitment mode, retained state, and configured limits.
- [x] 5.6 Add run-report observations for preparation high-water bytes, object-index spills, spool bytes written and replayed, and final resource outcome.

## 6. Correctness, resource, and performance gates

- [x] 6.1 Run byte-for-byte forced-document differential tests across JSON and TOON fixtures, generated values, formatting options, and multiple input sources.
- [x] 6.2 Add adversarial tests for deeply nested containers, many unique keys, repeated keys, oversized tokens, spool exhaustion, interrupted execution, and broken pipes.
- [x] 6.3 Add benchmark cases for wide and nested objects, root and nested arrays, scalar arrays, and tabular candidates with all required transcode observations.
- [x] 6.4 Run the accepted natural campaign and require `recovery` and `segments` direct sequence cases to stay below 64 MiB RSS on the recorded host.
- [x] 6.5 Compare transcode with forced document execution and publish wall, CPU, RSS, first-byte latency, preparation, and disk replay trade-offs without hiding slower cases.
- [x] 6.6 Update CLI explanation documentation, resource-limit documentation, compatibility metadata, and benchmark methodology for the new plan.
- [x] 6.7 Run formatting, lint, unit, integration, conformance, compatibility, and strict OpenSpec validation before enabling automatic transcode by default.

## 7. Single-pass JSON correction

- [x] 7.1 Revise the change contract so JSON duplicate names are a documented streaming limitation rather than a normalization promise.
- [x] 7.2 Remove whole-source JSON staging and duplicate prevalidation, stream each source once, and flush sequence RS at document start.
- [x] 7.3 Update differential, duplicate-key, atomic-output, and first-byte tests for the single-pass contract.
- [x] 7.4 Rerun the accepted natural campaign and publish corrected throughput, RSS, first-byte, and first-payload results.
- [x] 7.5 Update user documentation, run formatting and test gates, and pass strict OpenSpec validation.

## 8. Verification corrections

- [x] 8.1 Bound and spill direct-object duplicate-name tracking under a 1 MiB adversarial preparation budget.
- [x] 8.2 Route direct nested arrays through shared replay preparation and charge remaining transient composite state to the aggregate ledger.
- [x] 8.3 Record selected input, bounded probe observations, materialization, spooling, commitment, and explicit zero input-stage bytes in transcode reports.
- [x] 8.4 Add 1 MiB wide-object and nested-array regressions, update documentation, pass preflight, and confirm the transcode compatibility case.
