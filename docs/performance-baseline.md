---
type: Report
title: Performance review policy
description: Benchmark evidence, regression limits, and archive requirements.
generated: { by: codex/gpt-6-astra, at: 2026-09-10T03:46:17Z }
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
scope, collector provenance, and validated timing accuracy. Display precision
is presentation only: one decimal does not imply one-decimal accuracy, and
nanosecond storage does not imply nanosecond accuracy. Repeat no-op and
known-duration controls to establish the supported precision.

State whether the platform waiter accounts for only the waited-for child or
also includes waited-for descendants. Limit process-only comparisons to
verified non-forking jq, yq, and tq workloads; forked workloads require an
explicitly comparable RSS scope.

The current reviewed reports are kept in the benchmark archive
repository alongside their raw campaign outputs.

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
  run --profile standard --origin frozen --manifest PATH \
  --output "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/candidate.json" \
  --baseline "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/accepted.json" \
  --timing-calibration PATH_TO_VERIFIED_NATIVE_SUMMARY \
  --wall-regression-percent 50 --rss-regression-percent 50 \
  --minimum-regression-samples 5
```

The gate skips comparisons when the profile, machine, corpus artifact, or tool
identity differs. A reference-tool change is metadata, not a regression. Old
wrapper-based or sampled-memory reports keep their original method and are not
comparable baselines for native-accounting claims.

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
The unchanged five-pair report is
`.work/jq-parity-perf-20260910-capture-v9/report.toon`, SHA-256
`555bb9cab56b1483a0ef0e8d0390427cb610bd1360fa86b16e825d2284a04736`.
Its companion summary is `2026-09-10-jq-parity-capture-v9.md`. This isolated
run measures the reviewed runtime changes over frozen v7; it does not include
the concurrent report relocation or establish final whole-worktree acceptance.
No further optimization is required for these measured regressions under the
revised policy. Final source, compatibility, and documentation checks remain required.

The earlier field-retention checkpoint, executable SHA-256
`284c6df9d9ad23a6cf1565f57224857d1dd78cb62a46876cdf6ece1ce219dd6b`,
matches all nine outputs but still fails four checks. JSON projection and
sort have peak-RSS ratios of 1.266 and 1.319. Select has wall/RSS ratios
of 1.660/1.391, and regex has ratios of 1.530/1.279. The unchanged five-pair
campaign is `.work/jq-parity-perf-20260910-roots-v7/report.toon` in the
companion archive, SHA-256
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
five-pair report is in the companion archive at
`.work/jq-parity-perf-20260910-roots-v3/report.toon`, SHA-256
`738e0db0dfce005524ca7f6c391d875168f2387a414623b2be0eb916d571b697`.
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
`2026-09-09-jq-parity-bind-final1.md`. Its raw
`.work/jq-parity-perf-20260909-bind-final1/report.toon` campaign
(SHA-256 `0badb9cf077bf17fc994c0e7197c7efa88eb1b8230d134fed0b62739fa800b92`)
compared baseline `2d6ef92e7db033f3bab141558bf50a47660dca4ae92dd07eec7b473ac78dac36`
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
