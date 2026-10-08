---
type: Report
title: User function mapping
description: Measures mapping a named user filter across a feature collection.
workload: benchmark.user-filter-map
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

# User function mapping

## What this measures

The query defines a no-argument `magnitude` filter and applies it with `map`.
It combines user-function lookup with array construction over all features.

## Why it matters

Named mapping functions are common in reusable transformations. The case tests
the cost and semantics of invoking a filter once for every array member.

## Input and output

The input is a natural snapshot with a feature array. The output is one array
of magnitudes, in feature order, rather than a stream of separate numbers.

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
| Wall time | 3.0 ms | - | - | 3.9 ms | 5.0 ms | 5.1 ms |
| Wall time dispersion | 0.1 ms / 3.1 ms / 2.8-3.1 ms | - | - | 0.0 ms / 3.9 ms / 3.9-3.9 ms | 0.2 ms / 5.3 ms / 4.8-5.3 ms | 0.0 ms / 5.1 ms / 4.8-5.1 ms |
| First output | 3.0 ms | - | - | 3.9 ms | 5.0 ms | 5.1 ms |
| First output dispersion | 0.1 ms / 3.1 ms / 2.8-3.1 ms | - | - | 0.0 ms / 3.9 ms / 3.9-3.9 ms | 0.3 ms / 5.3 ms / 4.4-5.3 ms | 0.0 ms / 5.1 ms / 4.4-5.1 ms |
| User CPU | 3.0 ms | - | - | 2.8 ms | 3.1 ms | 2.8 ms |
| System CPU | 0.0 ms | - | - | 1.0 ms | 1.9 ms | 1.9 ms |
| Peak RSS | 3.4 MiB | - | - | 17.1 MiB | 19.8 MiB | 19.5 MiB |
| Logical throughput | 81387.6 records/s | - | - | 62244.9 records/s | 48576.5 records/s | 48288.1 records/s |
| Physical throughput | 55.5 MiB/s | - | - | 42.4 MiB/s | 33.1 MiB/s | 39.0 MiB/s |
| Output bytes | 1816.0 B | - | - | 1088.0 B | 1088.0 B | 1088.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | - | - | 1.9 ms | 2.0 ms | 1.9 ms |
| Wall time dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | - | - | 0.2 ms / 2.1 ms / 1.8-2.1 ms | 0.1 ms / 2.1 ms / 1.8-2.1 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms |
| First output | 1.2 ms | - | - | 1.9 ms | 2.0 ms | 1.9 ms |
| First output dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | - | - | 0.2 ms / 2.1 ms / 1.8-2.1 ms | 0.1 ms / 2.1 ms / 1.8-2.1 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms |
| User CPU | 0.0 ms | - | - | 0.0 ms | 0.9 ms | 0.9 ms |
| System CPU | 1.1 ms | - | - | 1.8 ms | 0.9 ms | 0.9 ms |
| Peak RSS | 2.7 MiB | - | - | 15.2 MiB | 15.5 MiB | 15.5 MiB |
| Logical throughput | 5993.2 records/s | - | - | 3599.0 records/s | 3535.4 records/s | 3595.3 records/s |
| Physical throughput | 4.3 MiB/s | - | - | 2.6 MiB/s | 2.6 MiB/s | 3.1 MiB/s |
| Output bytes | 58.0 B | - | - | 39.0 B | 39.0 B | 39.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 112.5 ms | - | - | 84.9 ms | 127.4 ms | 135.7 ms |
| Wall time dispersion | 2.2 ms / 116.3 ms / 110.3-116.3 ms | - | - | 0.0 ms / 84.9 ms / 83.5-84.9 ms | 1.5 ms / 128.9 ms / 123.7-128.9 ms | 0.5 ms / 136.3 ms / 134.5-136.3 ms |
| First output | 108.5 ms | - | - | 83.5 ms | 126.5 ms | 134.9 ms |
| First output dispersion | 2.1 ms / 110.6 ms / 106.4-110.6 ms | - | - | 0.1 ms / 84.4 ms / 83.4-84.4 ms | 1.2 ms / 127.6 ms / 123.4-127.6 ms | 1.2 ms / 136.1 ms / 133.7-136.1 ms |
| User CPU | 91.9 ms | - | - | 78.4 ms | 122.4 ms | 131.3 ms |
| System CPU | 19.9 ms | - | - | 6.0 ms | 5.0 ms | 4.0 ms |
| Peak RSS | 56.9 MiB | - | - | 77.1 MiB | 117.8 MiB | 99.4 MiB |
| Logical throughput | 95893.9 records/s | - | - | 127133.7 records/s | 84730.2 records/s | 79509.6 records/s |
| Physical throughput | 65.0 MiB/s | - | - | 86.1 MiB/s | 57.3 MiB/s | 63.8 MiB/s |
| Output bytes | 80999.0 B | - | - | 48629.0 B | 48629.0 B | 48629.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 19.8 ms | - | - | 18.8 ms | 27.0 ms | 28.2 ms |
| Wall time dispersion | 0.6 ms / 21.1 ms / 19.1-21.1 ms | - | - | 0.3 ms / 19.2 ms / 18.1-19.2 ms | 0.0 ms / 27.0 ms / 26.5-27.0 ms | 0.0 ms / 28.3 ms / 28.2-28.3 ms |
| First output | 19.1 ms | - | - | 18.8 ms | 26.5 ms | 28.2 ms |
| First output dispersion | 0.0 ms / 21.1 ms / 19.1-21.1 ms | - | - | 0.3 ms / 19.2 ms / 18.1-19.2 ms | 0.0 ms / 26.5 ms / 26.5-26.5 ms | 0.0 ms / 28.3 ms / 27.5-28.3 ms |
| User CPU | 14.9 ms | - | - | 15.7 ms | 23.9 ms | 25.0 ms |
| System CPU | 6.0 ms | - | - | 2.9 ms | 3.0 ms | 3.0 ms |
| Peak RSS | 13.4 MiB | - | - | 35.0 MiB | 37.7 MiB | 37.4 MiB |
| Logical throughput | 113096.7 records/s | - | - | 118564.9 records/s | 82694.8 records/s | 79144.1 records/s |
| Physical throughput | 76.7 MiB/s | - | - | 80.4 MiB/s | 56.0 MiB/s | 63.6 MiB/s |
| Output bytes | 16892.0 B | - | - | 10195.0 B | 10195.0 B | 10195.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
