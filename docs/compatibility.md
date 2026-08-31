# jq compatibility

`tq` follows jq 1.8.x semantics for the features it supports. That includes
navigation, pipes and comma generators, arrays and ordered objects, variables,
conditionals, operators, common built-ins, optional access, `try/catch`, path
updates, user filters, and modules from explicit roots. Stateful `reduce` and
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
record. Sequence framing may leave that final record incomplete; unframed output
publishes nothing. Strict TOON also rejects duplicate paths. Safe key folding,
sorted-key output, explicit jq stream input, slurp, raw/joined output,
proxy-on-error, non-TOON output, and non-identity filters use the existing plans.

## Input and output

The format detector reads a bounded prefix. It prefers canonical TOON. A JSON
object or array opener selects JSON before YAML, while YAML document,
directive, and root-sequence markers select YAML. `.jsonl` and `.ndjson` files
select strict one-value-per-line JSON Lines input. Use
`--input-format toon|yaml|json|jsonl` to select exactly one parser; `ndjson` is
an alias for `jsonl`.

`-x/--proxy-on-error` retains each bounded structured source before evaluation.
If its parser rejects the source, `tq` writes the original bytes unchanged and
treats that source as successful. It does not mask resource, I/O, query,
runtime, or output failures. Sources are independent except under `--slurp`,
where any parse rejection proxies the complete ordered source set.
`--stream-errors` is incompatible because it assigns a different meaning to
parse failures.

Structured output is a TOON Text Sequence. Each result is `RS`, canonical TOON,
and `LF`. This differs from jq's newline-delimited JSON. The framing separates
multiple results and preserves complete records before a late error. Select
`--output-format json` for JSON output, `--output-format jsonl` for compact
LF-terminated JSON Lines, `--output-format yaml` for exact-number-preserving
YAML 1.2 output, `-r` for raw strings, `-j` to join raw outputs, or `--unframed`
for exactly one TOON value. `--unframed` rejects zero or multiple results
instead of choosing one silently.

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
frames. `-L DIR` adds an explicit module root; repeat it to establish lookup
order. `include "name"` imports definitions in place, while `import "name" as
alias` exposes `alias::filter`. Canonical paths must remain within a configured
root. Module count, bytes, cycles, paths, metadata, and SHA-256 identities are
bounded or reported during compilation.

## Memory and limits

Ordinary document filters retain one decoded document. `--slurp` retains every
input document. Sorting, uniqueness, final reductions, and output-heavy
construction are blocking. A fold retains one immutable accumulator plus
bounded managed evaluation state; `foreach` can release extracted results as
each update completes. `--stream` has a separate event plan for JSON and TOON
and is the bounded-memory large-input mode; YAML remains document-at-a-time.

Limits are explicit: input/depth/token/line/lookahead bounds, VM steps and
result count, output bytes, and TOON preparation/spool ceilings. A resource
limit produces a classified diagnostic; it is never reported as a successful
query. SIGINT is cooperative and a closed downstream pipe is successful.

## Reviewed differences

The current jq 1.8.2, yq, and tq report is
`tests/compatibility/reviews/coverage-v1.json`. Its jq/tq difference allowlist is
small:

| Case | Difference | Reason |
| --- | --- | --- |
| `cli.sequence-framing` | raw bytes | tq's default is TOON Text Sequence framing |
| `numeric.policy-digits-over` | result/exit/error | bounded numeric digit envelope |
| `numeric.policy-exponent-over` | result/exit/error | bounded exponent expansion envelope |
| `numeric.policy-index-over` | result/exit/error | bounded index envelope |
| `date.range-error` | result/exit/error | portable UTC support stops at years 0000 and 9999 |
| `regex.unsupported-lookaround` | result/exit/error | the linear-time regex engine rejects Oniguruma look-around |

Features outside the MVP report a stable unsupported or deferred status.
Labels and breaks are deferred. Regex and UTC date built-ins work without extra
permissions. Environment, clock, local-timezone, and input-metadata access need
capability flags. See [regex, date, and platform compatibility](jq-regex-date-platform.md)
for engine and release-host differences.
