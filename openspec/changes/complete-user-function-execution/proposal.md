## Why

User filters are marked supported, but a valid call can still reach the older
tree evaluator and fail at runtime with an internal `UserCall` operation error.
Issue #8 exposes this with `def f: .+1; map(f)`, even though the same query works
in jq and tq already executes simpler user-filter calls.

## What Changes

- Make user-filter calls executable when they appear inside filter-taking
  built-ins such as `map`, without falling back to an evaluator that lacks user
  call frames.
- Preserve jq parameter, generator, lexical-capture, result-order, and bounded
  recursion behavior across those composed calls.
- Validate the complete executable operation graph before input is consumed,
  including function bodies and filter arguments. If a future operation cannot
  execute in the selected runtime, compilation returns a source-spanned stable
  capability diagnostic instead of exposing `VmError::Unsupported`.
- Add regression and differential coverage for issue #8 and other user filters
  nested in callback-style built-ins.
- Set a soft performance goal for representative user-function workloads: tq
  should take no more than twice jq's median wall time and use no more than 1.5
  times jq's peak resident memory.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `jq-user-functions-modules`: Require user filters to remain executable when
  passed to built-ins that evaluate filter arguments.
- `query-runtime`: Require compilation to validate executable closure across
  the root expression, user-function bodies, and callback arguments before
  constructing an executable program.

## Impact

The bytecode execution-admission walk, managed generator evaluator, user call
frames, callback-style built-in dispatch, diagnostics, core runtime tests, and
jq compatibility and benchmark cases are affected. No CLI syntax or public
option changes.
