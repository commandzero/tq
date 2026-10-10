---
type: Report
title: Any-match predicate
description: Measures stopping a collection scan when one feature satisfies a predicate.
workload: benchmark.issue5-predicate
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

# Any-match predicate

## What this measures

The query asks whether any feature has a magnitude at least zero. It exercises
predicate evaluation with a possible early exit from the feature stream.

## Why it matters

Existence checks are common in alerts and guards. Their cost depends on where
the first match occurs, so they differ from reductions that always scan all
values.

## Input and output

The input is a natural feature snapshot. The output is one boolean, `true` when
at least one feature meets the predicate and `false` otherwise.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq any predicate expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 2.9 ms | - | - | 3.5 ms | 4.4 ms | 4.7 ms |
| Wall time dispersion | 0.0 ms / 2.9 ms / 2.7-2.9 ms | - | - | 0.1 ms / 3.6 ms / 3.3-3.6 ms | 0.0 ms / 4.7 ms / 4.4-4.7 ms | 0.0 ms / 5.0 ms / 4.7-5.0 ms |
| First output | 2.9 ms | - | - | 3.3 ms | 4.4 ms | 4.4 ms |
| First output dispersion | 0.0 ms / 2.9 ms / 2.7-2.9 ms | - | - | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.0 ms / 4.4 ms / 4.4-4.4 ms | 0.0 ms / 5.0 ms / 4.3-5.0 ms |
| User CPU | 1.8 ms | - | - | 2.3 ms | 3.3 ms | 3.4 ms |
| System CPU | 0.9 ms | - | - | 1.1 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 3.3 MiB | - | - | 15.0 MiB | 17.7 MiB | 17.1 MiB |
| Logical throughput | 85374.4 records/s | - | - | 69794.1 records/s | 55404.2 records/s | 51771.7 records/s |
| Physical throughput | 58.2 MiB/s | - | - | 47.6 MiB/s | 37.7 MiB/s | 41.8 MiB/s |
| Output bytes | 5.0 B | - | - | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq any predicate expression. | yq rejects the catalog jq any predicate expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq any predicate expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.3 ms | - | - | 1.9 ms | 1.9 ms | 1.9 ms |
| Wall time dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | - | - | 0.1 ms / 2.1 ms / 1.8-2.1 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms |
| First output | 1.3 ms | - | - | 1.9 ms | 1.9 ms | 1.9 ms |
| First output dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | - | - | 0.1 ms / 2.1 ms / 1.8-2.1 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms |
| User CPU | 1.2 ms | - | - | 0.9 ms | 0.8 ms | 0.0 ms |
| System CPU | 0.0 ms | - | - | 0.9 ms | 0.9 ms | 1.8 ms |
| Peak RSS | 2.7 MiB | - | - | 15.3 MiB | 15.6 MiB | 15.1 MiB |
| Logical throughput | 5295.0 records/s | - | - | 3670.7 records/s | 3640.1 records/s | 3619.4 records/s |
| Physical throughput | 3.8 MiB/s | - | - | 2.7 MiB/s | 2.6 MiB/s | 3.1 MiB/s |
| Output bytes | 5.0 B | - | - | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq any predicate expression. | yq rejects the catalog jq any predicate expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq any predicate expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 88.5 ms | - | - | 75.0 ms | 115.1 ms | 122.7 ms |
| Wall time dispersion | 0.1 ms / 88.5 ms / 87.3-88.5 ms | - | - | 1.3 ms / 76.3 ms / 73.6-76.3 ms | 0.3 ms / 115.7 ms / 114.8-115.7 ms | 0.3 ms / 124.5 ms / 122.3-124.5 ms |
| First output | 85.3 ms | - | - | 75.0 ms | 115.1 ms | 122.3 ms |
| First output dispersion | 0.1 ms / 85.3 ms / 84.3-85.3 ms | - | - | 1.0 ms / 76.0 ms / 72.8-76.0 ms | 0.3 ms / 115.5 ms / 114.8-115.5 ms | 0.0 ms / 124.5 ms / 122.3-124.5 ms |
| User CPU | 70.3 ms | - | - | 70.7 ms | 108.6 ms | 115.9 ms |
| System CPU | 17.8 ms | - | - | 4.0 ms | 6.0 ms | 5.9 ms |
| Peak RSS | 56.6 MiB | - | - | 73.2 MiB | 117.6 MiB | 97.6 MiB |
| Logical throughput | 121965.6 records/s | - | - | 143935.6 records/s | 93737.5 records/s | 87988.1 records/s |
| Physical throughput | 82.6 MiB/s | - | - | 97.5 MiB/s | 63.4 MiB/s | 70.6 MiB/s |
| Output bytes | 5.0 B | - | - | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq any predicate expression. | yq rejects the catalog jq any predicate expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq any predicate expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 18.8 ms | - | - | 17.2 ms | 24.8 ms | 27.4 ms |
| Wall time dispersion | 0.1 ms / 19.0 ms / 18.7-19.0 ms | - | - | 0.1 ms / 17.2 ms / 16.9-17.2 ms | 0.2 ms / 25.2 ms / 24.6-25.2 ms | 0.1 ms / 27.5 ms / 26.2-27.5 ms |
| First output | 18.8 ms | - | - | 17.0 ms | 24.4 ms | 27.4 ms |
| First output dispersion | 0.1 ms / 19.0 ms / 18.7-19.0 ms | - | - | 0.0 ms / 17.0 ms / 16.9-17.0 ms | 0.0 ms / 25.2 ms / 24.4-25.2 ms | 0.1 ms / 27.5 ms / 26.2-27.5 ms |
| User CPU | 12.7 ms | - | - | 14.9 ms | 22.4 ms | 24.1 ms |
| System CPU | 5.9 ms | - | - | 2.1 ms | 2.1 ms | 2.9 ms |
| Peak RSS | 13.3 MiB | - | - | 29.0 MiB | 33.4 MiB | 33.4 MiB |
| Logical throughput | 118684.6 records/s | - | - | 130072.8 records/s | 90022.6 records/s | 81664.0 records/s |
| Physical throughput | 80.5 MiB/s | - | - | 88.2 MiB/s | 61.0 MiB/s | 65.6 MiB/s |
| Output bytes | 5.0 B | - | - | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq any predicate expression. | yq rejects the catalog jq any predicate expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
