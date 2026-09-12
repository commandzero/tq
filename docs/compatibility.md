---
type: Report
title: jq compatibility
description: Supported jq behavior and the manual compatibility evidence policy.
generated: { by: codex/gpt-5, at: 2026-09-09T17:44:58Z }
---

# jq compatibility

`tq` follows jq 1.8.x semantics for the features it supports. That includes
navigation, pipes and comma generators, arrays and ordered objects, variables,
conditionals, operators, common built-ins, optional access, `try/catch`, path
updates, user filters, and modules. Stateful `reduce` and
`foreach` folds preserve jq's generator order, accumulator scope, update count,
intermediate results, and output produced before a later error.

Collection and utility compatibility includes `to_entries`, `with_entries`,
`group_by`, `min_by`, `max_by`, `limit`, `paths`, `path`, `getpath`, `setpath`,
`tostream`, `tojson`, `fromjson`, `inputs`, two-argument `any` and `all`,
`ltrimstr`, `ascii_downcase`, `explode`, `implode`, `floor`, `ceil`, and `fabs`.
`inputs` advances the same ordered source cursor as top-level evaluation, so a
value it consumes is not evaluated again as a later top-level input.

Run `tq --help` for the current switches. Run `tq --explain-json FILTER` to see
the plan and what it retains in memory.

Identity JSON or strict TOON input written as canonical TOON may select the
`transcode` plan. This is an execution optimization, not a language extension:
bytes must match forced document execution for inputs without duplicate object
names. Streaming JSON cannot apply jq's last-value/first-position normalization
after publishing an earlier member. A duplicate therefore rejects the current
record. Sequence framing discards that final record before publication; unframed
output publishes nothing. Strict TOON also rejects duplicate paths. Safe key folding,
sorted-key output, explicit jq stream input, slurp, raw/joined output,
proxy-on-error, non-TOON output, and non-identity filters use the existing plans.

## Input and output

The format detector reads a bounded prefix. It prefers canonical TOON. A JSON
object or array opener selects JSON before YAML, while YAML document,
directive, and root-sequence markers select YAML. `.jsonl` and `.ndjson` files
select strict one-value-per-line JSON Lines input. A `.json5` extension selects
document-at-a-time JSON5 input. Use
`--input-format toon|yaml|json|json5|jsonl|json-seq|toon-seq|csv|tsv` to select exactly one parser;
`ndjson` is an alias for `jsonl`. JSON5 is input-only and is never selected by
content detection. See the [format compatibility matrix](formats.md) for the
native formats supported by jq, yq, and tq, and the
[native-format campaign review](../tests/compatibility/reviews/native-formats-v1.md)
for jq sequence agreement and deliberate yq row-profile differences.

`-x/--proxy-on-error` retains each bounded structured source before evaluation.
If its parser rejects the source, `tq` writes the original bytes unchanged and
treats that source as successful. It does not mask resource, I/O, query,
runtime, or output failures. Sources are independent except under `--slurp`,
where any parse rejection proxies the complete ordered source set.
`--stream-errors` is incompatible because it assigns a different meaning to
parse failures.

Default structured TOON output writes each result as canonical TOON followed by
LF, without RS. Zero results produce empty stdout; multiple results and late
errors do not require another mode. Completed results survive later errors.
Use `-o toon-seq` for explicit RS-framed TOON records without changing the input parser.
Select `-c` or
`--compact-output` for compact JSON, `--output-format json` for pretty JSON,
`--output-format jsonl` for compact LF-terminated JSON Lines,
`--output-format yaml` for exact-number-preserving YAML 1.2 output, `-r` for raw
strings, or `-j` to join raw outputs. `--unframed` explicitly requires one standalone TOON document and rejects zero
or multiple results before publishing output.

`-c` does not require `-o json` and does not change the input parser. Combining
it with an explicit `-o toon` is an error in either argument order. `--seq`
selects JSON Text Sequence input, JSON record framing when JSON output is
selected, and TOON Text Sequence framing when TOON output is selected. Use
`-i toon-seq` for TOON sequence input. Thus `jq --seq` corresponds to
`tq --seq -o json`, while `tq --seq` keeps TOON output. JSON sequence recovery and stream-error
details remain part of the manual conformance gate, not a claim implied by
format support.

