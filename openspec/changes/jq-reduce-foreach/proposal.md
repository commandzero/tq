## Why

`reduce` and `foreach` are jq's general stateful folding primitives and unlock
stream-friendly aggregation patterns that cannot be expressed efficiently by
the MVP's fixed built-ins.

## What Changes

- Add jq-compatible `reduce` and `foreach` grammar and lexical accumulator scope.
- Preserve generator ordering, zero-result behavior, intermediate extraction, and errors.
- Add bounded managed accumulator frames and accurate blocking/streaming analysis.
- Add jq differential cases and large-input memory regressions.

## Capabilities

### New Capabilities

- `jq-reduce-foreach`: Stateful generator folds with jq-compatible accumulator and extraction semantics.

### Modified Capabilities

None.

## Impact

The parser, resolver, capability analysis, compiler, VM continuation state,
resource observations, compatibility cases, and reduction benchmarks are affected.
