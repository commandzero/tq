## Context

The lexer currently converts every `@name` token into `Deferred("format-strings")`, so parsing stops before the resolver or VM can act. tq already supports source-spanned calls, jq string interpolation with generator semantics, compact bounded JSON serialization, an output-byte VM limit, and dual evaluator paths for admitted streaming expressions and the general interpreter.

The jq 1.7 contract has two syntax forms. Standalone `@name` formats the current input. A following string template copies literal segments unchanged and applies the selected format only to values emitted by interpolation expressions. The second form rules out treating format strings as a simple prefix operator over the completed string.

## Goals / Non-Goals

**Goals:**

- Reuse ordinary zero-argument built-in calls after parsing so both evaluator paths share one execution contract.
- Preserve interpolation ordering, Cartesian-product behavior, error flow, and source spans.
- Bound every formatter by `VmLimits::output_bytes`, including temporary buffers whose size can be predicted from input length.
- Match defined jq 1.7 behavior and choose a deterministic error for decoded bytes that cannot inhabit tq's UTF-8 string model.
- Aim for no more than 2.0 times jq's wall time and 1.5 times jq's peak resident memory on each comparable, correctness-gated format-string benchmark case.

**Non-Goals:**

- Adding jq development-version `@urid` support.
- Supporting raw byte strings or jq's undefined non-UTF-8 `@base64d` behavior.
- Implementing recursive built-ins, labels, or `break` from the broader issue.
- Changing CLI output modes or adding locale-dependent CSV behavior.

## Decisions

### Lower format syntax to built-in calls

Replace the deferred lexer token with `TokenKind::Format(Arc<str>)`. The parser lowers standalone `@name` to a zero-argument call whose internal name retains the `@` prefix. Supported format calls are registered as non-blocking built-ins, and the built-in registry version increments.

For a following template, the parser first builds the existing literal or interpolation expression. A plain literal remains unchanged. For an interpolated string, it rewrites each `InterpolationSegment::Expression(expr)` as `expr | @name` while preserving literal segments. Existing interpolation evaluation then handles zero-result expressions, multiple results, error propagation, and concatenation without a new bytecode operation.

The parser validates the format token before lowering so unknown names cannot disappear with a literal-only template. The resolver also recognizes supported format call names and keeps a format-specific guard for unresolved calls beginning with `@`, rather than emitting the generic unknown-function diagnostic.

This is smaller than adding format-specific AST, bytecode, continuation, and fallback-evaluator variants. It also keeps formatting available anywhere an ordinary filter can run.

### Put bounded conversion in a dedicated core module

Move the existing compact JSON writer and jq text conversion out of `eval.rs` into a focused internal formatting module. That module exposes bounded text conversion and one dispatcher for the nine formats. Interpolation, `tostring`, and format filters all use the same serializer, which prevents number and object-order differences.

Each encoder appends through a writer that checks the output limit before growth. Base64 encoding checks the RFC 4648 encoded length before allocating. Base64 decoding checks the maximum decoded length first, rejects malformed input, then requires valid UTF-8. URI encoding works on UTF-8 bytes and emits uppercase hexadecimal for every byte outside `A-Z`, `a-z`, `0-9`, `-`, `.`, `_`, and `~`.

Use the `base64` crate as a direct dependency. Implement HTML, URI, CSV, TSV, and shell escaping locally because their jq dialects are short, exact, and do not match general writer defaults closely enough to justify more dependencies.

### Encode jq's type rules explicitly

`@text`, `@html`, `@uri`, and `@base64` start with jq text conversion, where strings remain unquoted and other values use compact JSON. `@json` always uses compact JSON, including quotes around string inputs.

`@csv` and `@tsv` accept only arrays of scalars. CSV always quotes string fields and doubles embedded quotes. TSV escapes backslash, tab, carriage return, and line feed in strings. Both render null as an empty field and render numbers and booleans as jq text.

`@sh` accepts a scalar or an array of scalars. It single-quotes strings and represents an embedded single quote with jq's close, escaped quote, reopen sequence. Numbers, booleans, and null are unquoted jq text. Objects and nested containers fail.

### Test syntax and algorithms at their owning layers

Lexer and parser tests cover token spans, standalone calls, templates, literal-only templates, and unknown names. Resolver tests cover registry versioning, format diagnostics, and non-blocking capability analysis. Formatter unit tests use table-driven boundary cases for every escape rule and output limit. VM tests exercise both the admitted generator path and the general interpreter path. Compatibility JSONL cases compare defined results and error classes against jq 1.7.1.

### Measure the jq comparison as a soft target

Add representative format-string workloads to the existing benchmark catalogue. The set covers scalar and structured text conversion, expanding escapes, tabular rows, base64 encode/decode, and formatted interpolation. jq and tq must consume the same JSON artifact, write equivalent output to the same sink, and pass the corresponding compatibility case before their samples count.

Run the release tq binary and the manifest-recorded jq binary on the same benchmark host under the existing warmup and sample policy. For each comparable case, report `tq median wall time / jq median wall time` and `tq median peak RSS / jq median peak RSS`. The soft goals are ratios at or below 2.0 and 1.5 respectively. Report startup-heavy and throughput-heavy cases separately so process startup does not hide formatter cost.

A missed target remains visible in the benchmark report and prompts profiling, but it does not fail CI, block correctness work, or override tq's own accepted regression thresholds. This follows the main performance specification's treatment of cross-tool measurements as comparative evidence.

## Risks / Trade-offs

- [Parser lowering could change displayed AST text or source spans] -> Preserve the original format and interpolation spans on synthesized calls and add parse/display assertions.
- [The fast evaluator may drift from the fallback interpreter] -> Route both call sites through the same formatting dispatcher and run equivalent forced-path tests.
- [Base64 expansion or escaping can allocate beyond the VM limit] -> Check calculated lengths before allocation and append all variable expansion through the bounded writer.
- [jq permits implementation-dependent results for decoded non-UTF-8 bytes] -> Reject them with a documented runtime error and exclude undefined jq behavior from required differential parity.
- [CSV or shell escaping can look correct on common inputs while missing edge cases] -> Derive a jq 1.7.1 baseline table for quotes, empty strings, controls, Unicode, nulls, and invalid nested values.
- [Small cases may measure process startup more than formatting] -> Report startup and throughput workloads separately and apply the same invocation model to both tools.
- [A soft target may regress unnoticed] -> Record both ratios for every accepted campaign and call out misses without turning the jq comparison into a universal release gate.

## Migration Plan

Land parser, formatter, evaluator, and tests together. Existing queries that do not contain `@name` are unaffected. Queries using a recognized format move from a compile-time deferred error to execution. Unknown formats continue to fail during compilation. Rollback consists of reverting the change, which restores the existing deferred capability error without stored-data migration.
