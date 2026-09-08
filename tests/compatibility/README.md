# tq feature comparison

Audit evidence is stored in TOON. Case catalogs remain JSONL until their nested
data model is revised to make TOON token-efficient. See the
[fixture migration report](../../docs/test-fixture-savings.md) for storage rules,
lossless literal handling, and measured character and token savings.

## jq manual coverage

The [manual audit](reviews/manual-coverage.md) maps the jq 1.8 manual to
executable cases. The [source inventory](reviews/manual-source-examples.toon)
pins all 13 sections, 251 input/output examples, and 68 fenced snippets.
Section ledgers also account for prose examples and explain incomplete fragments.

Every section uses the corrected audit model: one case per executable example,
with source claims and exclusions in separate coverage notes. Multiple witnesses
use a `coverage_evidence` table of `note_id` and `case_id` relationships.
The [model and savings report](../../docs/test-fixture-savings.md#manual-ledger-model)
documents the schema; the [math example](reviews/manual-math-model.md) shows its
simplest form.

Run the coverage guards:

```sh
cargo test -p tq-test-support --test compatibility_manual
```

The separate reference check compares jq with the published manual outputs.
It requires a jq 1.8 reference executable. Two imported output strings lose a
space; the inventory preserves the published text and records the verified
reference output separately. The check sets `PAGER=less` for environment examples.

```sh
TQ_JQ=target/reference-build/jq/jq cargo test -p tq-test-support \
  --test compatibility_manual jq_reference -- --ignored
```

Execute the cases against jq and the current tq build:

```sh
cargo build -p tq-cli -p tq-test-support --bins
TQ_JQ=target/reference-build/jq/jq TQ_BIN=target/debug/tq \
  target/debug/tq-compat run --profile full --json target/manual-compatibility.json | tq -x
```

Use the full profile; smoke excludes jq-target cases. A covered example can
still fail compatibility. Manual cases keep tq enabled so unsupported features
and different results appear in the campaign report.

CLI adapters can set `env` for one child process, supply `trailing_args` after
the query, and set `omit_query` for commands such as `--from-file` or `--help`.
Set `expected.compare_stderr` for examples whose diagnostic bytes are observable
results, such as `debug` and `stderr`.

### JSON equivalence and output size

Use the [manual comparison](reviews/manual-comparison.md) for the current
compatibility verdicts and per-example character counts. The [TOON evidence](reviews/manual-comparison.toon)
retains executable identities, actual stdout, process outcomes, and reasons.

```sh
TQ_JQ=target/reference-build/jq/jq TQ_BIN=target/debug/tq \
  target/debug/tq-manual-compare tests/compatibility/reviews/manual-comparison.toon | tq -x
```

This writes both TOON and Markdown reports. Structured examples run with explicit
`-o json` and `-o toon`; raw CLI examples keep their original arguments.
Equivalent JSON means matching ordered values regardless of whitespace or object
key order. Process and expected-error contracts are checked separately from
successful JSON equivalence. The audit command reports failures but exits nonzero
only when it cannot complete the audit, so it is not a compatibility CI gate.

Verdicts are `match`, `expected-difference`, `failure`, `reference-discrepancy`,
or `unverified`. An expected difference must have a documented policy and matching
observed behavior. Missing features are failures unless an explicit policy excludes
them; a failed encoder never earns size savings.

Character counts use Unicode scalar values, including record separators and
trailing newlines. Totals include only successful equivalent jq, JSON, and TOON
results. The baseline is default JSON output, not `-c`; negative savings mean TOON
is longer. Character savings do not establish tokenizer savings, and the manual's
small examples are not representative workload benchmarks.

## Feature comparison

Most jq filters for selection and transformation work as written. Check this
table before moving a filter that uses less common language or CLI features.

| Feature | jq or yq example | tq |
| --- | --- | --- |
| Select and filter | `.items[] \| select(.enabled) \| .name` | Supported |
| Build output | `[.items[] \| {id, name}]` | Supported |
| Map, sort, unique | `.items \| map(.name) \| unique \| sort` | Supported |
| Update | `.items[] \|= .count += 1` | Supported |
| JSON, YAML, TOON input | `tq --input-format yaml '.items[]' file.yaml` | Supported |
| Common CLI modes | `-n`, `-R`, `-s`, `-r`, `-j`, `--stream`, `-e` | Supported |
| Output controls | `-a`, `-S`, `-C`, `-M`, `--raw-output0`, `--unbuffered` | Supported/adapted |
| CLI values | `--arg`, `--argjson`, `--rawfile`, `--slurpfile`, `--args`, `--jsonargs` | Supported |
| User filters and modules | `def f: .x; f`, `include "lib"` | Supported with explicit `-L` roots |
| Folds | `reduce .items[] as $x (0; . + $x)` | Supported |
| Recursive descent | `.. \| scalars` | Supported |
| Recursive built-ins | `recurse(.[]?)`, `walk(.)` | Supported |
| Lexical early exit | `label $out \| ..., break $out` | Supported |
| Interpolation | `"name=\(.name)"` | Supported |
| Regular expressions | `test("^prod-")` | Supported with documented engine differences |
| UTC date/time | `fromdateiso8601`, `gmtime` | Supported |
| Local time and environment | `now`, `env.HOME` | Supported with explicit capability flags |

## Migration differences

- tq structured output defaults to TOON Text Sequence. Use
  `--output-format json` for JSON consumers.
- tq has documented digit, exponent, and index limits for large numbers.
- jq JSON formatting switches require `--output-format json` because tq's
  structured default is TOON Text Sequence.
- YAML output uses exact-number-preserving YAML 1.2 flow syntax.
- `-L` is repeatable and searches only explicit, confined module roots.
- `env` requires `--allow-environment`; clock, timezone, and input metadata
  require `--allow-platform`.

The generated report records current capability counts and raw-byte
differences. Non-finite result built-ins remain deferred.

The complete evidence is in [coverage-v1.toon](reviews/coverage-v1.toon). The
older [reference candidate](baselines/jq-yq-mvp-v1.toon) compares jq and yq
only.
