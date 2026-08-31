## Why

Function calls currently parse each semicolon-delimited argument below comma precedence, so jq expressions such as `sort_by(.a,.b)` fail before resolution. This blocks valid jq generator syntax and leaves tq incompatible with jq for multi-key sorting and other filter arguments that emit more than one value.

## What changes

- Parse each function argument as a full comma expression while keeping semicolons as the separator between function parameters.
- Preserve comma expressions as one argument in the AST, so arity checks continue to count semicolon-delimited parameters rather than generated values.
- Make `sort_by` and `unique_by` compare the complete ordered result sequence produced by their key filter, matching jq's composite-key behavior.
- Add parser, evaluator, and compatibility coverage for comma generators in built-in and user-defined function calls.
- Set a soft performance goal for the issue-specific workload: tq's median wall time should be no more than 2 times jq's, and tq's peak resident memory should be no more than 1.5 times jq's on the same input.

## Capabilities

### New capabilities

None.

### Modified capabilities

- `jq-core-language`: Accept comma generators inside function arguments and define generator-aware keyed sorting behavior.

## Impact

The change affects function-call parsing in `crates/tq-core/src/parser.rs`, keyed sorting in `crates/tq-core/src/eval.rs`, jq compatibility tests, and the benchmark catalog. It changes no CLI flags, public Rust APIs, serialized formats, or dependencies.
