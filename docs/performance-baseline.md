---
type: Report
title: Performance review policy
description: Benchmark evidence, regression limits, and archive requirements.
generated: { by: codex/gpt-6-astra, at: 2026-09-26T05:34:45Z }
---

# Performance review policy

Keep corpus files, generated formats, full sample collections, and reviewed
reports in the separate benchmark archive checkout. The campaign
runner writes to its `.work/` directory there when it discovers the sibling
checkout. Set `TQ_BENCHMARK_ARCHIVE_ROOT` to select another archive location.

After review, add one concise `YYYY-MM-DD.md` report to the archive checkout.
Record the input size, tool versions, commands, timing, peak RSS, comparison
method, and important failures. Leave out the full corpus and per-sample data
from the report itself.

Run all authoritative benchmark and baseline commands outside restricted
sandboxes with the elevated permissions needed for native child accounting.
The required production contract launches each executable directly and, once
the native backend passes lifecycle and audited-counter validation, uses one
resource-aware waiter to collect the exact child's exit status, CPU usage, and
OS-recorded peak RSS. Its allocation preflight runs before corpus preparation;
missing RSS, invalid units, or missing collector provenance abort the campaign
before corpus work and publication. A draft or unverified native backend is not
accepted benchmark evidence.

Use `/usr/bin/time -l` on macOS and GNU `/usr/bin/time -v` on Linux only for
independent validation of native counters, with the same executable, input, and
command contract. These commands must remain separate from the production
measurement method. `ps` is optional and limited to explicitly requested
process-group RSS enforcement or diagnostics; record its scope and interval
because sampling can miss short peaks. Measurements without sampled limits
require no `ps`; catalog cases with RSS limits use separate enforcement
repetitions. Production measurements require neither platform `time` nor
Python allocation probes. Native Windows accounting
is deferred to issue #31 and cannot be verified through cross-compilation or
emulation.

Reports retain the direct spawn-to-exit boundary, input-delivery method, RSS
scope, collector provenance, and observed timing controls. Display precision
is presentation only. One decimal does not imply one-decimal accuracy, and
nanosecond storage does not imply nanosecond accuracy. No-op and known-duration
controls include startup and scheduling effects, not just timer error. Compare
tools on the same host and OS with equivalent measurement settings, using
repeated samples and dispersion to interpret small differences.

State whether the platform waiter accounts for only the waited-for child or
also includes waited-for descendants. Limit process-only comparisons to
verified non-forking jq, yq, and tq workloads; forked workloads require an
explicitly comparable RSS scope.

The current reviewed reports are kept in the benchmark archive
repository alongside their raw campaign outputs.

## TOON migration guard

`tq-toon-regression` compares immutable release/default baseline and candidate
executables on a fixed TOON migration matrix. It is a same-host local guard,
not calibrated publication approval or a replacement for the archive policies
below. Run it outside restricted sandboxes with an explicit deadline:

```bash
cargo build --release --locked -p tq-test-support --bin tq-toon-regression
target/release/tq-toon-regression prepare \
  --baseline /path/to/baseline/tq \
  --directory target/toon-migration-guard \
  --build-manifest /path/to/baseline-build.json \
  --deadline-seconds 1800
target/release/tq-toon-regression run \
  --candidate /path/to/candidate/tq \
  --directory target/toon-migration-guard \
  --build-manifest /path/to/candidate-build.json \
  --report-file target/toon-migration-guard/candidate-report.json \
  --deadline-seconds 1800
```

Build manifests identify the source revision and snapshot SHA-256, toolchain,
native target, release profile, default features, allocator, thread policy and
optimization settings. The guard preserves executable/input hashes, versions,
ordered semantic witnesses, process correctness, plan/preparation observations
and every selected workload. Version-specific TOON inputs may differ in bytes
but must reconstruct the same ordered model. A spool workload must actually
write and replay temporary data.

