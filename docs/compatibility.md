# jq compatibility

`tq` targets jq 1.8.x semantics for the implemented MVP surface. Filters use
the ordinary jq command shape: identity/literals, `.field`, `.[expr]`, indexes
and slices, `[]`, pipes and comma generators, arrays and ordered objects,
variables, conditionals, operators, core collection/selection/conversion/range/
ordering/aggregation built-ins, optional access, `try/catch`, path updates,
parameterized user filters, and explicit-root modules.
Stateful `reduce` and `foreach` folds preserve jq generator order, accumulator
scope, update multiplicity, intermediate extraction, and partial output before
a later error.
Run `tq --help` for the complete current switch set and `tq --explain-json
FILTER` for its plan and retention classification.

## Input and output

Each source uses bounded syntax detection. Canonical TOON remains preferred,
JSON object and array openers commit to the JSON decoder before YAML, and YAML
document/directive/root-sequence markers commit to YAML. Use
`--input-format toon|yaml|json` to select exactly one parser.

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

The current full jq 1.8.2/yq/tq report is
`tests/compatibility/reviews/coverage-v1.json`. Its jq/tq difference allowlist is
intentionally small:

| Case | Difference | Reason |
| --- | --- | --- |
| `cli.sequence-framing` | raw bytes | tq's default is TOON Text Sequence framing |
| `numeric.policy-digits-over` | result/exit/error | bounded numeric digit envelope |
| `numeric.policy-exponent-over` | result/exit/error | bounded exponent expansion envelope |
| `numeric.policy-index-over` | result/exit/error | bounded index envelope |
| `date.range-error` | result/exit/error | portable UTC support is intentionally bounded to years 0000 through 9999 |
| `regex.unsupported-lookaround` | result/exit/error | the linear-time regex engine rejects Oniguruma look-around |

Features outside the MVP have stable unsupported or deferred status. Labels
and breaks remain deferred. Regex and UTC date built-ins are supported; ambient
environment, clock, local-timezone, and input-metadata access requires explicit
capability flags. Engine and release-host differences are documented in
`docs/jq-regex-date-platform.md`.
