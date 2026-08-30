## Why

Queries that project a small value from each item and then apply `sort` currently materialize the complete input document before doing useful work. On the 1.1 GB GeoJSON benchmark this keeps roughly 22 GB resident and leaves JSON decoding, document construction, projection, and cleanup bound to one thread, even though the structural decoder can already consume the source incrementally.

## What Changes

- Add a hybrid streaming-blocking plan that runs a proven event or subtree prefix directly from decoder events, then retains only the values required by a blocking suffix.
- Reuse the query-independent JSON and TOON structural decoders for hybrid execution. Do not construct jq `[path, value]` records or a complete root value when the proof does not require them.
- Preserve jq values, result order, missing-path behavior, diagnostics, duplicate-key policy, cancellation, and configured resource limits across the streaming-to-blocking boundary.
- Feed retained values to bounded batches so independent preparation work can overlap decoding and Rayon can process eligible blocking operators without unbounded queues.
- Add a sound rewrite for `sort | length` when analysis proves the input is an array and sorting cannot affect observable values or errors.
- Explain hybrid plans and report decoder, queue, retained-value, and blocking-state high-water marks.
- Add correctness-gated benchmarks that distinguish hybrid execution from document execution. Blocking-sort cases must observe sorted content so dead-sort elimination cannot invalidate the measurement.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `automatic-stream-planning`: Permit a proven streaming prefix to feed a blocking suffix without retaining the complete input document.
- `query-runtime`: Represent, validate, optimize, and execute typed hybrid plans while preserving jq semantics.
- `resource-governance`: Explain hybrid retention and bound the handoff between incremental decoding and blocking execution.
- `performance-benchmarks`: Measure hybrid blocking workloads and prevent benchmark queries from timing work that the optimizer may remove.

## Impact

The planner and typed plan model in `tq-core` gain a streaming-prefix/blocking-suffix form. JSON and TOON event consumers in `tq-formats` feed a bounded handoff in `tq-cli`, while blocking operators continue to use `tq-core` evaluation and Rayon where eligible. Explain output, resource observations, compatibility tests, and the large benchmark catalogue gain hybrid-plan coverage. The change depends on the structural event decoder introduced by `add-streaming-transcode`; it does not change jq syntax or the public value model.
