---
type: Report
title: User function sorting
description: Measures sorting with a named key filter and projecting IDs.
workload: benchmark.user-filter-sort-by
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

# User function sorting

## What this measures

The query defines `magnitude`, uses it as the `sort_by` key, and maps the
ordered features to IDs. It exercises a user filter in a sorting callback.

## Why it matters

Sorting by a reusable key function is a practical pattern for reports and
ranking. It also combines callback overhead with a blocking operation.

## Input and output

The input is a natural feature snapshot. The output is one array of IDs ordered
by each feature's magnitude, not the sorted feature objects themselves.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.3 ms | - | - | 4.4 ms | 5.0 ms | 5.0 ms |
| Wall time dispersion | 0.1 ms / 3.4 ms / 3.2-3.4 ms | - | - | 0.3 ms / 4.8 ms / 4.1-4.8 ms | 0.1 ms / 5.3 ms / 4.9-5.3 ms | 0.0 ms / 5.6 ms / 5.0-5.6 ms |
| First output | 3.3 ms | - | - | 4.4 ms | 5.0 ms | 5.0 ms |
| First output dispersion | 0.1 ms / 3.3 ms / 3.2-3.3 ms | - | - | 0.0 ms / 4.4 ms / 4.1-4.4 ms | 0.1 ms / 5.3 ms / 4.9-5.3 ms | 0.0 ms / 5.4 ms / 5.0-5.4 ms |
| User CPU | 2.1 ms | - | - | 2.0 ms | 3.1 ms | 4.0 ms |
| System CPU | 1.0 ms | - | - | 2.0 ms | 1.9 ms | 1.1 ms |
| Peak RSS | 3.3 MiB | - | - | 17.4 MiB | 19.8 MiB | 19.7 MiB |
| Logical throughput | 72857.6 records/s | - | - | 55669.6 records/s | 48858.6 records/s | 48374.3 records/s |
| Physical throughput | 49.7 MiB/s | - | - | 37.9 MiB/s | 33.3 MiB/s | 39.0 MiB/s |
| Output bytes | 4138.0 B | - | - | 2922.0 B | 2922.0 B | 2922.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | - | - | 2.1 ms | 1.9 ms | 1.9 ms |
| Wall time dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | - | - | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms |
| First output | 1.2 ms | - | - | 2.1 ms | 1.9 ms | 1.9 ms |
| First output dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | - | - | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms |
| User CPU | 0.6 ms | - | - | 0.9 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.6 ms | - | - | 1.0 ms | 1.8 ms | 1.9 ms |
| Peak RSS | 2.7 MiB | - | - | 17.4 MiB | 15.9 MiB | 15.6 MiB |
| Logical throughput | 5988.0 records/s | - | - | 3401.4 records/s | 3634.5 records/s | 3599.0 records/s |
| Physical throughput | 4.3 MiB/s | - | - | 2.5 MiB/s | 2.6 MiB/s | 3.1 MiB/s |
| Output bytes | 119.0 B | - | - | 86.0 B | 86.0 B | 86.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 131.7 ms | - | - | 98.0 ms | 137.7 ms | 147.4 ms |
| Wall time dispersion | 2.3 ms / 134.0 ms / 128.6-134.0 ms | - | - | 0.9 ms / 101.6 ms / 97.1-101.6 ms | 0.1 ms / 140.0 ms / 137.6-140.0 ms | 0.4 ms / 147.9 ms / 147.0-147.9 ms |
| First output | 126.6 ms | - | - | 91.8 ms | 132.8 ms | 140.1 ms |
| First output dispersion | 2.0 ms / 128.6 ms / 123.2-128.6 ms | - | - | 0.0 ms / 96.0 ms / 91.7-96.0 ms | 1.0 ms / 133.9 ms / 131.8-133.9 ms | 0.1 ms / 141.2 ms / 140.0-141.2 ms |
| User CPU | 107.6 ms | - | - | 93.5 ms | 131.2 ms | 141.9 ms |
| System CPU | 24.9 ms | - | - | 5.0 ms | 6.0 ms | 5.0 ms |
| Peak RSS | 57.7 MiB | - | - | 89.3 MiB | 120.0 MiB | 99.6 MiB |
| Logical throughput | 81930.1 records/s | - | - | 110119.1 records/s | 78360.2 records/s | 73221.2 records/s |
| Physical throughput | 55.5 MiB/s | - | - | 74.6 MiB/s | 53.0 MiB/s | 58.8 MiB/s |
| Output bytes | 181603.0 B | - | - | 127649.0 B | 127649.0 B | 127649.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 22.7 ms | - | - | 21.0 ms | 29.3 ms | 31.2 ms |
| Wall time dispersion | 0.3 ms / 23.8 ms / 22.4-23.8 ms | - | - | 0.3 ms / 21.6 ms / 20.6-21.6 ms | 0.2 ms / 30.0 ms / 29.1-30.0 ms | 0.1 ms / 31.3 ms / 31.0-31.3 ms |
| First output | 22.4 ms | - | - | 21.0 ms | 29.3 ms | 30.7 ms |
| First output dispersion | 0.1 ms / 23.3 ms / 22.3-23.3 ms | - | - | 0.2 ms / 21.2 ms / 20.1-21.2 ms | 0.3 ms / 29.6 ms / 28.6-29.6 ms | 0.0 ms / 30.7 ms / 30.6-30.7 ms |
| User CPU | 14.7 ms | - | - | 18.9 ms | 26.8 ms | 28.0 ms |
| System CPU | 7.8 ms | - | - | 2.0 ms | 3.0 ms | 3.0 ms |
| Peak RSS | 13.6 MiB | - | - | 37.2 MiB | 39.9 MiB | 39.4 MiB |
| Logical throughput | 98627.0 records/s | - | - | 106553.5 records/s | 76279.6 records/s | 71655.4 records/s |
| Physical throughput | 66.9 MiB/s | - | - | 72.3 MiB/s | 51.7 MiB/s | 57.6 MiB/s |
| Output bytes | 37517.0 B | - | - | 26352.0 B | 26352.0 B | 26352.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
