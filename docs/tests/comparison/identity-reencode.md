---
type: Report
title: Identity re-encoding
description: Measures reading and writing a complete natural document unchanged.
workload: benchmark.identity-reencode
generated:
  by: codex
  at: 2026-10-08T15:38:02.825Z
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

# Identity re-encoding

## What this measures

The query is `.` on natural snapshots. Unlike the startup identity case, the
documents span the catalog's small, medium, and large tiers, exposing full
parse and output costs.

## Why it matters

Format conversion and pass-through filters are useful baselines for pipelines
that do not change the data. They show the cost of carrying a document through
the tool.

## Input and output

The input is a complete natural snapshot in the selected format. The output is
the same semantic document, including its structure and object members, not a
single field or a stream of projected values.

The retained commands emit pretty JSON from jq, compact JSON from yq, and
default TOON 4.1 from tq. Equal input formats do not imply equal output
encodings or byte counts. These timings measure the complete native
read/write paths, not a same-output JSON serialization comparison.

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
| Wall time | 6.2 ms | 19.3 ms | 30.9 ms | 7.0 ms | 6.0 ms | 6.8 ms |
| Wall time dispersion | 0.0 ms / 6.2 ms / 6.2-6.2 ms | 0.0 ms / 20.4 ms / 19.3-20.4 ms | 0.1 ms / 31.0 ms / 29.9-31.0 ms | 0.0 ms / 7.0 ms / 6.8-7.0 ms | 0.0 ms / 7.0 ms / 5.9-7.0 ms | 0.0 ms / 8.1 ms / 6.8-8.1 ms |
| First output | 3.3 ms | 19.1 ms | 30.6 ms | 6.5 ms | 4.4 ms | 6.5 ms |
| First output dispersion | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.0 ms / 20.2 ms / 19.1-20.2 ms | 0.0 ms / 30.7 ms / 29.5-30.7 ms | 0.0 ms / 6.5 ms / 6.4-6.5 ms | 0.0 ms / 5.4 ms / 4.3-5.4 ms | 0.0 ms / 7.5 ms / 6.5-7.5 ms |
| User CPU | 4.8 ms | 21.8 ms | 30.8 ms | 4.1 ms | 3.2 ms | 4.6 ms |
| System CPU | 0.0 ms | 3.8 ms | 13.4 ms | 2.0 ms | 2.1 ms | 1.8 ms |
| Peak RSS | 3.4 MiB | 25.5 MiB | 37.2 MiB | 22.8 MiB | 17.0 MiB | 22.5 MiB |
| Logical throughput | 39272.5 records/s | 12643.1 records/s | 7892.9 records/s | 34877.1 records/s | 40946.5 records/s | 35630.8 records/s |
| Physical throughput | 26.8 MiB/s | 8.6 MiB/s | 5.4 MiB/s | 23.8 MiB/s | 27.9 MiB/s | 28.7 MiB/s |
| Output bytes | 266477.0 B | 174153.0 B | 174153.0 B | 206385.0 B | 206385.0 B | 206385.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | 3.2 ms | 3.5 ms | 1.9 ms | 1.8 ms | 1.8 ms |
| Wall time dispersion | 0.0 ms / 1.2 ms / 1.1-1.2 ms | 0.0 ms / 3.3 ms / 3.1-3.3 ms | 0.0 ms / 3.7 ms / 3.5-3.7 ms | 0.1 ms / 2.1 ms / 1.8-2.1 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms |
| First output | 1.2 ms | 3.2 ms | 3.3 ms | 1.9 ms | 1.8 ms | 1.8 ms |
| First output dispersion | 0.0 ms / 1.2 ms / 1.1-1.2 ms | 0.0 ms / 3.3 ms / 3.1-3.3 ms | 0.0 ms / 3.7 ms / 3.3-3.7 ms | 0.1 ms / 2.1 ms / 1.8-2.1 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms |
| User CPU | 1.1 ms | 2.2 ms | 1.9 ms | 0.0 ms | 0.9 ms | 0.0 ms |
| System CPU | 0.0 ms | 1.1 ms | 1.8 ms | 1.9 ms | 0.9 ms | 1.7 ms |
| Peak RSS | 2.7 MiB | 13.5 MiB | 14.1 MiB | 15.0 MiB | 15.3 MiB | 14.6 MiB |
| Logical throughput | 5988.0 records/s | 2208.9 records/s | 1988.6 records/s | 3651.5 records/s | 3897.6 records/s | 3921.6 records/s |
| Physical throughput | 4.3 MiB/s | 1.6 MiB/s | 1.4 MiB/s | 2.6 MiB/s | 2.8 MiB/s | 3.3 MiB/s |
| Output bytes | 8052.0 B | 5314.0 B | 5314.0 B | 6239.0 B | 6239.0 B | 6239.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 267.7 ms | 721.9 ms | 1260.7 ms | 189.1 ms | 203.5 ms | 211.2 ms |
| Wall time dispersion | 0.3 ms / 269.5 ms / 267.4-269.5 ms | 4.8 ms / 726.8 ms / 714.3-726.8 ms | 0.9 ms / 1261.7 ms / 1243.5-1261.7 ms | 1.7 ms / 191.5 ms / 187.4-191.5 ms | 0.0 ms / 205.4 ms / 203.5-205.4 ms | 0.1 ms / 211.3 ms / 210.4-211.3 ms |
| First output | 78.1 ms | 688.1 ms | 1173.8 ms | 183.4 ms | 111.8 ms | 205.5 ms |
| First output dispersion | 0.0 ms / 80.2 ms / 78.0-80.2 ms | 1.6 ms / 689.7 ms / 677.7-689.7 ms | 7.7 ms / 1181.5 ms / 1165.7-1181.5 ms | 1.2 ms / 186.3 ms / 182.2-186.3 ms | 0.2 ms / 113.9 ms / 111.6-113.9 ms | 0.0 ms / 206.5 ms / 205.4-206.5 ms |
| User CPU | 154.4 ms | 748.4 ms | 1641.9 ms | 180.9 ms | 142.7 ms | 202.5 ms |
| System CPU | 27.1 ms | 194.3 ms | 468.7 ms | 6.9 ms | 8.8 ms | 4.1 ms |
| Peak RSS | 61.1 MiB | 460.1 MiB | 1042.6 MiB | 67.0 MiB | 117.0 MiB | 66.4 MiB |
| Logical throughput | 40314.1 records/s | 14949.0 records/s | 8560.1 records/s | 57072.4 records/s | 53027.0 records/s | 51106.5 records/s |
| Physical throughput | 27.3 MiB/s | 10.1 MiB/s | 5.8 MiB/s | 38.7 MiB/s | 35.9 MiB/s | 41.0 MiB/s |
| Output bytes | 11733774.0 B | 7654306.0 B | 7654306.0 B | 9079914.0 B | 9079914.0 B | 9079914.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 55.4 ms | 154.0 ms | 262.4 ms | 40.6 ms | 44.1 ms | 45.9 ms |
| Wall time dispersion | 0.6 ms / 55.9 ms / 54.7-55.9 ms | 1.7 ms / 155.7 ms / 149.3-155.7 ms | 0.3 ms / 262.7 ms / 257.3-262.7 ms | 0.1 ms / 41.7 ms / 40.4-41.7 ms | 0.1 ms / 44.2 ms / 42.8-44.2 ms | 1.0 ms / 46.9 ms / 44.7-46.9 ms |
| First output | 17.0 ms | 145.8 ms | 245.0 ms | 39.2 ms | 25.4 ms | 44.4 ms |
| First output dispersion | 0.0 ms / 17.0 ms / 16.9-17.0 ms | 1.5 ms / 147.3 ms / 141.4-147.3 ms | 0.4 ms / 245.4 ms / 241.1-245.4 ms | 0.1 ms / 40.2 ms / 39.1-40.2 ms | 0.1 ms / 25.5 ms / 24.4-25.5 ms | 1.0 ms / 45.4 ms / 43.3-45.4 ms |
| User CPU | 32.0 ms | 169.7 ms | 326.8 ms | 37.7 ms | 29.2 ms | 42.4 ms |
| System CPU | 4.8 ms | 43.5 ms | 104.6 ms | 2.0 ms | 2.2 ms | 2.0 ms |
| Peak RSS | 14.2 MiB | 106.1 MiB | 218.5 MiB | 33.0 MiB | 33.0 MiB | 32.7 MiB |
| Logical throughput | 40357.7 records/s | 14506.5 records/s | 8514.1 records/s | 55066.7 records/s | 50651.9 records/s | 48715.6 records/s |
| Physical throughput | 27.4 MiB/s | 9.8 MiB/s | 5.8 MiB/s | 37.3 MiB/s | 34.3 MiB/s | 39.1 MiB/s |
| Output bytes | 2430851.0 B | 1586307.0 B | 1586307.0 B | 1881638.0 B | 1881638.0 B | 1881638.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
