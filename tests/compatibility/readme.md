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
expanded manual execution campaign now contains 952 cases; this is a separate denominator
from the source examples and callable signatures. The arity case checks advertised availability, not function
semantics. New witnesses supplement the original 518 cases; they cannot replace
them or relax the strict gate's 303 original exact-match requirements. The archived
[behavior audit](../../openspec/changes/archive/2026-10-06-achieve-jq-manual-parity/source-behavior-audit.md)
preserves the source audit beyond published examples. The
[closeout evidence](reviews/parity-closeout.toon) distinguishes completed implementation
coverage from unresolved platform contracts transferred to #69/#70.
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
It requires the pinned jq 1.8.2 reference executable. Two imported output strings lose a
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
TQ_EMBEDDED_HOST=target/debug/tq-compat-embedded \
  target/debug/tq-compat run --profile full --json target/manual-compatibility.json
```

Use the full profile; smoke excludes jq-target cases. A covered example can
still fail compatibility. Manual cases keep tq enabled so unsupported features
and different results appear in the campaign report.

CLI adapters can set `env` for one child process, supply `trailing_args` after
the query, and set `omit_query` for commands such as `--from-file` or `--help`.
Set `expected.compare_stderr` for examples whose diagnostic bytes are observable
results, such as `debug` and `stderr`.

### Embedded capability-denial observations

The shared full campaign keeps process CLI admission separate from embedded
policy denial. `adapters.tq.execution_mode` is a closed enum: `process` (the
omitted-field default), `embedded-deny-environment`, or
`embedded-deny-platform`. Embedded modes are tq-only error contracts with
`runtime-policy`, stdin input, and the original query; they cannot override the
query, omit it, or supply authored CLI/trailing arguments. JSON/YAML/TOON input
expansion and strict TOON/JSON companion matching remain unchanged.

Build `tq-compat-embedded` together with the campaign binaries using `--bins`.
It is a test-support subprocess host, not a new public tq option. It calls
`parse_args_with_policy` and `run_with_io` with the selected authority explicitly
false, admitting the other authority. The parent retains normal subprocess
isolation, environment overrides, deadlines, and byte capture. Use
`TQ_EMBEDDED_HOST` to select an immutable host explicitly; otherwise discovery
checks beside `TQ_BIN`, then the repository's release/debug build directories.
It never substitutes the process CLI or searches PATH for an embedded host.
An invalid explicit override or missing required host fails the campaign.

Each embedded case's `tq_execution` records its mode and actual host canonical
path, version, byte count, and SHA-256. This identity replaces the top-level
`tools` tq CLI identity **for that case's tq observations only**. Keep host,
runner, and CLI builds together; do not claim that an embedded host tests an
arbitrarily supplied `TQ_BIN` executable. Capture immutable executable hashes
before/after release evidence runs.

The shared runner enforces tq's declared error class even when no reference
adapter applies. Embedded denial additionally requires runtime exit 5, empty
stdout/results, and a single redacted authority-specific diagnostic without
internal ambient names or the campaign sentinel. Semantic contract violations
are recorded in per-case `contract_failures`, preserving actual bytes/statuses
rather than recategorizing them as malformed output. They fail the command and
cannot be counted as supported capabilities. These report fields default to
absent/empty when reading historical schema-v1 reports; existing observation
fields are unchanged. Consumers must inspect `contract_failures` as well as
`semantic_diffs` and harness errors.

A shared campaign can still exit zero with `observed-differences`; it is not the
strict manual acceptance gate. Native-platform differences and calibrated
performance acceptance remain separate evidence, not waived by this repair.
Do not print ambient environment objects when reproducing denial failures;
retain raw evidence only in ignored campaign storage.

The macOS repair run on base `3e0dedb` plus the harness changes is retained at
`target/compatibility/embedded-denial-closeout-3e0dedb/`: `full.json`, `full.log`,
and before/after executable hash manifests. It used the reviewed jq 1.8.2
reference and hash-checked yq 4.53.2. Both denial cases passed all three input
representations with zero output, runtime-policy exit 5, and the embedded host
identity. The 1,220-case campaign had 4,804 executed observations and no harness
errors, but still failed: 93 cases had pairwise differences and four cases had
12 declared-error-contract violations. At that point, the four stale contracts were
`fold.foreach.partial-error` and `interpolation.partial-error` (declared
`runtime-explicit`, observed `runtime-type-path`), `regex.unsupported-lookaround`
(declared unsupported error, observed success), and `update.invalid-lvalue`
(declared compile error, observed runtime error). This is repair evidence, not
strict all-cases acceptance or a renewal of historical approvals.

#### Independently verified catalog contract corrections

The subsequent catalog closeout retains all four filters, inputs, and stable
IDs. Pinned jq 1.8.2 and real tq executions independently establish:

| Case | Required behavior |
| --- | --- |
| `fold.foreach.partial-error` | Emit `1`, then `3`, then raise `"boom"`; runtime exit 5 |
| `interpolation.partial-error` | Emit `"before=1"`, then raise `"boom"`; runtime exit 5 |
| `regex.unsupported-lookaround` | `test("(?=a)")` returns `true` on `"a"` and `"ba"`, `false` on `"b"`; successful result-sequence contract |
| `update.invalid-lvalue` | `(1 + 2) = 3` fails at runtime with exit 5 and no stdout; the error is catchable |

The three failures use the existing `runtime-type-path` normalized family.
That family includes explicit `error(value)` as well as type/path failures;
these two `error("boom")` filters are not reinterpreted as literal type errors.
Regression tests require their exact output prefixes, diagnostics, and caught
`"boom"` values, not just an arbitrary nonzero exit. The invalid-path regression
also distinguishes runtime catchability from compile failure. jq and tq retain
their different invalid-path diagnostic text; no byte-comparison relaxation or
new disparity approval follows from correcting the error class.

The historical `regex.unsupported-lookaround` ID stays unchanged for evidence
provenance, but its title, capability tags, and adapter note now describe
supported positive lookahead. Its baseline becomes **required**, not
informative. Positive, negative, and non-anchored search witnesses guard against
a constant boolean or a silently removed check. Historical review summaries
remain historical and are not rewritten.

Closeout evidence belongs under
`target/compatibility/catalog-contract-closeout-3e0dedb/`, separate from the
original failing run. It contains independent jq/tq probes, frozen-executable
identities, a hashed current-source snapshot, full campaign observations, and a
redacted summary. Pairwise differences remain observable even after declared
contract and harness failures are eliminated; this does not grant strict
manual or calibrated performance acceptance.

Focused regression checks:

```sh
cargo test --locked -p tq-test-support --test compatibility_embedded \
  --test compatibility_catalog_contracts --test compatibility_cases \
  --test compatibility_fake_executables --test compatibility_schema \
  --test compatibility_discovery --test compatibility_reporting
