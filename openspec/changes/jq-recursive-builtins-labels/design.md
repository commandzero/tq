## Context

See [proposal.md](proposal.md) for motivation. tq already has an explicit work-stack evaluator for user calls, filter callbacks, `reduce`, `foreach`, and recursive descent. It also has ordered error-catching continuations and a traversal cursor for `..`. The remaining issue #6 features must compose with those mechanisms without Rust-stack recursion, unbounded eager traversal, or a second callback execution path.

The lexer currently classifies label syntax as deferred, and the built-in registry rejects `recurse` and `walk`. Safe compilation admits a complete reachable operation graph before input is read. Existing resource limits, cancellation checks, and benchmark rules apply to every new continuation.

## Goals / Non-Goals

**Goals:**

- Represent label exits as structured control flow that observes jq's lexical and `try`/`catch` boundary ordering.
- Reuse the explicit evaluator for recursive filters, callback execution, and post-order rebuilding.
- Make every pending branch owned by a label or recursive operation identifiable and discardable on exit, cancellation, error, or downstream stop.
- Keep representative workloads within 2.0 times jq's median wall time and 1.5 times jq's peak RSS as soft goals.

**Non-Goals:**

- Add legacy `recurse_down` aliases or promote non-finite result built-ins.
- Add native-stack recursion or an embedded jq source prelude.
- Change public resource-limit classes, object ordering, or user-function calling conventions.

## Decisions

### Resolve labels into a separate lexical symbol space

Add dedicated AST forms for a label boundary and a break expression. During resolution, maintain a label scope stack separate from value-variable scopes and replace each source label name with a unique label symbol. A break with no matching symbol fails during compilation at its own source span.

Unique symbols make nested shadowing unambiguous and prevent a same-named value binding from affecting control flow. Resolving breaks by source name during evaluation was rejected because it would duplicate lexical scope rules and make serialized or transformed bytecode unsafe.

### Route breaks through ordered control-flow boundaries

Compile a label to an explicit boundary around its expression and compile a break to a structured exit carrying the resolved label symbol. The evaluator propagates that exit through continuations in execution order. A matching label consumes it and abandons all pending alternatives owned by that label. A nearer `try` boundary handles it first and exposes jq's catch value; a label nested inside a `try` consumes its matching exit before the outer catch can observe it.

Label boundaries record ownership for scheduled branches, call frames, and reducer continuations. Consuming a break prunes everything inside that boundary while retaining values already yielded and work outside the boundary. This also gives cancellation and early-stop cleanup one ownership model.

Directly jumping to a bytecode offset was rejected because a break may cross user calls and reducer frames and must dispose of their alternatives. Treating break as an ordinary public diagnostic was rejected because unmatched breaks are compile errors and matched breaks are not user-visible failures.

### Implement recursive built-ins as managed continuation families

Register `recurse/0`, `recurse/1`, `recurse/2`, and `walk/1` with their exact filter-argument signatures and planning metadata. Execute them on the explicit work-stack evaluator using dedicated continuations rather than injecting hidden jq definitions.

For `recurse`, a continuation yields the current value first, then schedules each result of the recursive filter in encounter order. The two-argument form evaluates its condition on each generated child before scheduling that child's yield and descendants. Pending siblings and descendants remain suspended until demanded, so `first`, `limit`, errors, cancellation, and output limits can stop traversal without completing the tree.

For `walk`, post-order frames retain the input container, member cursor, partial rebuild state, environment, and callback state. Child callbacks complete before the parent callback is scheduled. Branching rebuild state preserves jq's zero- and multi-result callback behavior rather than assuming one transformed value per child. Object members and array elements are scheduled in encounter order.

An embedded jq prelude was considered because jq defines these operations in jq. Native registry entries and continuations were chosen to preserve compile-time arity checks, planning metadata, source diagnostics, evaluator accounting, and the existing no-hidden-definitions model.

### Extend executable-closure admission and resource accounting together

Mark the new operations as supported only when their callback bodies are also executable by the managed evaluator. Planning SHALL classify `recurse` as a generator with data-dependent work and `walk` as a whole-value transform with data-dependent cardinality. Neither operation may enter an automatic streaming route that cannot preserve its required context.

Every node visit, callback result, branch, rebuilt value, and retained frame charges the existing work, result, output-byte, depth, and stack budgets at the same seams used by other callback built-ins. Cancellation checks occur between scheduled units of work. Admission metadata and dispatch support remain colocated and are covered by closure tests.

Maintaining a second recursive executor was rejected because it would split limit behavior and recreate the user-function callback bug below a new built-in boundary.

### Validate semantics and performance against jq 1.7

Add focused parser, resolver, bytecode, and evaluator tests before each implementation seam. Add jq 1.7 differential cases for ordering, callback cardinality, nested label shadowing, catches on both sides of a label boundary, calls and reducers, error order, and the issue examples. Deep generated inputs exercise stack safety, limits, cancellation, and early consumer stop. Extend fuzz generation only after deterministic cases cover each control-flow boundary.

Benchmark representative bounded `recurse`, structural `walk`, and early-break workloads in release mode against jq on the same host. Follow the repository benchmark contract: every benchmark run requires elevated execution and `/usr/bin/time -l`, and a missing peak-RSS reading is unmeasured rather than passing. Report median wall-time and peak-RSS ratios against the 2.0 and 1.5 soft targets; misses create evidence-backed follow-up work but do not change correctness.

## Risks / Trade-offs

- [A break prunes too much or too little queued work] -> Tag scheduled work with lexical boundary ownership and test breaks from nested calls, reducers, catches, and sibling generators.
- [Catch and label ordering diverges from jq's private break representation] -> Use differential cases with `try` both inside and outside the matching label, including nested shadowed labels.
- [Multi-result callbacks make `walk` rebuild state grow combinatorially] -> Charge every branch and partial rebuild to existing limits and stop producing states as soon as demand ends.
- [Recursive filters retain an entire traversed tree] -> Keep descendants as suspended work items, release completed frames promptly, and measure deep and wide cases separately.
- [Planner metadata admits an unsafe streaming route] -> Add analysis tests for every arity and callback composition and require executable-closure validation before input.
- [Dual evaluator behavior drifts] -> Admit these operations only on the managed route and return a stable compile-time capability error from any unsupported route.

## Migration Plan

Land syntax and resolution before enabling runtime admission, then add label control flow, recursive continuations, `walk` rebuilding, and registry promotion behind complete differential coverage. Remove the corresponding deferred fixtures only when safe compilation and both CLI execution modes produce jq-compatible results. No data or configuration migration is required.

Rollback restores the recursive-builtins and labels capability rejections while retaining parser diagnostics and test fixtures; it must not route admitted queries into a partially supported evaluator.
