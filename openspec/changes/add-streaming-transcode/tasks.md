## 1. Baselines and differential contracts

- [ ] 1.1 Add a test and benchmark-only forced-document override so identity cases can compare plans without adding a public CLI option.
- [ ] 1.2 Record pre-change wall, CPU, RSS, output bytes, and correctness results for the accepted object-heavy and array-heavy natural cases.
- [ ] 1.3 Add byte-for-byte cross-plan fixtures covering exact numbers, ordered objects, duplicate JSON keys, empty containers, malformed late input, and all TOON delimiters.

## 2. Sink-backed writing and direct runtime-value Serde

- [ ] 2.1 Extract shared TOON key, scalar, indentation, delimiter, and line-boundary rendering that targets `Write`.
- [ ] 2.2 Route value, sequence, and CLI TOON output through the sink-backed writer while keeping `encode()` as a compatibility wrapper.
- [ ] 2.3 Add golden and property tests proving the sink-backed writer matches existing canonical bytes and never adds a document-internal trailing newline.
- [ ] 2.4 Replace `tq_core::Value` deserialization through `serde_json::Value` with a direct ordered visitor and preserve the exact number envelope.
- [ ] 2.5 Replace `tq_core::Value` serialization through `to_json()` with direct variant serialization and add round-trip tests.

## 3. Shared container preparation

- [ ] 3.1 Introduce a result-scoped preparation ledger with aggregate memory, spool, output-byte, and nesting limits plus high-water observations.
- [ ] 3.2 Implement a private length-framed structural replay codec for keys, exact scalars, and container boundaries without JSON serialization.
- [ ] 3.3 Implement secure memory-to-disk transition, configurable spool location, restrictive permissions, byte accounting, cleanup, and cancellation tests.
- [ ] 3.4 Rebuild array preparation on the shared arena with count and scalar, tabular, or expanded eligibility metadata.
- [ ] 3.5 Replay each prepared array element once in input order and test late tabular invalidation and nested aggregate budgeting.
- [ ] 3.6 Implement in-memory JSON object normalization with first-position and last-value duplicate semantics.
- [ ] 3.7 Add sorted-run spill and deterministic merge for object key indexes that exceed the shared memory budget.
- [ ] 3.8 Add an atomic unframed publication buffer that shares the ledger and publishes only after successful single-result validation.

## 4. Structural transcode adapters

- [ ] 4.1 Define the query-independent structural consumer and decoder capability metadata for duplicate policy, array declarations, and late errors.
- [ ] 4.2 Add a JSON `DeserializeSeed` adapter that emits ordered structural events and exact numeric values without building a root value.
- [ ] 4.3 Bridge the incremental TOON decoder into the same structural consumer without converting through jq path/value stream records.
- [ ] 4.4 Implement the TOON transcode consumer over the sink and preparation arena for scalar, object, array, sequence, and unframed roots.
- [ ] 4.5 Add event-versus-document equivalence tests for valid, malformed, depth-limited, duplicate-key, and spool-limited inputs.

## 5. Output-aware planning and CLI integration

- [ ] 5.1 Record a narrow semantic-identity capability for normalized `.` during query analysis and add positive and negative proof tests.
- [ ] 5.2 Add a typed transcode plan carrying the identity, decoder, writer, framing, and resource proofs.
- [ ] 5.3 Select transcode after format detection for eligible JSON or TOON identity output and preserve existing plans for every rejected condition.
- [ ] 5.4 Dispatch transcode across stdin and ordered files while preserving per-document result order, sequence framing, unframed cardinality, and exit status behavior.
- [ ] 5.5 Add human-readable and JSON explain fields for proof, fallback cause, duplicate policy, commitment mode, retained state, and configured limits.
- [ ] 5.6 Add run-report observations for preparation high-water bytes, object-index spills, spool bytes written and replayed, and final resource outcome.

## 6. Correctness, resource, and performance gates

- [ ] 6.1 Run byte-for-byte forced-document differential tests across JSON and TOON fixtures, generated values, formatting options, and multiple input sources.
- [ ] 6.2 Add adversarial tests for deeply nested containers, many unique keys, repeated keys, oversized tokens, spool exhaustion, interrupted execution, and broken pipes.
- [ ] 6.3 Add benchmark cases for wide and nested objects, root and nested arrays, scalar arrays, and tabular candidates with all required transcode observations.
- [ ] 6.4 Run the accepted natural campaign and require `recovery` and `segments` direct sequence cases to stay below 64 MiB RSS on the recorded host.
- [ ] 6.5 Compare transcode with forced document execution and publish wall, CPU, RSS, first-byte latency, preparation, and disk replay trade-offs without hiding slower cases.
- [ ] 6.6 Update CLI explanation documentation, resource-limit documentation, compatibility metadata, and benchmark methodology for the new plan.
- [ ] 6.7 Run formatting, lint, unit, integration, conformance, compatibility, and strict OpenSpec validation before enabling automatic transcode by default.
