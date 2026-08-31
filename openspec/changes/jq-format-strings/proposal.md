## Why

Issue #6 shows that tq rejects jq format strings at compile time, including the common `@base64` filter. This leaves URL construction, tabular export, shell quoting, HTML escaping, and base64 conversion incompatible with jq 1.7 even though tq already has the value serialization and string interpolation machinery needed to implement them safely.

## What Changes

- Parse jq format tokens as executable filters instead of the deferred `format-strings` capability.
- Implement jq 1.7 behavior for `@text`, `@json`, `@html`, `@uri`, `@csv`, `@tsv`, `@sh`, `@base64`, and `@base64d`.
- Support the template form, such as `@uri "https://example.test?q=\(.query)"`, where literal text remains unchanged and only interpolation results are escaped.
- Enforce the existing VM output-byte limit while formatting and return stable compile or runtime diagnostics for unknown formats, invalid input types, malformed base64, and oversized results.
- Add jq differential cases for standalone filters, template interpolation, Unicode, escaping boundaries, type errors, and resource limits.
- Add correctness-gated jq comparison benchmarks with a soft target that tq takes no more than 2.0 times jq's wall time and no more than 1.5 times jq's peak resident memory on each comparable format-string case.
- Keep recursive built-ins and labels from issue #6 out of scope for this change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `jq-core-language`: Promote jq 1.7 format-string syntax and its nine documented formats from deferred syntax to supported, compatibility-tested behavior.

## Impact

The change affects lexing and parsing in `tq-core`, AST lowering and bytecode calls for formatted templates, built-in resolution and execution, query capability analysis, compatibility fixtures, core unit tests, and the benchmark catalogue. `tq-core` will need a direct RFC 4648 base64 dependency. No CLI flags or public Rust APIs change. The cross-tool performance target is reported evidence, not a CI or release gate.