Each row receives one warmup and three alternating measured baseline/candidate
pairs. A bounded seven-pair confirmation resolves a suspected hard-limit slowdown
or noisy hard-threshold evidence. Decisions use unrounded per-row wall-time ratios:
exactly `1.20` with zero dispersion passes; confirmed ratios above `1.20` fail.
Observed median ratios above `1.10` receive a nonblocking review note in the report
and CLI; exactly `1.10` does not. Crossing the advisory boundary alone neither
triggers confirmation nor blocks acceptance. The user approved the 20% hard limit
and deferred review of advisory regressions. Earlier 10%-policy reports keep their
original thresholds and outcomes; they are not relabeled as passes.
Improvements in another row cannot offset a hard-limit slowdown. MAD bands crossing
the hard threshold, disagreeing rounds, invalid samples or incomplete/interrupted
evidence cannot pass. The report retains all rounds, dispersion, output sizes,
both thresholds, advisory notes
and available native CPU/RSS diagnostics. Exit `0` means every row passed;
`1` means regression, correctness failure or incompatible evidence; `2` means
invalid invocation or incomplete/inconclusive evidence.

### TOON 4.1 migration candidate — 2026-10-07

The complete fixed ten-workload guard passed on native
`aarch64-apple-darwin` (Apple M4 Pro, Darwin 25.6.0), exit `0`, in 41.657 seconds.
This is local, uncalibrated evidence—not release-wide or all-platform approval.
Both builds use Rust 1.99.0 / LLVM 23.1.2, workspace release/default optimization
and mimalloc. Each workload has one warmup per executable and three alternating
measured pairs; no row required hard-limit confirmation.

The immutable 0.4.1 baseline remains commit
`286b5f6690bc1bee260cba007f3953c52178bf68`, executable SHA-256
`abcf52ac46be785203458024f108682c9d972af03d6940b3e2002b5d9ce88501`.
The 0.5.0 candidate is the reviewed working-tree snapshot on integrated revision
`407b35d81681b8ca4ab85968f7186eea645f250f`, with 903 source files, snapshot SHA-256
`2860439d15b72d2e93fc3407b710d767c1c20bf097ec981e4c18d939d96d61ff`,
and executable SHA-256
`8a0ba42cf81881829b93ee950db0d4d3f808a22af574b210484be7833b238d00`.
The later C9 manual-comparator rebuild enforces required process contracts;
all product sources, this frozen CLI and the guard worker remain byte-identical.
These timings retain their C8 source/binary binding, not a relabeled C9 run.

All workload identities, queries and sizes remain fixed at 32,768 records and
96 payload bytes. The user approved correcting only the candidate keyed TOON
representation from `k0, ...` to conforming `k0: ...`; its semantic source,
baseline representation and all eight other input files remain unchanged.
The corrected candidate file is 4,128,251 bytes, SHA-256
`7d68b2a2c8d27c7296fa0a31ba098a795585c22e0f6e0e40a2a8c28e5d277092`.
Nested/keyed decoding is explicitly a representation-migration comparison;
the other comparisons use identical input bytes. Every row independently
satisfied semantic/process correctness.

Displayed ratios/diagnostics are rounded; decisions use unrounded ratios.
CPU is median native user plus system time. RSS is median OS-recorded child
peak RSS, not arena usage or optional process-group sampling.

| Workload | Wall median µs B → C | MAD µs B / C | C / B | CPU median ms B → C | Peak RSS median MiB B → C | Stdout bytes B → C |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| flat-json-control | 33092 → 33064 | 494 / 993 | 0.999154 | 28.226 → 28.683 | 41.31 → 41.38 | 5248157 → 5248157 |
| decode-flat | 30715 → 33042 | 13 / 602 | 1.075761 | 26.023 → 27.699 | 43.55 → 43.69 | 5248157 → 5248157 |
| decode-nested | 46343 → 39768 | 414 / 283 | 0.858123 | 40.557 → 33.955 | 59.94 → 51.50 | 7022903 → 7022903 |
| decode-keyed | 51794 → 44961 | 503 / 104 | 0.868074 | 45.831 → 38.592 | 61.16 → 57.03 | 7339473 → 7339473 |
| document-flat | 60532 → 61123 | 447 / 561 | 1.009763 | 28.148 → 27.675 | 27.03 → 27.02 | 3609782 → 3609782 |
| transcode-flat | 48911 → 47160 | 105 / 616 | 0.964200 | 45.342 → 43.171 | 25.47 → 30.23 | 3609782 → 3609782 |
| transcode-nested | 84496 → 72655 | 118 / 24 | 0.859863 | 79.742 → 68.550 | 31.56 → 29.95 | 5843261 → 3877216 |
| transcode-keyed | 72087 → 82408 | 67 / 326 | 1.143174 | 67.420 → 76.971 | 21.56 → 33.77 | 5766606 → 4128251 |
| transcode-late | 85001 → 87769 | 1066 / 1063 | 1.032564 | 80.487 → 82.882 | 31.56 → 35.81 | 5843225 → 5843225 |
| spool-flat | 160232 → 59262 | 5521 / 743 | 0.369851 | 152.716 → 55.174 | 10.17 → 9.70 | 3609782 → 3609782 |

