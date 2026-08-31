## Context

See `proposal.md` for motivation. The call parser currently uses assignment precedence for each argument and treats semicolons as argument separators. As a result, it stops before a comma. The AST, resolver, bytecode, and user-function evaluator already represent each argument as a filter expression and already support generator behavior once a comma expression reaches them.

`sort_by` and `unique_by` are the exception. Their shared evaluator takes only the first value from the key filter. jq instead compares the ordered sequence of all values emitted by that filter. The implementation must address both layers to make the issue's query correct.

## Goals / Non-Goals

**Goals:**

- Preserve the existing semicolon-based arity model while allowing unparenthesized comma expressions within each argument.
- Match jq's composite-key semantics for `sort_by` and `unique_by`, including zero, one, or many key values.
- Keep error propagation, stable sorting, and resource accounting on existing evaluator paths.
- Aim for tq median wall time at or below 2 times jq and peak resident memory at or below 1.5 times jq on the same comma-generator sorting workload.

**Non-Goals:**

- Add deferred built-ins such as `IN`.
- Change comma precedence outside function calls.
- Treat commas as separators between function parameters.

## Decisions

### Parse each argument at comma precedence

`call_or_name` will parse an argument with the same comma-expression entry point used at the query root, then consume a semicolon or the closing parenthesis. This produces one `ExprKind::Comma` argument for `f(1,2)` and two such arguments for `f(1,2; 3,4)`.

This is preferable to special-casing commas in selected built-ins. The syntax applies to built-ins, user functions, and future functions. It also keeps arity tied to the AST argument vector, whose entries are separated only by semicolons.

### Store keyed-sort filter results as an array

For every input element, the keyed-sort evaluator will collect every successful value emitted by the filter, in order, into a `Value::Array`. It will compare that array with the existing jq-compatible value ordering. An empty generator produces `[]`; a single result produces `[value]`; multiple results produce `[first, second, ...]`.

Wrapping single values changes their internal representation but not their relative ordering because every element in one keyed-sort operation uses the same wrapper. This approach directly models jq's composite keys and lets `unique_by` reuse the same equality rule.

Taking only the first result was rejected because it silently ignores secondary keys. Expanding one input element into multiple independently keyed entries was also rejected because it changes array cardinality and does not match jq.

### Test syntax and behavior separately

Parser tests will assert that commas nest inside one call argument and semicolons still split arguments. Evaluator tests will cover multi-key sorting, multi-key deduplication, empty keys, error propagation, and a user-defined function with unparenthesized comma generators. A CLI compatibility case will preserve the issue reproduction as an end-to-end regression.

### Measure performance against jq

Add a benchmark row that runs `sort_by(.a,.b)` through release builds of tq and jq against the same generated JSON input. Use the existing benchmark harness, sampling policy, environment record, median wall-time summary, and maximum observed peak RSS. Report `tq wall median / jq wall median` and `tq peak RSS / jq peak RSS`.

The target ratios are at most `2.0` for wall time and `1.5` for peak RSS. This is a soft goal. A miss calls for profiling and a recorded follow-up, but it does not weaken correctness requirements or by itself block the change.

## Risks / Trade-offs

- [Collecting every key result uses more memory than taking the first] -> Charge evaluation through the existing generator and value limits, and collect only one key array per input element.
- [Composite key arrays may miss the performance goal on large inputs] -> Benchmark against jq before merge, profile any miss, and record the measured ratios and follow-up work.
- [Changing parser precedence could accidentally alter arity] -> Add resolver tests for one comma-generating argument and two semicolon-delimited arguments.
- [The optimized and general evaluator paths could diverge] -> Keep keyed sorting on its current blocking path and test through the public compile-and-run flow rather than a helper alone.

## Migration Plan

No data or configuration migration is needed. Ship the parser and evaluator changes together. Reverting both restores the previous behavior if a compatibility regression appears.
