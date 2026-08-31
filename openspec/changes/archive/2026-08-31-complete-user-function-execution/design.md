## Context

`Bytecode` can contain `UserCall` and `ParameterCall`, and the explicit
generator evaluator already implements their managed call frames. Execution
chooses that evaluator only when `generator_subset` admits every reachable
operation. A filter-taking built-in such as `map` is outside that subset, so
`def f: .+1; map(f)` is routed to `Evaluator`. That evaluator implements `map`
but falls through to `VmError::Unsupported` when `map` evaluates `f`.

The safe compilation API validates bytecode structure but does not validate
that one runtime path can execute the complete reachable operation graph. The
main user-function spec also requires recursion without native-stack recursion,
which rules out adding recursive `UserCall` handling to `Evaluator` as a quick
fix.

## Goals / Non-Goals

**Goals:**

- Execute user filters inside supported built-ins using the existing explicit
  work stack, captured environments, and managed user frames.
- Preserve jq callback cardinality and ordering for collection, selector, and
  ordering built-ins.
- Make successful compilation prove that trusted bytecode has an execution
  route before any input is read.
- Keep representative user-function workloads within 2.0 times jq's median
  wall time and 1.5 times jq's peak resident memory as a soft goal.

**Non-Goals:**

- Rework user-filter syntax, name resolution, module loading, or the bytecode
  call ABI.
- Promote currently deferred jq built-ins.
- Remove the recursive evaluator in this change.

## Decisions

### Run callback-style built-ins on the explicit work-stack evaluator

Extend the generator evaluator with continuation states for built-ins that
evaluate filter arguments. Each continuation retains the input collection,
current element or member, accumulator, lexical environment, user frames, and
remaining work. It schedules the filter argument through the same evaluator as
an ordinary expression. Existing `UserCall` and `ParameterCall` scheduling then
applies unchanged inside `map`, selectors, quantifiers, and ordering helpers.

Group callback built-ins by their result contract so they share machinery:

- collection transforms consume every callback result in encounter order;
- predicates consume truthiness while preserving short-circuit and error order;
- keyed ordering operations retain each input with its emitted comparison key;
- scalar selectors retain the jq rule for zero, one, or multiple callback
  results.

The admission walk must follow call arguments and user-function bodies. It may
select the explicit evaluator only when every reachable operation has a matching
continuation or leaf implementation.

Adding `UserCall` directly to `Evaluator` was rejected. Its Rust call graph
would make user recursion consume the native stack and would duplicate filter
parameter and capture semantics. Inlining function bodies was rejected because
recursive definitions cannot be expanded and source identity would be lost.

### Validate executable closure during trusted compilation

After structural bytecode validation, walk from the program root through child
instructions, call arguments, and referenced function bodies. Record the
runtime route selected for the whole closure. A trusted compiled program is
created only when that route covers every reachable operation.

If a user-function composition reaches an operation that has not yet been
ported, return `TQ-CAP-USER-FUNCTIONS` at the first unsupported source span.
This is a defensive boundary for incomplete future work, not the expected path
for the callback built-ins covered by this change. `VmError::Unsupported`
remains useful for malformed or externally decoded bytecode, but the safe API
must not produce bytecode that reaches it.

Checking the root operation alone was rejected because issue #8 occurs below a
built-in argument. Delaying the check until VM dispatch was rejected because it
consumes input and turns a compiler/runtime mismatch into a user-visible
runtime failure.

### Test execution routes, not only language forms

Add the issue #8 query as a CLI regression and jq differential case. Add a
table-driven core suite that combines user calls and filter parameters with
each callback result contract. The tests must assert output order, errors,
early stop behavior, call-stack limits, and that unsupported-operation text is
absent from trusted compilation and execution.

The existing direct-call tests remain. They did not catch this bug because
their operation graphs stayed inside `generator_subset`.

### Measure performance against jq without making it a correctness gate

Add representative direct-call, `map`, predicate, and keyed-ordering cases to
the existing benchmark campaign. Run release builds of tq and jq against the
same query and generated input on the same host. Use the campaign's median wall
time and peak RSS measurements.

The soft targets are:

- tq median wall time no greater than 2.0 times jq median wall time;
- tq peak RSS no greater than 1.5 times jq peak RSS.

A missing RSS measurement makes the memory result unavailable, not passing.
A target miss does not fail correctness or block the change by itself. Record
the ratio, profile the slow or memory-heavy case, and create follow-up work with
the benchmark evidence. This keeps the target visible without turning noisy
cross-process measurements into a hard release gate.

## Risks / Trade-offs

- [Callback continuations duplicate parts of legacy built-in dispatch] -> Move
  pure value calculations into shared helpers and keep only scheduling state in
  the explicit evaluator.
- [Multi-result callbacks can change collection and sort semantics] -> Compare
  each callback family against jq fixtures that preserve cardinality and order.
- [The admission walk drifts from runtime dispatch] -> Define support metadata
  beside each operation handler and add a test that compiles every admitted
  operation family through the safe API.
- [A partial port rejects a composition that previously failed at runtime] ->
  Return `TQ-CAP-USER-FUNCTIONS` before input and keep the rejected case in the
  compatibility catalog until its callback handler lands.
- [Small inputs make process startup dominate jq comparisons] -> Include inputs
  large enough to measure evaluator work and report the raw medians with each
  ratio.
- [Peak RSS is unavailable on some hosts] -> Mark the memory goal unmeasured and
  retain results from a host where the campaign can sample the complete process
  group.

## Migration Plan

Land executable-closure validation and callback continuations together so the
issue #8 query changes directly from an internal runtime failure to jq-matching
output. No data or configuration migration is required. If a callback family
must be rolled back, keep the closure validator and mark that family
unsupported at compile time rather than restoring the runtime leak.