Keyed transcode's unrounded ratio is `1.1431742200396742`, a **14.32% slowdown**.
Its MAD-derived dispersion band is
`[1.1375945893505557, 1.148764232157734]`: below the approved 1.20 hard limit,
but above the 1.10 advisory boundary. The report and CLI retain the deferred-review
note. The band is a dispersion heuristic, not a confidence interval.

The spool row actually wrote 10,521,441 temporary bytes and replayed 60,838,755
bytes, with arena high-water 219,718 bytes under its 262,144-byte preparation
limit. It did not merely declare spool eligibility.

Earlier C3–C6 reports retain their original 10% hard-limit failures/inconclusive
outcomes. Mitigations before the final candidate were borrowed ordered-key
matching, encounter-order member indexing without finish-time sorting, and
charged keyed-entry cursors that avoid retaining/copying all root keys.
Actual allocator capacity is reconciled for disk keys and member-index growth;
admission failures release owned capacity. Native product source and executable
are unchanged between C6 and the fresh C7 amended-policy run.
The final C8 candidate additionally corrects admission of the existing staged
non-strict TOON-sequence stream path; native formatting and replay code are
unchanged. Fresh full-matrix evidence was collected after that correction.

The first C8 run remains **inconclusive**, exit `2`: nested decoding's initial
MAD ratio band `[0.7811207342295761, 1.2288710431935133]` crossed the hard limit,
while seven-pair confirmation was below it (`0.8756948330535348`).
No gate was relaxed. A fresh complete run after the preceding paired executions
produced the passing table above, with the same binaries, inputs,
matrix, one-warmup/three-pair protocol and thresholds. This does not establish
general cold-start stability. The earlier report is retained at
`target/toon-4-1/performance-corrected-keyed-v1/candidate-c8-policy20-report.json`,
SHA-256 `a0f048f8c76b33e4666b987066ccbc22d62b225b62912f3201ba70cb98532100`.

Raw initial samples, per-row dispersion bands, correctness/resource observations,
CPU/RSS diagnostics and immutable identities are retained in
`target/toon-4-1/performance-corrected-keyed-v1/candidate-c8-policy20-warm-report.json`,
SHA-256 `6a79ba9ca87268395f183e42aa85771847b6d078ea16c4162a1a0e53e571d203`.
`target/toon-4-1/candidate-c8/performance-summary.json` retains the extracted
individual wall samples and diagnostic medians. These local artifacts do not
populate calibrated archive approval or resolve #31.

## Self-regression policy

The local tq-only defaults are:

- Median wall time may increase by at most 50%.
- Peak RSS may increase by at most 50%.
- A row needs at least five measured samples before it can fail the gate.

Run self-regression checks against JSON reports in the archive checkout's
`.work/` directory:

```console
export TQ_BENCHMARK_ARCHIVE_ROOT=/path/to/benchmark-archive
TQ_BIN="$PWD/target/release/tq" cargo run -p tq-test-support --bin tq-bench --release -- \
  run --suite natural-corpus --profile extended --origin frozen --manifest PATH \
  --output "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/candidate.json" \
  --baseline "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/accepted.json" \
  --timing-calibration PATH_TO_VERIFIED_NATIVE_SUMMARY \
  --wall-regression-percent 50 --rss-regression-percent 50 \
  --minimum-regression-samples 5
```

