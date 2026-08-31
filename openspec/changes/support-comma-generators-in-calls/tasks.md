## 1. Parse generator arguments

- [ ] 1.1 Change function-call parsing so each semicolon-delimited argument uses comma precedence, and verify parser tests produce one comma-expression argument for `sort_by(.a,.b)`.
- [ ] 1.2 Add parser and resolver regressions for `f(1,2; 3,4)`, nested parentheses, and malformed delimiters, and verify semicolons still determine arity with `cargo test -p tq-core parser` and `cargo test -p tq-core resolve`.

## 2. Evaluate composite sort keys

- [ ] 2.1 Update the shared `sort_by` and `unique_by` evaluator to collect each key filter's complete ordered output into an array, propagate errors, and verify focused evaluator tests pass.
- [ ] 2.2 Add evaluator cases for single keys, multiple keys, duplicate composite keys, empty generators, and user-defined calls with unparenthesized comma generators, then verify them with `cargo test -p tq-core`.

## 3. Lock in compatibility

- [ ] 3.1 Add an end-to-end CLI regression using the issue #7 input and `sort_by(.a,.b)`, and verify `cargo test -p tq-cli --test extended_cli` matches jq's output.
- [ ] 3.2 Add or update the jq compatibility case and generated review artifacts for comma-generating function arguments, and verify the repository's compatibility evaluation reports no unexpected tq mismatch.
- [ ] 3.3 Add an issue-specific benchmark for `sort_by(.a,.b)` using identical JSON input and release builds of tq and jq; record the wall-time median and maximum peak-RSS ratios, with soft goals of at most `2.0` and `1.5` respectively, and document profiling or follow-up work if either goal is missed.
- [ ] 3.4 Run `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace` to verify the complete change.
