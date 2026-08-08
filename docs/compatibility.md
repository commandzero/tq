# jq compatibility

`tq` targets jq 1.8.x semantics for the implemented MVP surface. Filters use
the ordinary jq command shape: identity/literals, `.field`, `.[expr]`, indexes
and slices, `[]`, pipes and comma generators, arrays and ordered objects,
variables, conditionals, operators, core collection/selection/conversion/range/
ordering/aggregation built-ins, optional access, `try/catch`, and path updates.
Run `tq --help` for the complete current switch set and `tq --explain-json
FILTER` for its plan and retention classification.

## Input and output

Input detection tries TOON, then YAML, then JSON for each source. Use
`--input-format toon|yaml|json` to select one parser. JSON is valid YAML, so
this option is the only way to require JSON parsing.

Structured output is a TOON Text Sequence: each result is `RS`, canonical TOON,
and `LF`. This deliberately differs from jq's newline-delimited JSON and keeps
multiple results and a late error unambiguous. Select `--output-format json`
for JSON output, `--output-format yaml` for exact-number-preserving YAML 1.2
flow output, `-r` for raw strings, `-j` to join raw outputs, or
`--unframed` for exactly one TOON value. `--unframed` rejects zero or multiple
results instead of choosing one silently.

The extended jq-shaped CLI supports short clusters plus `--raw-output0`,
`-a/--ascii-output`, `-S/--sort-keys`, explicit color/monochrome output,
`--tab`, reviewed `--indent`, and `--unbuffered`. JSON-specific switches require
explicit JSON output. `--arg`, `--argjson`, and `--argtoon` populate both direct
variables and `$ARGS.named`; `--args`/`--jsonargs` populate `$ARGS.positional`;
and `--rawfile`/`--slurpfile` use the configured per-source byte limit. See
`docs/jq-1.8-cli-options.md` for the complete classification.

## Memory and limits

Ordinary document filters retain one decoded document. `--slurp` retains every
input document. Sorting, uniqueness, reduction, and output-heavy construction
are blocking. `--stream` has a separate event plan for JSON and TOON and is the
bounded-memory large-input mode; YAML remains document-at-a-time. Automatic
conversion of an arbitrary ordinary filter into an event plan is deferred.

Limits are explicit: input/depth/token/line/lookahead bounds, VM steps and
result count, output bytes, and TOON preparation/spool ceilings. A resource
limit produces a classified diagnostic; it is never reported as a successful
query. SIGINT is cooperative and a closed downstream pipe is successful.

## Reviewed differences

The current full jq 1.8.2/yq/tq report is
`tests/compatibility/reviews/coverage-v1.json`. Its jq/tq difference allowlist is
intentionally small:

| Case | Difference | Reason |
| --- | --- | --- |
| `cli.sequence-framing` | raw bytes | tq's default is TOON Text Sequence framing |
| `numeric.policy-digits-over` | result/exit/error | bounded numeric digit envelope |
| `numeric.policy-exponent-over` | result/exit/error | bounded exponent expansion envelope |
| `numeric.policy-index-over` | result/exit/error | bounded index envelope |

Features outside the MVP have stable unsupported or deferred status. These
features include user functions and modules, `reduce` and `foreach`, labels and
breaks, recursive descent, interpolation, regex, date, platform, and
environment built-ins, automatic stream planning, and module loading. Each
feature has a separate OpenSpec change under `openspec/changes/`.
