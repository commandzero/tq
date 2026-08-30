## 1. Planning and observability

- [x] 1.1 Extend hybrid JSON plan reporting with parallel selected-decode eligibility and fallback causes
- [x] 1.2 Define finite element-count, byte-count, and in-flight batch defaults plus high-water observations

## 2. Parallel selected decoder

- [x] 2.1 Implement a bounded lexical framer that follows a static prefix and captures complete target-array element batches
- [x] 2.2 Decode and project owned batches on the shared Rayon pool with cancellation-aware scheduling
- [x] 2.3 Reorder completed batches by ordinal, translate record paths and diagnostics, and apply backpressure
- [x] 2.4 Integrate the parallel path for eligible hybrid-blocking JSON plans with serial fallback

## 3. Correctness and resource validation

- [x] 3.1 Add serial-versus-parallel differential tests for projections, missing values, paths, duplicate keys, and stable ordering
- [x] 3.2 Add malformed-input and depth, token, numeric, byte, cancellation, and bounded-retention tests
- [x] 3.3 Run formatting plus focused and workspace test suites

## 4. Performance validation

- [x] 4.1 Build the release candidate and correctness-gate it on the largest catalogued JSON file
- [x] 4.2 Record one-worker and fourteen-worker wall, CPU, peak-RSS, worker-count, and output-digest results in `~/Development/commandzero/tq-benchmarks`
- [x] 4.3 Compare the candidate with the accepted serial fast-discard baseline and document whether the wall-time gate is met

## 5. Verification remediation

- [x] 5.1 Support arbitrary static key/index selection prefixes and report precise dynamic-dependency fallback causes
- [x] 5.2 Preserve source diagnostics across batches and make active worker decoding cooperatively cancellable
- [x] 5.3 Add deterministic coverage for ordered failures, reorder bounds, cancellation, numeric limits, downstream failures, and cross-batch stability
- [x] 5.4 Include output digests on every benchmark row and refresh the candidate benchmark evidence
- [x] 5.5 Re-run formatting, linting, workspace tests, and strict OpenSpec validation
