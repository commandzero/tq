---
type: Report
title: Sort before counting
description: Measures work whose sorted intermediate does not affect the final count.
workload: benchmark.dead-sort-length
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

# Sort before counting

## What this measures

The query collects the `release` values, sorts them, and then asks for the
length. The final count does not depend on the order, so the case exposes the
cost of doing unnecessary intermediate work.

## Why it matters

Real queries sometimes carry an expensive step that a later operation does not
use. This is a useful stress case for planning and optimization decisions.

## Input and output

The input is a natural feature snapshot. The output is one integer, the number
of collected values. It contains no sorted values.

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
| Wall time | 3.2 ms | 10.0 ms | 22.0 ms | 4.2 ms | 4.7 ms | 5.1 ms |
| Wall time dispersion | 0.2 ms / 3.5 ms / 2.9-3.5 ms | 0.1 ms / 10.1 ms / 9.9-10.1 ms | 0.0 ms / 22.0 ms / 21.9-22.0 ms | 0.2 ms / 4.4 ms / 3.9-4.4 ms | 0.1 ms / 4.8 ms / 4.6-4.8 ms | 0.1 ms / 5.3 ms / 5.0-5.3 ms |
| First output | 3.2 ms | 10.0 ms | 22.0 ms | 4.2 ms | 4.3 ms | 5.1 ms |
| First output dispersion | 0.2 ms / 3.5 ms / 2.9-3.5 ms | 0.1 ms / 10.1 ms / 9.9-10.1 ms | 0.0 ms / 22.0 ms / 21.9-22.0 ms | 0.2 ms / 4.4 ms / 3.9-4.4 ms | 0.0 ms / 4.4 ms / 4.3-4.4 ms | 0.1 ms / 5.3 ms / 5.0-5.3 ms |
| User CPU | 1.0 ms | 8.5 ms | 17.4 ms | 3.1 ms | 2.3 ms | 2.8 ms |
| System CPU | 1.8 ms | 5.3 ms | 14.8 ms | 1.0 ms | 2.3 ms | 3.5 ms |
| Peak RSS | 3.3 MiB | 20.7 MiB | 36.0 MiB | 15.7 MiB | 19.7 MiB | 18.0 MiB |
| Logical throughput | 75988.8 records/s | 24353.7 records/s | 11104.5 records/s | 57628.7 records/s | 51607.4 records/s | 47591.2 records/s |
| Physical throughput | 51.8 MiB/s | 16.6 MiB/s | 7.6 MiB/s | 39.3 MiB/s | 35.1 MiB/s | 38.4 MiB/s |
| Output bytes | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.1 ms | 3.0 ms | 3.0 ms | 1.9 ms | 1.8 ms | 2.1 ms |
| Wall time dispersion | 0.0 ms / 1.2 ms / 1.1-1.2 ms | 0.0 ms / 3.0 ms / 2.9-3.0 ms | 0.1 ms / 3.1 ms / 2.9-3.1 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 2.0 ms / 1.8-2.0 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms |
| First output | 1.1 ms | 3.0 ms | 3.0 ms | 1.9 ms | 1.8 ms | 2.1 ms |
| First output dispersion | 0.0 ms / 1.2 ms / 1.1-1.2 ms | 0.0 ms / 3.0 ms / 2.9-3.0 ms | 0.1 ms / 3.1 ms / 2.9-3.1 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 2.0 ms / 1.8-2.0 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms |
| User CPU | 0.0 ms | 1.0 ms | 1.5 ms | 0.9 ms | 0.8 ms | 0.0 ms |
| System CPU | 1.0 ms | 1.9 ms | 1.5 ms | 0.9 ms | 0.9 ms | 3.3 ms |
| Peak RSS | 2.7 MiB | 13.2 MiB | 13.9 MiB | 15.5 MiB | 15.7 MiB | 17.9 MiB |
| Logical throughput | 6087.0 records/s | 2337.2 records/s | 2318.6 records/s | 3661.1 records/s | 3932.6 records/s | 3398.1 records/s |
| Physical throughput | 4.4 MiB/s | 1.7 MiB/s | 1.7 MiB/s | 2.7 MiB/s | 2.8 MiB/s | 2.9 MiB/s |
| Output bytes | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 109.4 ms | 310.3 ms | 856.6 ms | 101.9 ms | 121.1 ms | 133.9 ms |
| Wall time dispersion | 0.9 ms / 111.6 ms / 108.6-111.6 ms | 0.8 ms / 311.1 ms / 299.4-311.1 ms | 2.5 ms / 859.0 ms / 851.4-859.0 ms | 0.8 ms / 104.5 ms / 101.2-104.5 ms | 0.5 ms / 123.3 ms / 120.6-123.3 ms | 0.4 ms / 134.9 ms / 133.6-134.9 ms |
| First output | 106.4 ms | 287.6 ms | 787.7 ms | 101.9 ms | 121.1 ms | 133.9 ms |
| First output dispersion | 1.0 ms / 108.5 ms / 105.4-108.5 ms | 0.7 ms / 288.3 ms / 277.7-288.3 ms | 4.9 ms / 792.9 ms / 782.8-792.9 ms | 0.8 ms / 104.5 ms / 101.2-104.5 ms | 1.0 ms / 123.3 ms / 120.1-123.3 ms | 0.4 ms / 134.9 ms / 133.6-134.9 ms |
| User CPU | 81.6 ms | 337.1 ms | 1102.3 ms | 99.5 ms | 117.8 ms | 131.6 ms |
| System CPU | 27.2 ms | 133.7 ms | 413.9 ms | 2.0 ms | 4.0 ms | 3.0 ms |
| Peak RSS | 56.9 MiB | 325.7 MiB | 954.2 MiB | 23.6 MiB | 87.7 MiB | 22.0 MiB |
| Logical throughput | 98618.3 records/s | 34781.7 records/s | 12599.2 records/s | 105866.2 records/s | 89131.2 records/s | 80586.6 records/s |
| Physical throughput | 66.8 MiB/s | 23.6 MiB/s | 8.5 MiB/s | 71.7 MiB/s | 60.3 MiB/s | 64.7 MiB/s |
| Output bytes | 6.0 B | 6.0 B | 6.0 B | 6.0 B | 6.0 B | 6.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.6 ms | 64.9 ms | 190.7 ms | 23.5 ms | 26.1 ms | 31.1 ms |
| Wall time dispersion | 0.4 ms / 21.0 ms / 19.3-21.0 ms | 0.2 ms / 66.4 ms / 64.7-66.4 ms | 0.4 ms / 191.1 ms / 167.3-191.1 ms | 0.1 ms / 23.6 ms / 22.4-23.6 ms | 0.3 ms / 26.4 ms / 25.6-26.4 ms | 1.5 ms / 34.8 ms / 29.6-34.8 ms |
| First output | 20.1 ms | 60.4 ms | 173.5 ms | 23.3 ms | 26.1 ms | 31.1 ms |
| First output dispersion | 0.9 ms / 21.0 ms / 19.1-21.0 ms | 0.2 ms / 60.6 ms / 59.4-60.6 ms | 2.5 ms / 176.0 ms / 157.1-176.0 ms | 0.0 ms / 23.3 ms / 22.4-23.3 ms | 0.3 ms / 26.4 ms / 25.4-26.4 ms | 1.5 ms / 34.8 ms / 29.6-34.8 ms |
| User CPU | 16.5 ms | 60.4 ms | 202.5 ms | 21.4 ms | 23.2 ms | 29.8 ms |
| System CPU | 3.9 ms | 25.4 ms | 109.1 ms | 1.0 ms | 2.9 ms | 4.2 ms |
| Peak RSS | 13.4 MiB | 77.5 MiB | 236.3 MiB | 20.0 MiB | 37.7 MiB | 22.1 MiB |
| Logical throughput | 108699.9 records/s | 34422.7 records/s | 11713.9 records/s | 95047.7 records/s | 85557.8 records/s | 71775.1 records/s |
| Physical throughput | 73.7 MiB/s | 23.3 MiB/s | 7.9 MiB/s | 64.5 MiB/s | 57.9 MiB/s | 57.7 MiB/s |
| Output bytes | 5.0 B | 5.0 B | 5.0 B | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