The extended jq-shaped CLI supports short clusters plus `--raw-output0`,
`-a/--ascii-output`, `-S/--sort-keys`, explicit color/monochrome output,
`--tab`, reviewed `--indent`, and `--unbuffered`. JSON Lines output is always
compact and rejects pretty, indentation, tab, raw, joined, and forced-color
output. `--arg`, `--argjson`, and `--argtoon` populate both direct
variables and `$ARGS.named`; `--args`/`--jsonargs` populate `$ARGS.positional`;
and `--rawfile`/`--slurpfile` use the configured per-source byte limit. See
`docs/jq-1.8-cli-options.md` for the complete classification.

`def` uses jq lexical scope and supports both lazy filter parameters (`f`) and
eager value parameters (`$value`), including recursive references. Calls are
resolved by name and arity before input is read and execute through bounded VM
frames. Repeated `-L DIR` options set explicit module lookup order. Without
them, the process CLI searches its default roots and loads a `~/.jq` startup
file when present. Prefix substitutions support `~/` and `$ORIGIN/`.
`include "name"` imports definitions in place, while `import "name" as alias`
exposes `alias::filter`. JSON data imports use the variable namespace, such as
`import "data" as $d; $d::d`.

Canonical paths must remain within configured roots. Module reads, counts, and
dependency cycles are bounded. `modulemeta` also accepts runtime-derived module
names and shares a bounded metadata cache. Embedded callers that deny filesystem
access cannot load implicit modules or startup files.

## Memory and limits

Ordinary document filters retain one decoded document. `--slurp` retains every
input document. Sorting, uniqueness, final reductions, and output-heavy
construction are blocking. A fold retains one immutable accumulator plus
bounded managed evaluation state; `foreach` can release extracted results as
each update completes. `--stream` is jq-compatible input projection for JSON,
JSON Lines, JSON sequences, and TOON. It turns decoder events into path/value
records that ordinary queries can transform or consume through `input` and
`inputs`. Eligible filters may use an event plan; other filters use ordinary
evaluation over the projected records.
Projection retains bounded decoder state, but a query such as `[inputs]` can
still collect every projected record. YAML remains document-at-a-time.

Limits are explicit: input/depth/token/line/lookahead bounds, VM steps and
result count, output bytes, and TOON preparation/spool ceilings. A resource
limit produces a classified diagnostic; it is never reported as a successful
query. SIGINT is cooperative and a closed downstream pipe is successful.

## Compatibility evidence and disparities

The manual campaign pins jq 1.8.1, the imported source documents, and the
original 518 cases, including 303 protected matches. It compares ordered JSON
results and process behavior, exact compact JSON bytes, and whether TOON
preserves the JSON execution contract. See the
[campaign instructions and review inventories](../tests/compatibility/README.md).
The older `coverage-v1.toon` campaign is historical evidence, not approval for
the current implementation or its former expected-difference labels.

The [current macOS report](tests/jq-manual/index.md)
records 905 cases: 900 exact matches, five reviewed safe-library disparities,
and zero unresolved failures. Compact JSON matches 871 of 876 applicable cases;
TOON preserves all 876 JSON execution contracts. The frozen macOS tq executable
is `target/release/tq-bind-final-AxfMRp`, SHA-256
`e9478b0ad46e951a1654efbe4d9d52440da49b8b57283754a0c86fb69f1d4f0c`.
All original protected matches remain exact. These results are macOS evidence
only and do not establish native Linux or Windows compatibility. The [final
Linux report](tests/comparison-x86-64-linux.md)
records 896 exact results and nine reviewed observations across the same
905-case inventory, with zero unreviewed failures. Compact JSON matches 867 of
876 applicable cases and TOON preserves all 876 JSON execution contracts. The
Linux completion report is green for the recorded x86_64 executable; these
campaign reports are checkpoints, not a claim that every remaining parity
requirement or release gate is complete. Its target-scoped approvals are not
waivers for other builds or platforms. Native
Windows execution is explicitly deferred until a runner is available, with
its tests retained and no passing claim.

Manual parity remains under implementation and review. Missing behavior,
timeouts, skips, crashes, and unexplained mismatches fail the gate. A specifically
reviewed safe-library disparity can satisfy the separate completion gate but
never becomes an exact match. The
[disparity register](jq-compatibility-disparities.md) records measured math,
regex, and platform restrictions and their reconsideration criteria.

Labels and breaks are implemented. The process CLI permits environment, clock,
local-timezone, and input-metadata access without extra allow flags; embedded
callers retain capability controls. See
[migration and security notes](jq-parity-migration.md) and
[regex, date, and platform compatibility](jq-regex-date-platform.md).