TQ_JQ=target/reference-build/jq/jq cargo test --locked -p tq-test-support \
  --test compatibility_catalog_contracts jq_reference -- --ignored
```

### JSON equivalence and output size

The [jq manual report](../../docs/tests/jq-manual/index.md) records the latest
macOS comparison against pinned jq 1.8.2, including compatibility verdicts,
per-example outputs, and tokenizer counts. It is not an all-platform completion report. Generate fresh full
TOON evidence under ignored `target/` storage to retain executable identities,
actual stdout, process outcomes, and reasons without committing campaign dumps.

```sh
TQ_JQ=target/reference-build/jq/jq TQ_BIN=target/debug/tq \
  target/debug/tq-manual-compare \
    --markdown-dir target/jq-manual-review \
    target/manual-comparison.toon | tq -x
```

This separate command writes the TOON evidence and one Markdown token report per
case collection under `target/jq-manual-review`. The index shows unique-case
totals; section pages can share cases referenced by more than one collection.
Use `--render-only` with the saved TOON evidence to regenerate pages without
running the reference tools. Human-readable source reviews are in
[docs/tests/jq-manual](../../docs/tests/jq-manual/index.md); their machine-readable
collections are in `reviews/jq-manual`.
Only promote reviewed campaign results to `docs/tests/jq-manual`; retain the
historical labels until current executable identities and completion gates
have been verified.

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

Release jobs use the official jq 1.8.2 `jq-linux-amd64` and
`jq-macos-arm64` artifacts with checked-in SHA-256 pins. The Linux artifact
is statically linked, so it does not require the former Red Hat runtime
libraries on the hosted Ubuntu runner. Explicit artifact overrides must still
pass both download-digest and executable/build-identity checks. A reference
change does not renew disparity approvals: compare fresh observations and
review each difference before updating an approval.

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
rejects every disparity. See the [numbered difference list](../../docs/tests/jq-manual/coverage.md#differences-by-test)
for current observations and their practical impact.

The [macOS registry](reviews/disparities-aarch64-macos.toon) and
[Linux registry](reviews/disparities-x86_64-linux.toon) retain target-scoped
approval records. Each record is tied to its recorded executable build, not a
standing waiver for future builds or other targets.

Revalidate observations before replacing their identities.
Never copy an old approval onto an unexamined mismatch.

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
