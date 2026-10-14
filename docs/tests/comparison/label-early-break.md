---
type: Report
title: Early break with a label
description: Measures stopping a feature traversal after the first result.
workload: benchmark.label-early-break
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

# Early break with a label

## What this measures

The labeled query emits a feature ID and then breaks to the outer label. It
tests non-local control flow and whether later features are left unread.

## Why it matters

First-match queries should stop work when the answer is known. Labels provide a
way to express that exit when a simple predicate is not enough.

## Input and output

The input is a natural feature snapshot. The output is the early result
sequence, normally the first feature ID, rather than every feature ID in the
document.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq label/break expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.1 ms | - | - | 3.4 ms | 4.7 ms | 4.6 ms |
| Wall time dispersion | 0.0 ms / 3.2 ms / 2.8-3.2 ms | - | - | 0.1 ms / 3.9 ms / 3.3-3.9 ms | 0.3 ms / 5.2 ms / 4.4-5.2 ms | 0.0 ms / 4.7 ms / 4.6-4.7 ms |
| First output | 3.1 ms | - | - | 3.4 ms | 4.4 ms | 4.3 ms |
| First output dispersion | 0.0 ms / 3.2 ms / 2.8-3.2 ms | - | - | 0.1 ms / 3.9 ms / 3.3-3.9 ms | 0.1 ms / 5.2 ms / 4.3-5.2 ms | 0.0 ms / 4.4 ms / 4.3-4.4 ms |
| User CPU | 2.0 ms | - | - | 1.9 ms | 3.2 ms | 3.0 ms |
| System CPU | 1.0 ms | - | - | 1.7 ms | 1.1 ms | 1.5 ms |
| Peak RSS | 3.3 MiB | - | - | 15.3 MiB | 17.7 MiB | 17.4 MiB |
| Logical throughput | 77558.8 records/s | - | - | 70971.5 records/s | 51959.1 records/s | 53251.9 records/s |
| Physical throughput | 52.9 MiB/s | - | - | 48.4 MiB/s | 35.4 MiB/s | 43.0 MiB/s |
| Output bytes | 13.0 B | - | - | 11.0 B | 11.0 B | 11.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq label/break expression. | yq rejects the catalog jq label/break expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq label/break expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | - | - | 1.9 ms | 2.0 ms | 1.9 ms |
| Wall time dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | - | - | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.1 ms / 2.0 ms / 1.8-2.0 ms |
| First output | 1.2 ms | - | - | 1.9 ms | 2.0 ms | 1.9 ms |
| First output dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | - | - | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.1 ms / 2.0 ms / 1.8-2.0 ms |
| User CPU | 1.1 ms | - | - | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | - | - | 1.8 ms | 1.9 ms | 1.8 ms |
| Peak RSS | 2.7 MiB | - | - | 15.0 MiB | 15.6 MiB | 15.3 MiB |
| Logical throughput | 5988.0 records/s | - | - | 3653.4 records/s | 3586.1 records/s | 3717.5 records/s |
| Physical throughput | 4.3 MiB/s | - | - | 2.6 MiB/s | 2.6 MiB/s | 3.2 MiB/s |
| Output bytes | 13.0 B | - | - | 11.0 B | 11.0 B | 11.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq label/break expression. | yq rejects the catalog jq label/break expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq label/break expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 88.7 ms | - | - | 73.5 ms | 115.8 ms | 124.1 ms |
| Wall time dispersion | 0.0 ms / 89.5 ms / 88.7-89.5 ms | - | - | 0.7 ms / 74.6 ms / 72.7-74.6 ms | 1.2 ms / 120.5 ms / 114.6-120.5 ms | 0.5 ms / 124.6 ms / 122.6-124.6 ms |
| First output | 85.4 ms | - | - | 72.8 ms | 115.8 ms | 123.3 ms |
| First output dispersion | 0.0 ms / 86.5 ms / 85.4-86.5 ms | - | - | 1.0 ms / 74.6 ms / 71.8-74.6 ms | 2.1 ms / 120.1 ms / 113.8-120.1 ms | 1.0 ms / 124.6 ms / 122.3-124.6 ms |
| User CPU | 62.2 ms | - | - | 68.2 ms | 110.2 ms | 119.6 ms |
| System CPU | 26.1 ms | - | - | 5.2 ms | 5.0 ms | 4.0 ms |
| Peak RSS | 56.6 MiB | - | - | 73.3 MiB | 117.7 MiB | 97.3 MiB |
| Logical throughput | 121724.8 records/s | - | - | 146929.9 records/s | 93157.4 records/s | 86991.6 records/s |
| Physical throughput | 82.5 MiB/s | - | - | 99.5 MiB/s | 63.0 MiB/s | 69.8 MiB/s |
| Output bytes | 13.0 B | - | - | 11.0 B | 11.0 B | 11.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq label/break expression. | yq rejects the catalog jq label/break expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq label/break expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 18.6 ms | - | - | 16.8 ms | 25.1 ms | 26.2 ms |
| Wall time dispersion | 0.0 ms / 19.6 ms / 18.6-19.6 ms | - | - | 0.2 ms / 18.0 ms / 16.6-18.0 ms | 0.2 ms / 25.3 ms / 24.9-25.3 ms | 0.2 ms / 26.3 ms / 25.6-26.3 ms |
| First output | 18.1 ms | - | - | 16.8 ms | 25.1 ms | 26.2 ms |
| First output dispersion | 0.1 ms / 19.1 ms / 18.0-19.1 ms | - | - | 0.2 ms / 18.0 ms / 16.6-18.0 ms | 0.2 ms / 25.3 ms / 24.3-25.3 ms | 0.2 ms / 26.3 ms / 25.4-26.3 ms |
| User CPU | 12.6 ms | - | - | 14.5 ms | 23.6 ms | 25.0 ms |
| System CPU | 5.8 ms | - | - | 2.0 ms | 1.1 ms | 1.0 ms |
| Peak RSS | 13.3 MiB | - | - | 29.2 MiB | 33.7 MiB | 33.4 MiB |
| Logical throughput | 120114.0 records/s | - | - | 132857.6 records/s | 88880.0 records/s | 85316.0 records/s |
| Physical throughput | 81.5 MiB/s | - | - | 90.1 MiB/s | 60.2 MiB/s | 68.5 MiB/s |
| Output bytes | 13.0 B | - | - | 11.0 B | 11.0 B | 11.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq label/break expression. | yq rejects the catalog jq label/break expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
