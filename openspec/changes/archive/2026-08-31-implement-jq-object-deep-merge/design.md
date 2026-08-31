## Context

The parser and bytecode already represent `*` and `*=`. The evaluator sends both forms directly to the numeric helper in `crates/tq-core/src/eval.rs`, which produces the failure in issue #9. Runtime objects use shared `Arc<IndexMap<Arc<str>, Value>>` storage, and object addition already demonstrates the expected first-position insertion behavior for a shallow merge. Structured input has a default nesting limit of 256.

## Goals / Non-goals

**Goals:**

- Keep the overload decision in one evaluator helper so `*` and `*=` cannot disagree.
- Reuse immutable values and clone only object maps on paths changed by the merge.
- Match jq result content and object order for supported values.
- Target a median wall time no greater than 2.0 times jq and peak resident memory no greater than 1.5 times jq on the same representative recursive-merge workload.

**Non-goals:**

- Change shallow object addition.
- Add coercion between objects, numbers, or other JSON value types.
- Change parsing, bytecode, planning, input streaming, or the public value model.
- Optimize merging beyond the repository's current structured-data depth and memory limits.
- Turn the jq-relative performance targets into CI, merge, or release gates.

## Decisions

### Dispatch multiplication by operand type

Add a `binary_multiply` helper beside `binary_add` and `binary_subtract`. It will call `Number::multiply` for two numbers, call an object merge helper for two objects, and return a runtime type error for every other pair. Both `BinaryOperator::Multiply` and `AssignmentOperator::Multiply` will call this helper.

Keeping dispatch in the evaluator matches the existing overloaded addition design. Changing `Number::multiply` would mix JSON container behavior into the numeric type, while separate binary and update implementations would duplicate the semantic rules.

### Merge objects with copy-on-write map reconstruction

The merge helper will clone the left `IndexMap`, then visit right-hand entries in order. For an occupied key whose old and new values are both objects, it will recursively merge those objects and replace the value at the existing position. For every other occupied key, it will replace the value with the right-hand value at the existing position. A vacant key will append at the end.

This follows jq's right-biased recursive merge and the existing ordered-object contract. It also preserves shared subtrees that the merge does not touch. Building a new sorted map would break encounter order. Mutating either input through shared storage would violate the runtime value model.

A direct recursive helper is appropriate because the structured input limit caps normal operand depth at 256 and query literals have a lower compile-time structural limit. An explicit work stack would add state management without changing observable behavior under those limits.

### Test the operator contract at two levels

Focused evaluator tests will cover recursive merges, right-biased conflicts, key order, numeric multiplication, unsupported mixed types, and `*=`. Versioned compatibility cases will record jq's result sequence and error contract for the binary and update forms. Existing numeric cases remain regression coverage.

The repository requires compatibility-case-first development. Unit tests alone would miss result ordering or diagnostic-class differences at the CLI boundary.

### Measure the soft performance target against jq

Add a correctness-gated JSON benchmark that reads the same input in release builds and evaluates `.[0] * .[1]` in both tools. The fixture should be large enough for merge work to dominate process startup and should contain nested shared keys, replacements, and right-only keys. Use the benchmark harness's normal warmup and sampling policy on one recorded host.

Report tq's median wall time divided by jq's median wall time and tq's peak RSS divided by jq's peak RSS. The targets are at most `2.0` for wall time and `1.5` for peak RSS. A miss should remain visible and produce a follow-up optimization note, but it will not fail preflight or block the correctness fix. This follows the repository policy that cross-tool results are comparative evidence rather than universal pass or fail thresholds.

## Risks / Trade-offs

- Recursive merging clones each changed object map. Deep, wide merges can allocate in proportion to the left-side maps along changed paths. This is consistent with immutable runtime updates and remains subject to existing input and output limits.
- The helper relies on `IndexMap` replacement retaining an occupied key's position. Tests will assert top-level and nested output order so a dependency behavior change is visible.
- Raising nesting limits far beyond the current bound could make direct recursion unsafe. Revisit the helper before raising those limits.
- Peak RSS and short runtimes can be noisy. Use a workload large enough to outlast startup, apply the existing sample policy, and record the host and binary identities with the ratios.

## Migration plan

Ship the behavior as a backward-compatible jq conformance fix. No stored data or configuration migration is needed. Rollback consists of reverting the evaluator helper and its compatibility cases.
