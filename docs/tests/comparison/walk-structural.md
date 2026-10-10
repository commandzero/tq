---
type: Report
title: Structural walking
description: Measures a post-order walk that increments every numeric value.
workload: benchmark.walk-structural
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

# Structural walking

## What this measures

The `walk` query visits the whole document and adds one to every value whose
type is `number`. It combines recursive traversal, type checks, and rebuilding
the original structure.

## Why it matters

Tree-wide edits support migrations and normalization when the fields are not
known in advance. The result must preserve shape while changing selected
leaves.

## Input and output

The input is a natural nested snapshot. The output is one complete document
with every numeric leaf incremented; strings, booleans, arrays, and object keys
keep their roles.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq walk expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 21.0 ms | - | - | 11.2 ms | 12.2 ms | 12.2 ms |
| Wall time dispersion | 0.0 ms / 22.0 ms / 21.0-22.0 ms | - | - | 0.0 ms / 11.3 ms / 11.2-11.3 ms | 0.1 ms / 12.4 ms / 11.1-12.4 ms | 0.0 ms / 12.3 ms / 12.2-12.3 ms |
| First output | 18.0 ms | - | - | 8.6 ms | 10.6 ms | 10.7 ms |
| First output dispersion | 0.0 ms / 18.1 ms / 18.0-18.1 ms | - | - | 0.0 ms / 9.6 ms / 8.6-9.6 ms | 0.0 ms / 10.7 ms / 9.6-10.7 ms | 0.0 ms / 10.7 ms / 10.7-10.7 ms |
| User CPU | 19.4 ms | - | - | 8.8 ms | 9.4 ms | 10.0 ms |
| System CPU | 0.9 ms | - | - | 1.0 ms | 1.2 ms | 1.2 ms |
| Peak RSS | 3.9 MiB | - | - | 17.3 MiB | 19.8 MiB | 19.5 MiB |
| Logical throughput | 11626.8 records/s | - | - | 21826.6 records/s | 19949.3 records/s | 19926.5 records/s |
| Physical throughput | 7.9 MiB/s | - | - | 14.9 MiB/s | 13.6 MiB/s | 16.1 MiB/s |
| Output bytes | 267379.0 B | - | - | 207287.0 B | 207287.0 B | 207287.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq walk expression. | yq rejects the catalog jq walk expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq walk expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.9 ms | - | - | 2.1 ms | 2.2 ms | 2.1 ms |
| Wall time dispersion | 0.1 ms / 2.1 ms / 1.8-2.1 ms | - | - | 0.0 ms / 2.4 ms / 2.1-2.4 ms | 0.0 ms / 2.3 ms / 2.1-2.3 ms | 0.0 ms / 2.1 ms / 2.1-2.1 ms |
| First output | 1.9 ms | - | - | 2.1 ms | 2.2 ms | 2.1 ms |
| First output dispersion | 0.1 ms / 2.1 ms / 1.8-2.1 ms | - | - | 0.0 ms / 2.4 ms / 2.1-2.4 ms | 0.0 ms / 2.3 ms / 2.1-2.3 ms | 0.0 ms / 2.1 ms / 2.1-2.1 ms |
| User CPU | 0.8 ms | - | - | 0.0 ms | 0.9 ms | 1.0 ms |
| System CPU | 0.8 ms | - | - | 2.0 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 2.7 MiB | - | - | 17.1 MiB | 15.5 MiB | 15.7 MiB |
| Logical throughput | 3680.3 records/s | - | - | 3322.3 records/s | 3144.7 records/s | 3389.8 records/s |
| Physical throughput | 2.7 MiB/s | - | - | 2.4 MiB/s | 2.3 MiB/s | 2.9 MiB/s |
| Output bytes | 8065.0 B | - | - | 6252.0 B | 6252.0 B | 6252.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq walk expression. | yq rejects the catalog jq walk expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq walk expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 919.9 ms | - | - | 382.0 ms | 426.6 ms | 440.3 ms |
| Wall time dispersion | 0.6 ms / 920.6 ms / 916.2-920.6 ms | - | - | 0.4 ms / 382.4 ms / 381.5-382.4 ms | 2.3 ms / 430.8 ms / 424.2-430.8 ms | 7.5 ms / 452.5 ms / 432.8-452.5 ms |
| First output | 727.6 ms | - | - | 283.1 ms | 327.6 ms | 338.1 ms |
| First output dispersion | 0.2 ms / 727.8 ms / 724.4-727.8 ms | - | - | 0.1 ms / 283.2 ms / 282.3-283.2 ms | 2.2 ms / 331.4 ms / 325.4-331.4 ms | 6.7 ms / 351.5 ms / 331.5-351.5 ms |
| User CPU | 797.6 ms | - | - | 322.9 ms | 370.6 ms | 384.7 ms |
| System CPU | 33.0 ms | - | - | 8.0 ms | 6.0 ms | 7.0 ms |
| Peak RSS | 81.7 MiB | - | - | 109.3 MiB | 117.4 MiB | 131.5 MiB |
| Logical throughput | 11731.3 records/s | - | - | 28250.1 records/s | 25299.9 records/s | 24510.2 records/s |
| Physical throughput | 7.9 MiB/s | - | - | 19.1 MiB/s | 17.1 MiB/s | 19.7 MiB/s |
| Output bytes | 11774125.0 B | - | - | 9120265.0 B | 9120265.0 B | 9120265.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq walk expression. | yq rejects the catalog jq walk expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq walk expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 186.6 ms | - | - | 80.4 ms | 89.9 ms | 90.1 ms |
| Wall time dispersion | 1.0 ms / 190.2 ms / 185.6-190.2 ms | - | - | 0.2 ms / 83.5 ms / 80.2-83.5 ms | 0.5 ms / 90.4 ms / 88.9-90.4 ms | 0.1 ms / 90.2 ms / 89.8-90.2 ms |
| First output | 148.5 ms | - | - | 61.1 ms | 70.7 ms | 70.7 ms |
| First output dispersion | 0.0 ms / 151.8 ms / 148.5-151.8 ms | - | - | 0.9 ms / 64.4 ms / 60.2-64.4 ms | 0.1 ms / 70.8 ms / 68.6-70.8 ms | 0.0 ms / 70.7 ms / 69.5-70.7 ms |
| User CPU | 160.7 ms | - | - | 66.1 ms | 77.4 ms | 75.4 ms |
| System CPU | 10.9 ms | - | - | 3.0 ms | 2.1 ms | 4.0 ms |
| Peak RSS | 18.7 MiB | - | - | 41.5 MiB | 41.7 MiB | 41.7 MiB |
| Logical throughput | 11971.8 records/s | - | - | 27796.4 records/s | 24857.9 records/s | 24788.6 records/s |
| Physical throughput | 8.1 MiB/s | - | - | 18.8 MiB/s | 16.8 MiB/s | 19.9 MiB/s |
| Output bytes | 2439214.0 B | - | - | 1890001.0 B | 1890001.0 B | 1890001.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq walk expression. | yq rejects the catalog jq walk expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
