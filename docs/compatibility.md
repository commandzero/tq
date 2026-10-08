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

- [Coverage summary](tests/jq-manual/coverage.md): documented manual differences and their practical impact.
- [Manual comparison](tests/jq-manual/index.md): per-case outputs and token observations with their recorded provenance.
- [Campaign instructions](../tests/compatibility/readme.md): reference setup and reproduction commands.
- [Native-format review](../tests/compatibility/reviews/native-formats-v1.md): format-specific comparison evidence.

The 0.4.1 manual comparisons and token totals remain historical evidence; they
are not renewed native TOON 4.1 acceptance. This implementation documentation
does not claim fresh manual execution, refreshed result/token summaries, or
release-wide compatibility acceptance.

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

The prior performance reports remain historical evidence. No calibrated or
all-platform performance acceptance is established; #31 and the native-platform
follow-ups remain unresolved.

- Ordered JSON results and process behavior are compared.
- Compact JSON also requires exact stdout bytes.
- TOON must preserve the JSON execution contract.
- Expected differences remain differences, never exact matches.
- Missing behavior, timeouts, skips, crashes, and unexplained mismatches do not pass the gate.
