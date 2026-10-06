---
type: Report
title: jq compatibility
description: Supported jq behavior, intentional differences, and compatibility evidence.
---

# jq compatibility

`tq` follows jq 1.8.x semantics for supported queries. The manual comparison uses pinned jq 1.8.2.

## Main differences from jq

- Default output is TOON, not JSON.
- Default colors and quote styling differ from jq.
- Some math results differ by 1–2 binary64 ULP (steps between representable numbers).
- Bessel functions `jn` and `yn` require a finite order with absolute value at most 1,024.
- Regex and platform behavior have documented limits.
- Compatibility is tested, not a claim of complete jq parity.

See the [current results and numbered differences](tests/jq-manual/coverage.md#differences-by-test).

## Supported queries

- Navigation, pipes, and comma generators.
- Arrays, ordered objects, and variables.
- Conditionals, operators, and common built-ins.
- Optional access and `try/catch`.
- Path queries and updates.
- `reduce` and `foreach`, preserving generator order and results emitted before later errors.
- User-defined filters with lexical scope, recursion, and bounded VM execution.
- Modules with explicit or default lookup roots and bounded filesystem access.

`input` and `inputs` share the top-level input cursor. Consumed values are not evaluated again as later top-level inputs.

- The CLI enables environment, clock, timezone, and input-metadata access.
- Embedded callers retain capability controls.
- Call resolution checks names and arities before reading input.

See [migration and security notes](jq-parity-migration.md) and [regex, date, and platform limits](jq-regex-date-platform.md).

## Input and output

### Choosing a format

- Use `--input-format` to select a parser explicitly.
- Content detection prefers canonical TOON and distinguishes JSON from YAML using a bounded prefix.
- `.jsonl` and `.ndjson` files select strict JSON Lines input.
- `.json5` files select JSON5 input; content detection never selects JSON5.
- JSON5 is input-only.

See the [format matrix](formats.md) for supported formats.

### Choosing output

- Default TOON output ends each result with LF; 0 results produce empty stdout.
- Completed results remain available if a later result fails.
- `-c` selects compact JSON without needing `-o json`.
- `-o json` selects pretty JSON.
- `-o jsonl` selects compact JSON Lines.
- `-o yaml` selects exact-number-preserving YAML 1.2.
- `-r` writes strings raw; `-j` also omits their output separators.
- `-o toon-seq` selects RS-framed TOON output.
- `--unframed` requires exactly 1 standalone TOON result before publishing it.
- `-c` cannot be combined with explicit `-o toon`.

### Sequence input

- `--seq` selects JSON Text Sequence input and sequence-framed output.
- `tq --seq` keeps TOON output.
- `tq --seq -o json` corresponds to `jq --seq`.
- `-i toon-seq` selects TOON sequence input.

### Parse-error passthrough

- `-x/--proxy-on-error` writes original source bytes only when parsing rejects them.
- Valid sources still run through the filter.
- Query, runtime, resource, I/O, and output errors are not hidden.
- Under `--slurp`, a parse rejection proxies the complete ordered source set.
- `input` and `inputs` use the same rule; rejected sources are not supplied as values.
- `--stream-errors` cannot be combined with passthrough.

Use `tq --help` or the [CLI option inventory](jq-1.8-cli-options.md) for flags and argument binding.

## Color presentation

- `-C` forces color; `-M` disables it.
- Automatic color stays off when output is redirected.
- Nonempty `NO_COLOR` disables automatic color; the last explicit `-C` or `-M` wins.
- `TQ_COLORS` takes priority over `JQ_COLORS`; both accept 7 or 8 entries.
- An empty or invalid selected palette uses tq's defaults.
- Setting a palette does not enable color by itself.
- tq styles enclosing quotes as structure rather than string or key content.
- Matching jq's palette does not make ANSI bytes identical: quote styling and reset boundaries still differ.
- These are expected presentation differences, not changes to JSON data or process behavior.
- Removing tq-generated ANSI styles recovers the plain serialization.
- Raw strings and proxy bytes remain verbatim.
- Use `-M` when output must be directly parseable.
- Embedded writers default to plain output and respect environment and terminal capability controls.

## Memory and limits

- Ordinary queries retain 1 decoded document.
- `--slurp` retains all input documents.
- Sorting, uniqueness, collection, and output construction can retain much more data.
- `--stream` projects supported input into jq-style path/value records.
- Streaming does not bound a collecting query such as `[inputs]`.
- YAML remains document-at-a-time.
- `tq --explain-json FILTER` shows the selected plan and its memory behavior.

The CLI derives these ceilings from available memory at startup:

| Control | Share | Fallback |
| --- | ---: | ---: |
| `--prepare-memory-bytes` | 1/8 | 8 MiB |
| `--hybrid-in-flight-bytes` | 1/32 | 8 MiB |
| `--decode-in-flight-bytes` | 1/32 | 64 MiB |

- Explicit flags override these defaults.
- These are ceilings, not reservations or bounds on total process memory.
- Array preparation can spill completed elements to private temporary storage.
- A single value that cannot fit still fails with a resource diagnostic.
- Input, evaluation, output, and spool limits are explicit; exceeding them is not successful execution.
- SIGINT is cooperative; a closed downstream pipe is treated as successful.

Identity JSON or strict TOON conversion may use the optimized `transcode` plan:

- It is an optimization, not a query-language extension.
- Duplicate JSON keys or TOON paths are rejected in this path.
- Completed records survive later errors; a rejected record is not published.

## Compatibility evidence

- [Coverage summary](tests/jq-manual/coverage.md): current results and the practical impact of each difference.
- [Manual comparison](tests/jq-manual/index.md): per-test outputs and tokenizer measurements.
- [Campaign instructions](../tests/compatibility/readme.md): reference setup and reproduction commands.
- [Native-format review](../tests/compatibility/reviews/native-formats-v1.md): format-specific comparison evidence.

The latest 2026-10-06 macOS **newhelp-final** and Linux **help-final**
[closeout evidence](../tests/compatibility/reviews/parity-closeout.toon) is bound
to 884-file snapshots at base `3e0dedb` plus uncommitted fixes, not clean-commit
releases or claims about later edits. Final `crates/tq-cli/src/args.rs` help and
its regression distinguish CLI ambient access from embedded admission, removing
the false redaction wording without changing runtime policy. Earlier evidence
remains checkpoint-only; final product/helper hashes differ. Frozen release
runs against jq 1.8.2 record macOS ARM64 **943/952**
primary, **919/921** compact, **921/921** TOON matches and Linux x86_64
**937/952**, **913/921**, **921/921**. Both preserve all 518 original cases and
303 protected exact contracts. Strict manual exit 1 remains on both.

macOS retains the existing user-accepted scoped `erfc(2)` 2-ULP and
`tgamma(0.5)` 1-ULP differences, 6 intentional ANSI presentations governed by
the main `output-colors` contract, and jq's internal `--run-tests` diagnostic
suffix (both supplied programs pass and exit 0). These are not exact matches
or fresh executable approvals; current strict reports apply no reviewed disparities.
Safe Rust `libm` and pinned jq round the two math inputs differently; the exact
regression witnesses are in `tests/compatibility/cases/manual-math-boundaries.jsonl`.
Color regressions are in `crates/tq-cli/tests/output_colors.rs`; supplied-test
runner regressions are in `crates/tq-cli/tests/extended_cli.rs`. The closeout
record maps cases, causes, regression paths and reconsideration conditions;
library/reference or presentation/runner changes require exact witness reruns.

Windows was unreachable for renewal. Historical v6 results (912/952 primary,
879/921 compact, 921/921 TOON; 60 unique differences) are not current-candidate
native verification. All 81 retained Windows v6 manifest entries and the four
historical metadata references hash-match, including retained full raw manual
evidence; this establishes retention integrity only. Live #69/#70/#31 bodies
were verified on 2026-10-06 with all three issues open. The user-approved scope
transfer assigns only the enumerated
Windows 60 to [#69](https://github.com/commandzero/tq/issues/69) and Linux 15 to
[#70](https://github.com/commandzero/tq/issues/70); they remain unresolved, with
target/case ownership, practical impact, and reconsideration criteria in the
closeout record. Recheck exact witnesses and protected contracts after
implementation or reference/library/platform changes before changing verdicts.

Corrected shared full campaigns cover 1,220 cases and 4,804 executed observations
with zero harness errors or declared-contract failures, retaining pairwise
mismatches and unsupported observations as `observed-differences`, not strict
acceptance. macOS retains the separately hashed P2 campaign that **predates the
help fix**, not a final-help full rerun: only diagnostic help strings/test differ
among crate sources, but final CLI/helper/embedded hashes are not equal. Linux
reruns the full campaign with final-help binaries. Its workspace records **1,891
passed, 0 failed, 11 ignored**, with four reference/relocation entries explicitly
passed separately; remaining ignored accounting/worker/helper entries are not
acceptance or calibration proof. macOS final full preflight/all-feature tests
pass. P2 adversarial denial and companion contracts remain tested.

The parent rendered the newest macOS report into all 21 generated Results blocks
and updated index provenance; only the help capture changes. Help regression
`args::tests::help_distinguishes_cli_ambient_access_from_embedded_admission`
passes on both platforms; parser/capability/help changes require rechecking it,
release `--help` and embedded guards. No fresh disparity approvals follow.

Calibration/performance acceptance remains deferred under
[#31](https://github.com/commandzero/tq/issues/31) and platform follow-ups. No fresh
calibration is established on any platform; Windows accounting checks do not
prove macOS calibration. Unfinished macOS calibration and affected benchmark
comparisons remain unpublished as accepted performance evidence. Native Windows
renewal is the only pending fresh native platform campaign. Linux full validator
preflight remains a #70 follow-up; active OpenSpec PR-boundary and overall task/
artifact verification remain parent obligations, not implied passes. Final help/
version/documentation proof reports no blocking mismatch; the separate macOS
PR-boundary check still exits 1 on uncommitted OpenSpec changes.

- Ordered JSON results and process behavior are compared.
- Compact JSON also requires exact stdout bytes.
- TOON must preserve the JSON execution contract.
- Expected differences remain differences, never exact matches.
- Missing behavior, timeouts, skips, crashes, and unexplained mismatches do not pass the gate.
