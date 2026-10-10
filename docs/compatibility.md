---
type: Report
title: jq compatibility
description: Supported jq behavior, intentional differences, and compatibility evidence.
generated: { by: "process:toon-4-1-evidence-review", at: "2026-10-07T05:53:56.657Z" }
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

- [Manual comparison results](tests/jq-manual/overview.md): measured aggregate results and tokenizer measurements.
- [Manual review navigation](tests/jq-manual/index.md): links to review sections.
- [Campaign instructions](../tests/compatibility/readme.md): reference setup and reproduction commands.
- [Native-format review](../tests/compatibility/reviews/native-formats-v1.md): format-specific comparison evidence.

Manual comparison run identities (retained report headers do not record campaign IDs or capture timestamps):

| Platform | Target | Campaign ID | Captured at | tq version | tq SHA-256 | jq version | jq SHA-256 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| macos | aarch64-macos | not recorded | not recorded | tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown) | `8a0ba42cf81881829b93ee950db0d4d3f808a22af574b210484be7833b238d00` | jq-1.8.2 | `2d75340ba57a4b4b4c8708a21c2dc8e958a48aaa8bba13b27f77f6e4c0eca07e` |
| linux | x86_64-linux | not recorded | not recorded | tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown) | `474625b11f0ec3a1fa125bbeb4e55008efa50ec713b0c398822ada48f0289b91` | jq-1.8.2 | `b1c22172dd303f3be49e935aa56aa48a8b7a46e0bc838b4997d3bb451495870f` |
| windows | x86_64-windows | not recorded | not recorded | tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown) | `a1cec37e4c6e70bcd06647a5b5b5617ece0b88e41fe8a1ad9590bafc00992428` | jq-1.8.2 | `a6fc67fedaf9128a3309a1e2ebb8b986aeccf70122ee46d2cb4849e423f0c627` |

The fresh **2026-10-07 tq 0.5.0 / native TOON 4.1 C9** campaigns use frozen
release/default products and rebuilt native manual comparators against unchanged
pinned jq 1.8.2. macOS ARM64 retains **943/952 primary, 919/921 compact,
921/921 TOON**; Linux AMD64 retains **937/952, 913/921, 921/921**; native Windows
AMD64 MSVC/PowerShell retains **912/952, 879/921, 921/921**. All strict commands
exit **1**, preserving the existing 9/15/60 combined exact differences. All
518 original IDs and 303 protected IDs remain present. macOS/Linux preserve
303 protected primary/applicable-compact exact contracts; Windows preserves
291 primary and 289 primary-plus-compact matches, not 303 exact. All 4,667
manual observations per target execute and exit; there are no new primary
failures. Windows's known required-stderr CRLF mismatch in
`manual.audit.streaming.json-root-boundary-nested-empty` is explicitly removed
from savings, leaving 857 eligible samples instead of the older 858.

Source, target, profile/features, compiler, all 15 executable/reference
identities, raw observations and all-case baseline-to-candidate review are
retained locally under ignored `target/toon-4-1/candidate-c9/`. The source
snapshot has 903 files on integrated revision
`407b35d81681b8ca4ab85968f7186eea645f250f`, SHA-256
`b0a3e076abc748319859ea9810fd75bc93132b05c5b07cc116f97ffe27ecbc62`.
C9 changes only the manual comparator's required-process eligibility gates;
all product sources and frozen C8 tq/embedded binaries remain byte-identical.
Later documentation/checklist/spec synchronization is outside that snapshot.
This is not a clean release-commit build or a debug/bench substitute. Earlier
C8 and 0.4.1 reports remain immutable, not relabeled.

