## 1. Parse generator arguments

- [x] 1.1 Change function-call parsing so each semicolon-delimited argument uses comma precedence, and verify parser tests produce one comma-expression argument for `sort_by(.a,.b)`.
- [x] 1.2 Add parser and resolver regressions for `f(1,2; 3,4)`, nested parentheses, and malformed delimiters, and verify semicolons still determine arity with `cargo test -p tq-core parser` and `cargo test -p tq-core resolve`.

## 2. Evaluate composite sort keys

- [x] 2.1 Update the shared `sort_by` and `unique_by` evaluator to collect each key filter's complete ordered output into an array, propagate errors, and verify focused evaluator tests pass.
- [x] 2.2 Add evaluator cases for single keys, multiple keys, duplicate composite keys, empty generators, and user-defined calls with unparenthesized comma generators, then verify them with `cargo test -p tq-core`.

## 3. Lock in compatibility

- [x] 3.1 Add an end-to-end CLI regression using the issue #7 input and `sort_by(.a,.b)`, and verify `cargo test -p tq-cli --test extended_cli` matches jq's output.
- [x] 3.2 Add or update the jq compatibility case and generated review artifacts for comma-generating function arguments, and verify the repository's compatibility evaluation reports no unexpected tq mismatch.
- [x] 3.3 Add an issue-specific benchmark for `sort_by(.a,.b)` using identical JSON input and release builds of tq and jq; record the wall-time median and maximum peak-RSS ratios, with soft goals of at most `2.0` and `1.5` respectively, and document profiling or follow-up work if either goal is missed.
  - 2026-08-31 on macOS 26.6/aarch64 (14 logical CPUs), the benchmark harness used release `tq 0.1.0` and Apple `jq 1.7.1` against its generated JSON smoke fixture. Across 30 measured samples, jq's median wall time was `31,354 µs` and tq's was `31,067 µs`, a ratio of `0.991`; RSS was unavailable in the sandbox. A supplemental seven-run `/usr/bin/time -lp` measurement used the same release binaries and a generated 50,000-object JSON array: jq's median was `0.10 s` with `48,267,264` bytes maximum peak RSS, while tq's median was `0.07 s` with `69,697,536` bytes maximum peak RSS. The resulting wall-time ratio of `0.70` and peak-RSS ratio of `1.44` passed both soft goals, so no profiling follow-up is required.
- [x] 3.4 Run `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace` to verify the complete change.
  - Formatting, workspace checks, changed-code Clippy, strict OpenSpec validation, and all 325 workspace tests passed. The exact Clippy command also reports nine unrelated nightly baseline findings in package metadata and pre-existing code.
