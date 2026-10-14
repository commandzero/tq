---
type: Report
title: Sort by multiple keys
description: Measures sorting feature objects by magnitude and then ID.
workload: benchmark.comma-generator-sort
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

# Sort by multiple keys

## What this measures

The query sorts the feature objects by `.properties.mag` and `.id`. The comma
generator supplies a second key for ties, so the whole collection is retained
until ordering is known.

## Why it matters

Stable tie-breaking makes reports and pagination repeatable. It also costs more
than sorting a single scalar because the result keeps complete feature objects.

## Input and output

The input is a natural feature snapshot. The output is one array of the
original feature objects, ordered by magnitude and then ID, rather than an
array of just the sort keys.

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
| Wall time | 6.2 ms | 23.6 ms | 36.9 ms | 5.8 ms | 7.0 ms | 7.0 ms |
| Wall time dispersion | 0.2 ms / 6.7 ms / 6.1-6.7 ms | 0.0 ms / 24.7 ms / 23.6-24.7 ms | 0.1 ms / 37.0 ms / 33.2-37.0 ms | 0.0 ms / 7.0 ms / 5.8-7.0 ms | 0.0 ms / 7.2 ms / 7.0-7.2 ms | 0.0 ms / 7.2 ms / 6.9-7.2 ms |
| First output | 3.3 ms | 23.3 ms | 33.8 ms | 4.4 ms | 5.4 ms | 5.4 ms |
| First output dispersion | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.0 ms / 24.4 ms / 23.3-24.4 ms | 0.1 ms / 33.9 ms / 32.8-33.9 ms | 0.0 ms / 4.4 ms / 4.3-4.4 ms | 0.0 ms / 5.4 ms / 5.4-5.4 ms | 0.0 ms / 5.4 ms / 5.4-5.4 ms |
| User CPU | 4.2 ms | 23.9 ms | 30.3 ms | 3.4 ms | 4.2 ms | 4.0 ms |
| System CPU | 0.8 ms | 9.3 ms | 18.8 ms | 1.3 ms | 1.7 ms | 2.0 ms |
| Peak RSS | 3.4 MiB | 30.1 MiB | 41.9 MiB | 17.4 MiB | 20.0 MiB | 19.5 MiB |
| Logical throughput | 39071.3 records/s | 10336.8 records/s | 6609.1 records/s | 41953.2 records/s | 34703.5 records/s | 35027.3 records/s |
| Physical throughput | 26.6 MiB/s | 7.0 MiB/s | 4.5 MiB/s | 28.6 MiB/s | 23.6 MiB/s | 28.3 MiB/s |
| Output bytes | 246567.0 B | 173854.0 B | 173854.0 B | 206093.0 B | 206093.0 B | 206093.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.3 ms | 3.3 ms | 3.8 ms | 1.9 ms | 1.9 ms | 1.9 ms |
| Wall time dispersion | 0.0 ms / 1.3 ms / 1.3-1.3 ms | 0.0 ms / 3.5 ms / 3.3-3.5 ms | 0.0 ms / 3.8 ms / 3.6-3.8 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 1.9 ms / 1.9-1.9 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms |
| First output | 1.3 ms | 3.3 ms | 3.3 ms | 1.9 ms | 1.9 ms | 1.9 ms |
| First output dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.0 ms / 3.8 ms / 3.3-3.8 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 1.9 ms / 1.9-1.9 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms |
| User CPU | 1.2 ms | 1.1 ms | 0.9 ms | 0.0 ms | 0.0 ms | 0.9 ms |
| System CPU | 0.0 ms | 2.3 ms | 2.8 ms | 1.9 ms | 1.8 ms | 1.1 ms |
| Peak RSS | 2.7 MiB | 13.7 MiB | 14.4 MiB | 15.3 MiB | 15.9 MiB | 15.5 MiB |
| Logical throughput | 5267.1 records/s | 2100.8 records/s | 1860.7 records/s | 3589.7 records/s | 3659.2 records/s | 3638.3 records/s |
| Physical throughput | 3.8 MiB/s | 1.5 MiB/s | 1.3 MiB/s | 2.6 MiB/s | 2.6 MiB/s | 3.1 MiB/s |
| Output bytes | 7100.0 B | 5013.0 B | 5013.0 B | 5945.0 B | 5945.0 B | 5945.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 300.9 ms | 984.9 ms | 1476.3 ms | 190.1 ms | 232.6 ms | 239.8 ms |
| Wall time dispersion | 0.2 ms / 303.4 ms / 300.6-303.4 ms | 2.3 ms / 987.2 ms / 964.3-987.2 ms | 3.8 ms / 1500.3 ms / 1472.5-1500.3 ms | 0.2 ms / 201.4 ms / 189.9-201.4 ms | 0.5 ms / 233.8 ms / 232.1-233.8 ms | 0.7 ms / 242.8 ms / 239.1-242.8 ms |
| First output | 100.2 ms | 928.1 ms | 1383.9 ms | 89.7 ms | 131.3 ms | 139.1 ms |
| First output dispersion | 1.1 ms / 101.3 ms / 99.1-101.3 ms | 2.9 ms / 930.9 ms / 912.9-930.9 ms | 10.4 ms / 1396.9 ms / 1373.5-1396.9 ms | 0.1 ms / 101.1 ms / 89.6-101.1 ms | 0.5 ms / 132.7 ms / 130.8-132.7 ms | 1.0 ms / 141.3 ms / 138.1-141.3 ms |
| User CPU | 192.2 ms | 1149.4 ms | 1879.8 ms | 135.7 ms | 176.9 ms | 188.6 ms |
| System CPU | 28.6 ms | 301.8 ms | 542.9 ms | 7.0 ms | 8.0 ms | 6.0 ms |
| Peak RSS | 61.1 MiB | 699.4 MiB | 1238.4 MiB | 83.2 MiB | 117.6 MiB | 99.3 MiB |
| Logical throughput | 35867.3 records/s | 10957.7 records/s | 7310.1 records/s | 56769.2 records/s | 46399.2 records/s | 45004.2 records/s |
| Physical throughput | 24.3 MiB/s | 7.4 MiB/s | 4.9 MiB/s | 38.5 MiB/s | 31.4 MiB/s | 36.1 MiB/s |
| Output bytes | 10870018.0 B | 7654001.0 B | 7654001.0 B | 9079616.0 B | 9079616.0 B | 9079616.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 57.0 ms | 200.7 ms | 307.9 ms | 38.3 ms | 46.3 ms | 47.1 ms |
| Wall time dispersion | 0.1 ms / 57.1 ms / 56.1-57.1 ms | 1.1 ms / 201.8 ms / 198.0-201.8 ms | 4.2 ms / 312.1 ms / 303.8-312.1 ms | 0.1 ms / 38.4 ms / 37.5-38.4 ms | 0.1 ms / 46.4 ms / 45.8-46.4 ms | 0.0 ms / 48.1 ms / 47.0-48.1 ms |
| First output | 21.2 ms | 188.5 ms | 287.6 ms | 20.2 ms | 28.6 ms | 28.7 ms |
| First output dispersion | 0.0 ms / 21.2 ms / 21.1-21.2 ms | 0.2 ms / 188.7 ms / 186.3-188.7 ms | 1.0 ms / 290.5 ms / 286.5-290.5 ms | 0.0 ms / 20.2 ms / 20.1-20.2 ms | 0.3 ms / 28.9 ms / 27.5-28.9 ms | 0.1 ms / 30.7 ms / 28.6-30.7 ms |
| User CPU | 33.4 ms | 225.0 ms | 367.9 ms | 24.9 ms | 33.1 ms | 34.9 ms |
| System CPU | 6.3 ms | 63.3 ms | 123.0 ms | 3.1 ms | 3.0 ms | 2.0 ms |
| Peak RSS | 14.2 MiB | 155.4 MiB | 268.9 MiB | 37.1 MiB | 39.7 MiB | 39.6 MiB |
| Logical throughput | 39181.3 records/s | 11129.0 records/s | 7255.0 records/s | 58332.0 records/s | 48227.6 records/s | 47466.3 records/s |
| Physical throughput | 26.6 MiB/s | 7.5 MiB/s | 4.9 MiB/s | 39.6 MiB/s | 32.7 MiB/s | 38.1 MiB/s |
| Output bytes | 2251731.0 B | 1585998.0 B | 1585998.0 B | 1881336.0 B | 1881336.0 B | 1881336.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