The gate marks comparisons unavailable, not regressed, when suite, profile,
machine, corpus artifact, or tool identity differs. Quick-profile or
quick-sampled evidence is ineligible on either side, even when the other
contracts match and a one-sample minimum is requested. Use a like-for-like
extended baseline with at least five samples. A reference-tool change is
metadata, not a regression. Old wrapper-based or sampled-memory reports keep
their original method and are not comparable baselines for native-accounting
claims.
Reports predating the explicit suite field require a reviewed migration before
the current comparator can read them; never infer or silently relabel their
historical suite or sampling profile.

For issue #30 acceptance, review wall time and peak RSS independently for every
comparable workload. Disclose each increase above 20% with baseline, candidate,
sample count, dispersion, and an explanation. Documented increases greater than
20% and at most 50% are acceptable when all other gates pass; an increase above
50% blocks acceptance until mitigated and remeasured. Exactly 20% does not
cross the disclosure threshold, and exactly 50% does not block acceptance.
Cross-tool jq/yq ratios remain comparative evidence rather than tq
self-regression evidence.

## jq-relative soft objective

Issue #5 workloads report an informational comparison against jq on the same
JSON input and recorded host. The target is at most 2.0 times jq's median wall
time and at most 1.5 times jq's maximum observed peak RSS. The report labels
each metric `met`, `missed`, or `not-comparable`. A miss remains visible but
does not fail the campaign or replace tq's self-regression gate.

The `inputs` workload uses a reviewed deterministic 65,536-document corpus
with separately identified JSON, YAML, and TOON sequence artifacts; natural
benchmark sources are never repeated or resized to construct that workload.

## jq parity acceptance thresholds

For `achieve-jq-manual-parity`, record every median wall-time or peak-RSS
increase above 20%. Documented increases up to and including 50% are
acceptable; anything above 50% blocks acceptance. This decision supersedes
the 20% RSS acceptance limit for this change. It does not rewrite historical
measurements or their original gate results, or change the runner defaults above.

## Parser checkpoint under the revised thresholds

The decoder capture-path component checkpoint, executable SHA-256
`8753d36404758d9b30dfcb41da55f050026cf5fd6a87de906f9876aef8e8f4a6`,
preserves all nine outputs. All measured wall-time and RSS increases are
below the revised 50% blocking limit. These seven observations exceed the
20% disclosure threshold:

| Workload | Metric | Increase |
| --- | --- | ---: |
| JSON projection | Median wall time | 34.8% |
| TOON projection | Median wall time | 25.8% |
| JSON sort | Median wall time | 29.8% |
| JSON projection | Peak RSS | 25.9% |
| JSON select | Peak RSS | 28.6% |
| JSON sort | Peak RSS | 28.0% |
| JSON regex | Peak RSS | 21.7% |

The original report records four failures against its then-current 20% RSS
gate. Those measurements and verdicts remain unchanged. Select and regex
wall-time ratios are 1.093 and 0.646.
The unchanged five-pair report is identified by SHA-256
`555bb9cab56b1483a0ef0e8d0390427cb610bd1360fa86b16e825d2284a04736`.
Its durable summary is `2026-09-10-jq-parity-capture-v9.md`. This isolated
run measures the reviewed runtime changes over frozen v7; it does not include
the concurrent report relocation or establish final whole-worktree acceptance.
No further optimization is required for these measured regressions under the
revised policy. Final source, compatibility, and documentation checks remain required.

The earlier field-retention checkpoint, executable SHA-256
`284c6df9d9ad23a6cf1565f57224857d1dd78cb62a46876cdf6ece1ce219dd6b`,
matches all nine outputs but still fails four checks. JSON projection and
sort have peak-RSS ratios of 1.266 and 1.319. Select has wall/RSS ratios
of 1.660/1.391, and regex has ratios of 1.530/1.279. The unchanged five-pair
campaign is identified in the companion archive by SHA-256
`998f86378e02696922acd818e5819b1c8c900d740996fa4fe9c783beabf60cea`.
Its durable summary is `2026-09-10-jq-parity-roots-v7.md`. Select/regex retained
memory fell substantially, but remaining memory and execution costs still need
work. No threshold or workload exception is accepted.

