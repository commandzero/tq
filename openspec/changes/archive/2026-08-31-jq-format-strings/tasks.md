## 1. Parse and resolve format syntax

- [x] 1.1 Replace the deferred format lexer token with a source-spanned format-name token and verify lexer tests cover valid names, a missing name, and adjacent template strings.
- [x] 1.2 Lower standalone formats and formatted templates to built-in calls, formatting only interpolation expressions, and verify parser/display tests cover literal preservation plus zero-result and multi-result expressions.
- [x] 1.3 Register the nine zero-arity non-blocking format calls, increment the registry version, add a source-spanned unknown-format diagnostic, and verify resolver and capability-analysis tests pass.

## 2. Implement bounded formatters

- [x] 2.1 Extract jq text and compact JSON serialization into an internal formatting module used by interpolation and `tostring`, and verify existing interpolation, numeric-literal, and object-order tests remain unchanged.
- [x] 2.2 Implement bounded `@text`, `@json`, `@html`, and `@uri` conversion and verify table-driven tests cover every escaped character, Unicode byte encoding, structured values, and exact output-limit boundaries.
- [x] 2.3 Implement bounded `@csv`, `@tsv`, and `@sh` conversion and verify tests cover empty rows, quotes, control characters, null, booleans, numbers, Unicode, and rejected container values.
- [x] 2.4 Add a direct RFC 4648 base64 dependency, implement bounded `@base64` and strict UTF-8 `@base64d`, and verify round-trip, malformed input, non-UTF-8 input, padding, and allocation-bound tests.

## 3. Integrate execution paths and diagnostics

- [x] 3.1 Route both the admitted generator evaluator and general interpreter through the shared formatter dispatcher, update stream-admission matching, and verify the same format queries produce identical outcomes in forced-path VM tests.
- [x] 3.2 Map invalid format input to stable runtime type or data diagnostics and every size breach to `VmError::Resource { resource: "output-bytes" }`, then verify unit tests assert error class and boundary behavior.

## 4. Compatibility and regression coverage

- [x] 4.1 Add jq-target compatibility JSONL cases for all nine standalone formats, formatted interpolation, multiplicity, Unicode, invalid types, and malformed base64, and verify the compatibility schema/discovery tests accept the cases.
- [x] 4.2 Extend the query fuzz target with standalone and template format syntax plus bounded arbitrary values, and verify the fuzz target builds without adding an unbounded allocation path.
- [x] 4.3 Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`; fix all failures and confirm the issue's `"hello" | @base64` reproduction emits `"aGVsbG8="` from the release binary.

## 5. Performance comparison

- [x] 5.1 Add correctness-gated jq and tq benchmark cases for scalar and structured conversion, escape expansion, CSV/TSV rows, shell quoting, base64 encode/decode, and formatted interpolation; verify startup-heavy and throughput-heavy workloads use identical inputs and output sinks within each comparison.
- [x] 5.2 Run the format-string campaign outside the restricted sandbox with elevated child-process inspection permissions, release tq, and the manifest-recorded jq binary; on macOS collect every authoritative RSS sample with `/usr/bin/time -l`, report per-case median wall-time and peak-RSS ratios against the soft limits of 2.0 and 1.5, and retain every miss as visible comparative evidence without making it a CI or release failure.
