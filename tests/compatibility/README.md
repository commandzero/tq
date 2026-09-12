# tq feature comparison

Audit evidence is stored in TOON. Case catalogs remain JSONL until their nested
data model is revised to make TOON token-efficient. See the
[fixture migration report](../../docs/test-fixture-savings.md) for storage rules,
lossless literal handling, and measured character and token savings.

## jq manual coverage

The [manual audit](../../docs/tests/jq-manual/coverage.md) maps the jq 1.8 manual to
executable cases. The [source inventory](reviews/jq-manual/source-examples.toon)
pins all 13 sections, 251 input/output examples, and 68 fenced snippets.
Section ledgers also account for prose examples and explain incomplete fragments.

The initial expansion adds seventeen [prose-boundary witnesses](reviews/jq-manual/prose-boundaries.toon)
and a [220-signature arity inventory](reviews/jq-manual/arity-inventory.toon), plus
eight [math boundary witnesses](reviews/jq-manual/math-boundaries.toon), eight
[path boundary witnesses](reviews/jq-manual/path-boundaries.toon), and one
[regex boundary witness](reviews/jq-manual/regex-boundaries.toon), for
554 required cases before subsequent semantic and composition witnesses. The
expanded execution campaign now contains 905 cases; this is a separate denominator
from the source examples and callable signatures. The arity case checks advertised availability, not function
semantics. New witnesses supplement the original 518 cases; they cannot replace
them or relax the 303 original exact-match requirements. The active
[behavior audit](../../openspec/changes/achieve-jq-manual-parity/source-behavior-audit.md)
tracks evidence still needed beyond the published examples.
The source-linked [semantic completeness index](reviews/jq-manual/completeness.toon)
maps all 220 callable signatures to a bounded public-VM execution witness and
keeps exact value/error/empty/flag/process verdicts in their owning ledgers.