The earlier root-staging checkpoint, executable SHA-256
`1c7a77c13b8658a86fd4b16afccfb53a8febf296a151a57acf07e5403bfc3b99`,
also matches outputs on all nine workloads but fails four checks. JSON
projection and sort have peak-RSS ratios of 1.262 and 1.213. JSON select
has wall/RSS ratios of 2.024/4.187, and regex has ratios of 1.809/3.751.
Root-staging memory and execution costs remain under investigation. The
five-pair report is identified in the companion archive by SHA-256
`738e0db0dfce005524ca7f6c391d875168f2387a414623b2be0eb916d571b697`.
Its durable summary is `2026-09-10-jq-parity-roots-v3.md`.
The 1.50 wall-time and 1.20 RSS limits are unchanged. This is not accepted
performance evidence.

The shared JSON parser supersedes the earlier passing implementation below.
Its selection-aware skip checkpoint, executable SHA-256
`967d94f9c1f802278078993d7df44ffdd83f81a18590e615b0c601bded0bcd3d`,
matches outputs on all nine workloads but fails four regression checks.
Projection, sort, and select wall-time ratios are 1.850, 1.748, and 1.575.
Regex peak RSS is 1.206 times baseline. The limits remain 1.50 and 1.20.
The authoritative report SHA-256 is
`d93da995de2c67a3fbb4a53e15e9025ceab6c61e015ed6765a3759db9bec1054`.
Further parser and root-validation changes require fresh measurements before
acceptance. This checkpoint is not final performance evidence.

## Earlier reviewed performance evidence (2026-09-09)

The companion archive's durable summary is
`2026-09-09-jq-parity-bind-final1.md`. Its raw campaign is identified by
SHA-256 `0badb9cf077bf17fc994c0e7197c7efa88eb1b8230d134fed0b62739fa800b92`.
It compared baseline `2d6ef92e7db033f3bab141558bf50a47660dca4ae92dd07eec7b473ac78dac36`
with candidate `e9478b0ad46e951a1654efbe4d9d52440da49b8b57283754a0c86fb69f1d4f0c`
on an Apple M4 Pro. Each workload used one warmup and five alternating pairs;
output equality was checked before timing. The single-document TOON workload
used explicit `--unframed` on both revisions, and JSON workloads used `-c`.
All nine workloads passed. The largest measured wall-time ratio was 1.176 and
the largest peak-RSS ratio was 1.182, both below the 1.50 and 1.20 limits.

| Workload | Baseline → candidate wall (ms) | Wall ratio | Baseline → candidate peak RSS (MiB) | RSS ratio |
| --- | ---: | ---: | ---: | ---: |
| JSON identity → TOON | 258.673 → 271.037 | 1.048 | 15.11 → 15.59 | 1.032 |
| JSON numeric array | 12.329 → 12.448 | 1.010 | 7.08 → 7.39 | 1.044 |
| JSON projection | 44.137 → 51.883 | 1.176 | 3.58 → 4.05 | 1.131 |
| TOON projection | 105.963 → 111.481 | 1.052 | 3.66 → 4.23 | 1.158 |
| JSON select | 171.515 → 177.183 | 1.033 | 4.52 → 5.22 | 1.156 |
| JSON sort | 47.080 → 51.891 | 1.102 | 5.13 → 5.77 | 1.125 |
| JSON regex | 224.482 → 232.033 | 1.034 | 5.25 → 6.20 | 1.182 |
| JSON Lines projection | 1058.251 → 1011.972 | 0.956 | 5.75 → 6.40 | 1.114 |
| TOON sequence projection | 1050.830 → 1027.895 | 0.978 | 27.47 → 28.52 | 1.038 |

These measurements are one reviewed host/corpus comparison, not a universal
performance guarantee.
