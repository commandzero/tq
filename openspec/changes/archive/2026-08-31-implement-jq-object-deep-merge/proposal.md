## Why

`tq` currently rejects object operands for `*`, even though jq uses object multiplication for recursive merge. This blocks the common pattern of layering nested configuration defaults and is the behavior reported in GitHub issue #9.

## What changes

- Overload `*` so two objects merge recursively while number operands keep their existing multiplication behavior.
- At each shared key, recursively merge object pairs and otherwise use the right-hand value.
- Apply the same overloaded behavior to `*=` updates.
- Preserve deterministic insertion order and return a runtime type error for unsupported operand combinations.
- Add evaluator and jq compatibility coverage for nested merges, conflicts, updates, ordering, numeric multiplication, and invalid types.
- Add a correctness-gated benchmark with soft goals of no more than 2.0 times jq's median wall time and 1.5 times jq's peak resident memory for the same recursive-merge workload.

## Capabilities

### New capabilities

None.

### Modified capabilities

- `jq-core-language`: Define jq-compatible recursive object merge for `*` and `*=`, including conflict, ordering, and error behavior.

## Impact

The evaluator dispatch in `crates/tq-core/src/eval.rs` will gain one shared multiplication helper used by binary expressions and update assignments. Core evaluator tests, operator and update compatibility cases, and the benchmark catalog will change. The parser, bytecode format, runtime value model, CLI interface, and dependencies do not need changes.
