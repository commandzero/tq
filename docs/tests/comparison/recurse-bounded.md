---
type: Report
title: Bounded recursion
description: Measures recursive traversal with a fixed output limit.
workload: benchmark.recurse-bounded
generated:
  by: codex/gpt-5.6-luna
  at: 2026-09-10T20:15:01Z
benchmark_runs:
- campaign_id: 2026-09-11
  provenance: Native Linux workload report; captured tool identities are listed in the Results section. No executable hashes or unique campaign ID retained.
  binaries:
    tq:
      version: tq 0.1.0 (TOON v3; jq target 1.8.x; revision a4b4d916-worktree)
      sha256: null
    jq:
      version: jq-1.8.1
      sha256: null
    yq:
      version: yq (https://github.com/mikefarah/yq/) version v4.53.2
      sha256: null
- campaign_id: 2026-10-08T15:10:27.867608118Z
  binaries:
    jq:
      version: jq-1.8.1
      sha256: 020468de7539ce70ef1bceaf7cde2e8c4f2ca6c3afb84642aabc5c97d9fc2a0d
    tq:
      version: tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)
      sha256: 474625b11f0ec3a1fa125bbeb4e55008efa50ec713b0c398822ada48f0289b91
    yq:
      version: yq (https://github.com/mikefarah/yq/) version v4.53.2
      sha256: d56bf5c6819e8e696340c312bd70f849dc1678a7cda9c2ad63eebd906371d56b
---

# Bounded recursion

## What this measures

The query recursively visits child values with `recurse(.[]?)` and keeps at
most 2,048 results. It tests traversal while putting a clear bound on work and
output.

## Why it matters

Recursive queries are useful for irregular documents, but an unbounded walk can
run away on large or cyclic-looking data. A fixed limit is a practical guard.

## Input and output

The input is a natural nested snapshot. The output is the depth-first sequence
of visited values up to the limit, including containers and scalars as the
recursion produces them.

## Results

<!-- benchmark-results:start -->
Last updated: 2026-10-08
Suite: `natural-corpus` | Profile: `standard` | Campaign status: `observed-failures`

Tools: `jq` (jq-1.8.1), `tq` (tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, AMD Ryzen 7 7700 8-Core Processor, 16 logical CPUs, 61.9 GiB RAM; kernel `Linux 7.2.4-ogc3.1.fc44.x86_64 #1 SMP PREEMPT_DYNAMIC Sun Sep 13 01:41:06 UTC 2026`; compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Missing or invalid values are `-`, never estimates. RSS collector provenance (outside measurement tables): `linux-wait4`.
Measurement method (outside measurement tables): `tq-bench` native measurement: RSS scope `wait4 child lifetime including pre exec waited descendants and threads`; residual RSS floor `2.6 MiB` retained, not subtracted; observed control excess `0.9 ms`; primary timing is sampler-free.

Compare columns with the same input format to isolate tool differences. Missing adapters have `-` measurement cells and `not recorded` outcome/detail cells.

Timing rows show numeric medians followed by compact MAD / p95 / range rows, with one decimal place and a unit in each measurement cell. CPU, throughput, and output cells use the captured summary values.

### usgs-all-day

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq recurse expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 12.4 ms | - | - | 19.0 ms | 20.6 ms | 20.5 ms |
| Wall time dispersion | 0.3 ms / 12.8 ms / 12.1-12.8 ms | - | - | 0.1 ms / 19.3 ms / 18.9-19.3 ms | 0.7 ms / 21.7 ms / 19.9-21.7 ms | 0.4 ms / 22.2 ms / 20.1-22.2 ms |
| First output | 3.3 ms | - | - | 3.3 ms | 5.4 ms | 5.4 ms |
| First output dispersion | 0.0 ms / 3.3 ms / 3.3-3.3 ms | - | - | 0.0 ms / 4.3 ms / 3.3-4.3 ms | 0.0 ms / 5.4 ms / 4.4-5.4 ms | 0.0 ms / 5.4 ms / 4.4-5.4 ms |
| User CPU | 8.6 ms | - | - | 9.3 ms | 10.6 ms | 13.6 ms |
| System CPU | 0.0 ms | - | - | 8.0 ms | 7.5 ms | 4.9 ms |
| Peak RSS | 3.4 MiB | - | - | 15.1 MiB | 17.8 MiB | 17.5 MiB |
| Logical throughput | 19648.9 records/s | - | - | 12846.8 records/s | 11843.5 records/s | 11888.5 records/s |
| Physical throughput | 13.4 MiB/s | - | - | 8.8 MiB/s | 8.1 MiB/s | 9.6 MiB/s |
| Output bytes | 639851.0 B | - | - | 516037.0 B | 516037.0 B | 516037.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recurse expression. | yq rejects the catalog jq recurse expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq recurse expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.6 ms | - | - | 3.5 ms | 3.7 ms | 3.8 ms |
| Wall time dispersion | 0.0 ms / 1.6 ms / 1.6-1.6 ms | - | - | 0.0 ms / 3.8 ms / 3.5-3.8 ms | 0.0 ms / 3.7 ms / 3.6-3.7 ms | 0.1 ms / 4.1 ms / 3.6-4.1 ms |
| First output | 1.2 ms | - | - | 3.3 ms | 3.3 ms | 3.8 ms |
| First output dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | - | - | 0.0 ms / 3.8 ms / 3.3-3.8 ms | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.1 ms / 4.1 ms / 3.6-4.1 ms |
| User CPU | 0.8 ms | - | - | 1.2 ms | 0.0 ms | 0.9 ms |
| System CPU | 0.8 ms | - | - | 2.4 ms | 3.5 ms | 3.0 ms |
| Peak RSS | 2.7 MiB | - | - | 15.2 MiB | 15.7 MiB | 15.6 MiB |
| Logical throughput | 4342.4 records/s | - | - | 2004.6 records/s | 1917.3 records/s | 1851.9 records/s |
| Physical throughput | 3.1 MiB/s | - | - | 1.5 MiB/s | 1.4 MiB/s | 1.6 MiB/s |
| Output bytes | 31260.0 B | - | - | 25389.0 B | 25389.0 B | 25389.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recurse expression. | yq rejects the catalog jq recurse expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq recurse expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 432.8 ms | - | - | 262.5 ms | 305.9 ms | 312.5 ms |
| Wall time dispersion | 0.4 ms / 433.2 ms / 430.5-433.2 ms | - | - | 0.6 ms / 263.1 ms / 260.7-263.1 ms | 0.4 ms / 308.1 ms / 305.5-308.1 ms | 0.0 ms / 313.1 ms / 312.5-313.1 ms |
| First output | 79.0 ms | - | - | 68.3 ms | 110.8 ms | 117.0 ms |
| First output dispersion | 0.0 ms / 79.0 ms / 78.1-79.0 ms | - | - | 0.3 ms / 68.6 ms / 67.6-68.6 ms | 0.1 ms / 112.8 ms / 110.6-112.8 ms | 0.1 ms / 117.1 ms / 116.0-117.1 ms |
| User CPU | 229.0 ms | - | - | 143.1 ms | 187.6 ms | 194.3 ms |
| System CPU | 29.7 ms | - | - | 10.9 ms | 12.5 ms | 13.4 ms |
| Peak RSS | 61.1 MiB | - | - | 73.3 MiB | 117.3 MiB | 97.4 MiB |
| Logical throughput | 24935.3 records/s | - | - | 41113.2 records/s | 35282.8 records/s | 34536.1 records/s |
| Physical throughput | 16.9 MiB/s | - | - | 27.8 MiB/s | 23.9 MiB/s | 27.7 MiB/s |
| Output bytes | 22730611.0 B | - | - | 18263101.0 B | 18263101.0 B | 18263101.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recurse expression. | yq rejects the catalog jq recurse expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq recurse expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 90.9 ms | - | - | 68.9 ms | 75.3 ms | 77.5 ms |
| Wall time dispersion | 0.8 ms / 91.8 ms / 90.1-91.8 ms | - | - | 0.2 ms / 69.1 ms / 68.0-69.1 ms | 0.1 ms / 75.6 ms / 75.2-75.6 ms | 0.8 ms / 78.6 ms / 76.8-78.6 ms |
| First output | 17.0 ms | - | - | 17.0 ms | 24.4 ms | 25.4 ms |
| First output dispersion | 0.0 ms / 17.0 ms / 17.0-17.0 ms | - | - | 0.1 ms / 17.1 ms / 16.0-17.1 ms | 0.0 ms / 24.4 ms / 24.4-24.4 ms | 0.0 ms / 26.5 ms / 25.4-26.5 ms |
| User CPU | 52.5 ms | - | - | 37.2 ms | 41.6 ms | 46.4 ms |
| System CPU | 2.1 ms | - | - | 5.0 ms | 8.6 ms | 7.0 ms |
| Peak RSS | 14.3 MiB | - | - | 29.1 MiB | 33.8 MiB | 33.5 MiB |
| Logical throughput | 24564.8 records/s | - | - | 32423.8 records/s | 29676.3 records/s | 28818.4 records/s |
| Physical throughput | 16.7 MiB/s | - | - | 22.0 MiB/s | 20.1 MiB/s | 23.1 MiB/s |
| Output bytes | 4809395.0 B | - | - | 3866539.0 B | 3866539.0 B | 3866539.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recurse expression. | yq rejects the catalog jq recurse expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
