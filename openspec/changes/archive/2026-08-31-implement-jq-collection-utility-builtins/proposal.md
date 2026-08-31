## Why

Issue #5 identifies 23 jq standard-library filters that fail during resolution, even though tq already has the language and runtime pieces needed for most of them. The missing filters leave common jq programs unusable and make the compatibility guide overstate tq's current built-in coverage.

## What changes

- Add jq-compatible collection filters: `to_entries`, `with_entries`, `group_by`, `min_by`, `max_by`, and `limit`.
- Add path and stream conversion filters: `paths`, `path`, `getpath`, `setpath`, and `tostream`.
- Add JSON conversion, predicate, string, character, and math filters: `tojson`, `fromjson`, `any`, `all`, `ltrimstr`, `ascii_downcase`, `explode`, `implode`, `floor`, `ceil`, and `fabs`.
- Add `inputs` with jq-compatible consumption of the remaining ordered input sources during one CLI invocation.
- Classify each filter for planning and resource accounting, including blocking collection operations, generators, result limits, nesting limits, and output-byte limits.
- Add data-driven jq compatibility cases for successful results, generator cardinality, type errors, path errors, and edge conditions. Update the compatibility documentation to match the implemented set.
- Add a soft performance goal for correctness-equivalent JSON workloads: tq's median wall time should be no more than 2.0 times jq's, and its maximum observed peak resident memory should be no more than 1.5 times jq's. Report misses for investigation without blocking acceptance.

## Capabilities

### New capabilities

None.

### Modified capabilities

- `jq-core-language`: Expand the supported built-in contract with the collection, path, conversion, predicate, string, character, math, and remaining-input filters named in issue #5.
- `tq-cli`: Define how `inputs` consumes remaining stdin or file documents without evaluating those documents again in the outer CLI input loop.
- `cross-tool-compatibility`: Require manifest coverage and reviewed jq baselines for the newly supported filters.
- `performance-benchmarks`: Add comparative timing and peak-memory objectives for representative issue #5 workloads.

## Impact

The change affects built-in resolution, bytecode planning, evaluator dispatch, path traversal and mutation helpers, VM resource accounting, the CLI's input driver, and the benchmark catalogue. It also adds compatibility manifest cases and updates `docs/compatibility.md` and requirement traceability. No new third-party dependency or CLI flag is expected.