All 21 generated manual section Results and the unique-case summary are renewed
from the actual macOS C9 report, not old stdout rendered with a new writer.
The 877 eligible macOS samples count 11,391 JSON versus 8,152 TOON tokens with
`o200k_base` (−28.43%) and 11,365 versus 8,165 with `cl100k_base` (−28.16%).
Exact complete ordinary stdout includes LF; signed growth, unavailable
zero-denominator percentages, exclusions and section deduplication are retained.
Saved-report rendering recomputes aggregate and section totals from eligible
rows rather than trusting persisted summaries. Token blocks are excluded when
process differences are present or the TOON process contract is not explicitly
matched, including historical rows missing either eligibility field. Raw
observations and token blocks remain unchanged.
Linux's separately scoped 871-sample and Windows's 857-sample totals are in the
[native coverage table](tests/jq-manual/coverage.md#native-platform-results).
The Windows origin-module witness also retains changed process observations:
tq now emits `origin-path`, but pinned jq still reports a missing module. It
remains a #69 failure without token eligibility, not a new exact match.
These correctness-corpus counts are not representative-workload or all-platform
savings claims. Authored inventory/audit text outside Results is historical.

The final macOS full compatibility campaign freshly covers **1,220 cases**,
**4,804 executed** and **1,961 unsupported** observations, with **218 pairwise
differences**, zero harness errors, non-exited executions or declared-contract
failures. Actual embedded environment/platform denials use the frozen host for
JSON/YAML/TOON: six observations retain exit 5, the required redacted denial,
and empty stdout/results. `observed-differences` is not strict manual acceptance.
Actual corpus conversion and sequence-boundary recovery also run with the
native candidate. Full macOS preflight passes with `RUST_TEST_THREADS=1`;
the preceding parallel run's two OS `ps`-inspection timeouts remain nonpassing
evidence, without changing RSS limits or inspection deadlines. All 72
all-feature native TOON tests and four explicitly executed ordinarily ignored
reference/relocation tests pass. Remaining ignored accounting/helper tests are
not calibration proof.

The macOS differences remain the existing scoped `erfc(2)` 2-ULP and
`tgamma(0.5)` 1-ULP results, six ANSI presentations and jq's internal
`--run-tests` suffix. They are not exact matches or fresh disparity approvals.
Safe Rust `libm` and pinned jq independently round the two math inputs; exact
witnesses remain in `tests/compatibility/cases/manual-math-boundaries.jsonl`.
Color and supplied-program regressions remain in
`crates/tq-cli/tests/output_colors.rs` and `extended_cli.rs`. Reconsider each
after implementation/reference/library changes; do not normalize captured
bytes or introduce blanket tolerances.

The unchanged [PR68 closeout ledger](../tests/compatibility/reviews/parity-closeout.toon)
retains historical 0.4.1 native observations and ownership. PR68's owner-created
[parity archive](../openspec/changes/archive/2026-10-06-achieve-jq-manual-parity/tasks.md)
is inherited from its merged final tree, not re-archived by this migration.
Its Windows record retains 60 unique differences, all 303 protected IDs but
only 291 primary and 289 primary-plus-compact protected matches—not 303 exact.
Windows newline/reference/module obligations remain separately owned by
[#69](https://github.com/commandzero/tq/issues/69), and Linux's 15 differences by
[#70](https://github.com/commandzero/tq/issues/70). Their historical workspaces,
overlays and calibrated-accounting limits are not silently renamed candidate proof.

The comparison's correctness-corpus results are independent of performance
acceptance. The separate same-host migration guard was owned by the comparison
PR. Its reported 20% hard limit and 14.32% keyed-transcode slowdown advisory are
historical comparison-PR evidence, not manual-comparison measurements. The
prior noisy run remains inconclusive. No calibrated or all-platform performance
acceptance is established. See [compatibility follow-ups](https://github.com/commandzero/tq/issues/31).
[#31](https://github.com/commandzero/tq/issues/31) and native-platform follow-ups
remain unresolved; native renewal cannot waive their actual obligations.

- Ordered JSON results and process behavior are compared.
- Compact JSON also requires exact stdout bytes.
- TOON must preserve the JSON execution contract.
- Expected differences remain differences, never exact matches.
- Missing behavior, timeouts, skips, crashes, and unexplained mismatches do not pass the gate.
