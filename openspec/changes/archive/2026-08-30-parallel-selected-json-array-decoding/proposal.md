## Why

The hybrid blocking pipeline now retains only selected values, but its JSON decoder still frames, validates, and projects every selected array element on one thread. On the 1.12 GiB benchmark this serial stage dominates wall time, leaving the existing Rayon sort workers idle and producing only a modest speedup over one worker.

## What Changes

- Add a bounded parallel selected-array decoder for automatically proven JSON hybrid-blocking plans.
- Frame the selected array serially, decode and project independent element batches on Rayon workers, and reorder completed batches before VM consumption.
- Preserve jq-compatible value order, deterministic diagnostics, input limits, cancellation, and fallback behavior.
- Report parallel decode eligibility and bounded in-flight observations.
- Add correctness-gated 1-worker and multi-worker benchmarks on the largest catalogued JSON input.

## Capabilities

### New Capabilities

- `parallel-selected-json-decoding`: Bounded, ordered parallel decoding of independent elements from a statically selected JSON array.

### Modified Capabilities

- `automatic-stream-planning`: Allow an eligible hybrid-blocking JSON plan to select parallel selected-array decoding before semantic input consumption.
- `query-runtime`: Require deterministic values and diagnostics when decode work completes out of order.
- `resource-governance`: Bound and expose parallel decode batches, bytes, and cancellation behavior.
- `performance-benchmarks`: Correctness-gate and measure the parallel selected-decoding workload at one and multiple workers.

## Impact

The change affects JSON streaming in `tq-formats`, automatic hybrid execution and reporting in `tq-cli`, Rayon scheduling, resource-limit tests, and the external benchmark archive. It adds no CLI-breaking behavior: unsupported inputs and plans continue through the existing serial decoder.
