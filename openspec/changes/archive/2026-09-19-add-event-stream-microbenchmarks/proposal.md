## Why

Issue #48 reports event-stream wall-time increases of 61.8% for JSON and 49.8% for TOON against v0.3.0, based on 1 measured sample per row. A focused slice of issue #32 can distinguish decoder, event-record, per-event VM, and output costs before choosing an optimization.

## What Changes

- Add Criterion benchmarks for structural decoding, jq stream-record production, compiled event-plan evaluation, result encoding and framing, and the in-process CLI runner.
- Use the catalog query `select(length == 2 and (.[0] | length) == 1)` with equivalent deterministic JSON and TOON fixtures, multiple bounded sizes, and accepted, rejected, and container-end records.
- Define setup, allocation, evaluation, output, and teardown costs for each group. Check ordered results outside timed loops.
- Provide reproducible same-host comparisons against v0.3.0, retaining benchmark identities, build settings, samples, and uncertainty.
- Add contributor commands and CI build/smoke validation without timing thresholds on shared runners.
- Keep native `tq-bench` as the authority for spawn-to-exit timing and peak RSS. Completing this slice does not close #48 or the full cross-crate scope of #32.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `performance-benchmarks`: Add a bounded in-process event-stream attribution contract alongside the existing native campaign requirements.

## Impact

- Development-only Criterion dependencies and explicit benchmark targets in `tq-core`, `tq-toon`, `tq-formats`, and `tq-cli`, with fixture helpers shared only where useful.
- Contributor documentation, CI smoke coverage, and developer evidence for issue #48.
- No production behavior or public API changes solely to expose private internals to benchmarks.
- No cross-tool orchestration, native Windows support, RSS measurement through Criterion, or regression fix in this change.

References: [Criterion scope #32](https://github.com/commandzero/tq/issues/32), [event-stream regression #48](https://github.com/commandzero/tq/issues/48), [native Windows deferral #31](https://github.com/commandzero/tq/issues/31).
