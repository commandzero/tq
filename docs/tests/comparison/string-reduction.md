---
type: Report
title: String reduction
description: Measures concatenation of a field collected from every feature.
workload: benchmark.string-reduction
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

# String reduction

## What this measures

The query collects non-null place names and combines them with `add`. It
exercises string accumulation over the entire feature collection.

## Why it matters

Reports and exports often assemble text from many records. This case makes
allocation and final-output cost visible when the result is one large string.

## Input and output

The input is a natural snapshot with `properties.place` strings. The output is
one concatenated string in feature order, not a list of place names.

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

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.0 ms | 10.4 ms | 22.0 ms | 5.1 ms | 6.5 ms | 6.1 ms |
| Wall time dispersion | 0.0 ms / 3.5 ms / 3.0-3.5 ms | 0.0 ms / 10.4 ms / 9.6-10.4 ms | 0.5 ms / 22.5 ms / 21.4-22.5 ms | 0.3 ms / 5.3 ms / 4.8-5.3 ms | 0.1 ms / 6.6 ms / 6.1-6.6 ms | 0.2 ms / 6.5 ms / 5.9-6.5 ms |
| First output | 3.0 ms | 10.4 ms | 22.0 ms | 5.1 ms | 6.5 ms | 6.1 ms |
| First output dispersion | 0.0 ms / 3.3 ms / 3.0-3.3 ms | 0.0 ms / 10.4 ms / 9.6-10.4 ms | 0.5 ms / 22.5 ms / 21.4-22.5 ms | 0.3 ms / 5.3 ms / 4.8-5.3 ms | 0.1 ms / 6.6 ms / 6.1-6.6 ms | 0.2 ms / 6.5 ms / 5.9-6.5 ms |
| User CPU | 2.9 ms | 8.7 ms | 16.9 ms | 2.8 ms | 4.2 ms | 4.8 ms |
| System CPU | 0.0 ms | 5.5 ms | 15.1 ms | 1.9 ms | 2.0 ms | 1.1 ms |
| Peak RSS | 3.3 MiB | 20.8 MiB | 36.4 MiB | 17.1 MiB | 19.8 MiB | 19.6 MiB |
| Logical throughput | 80528.1 records/s | 23412.0 records/s | 11096.5 records/s | 48069.3 records/s | 37747.5 records/s | 40059.1 records/s |
| Physical throughput | 54.9 MiB/s | 16.0 MiB/s | 7.6 MiB/s | 32.8 MiB/s | 25.7 MiB/s | 32.3 MiB/s |
| Output bytes | 6806.0 B | 6806.0 B | 6806.0 B | 6806.0 B | 6806.0 B | 6806.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.3 ms | 3.0 ms | 3.3 ms | 1.9 ms | 2.0 ms | 2.0 ms |
| Wall time dispersion | 0.1 ms / 1.3 ms / 1.2-1.3 ms | 0.0 ms / 3.0 ms / 3.0-3.0 ms | 0.0 ms / 3.5 ms / 3.3-3.5 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.1 ms / 2.2 ms / 1.9-2.2 ms | 0.1 ms / 2.3 ms / 1.9-2.3 ms |
| First output | 1.3 ms | 3.0 ms | 3.3 ms | 1.9 ms | 2.0 ms | 2.0 ms |
| First output dispersion | 0.1 ms / 1.3 ms / 1.2-1.3 ms | 0.0 ms / 3.0 ms / 3.0-3.0 ms | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.1 ms / 2.2 ms / 1.9-2.2 ms | 0.1 ms / 2.3 ms / 1.9-2.3 ms |
| User CPU | 1.1 ms | 1.0 ms | 2.8 ms | 0.0 ms | 0.0 ms | 0.9 ms |
| System CPU | 0.0 ms | 2.0 ms | 0.7 ms | 1.8 ms | 1.8 ms | 0.9 ms |
| Peak RSS | 2.7 MiB | 13.3 MiB | 13.8 MiB | 15.1 MiB | 15.7 MiB | 15.4 MiB |
| Logical throughput | 5524.9 records/s | 2314.8 records/s | 2111.0 records/s | 3612.0 records/s | 3578.7 records/s | 3586.1 records/s |
| Physical throughput | 4.0 MiB/s | 1.7 MiB/s | 1.5 MiB/s | 2.6 MiB/s | 2.6 MiB/s | 3.0 MiB/s |
| Output bytes | 204.0 B | 204.0 B | 204.0 B | 204.0 B | 204.0 B | 204.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `process exited with classified error Resource (exit status 5)`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 115.8 ms | 311.0 ms | 893.5 ms | - | - | - |
| Wall time dispersion | 1.0 ms / 122.1 ms / 114.8-122.1 ms | 2.0 ms / 313.0 ms / 305.5-313.0 ms | 6.2 ms / 899.8 ms / 810.5-899.8 ms | - | - | - |
| First output | 108.5 ms | 290.0 ms | 819.1 ms | - | - | - |
| First output dispersion | 1.0 ms / 114.9 ms / 107.5-114.9 ms | 1.2 ms / 291.2 ms / 283.3-291.2 ms | 5.2 ms / 824.3 ms / 751.5-824.3 ms | - | - | - |
| User CPU | 90.6 ms | 337.9 ms | 1097.3 ms | - | - | - |
| System CPU | 25.5 ms | 128.6 ms | 456.9 ms | - | - | - |
| Peak RSS | 56.9 MiB | 324.4 MiB | 1029.7 MiB | - | - | - |
| Logical throughput | 93234.6 records/s | 34698.5 records/s | 12078.0 records/s | - | - | - |
| Physical throughput | 63.2 MiB/s | 23.5 MiB/s | 8.2 MiB/s | - | - | - |
| Output bytes | 299270.0 B | 299270.0 B | 299270.0 B | - | - | - |
| Outcome | timed | timed | timed | resource-limit | resource-limit | resource-limit |
| Details | none | none | none | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | not timed | not timed | not timed |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `process exited with classified error Resource (exit status 5)`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.4 ms | 65.2 ms | 191.4 ms | - | - | - |
| Wall time dispersion | 0.0 ms / 21.4 ms / 20.4-21.4 ms | 0.1 ms / 66.2 ms / 65.1-66.2 ms | 0.2 ms / 191.6 ms / 180.9-191.6 ms | - | - | - |
| First output | 20.2 ms | 60.5 ms | 176.8 ms | - | - | - |
| First output dispersion | 0.4 ms / 21.2 ms / 19.8-21.2 ms | 0.2 ms / 61.2 ms / 60.4-61.2 ms | 0.3 ms / 177.1 ms / 167.4-177.1 ms | - | - | - |
| User CPU | 16.0 ms | 63.4 ms | 202.8 ms | - | - | - |
| System CPU | 4.3 ms | 26.0 ms | 100.5 ms | - | - | - |
| Peak RSS | 13.6 MiB | 76.6 MiB | 234.7 MiB | - | - | - |
| Logical throughput | 109461.5 records/s | 34273.8 records/s | 11672.9 records/s | - | - | - |
| Physical throughput | 74.2 MiB/s | 23.2 MiB/s | 7.9 MiB/s | - | - | - |
| Output bytes | 62828.0 B | 62828.0 B | 62828.0 B | - | - | - |
| Outcome | timed | timed | timed | resource-limit | resource-limit | resource-limit |
| Details | none | none | none | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | not timed | not timed | not timed |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