Every section uses the corrected audit model: one case per executable example,
with source claims and exclusions in separate coverage notes. Multiple witnesses
use a `coverage_evidence` table of `note_id` and `case_id` relationships.
The [model and savings report](../../docs/test-fixture-savings.md#manual-ledger-model)
documents the schema; the [math example](../../docs/tests/jq-manual/math-model.md) shows its
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

Use the [jq manual report](../../docs/tests/jq-manual/index.md) for the current
compatibility verdicts, per-example outputs, and tokenizer counts. Generate full
TOON evidence under ignored `target/` storage to retain executable identities,
actual stdout, process outcomes, and reasons without committing campaign dumps.

```sh
TQ_JQ=target/reference-build/jq/jq TQ_BIN=target/debug/tq \
  target/debug/tq-manual-compare \
    --markdown-dir docs/tests/jq-manual \
    target/manual-comparison.toon | tq -x
```

This separate command writes the TOON evidence and one Markdown token report per
case collection under `docs/tests/jq-manual`. The index shows unique-case
totals; section pages can share cases referenced by more than one collection.
Use `--render-only` with the saved TOON evidence to regenerate pages without
running the reference tools. Human-readable source reviews are in
[docs/tests/jq-manual](../../docs/tests/jq-manual/index.md); their machine-readable
collections are in `reviews/jq-manual`.

`reviews/` stores versioned test metadata, not campaign output: source-to-case
mappings, reference pins, approved disparity expectations, and small historical
summaries used by regression tests. Full comparison and execution dumps belong
under ignored `target/` storage. Reporting tests use explicitly synthetic,
small observations; they do not certify measured jq compatibility. Publish
user-facing results in `docs/tests/` after running the separate comparison
campaign.
 Structured examples run with explicit
`-o json` and TOON output, plus an independent jq/tq `-c` comparison. Default TOON emits LF-terminated values for any result count. Only original
sequence adapters use `--seq`. Independently captured JSON results resolve TOON value boundaries; decoding
must match each value and consume the complete default stdout.
Raw CLI examples keep their original arguments. Compact JSON requires exact stdout bytes, so a
literal representation change cannot pass through semantic normalization. TOON
must preserve the JSON execution's ordered results and process contract. Explicit
stderr payloads remain observable in all three campaigns. Their counts are
reported separately; a semantic match cannot excuse an encoding failure.
Equivalent JSON means matching ordered values regardless of whitespace or object
key order. Process and expected-error contracts are checked separately from
successful JSON equivalence. Tokenizer counts include the selected TOON framing and
trailing newlines. The strict gate exits nonzero unless every admitted
case is an executed match with complete reference and source coverage; mismatches,
expected differences, reference discrepancies, skips, missing references, timeouts,
crashes, and normalization errors all fail the command. The frozen gap closure
inventory is [here](reviews/jq-manual/gap-inventory.toon). The
[reference pin](reviews/jq-manual/reference-pin.toon) freezes the imported inventory,
all fourteen source documents, the original 518 case IDs and 303 matching cases,
and the jq binary and build configuration for each verified host. The command
rejects changed or unverified reference builds before executing cases. Reference
installation paths are not pinned. Passing this gate alone
does not establish complete manual compatibility.

Ordinary fixture runs do not need the companion repository. To audit its source
files against the pins too, add `--source-root /path/to/tq-benchmarks`. Changed or
missing sections fail that audit before case execution. New source or reference
builds require a reviewed pin update; they are never accepted automatically.

Fixture adapters use `env` for literal environment values and `env_paths` for
repository-relative paths such as a controlled `HOME`. The runner resolves these
paths per checkout and rejects missing paths, escapes, and conflicting values.
It changes only the child process environment, not the developer's environment.

The relocation check copies only committed fixtures and the compatibility corpus
to a temporary checkout, then executes every case with no companion manual:

```sh
rtk cargo test -p tq-test-support --test manual_portability -- --ignored --test-threads=1
```

Current verdicts are `match`, `disparity`, `failure`, `reference-discrepancy`,
or `unverified`. Historical reports may contain `expected-difference`; that label
does not pass either gate. Missing features remain failures, and a failed encoder
never earns size savings.

The optional completion gate accepts a TOON array of reviewed disparity records:

The executable-origin fixture expects tq under `target/debug/` or
`target/release/` and jq under `target/reference-build/jq/`. When verifying an
immutable candidate built with another Cargo target directory, copy it into
one of the expected tq directories under a distinct filename first. Changing
the executable directory changes `$ORIGIN`; do not treat a missing fixture
caused by that layout change as a language-semantic regression.

```bash
set -o pipefail
TQ_JQ=target/reference-build/jq/jq TQ_BIN=target/debug/tq \
  target/debug/tq-manual-compare --completion APPROVALS.toon REPORT.toon | tq -x
```

Each approval must match the case fingerprint, executable identities, contract,
and observed outputs. Unknown, stale, duplicate, and unlisted approvals fail.
Approvals cannot excuse regressions outside the frozen gap inventory. Exact
matches and disparities are counted separately; the default exact gate still
rejects every disparity. See the [disparity policy](../../docs/jq-compatibility-disparities.md)
for evidence and reconsideration requirements.

The [macOS registry](reviews/disparities-aarch64-macos.toon) contains five
reviewed observations renewed against the final macOS completion executable.
It is tied to that recorded executable build, not a standing waiver for future
builds or other targets. Revalidate observations before replacing their
identities; never copy an old approval onto an unexamined mismatch. The [Linux
completion report](../../docs/tests/comparison-x86-64-linux.md) records 896 exact
results and nine reviewed observations across the same 905-case inventory, with
compact JSON 867/876 and TOON 876/876. This is a completion-campaign
checkpoint, not a claim that every remaining parity or release gate is complete.
Its nine target-scoped approvals are bound to the recorded x86_64 executable;
they do not waive another build or platform.

Token counts use the `o200k_base` and `cl100k_base` encodings over complete stdout,
including trailing newlines. The token report's `Diff` value is TOON tokens minus
JSON tokens. `%` is the signed percent difference `(TOON - JSON) / JSON`, so
savings are negative and growth is positive. Size totals include only
successful default TOON outputs; sequence framing remains visible
in the cases but is excluded from the totals. The baseline is default JSON output,
not `-c`. The manual's small examples are not representative workload benchmarks.

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
| User filters and modules | `def f: .x; f`, `include "lib"` | Supported with default or explicit `-L` roots |
| Folds | `reduce .items[] as $x (0; . + $x)` | Supported |
| Recursive descent | `.. \| scalars` | Supported |
| Recursive built-ins | `recurse(.[]?)`, `walk(.)` | Supported |
| Lexical early exit | `label $out \| ..., break $out` | Supported |
| Interpolation | `"name=\(.name)"` | Supported |
| Regular expressions | `test("^prod-")` | Supported with documented engine differences |
| UTC date/time | `fromdateiso8601`, `gmtime` | Supported |
| Local time and environment | `now`, `env.HOME` | Enabled by default in the process CLI; embedded capability controls apply |

## Migration differences

- tq structured output defaults to LF-terminated TOON values. Use `-o toon-seq`
  for explicit record boundaries, `--unframed` for exactly one document, and
  `--output-format json` for JSON consumers.
- `--seq` reads JSON sequences and writes TOON sequences by default;
  `--seq -o json` matches jq's sequence output, and `--seq -c` makes it compact.
- tq has documented digit, exponent, and index limits for large numbers.
- `-c` selects compact JSON directly; `--output-format json` selects pretty JSON.
- YAML output uses exact-number-preserving YAML 1.2 flow syntax.
- `-L` is repeatable and replaces the default search roots with explicit,
  confined module roots.
- The process CLI enables environment, clock, timezone, and input metadata.
  Embedded callers retain explicit capability controls.

The generated report records current capability counts and raw-byte
differences. Runtime non-finite values project to valid JSON and TOON values.

The [historical coverage summary](reviews/coverage-summary.toon) retains the
original case inventory, jq-target differences, and tq error classifications
used by regression tests. Full historical process dumps are not source fixtures. The
older [reference candidate](baselines/jq-yq-mvp-v1.toon) compares jq and yq
only.
