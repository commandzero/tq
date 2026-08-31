## 1. Reproduce and classify callback execution

- [x] 1.1 Add a core regression for `def f: .+1; map(f)` that records the current evaluator-routing failure and verify the test fails with the existing `UserCall` unsupported error before implementation.
- [x] 1.2 Inventory every supported built-in that evaluates a filter argument, group each by collection, predicate, keyed-ordering, or scalar-selection semantics, and verify each group has a jq reference case covering result cardinality and order.

## 2. Prove executable bytecode closure

- [x] 2.1 Replace the independent `generator_subset` whitelist with execution-support metadata that walks root operations, arguments, and referenced user-function bodies, and verify unit tests detect an unsupported operation below a built-in argument.
- [x] 2.2 Run executable-closure validation after structural bytecode validation and return source-spanned `TQ-CAP-USER-FUNCTIONS` before constructing a compiled program when a user-function composition lacks a runtime handler; verify no input reader is opened in the failure test.
- [x] 2.3 Keep malformed externally decoded bytecode defensively rejectable by the VM while proving trusted compiled programs cannot reach `VmError::Unsupported`; verify both trusted and untrusted bytecode tests pass.

## 3. Execute user filters inside built-ins

- [x] 3.1 Add reusable explicit continuation state for evaluating callback filters with their lexical environment and managed user frames, and verify direct, captured, parameterized, and filter-parameter calls produce the jq result sequence.
- [x] 3.2 Move collection-transform callbacks, including `map` and `map_values`, onto the continuation path and verify arrays, objects, empty callbacks, and multi-result callbacks match jq order and cardinality.
- [x] 3.3 Move predicate and scalar-selection callbacks onto the continuation path and verify truthiness, short-circuiting, first-error order, and multi-result behavior against the cases from task 1.2.
- [x] 3.4 Move keyed ordering and grouping callbacks onto the continuation path and verify stable equal-key order, multiple emitted keys, empty keys, and callback errors match the accepted jq baselines.
- [x] 3.5 Enforce VM step, call-stack, fork-stack, cancellation, and early-consumer-stop behavior across callback continuations; verify recursive and high-cardinality tests report bounded high-water marks without native stack growth.

## 4. Compatibility and regression coverage

- [x] 4.1 Add issue #8 to `tests/compatibility/cases/functions-modules.jsonl` and verify tq returns `[2,3]` with the same normalized result as jq for JSON, YAML, and TOON adapters.
- [x] 4.2 Add table-driven core and CLI tests for all callback groups and verify trusted queries never print `bytecode operation is not executable in this language wave`.
- [x] 4.3 Extend the user-functions fuzz target with callback-style built-ins and verify the bounded fuzz smoke run completes without panics, hangs, or uncontrolled recursion.

## 5. Performance goals

- [x] 5.1 Add representative direct-call, `map`, predicate, and keyed-ordering user-function cases to the benchmark campaign and verify release builds of tq and jq run identical queries and inputs with recorded median wall time and peak RSS.
- [x] 5.2 Compare tq with jq on each representative case, aiming for tq median wall time at or below 2.0 times jq and tq peak RSS at or below 1.5 times jq; verify the report records raw measurements, ratios, and unavailable RSS rather than treating a missing measurement as a pass.
- [x] 5.3 If a soft target is missed, attach the benchmark evidence and profiling notes to follow-up work; verify the miss remains visible without failing correctness checks by itself.

## 6. Final verification

- [x] 6.1 Run `cargo test -p tq-core`, `cargo test -p tq-cli --test compatibility_cases functions_and_modules_cover_scope_calls_loading_and_failures`, and the compatibility smoke campaign; verify all user-function cases pass without baseline regressions.
- [x] 6.2 Run `./scripts/preflight.sh` and verify formatting, compilation, lint, workspace tests, and strict OpenSpec validation all pass.
