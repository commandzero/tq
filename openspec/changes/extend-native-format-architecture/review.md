# Implementation review and verification

Reviewed the implementation against documentation baseline
`ea81b21e1ea8d8e711dd53cb4ce79aff1ea037f2`, including new files.
The standards and spec reviews ran independently in parallel, without running
builds or tests during benchmark measurements. Both reviewers rechecked repairs.

## Standards

No outstanding concrete finding remains. Repairs preserve failed benchmark
warmups, record probe-script identity, validate fixture object-key order, and
align JSON-sequence benchmark output framing. The direct codec-consumer adapter
retains allocation-avoiding callbacks and classifies typed event errors
separately from the existing trait's text-hook failures.

The reviewers noted possible duplication in callback stop/error adapters.
This is a maintainability judgment, not a documented-standard violation.
The adapters currently serve distinct observation, selected-record, and direct
codec consumer contracts; no speculative unification was added.

## Spec

No outstanding concrete implementation finding remains. Regression tests cover
the repaired empty auto-detected proxy sequence and stale paths after consecutive
recoverable parse failures. Transcode decoding now enters through committed
native input rather than selecting codecs in the runner.

At the original implementation checkpoint, performance goals were not fully met.
Paired complete-Document JSON timing
increases are 15.74% to 21.91%, above the less-than-10% goal. No paired refactor
increase remains above 25%. New-format soft reference misses are explicit:
large JSON sequence input is 23.79x jq time and 3.44x jq RSS; CSV and TSV are
about 2.5x yq time with about 0.023x yq RSS. These are soft-objective misses,
not correctness failures or passing performance claims.

## Verification

- `cargo fmt --all --check`: passed.
- `cargo check --workspace --all-targets`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: exited
  successfully. The existing `clippy::assert_is_empty` allowance produces an
  unknown-lint warning on Homebrew rustc 1.98.0; no new code warning remains.
- `cargo test --workspace`: 446 passed across 51 suites, run once after repairs.
- `openspec validate --all --strict`: 19 passed.
- Final native compatibility campaign: 33 cases; all 23 jq/tq cases agree.
  Seven deliberate yq differences are covered by the versioned review.
- Native-input sanitizer fuzzing: 444,173 executions, no crash or timeout.
- Refactor matrix: all 30 correctness gates and seven RSS samples per row passed;
  every target miss received 21 alternating baseline/candidate pairs.
- Native reference matrix: all 12 rows passed correctness, with 30 small or
  seven large measured samples and two warmups per row.

All benchmark commands ran elevated outside the sandbox with `/usr/bin/time -l`.
The measured candidate SHA-256 is
`9ab2c14a6a458d39fde97f60cd2f72862fbada453be7a8b981fa57b6ad1e0227`.
Full commands, hashes, host metadata, individual samples, preserved failed
attempts, and the reviewed performance report are in the separate `tq-benchmarks`
checkout, `2026-09-05-native-format-architecture.md` and its `.work/` references.

Outstanding concrete findings: Standards 0; Spec 0. Performance target misses
remain recorded separately above.

## Targeted sequence performance follow-up

The user accepted the remaining results and requested improvements to CSV time,
TSV time, JSON-sequence time and memory, `large-toon-seq-extract` time, and
`large-jsonl-extract` time. This follow-up compares against implementation commit
`0f16753`, not the earlier pre-refactor baseline.

Process sampling showed per-Document worker creation, channel waits, and joins
dominating extraction. The ordinary driver now uses synchronous `drain_results`.
The VM keeps pull-compatible trivial-root shortcuts inside that method while
leaving the budgeted `for_each_result` interface and automatic callers unchanged.
This distinction preserves direct-root report observations and shared automatic
suffix step limits. Normal execution no longer retains per-Document report
observations; requesting a report preserves the existing entries and therefore
still requires storage proportional to Document count.

The first synchronous checkpoint exposed an 18.26% `--stream .` time regression.
A CLI report regression test caught the changed trivial-root observations.
An attempted global shortcut then failed the existing hybrid shared-step-limit
test. Separating ordinary draining from budgeted automatic evaluation repaired
both contracts without weakening either assertion.

Final correctness verification:

- Native CLI tests: 28 passed, including four added regression tests.
- `cargo check --workspace --all-targets`: passed.
- Formatting and strict Clippy: passed with the same pre-existing unknown-lint
  warning recorded above.
- Complete workspace suite: 450 passed across 51 suites after repairing the
  earlier failed shared-step-limit check.
- Strict OpenSpec validation: all 19 passed.
- Native compatibility replay: all 23 jq/tq cases agree; the same seven reviewed
  yq differences remain among 33 cases.
- Independent standards and spec reviews: no outstanding concrete findings.
  Benchmark review repairs preserve failed validations and warmups, record
  effective thresholds, and reject nonfinite or invalid improvement fractions.
  Controlled negative checks verified failed-validation evidence and NaN rejection.

Benchmark evidence and final measurements are recorded in the separate
`tq-benchmarks` checkout, `2026-09-05-native-sequence-performance.md`, under
`.work/native-sequence-perf-20260905/`. Earlier attempts remain as checkpoints.

Final seven-pair results for 131,072 Documents, using elevated `/usr/bin/time -l`
for each sample and checking exact output bytes:

| Workload | Before ms | After ms | Time reduction | Peak RSS reduction |
| --- | ---: | ---: | ---: | ---: |
| CSV | 4232.47 | 136.69 | 96.77% | 67.37% |
| TSV | 4209.27 | 136.59 | 96.76% | 67.32% |
| JSON sequence | 4336.72 | 207.02 | 95.23% | 67.23% |
| TOON sequence | 4322.08 | 194.62 | 95.50% | 65.41% |
| JSONL | 4254.40 | 155.50 | 96.35% | 67.02% |

All five pass the focused improvement gate. The measured final binary SHA-256 is
`c776e9c99e2d5a58a8e16e9f30b05a3340756b6265b7587e810c4d03f19e1d90`.

Seven paired checks on the original frozen fixtures confirm
`large-jsonl-extract` time down 96.41% and `large-toon-seq-extract` down 95.59%.
The repaired `large-json-events` control is 0.53% faster with 0.39% more RSS.
All ten original-workload probes remain below 10% growth in either metric.

The final pinned reference campaign passes all 12 correctness gates and every
soft time/RSS objective. Large JSON-sequence extraction is 1.330x jq time and
1.364x jq peak RSS. CSV and TSV are 0.103x and 0.105x yq time, with less than
0.009x yq RSS. These supersede the new-format reference misses at the original
implementation checkpoint. The original 4,096-Document reproduction now passes
at 13.0 ms, below its unchanged 80 ms diagnostic threshold.
