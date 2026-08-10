## Context

Current aggregation built-ins are fixed blocking operations. `reduce` and
`foreach` require a generator to suspend while an accumulator update and
optional extraction run for each emitted value.

## Goals / Non-Goals

**Goals:** Preserve jq ordering, scope, errors, multiplicity, and cleanup with
bounded managed fold state, while analysis reports whether a fold can remain
streaming. Internal evaluator structure is not part of the compatibility
contract.

**Non-Goals:** Parallel folds, algebraic reordering, or automatic distributed execution.

## Decisions

- Lower folds to validated generator, initializer, update, and extraction
  bytecode operands, then execute them with bounded managed fold state. The
  evaluator MAY use explicit continuations, callbacks, or another implementation
  strategy as long as generator depth cannot grow without the configured
  call/work bounds and jq-observable behavior remains identical.
- Store the accumulator as a normal immutable `Value`; path-copying preserves
  sharing while each update remains observable in jq order.
- Treat general folds conservatively in analysis, then admit event execution
  only when the generator and bodies satisfy the event-plan effect contract.

## Risks / Trade-offs

- [A body emits many accumulator values] → Preserve jq multiplicity and enforce result/work limits.
- [Large accumulators grow legitimately] → Explain retained state and add high-water/RSS benchmarks.
- [Errors after prior results] → Retain framed output and cleanly unwind all fold state.
