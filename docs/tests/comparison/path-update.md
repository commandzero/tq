---
type: Report
title: Path update
description: Measures updating one nested object while returning the full document.
workload: benchmark.path-update
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

# Path update

## What this measures

The update query adds `benchmarked: true` inside `.metadata` and emits the
complete document. It exercises path focus, object update, and full-document
serialization.

## Why it matters

Enriching records in place is a normal ETL operation. The work includes both
finding a nested path and carrying the untouched parts of the document through.

## Input and output

The input is a natural snapshot with a metadata object. The output is the whole
snapshot with one metadata field added, not just the changed metadata value.

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
| Wall time | 6.2 ms | 20.4 ms | 31.0 ms | 5.0 ms | 6.1 ms | 7.1 ms |
| Wall time dispersion | 0.1 ms / 6.7 ms / 6.1-6.7 ms | 0.0 ms / 20.4 ms / 20.3-20.4 ms | 0.0 ms / 31.8 ms / 31.0-31.8 ms | 0.1 ms / 5.9 ms / 4.9-5.9 ms | 0.0 ms / 6.1 ms / 6.0-6.1 ms | 0.0 ms / 7.1 ms / 7.1-7.1 ms |
| First output | 3.3 ms | 20.1 ms | 30.6 ms | 3.3 ms | 4.4 ms | 5.4 ms |
| First output dispersion | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.0 ms / 20.2 ms / 20.1-20.2 ms | 0.1 ms / 30.7 ms / 29.6-30.7 ms | 0.0 ms / 4.3 ms / 3.3-4.3 ms | 0.0 ms / 4.4 ms / 4.3-4.4 ms | 0.0 ms / 5.4 ms / 5.4-5.4 ms |
| User CPU | 3.6 ms | 19.4 ms | 30.5 ms | 2.7 ms | 3.0 ms | 4.3 ms |
| System CPU | 1.3 ms | 6.1 ms | 12.8 ms | 1.3 ms | 2.0 ms | 1.1 ms |
| Peak RSS | 3.4 MiB | 26.2 MiB | 37.4 MiB | 15.0 MiB | 17.7 MiB | 17.6 MiB |
| Logical throughput | 39259.9 records/s | 11980.2 records/s | 7864.4 records/s | 48848.8 records/s | 40224.2 records/s | 34250.4 records/s |
| Physical throughput | 26.8 MiB/s | 8.2 MiB/s | 5.4 MiB/s | 33.3 MiB/s | 27.4 MiB/s | 27.6 MiB/s |
| Output bytes | 266502.0 B | 174172.0 B | 174172.0 B | 206405.0 B | 206405.0 B | 206405.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.3 ms | 3.5 ms | 3.6 ms | 1.9 ms | 1.9 ms | 2.0 ms |
| Wall time dispersion | 0.0 ms / 1.5 ms / 1.3-1.5 ms | 0.0 ms / 3.5 ms / 3.3-3.5 ms | 0.1 ms / 4.1 ms / 3.5-4.1 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms |
| First output | 1.3 ms | 3.3 ms | 3.3 ms | 1.9 ms | 1.9 ms | 2.0 ms |
| First output dispersion | 0.0 ms / 1.5 ms / 1.3-1.5 ms | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.0 ms / 4.1 ms / 3.3-4.1 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms |
| User CPU | 0.6 ms | 1.1 ms | 1.2 ms | 1.8 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | 2.3 ms | 2.4 ms | 0.0 ms | 1.9 ms | 1.8 ms |
| Peak RSS | 2.7 MiB | 13.5 MiB | 14.3 MiB | 15.1 MiB | 15.5 MiB | 15.4 MiB |
| Logical throughput | 5279.0 records/s | 2026.0 records/s | 1949.9 records/s | 3693.9 records/s | 3608.2 records/s | 3584.2 records/s |
| Physical throughput | 3.8 MiB/s | 1.5 MiB/s | 1.4 MiB/s | 2.7 MiB/s | 2.6 MiB/s | 3.0 MiB/s |
| Output bytes | 8077.0 B | 5333.0 B | 5333.0 B | 6259.0 B | 6259.0 B | 6259.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 268.2 ms | 722.1 ms | 1220.2 ms | 159.2 ms | 202.5 ms | 210.3 ms |
| Wall time dispersion | 0.9 ms / 270.0 ms / 267.3-270.0 ms | 1.3 ms / 723.4 ms / 715.7-723.4 ms | 4.5 ms / 1224.7 ms / 1198.5-1224.7 ms | 0.2 ms / 160.0 ms / 159.1-160.0 ms | 0.0 ms / 202.5 ms / 202.2-202.5 ms | 0.0 ms / 215.5 ms / 210.2-215.5 ms |
| First output | 79.2 ms | 683.6 ms | 1147.5 ms | 68.6 ms | 110.5 ms | 117.0 ms |
| First output dispersion | 1.0 ms / 80.2 ms / 77.8-80.2 ms | 0.2 ms / 685.1 ms / 683.4-685.1 ms | 5.0 ms / 1152.6 ms / 1127.9-1152.6 ms | 0.0 ms / 69.5 ms / 68.6-69.5 ms | 0.0 ms / 111.7 ms / 110.5-111.7 ms | 1.2 ms / 123.3 ms / 115.8-123.3 ms |
| User CPU | 151.0 ms | 753.2 ms | 1651.7 ms | 102.5 ms | 142.0 ms | 156.5 ms |
| System CPU | 32.1 ms | 201.9 ms | 420.9 ms | 6.2 ms | 8.0 ms | 3.9 ms |
| Peak RSS | 60.9 MiB | 461.0 MiB | 964.1 MiB | 73.1 MiB | 117.9 MiB | 97.4 MiB |
| Logical throughput | 40235.2 records/s | 14944.8 records/s | 8844.4 records/s | 67782.1 records/s | 53302.3 records/s | 51327.7 records/s |
| Physical throughput | 27.3 MiB/s | 10.1 MiB/s | 6.0 MiB/s | 45.9 MiB/s | 36.1 MiB/s | 41.2 MiB/s |
| Output bytes | 11733799.0 B | 7654325.0 B | 7654325.0 B | 9079934.0 B | 9079934.0 B | 9079934.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 55.0 ms | 152.4 ms | 259.5 ms | 34.2 ms | 42.8 ms | 44.1 ms |
| Wall time dispersion | 0.0 ms / 55.3 ms / 55.0-55.3 ms | 1.7 ms / 154.5 ms / 150.7-154.5 ms | 4.8 ms / 264.3 ms / 252.2-264.3 ms | 0.1 ms / 35.2 ms / 34.1-35.2 ms | 0.0 ms / 42.8 ms / 42.7-42.8 ms | 0.2 ms / 44.9 ms / 44.0-44.9 ms |
| First output | 17.9 ms | 143.8 ms | 242.9 ms | 16.0 ms | 24.4 ms | 25.5 ms |
| First output dispersion | 0.1 ms / 18.0 ms / 17.0-18.0 ms | 1.6 ms / 145.4 ms / 142.1-145.4 ms | 4.0 ms / 247.6 ms / 238.9-247.6 ms | 0.0 ms / 16.9 ms / 15.9-16.9 ms | 0.0 ms / 24.4 ms / 24.3-24.4 ms | 0.0 ms / 26.5 ms / 25.5-26.5 ms |
| User CPU | 29.1 ms | 169.1 ms | 326.6 ms | 21.3 ms | 29.0 ms | 29.1 ms |
| System CPU | 8.0 ms | 42.2 ms | 104.1 ms | 1.9 ms | 2.0 ms | 4.0 ms |
| Peak RSS | 14.3 MiB | 106.6 MiB | 227.7 MiB | 29.0 MiB | 33.6 MiB | 33.2 MiB |
| Logical throughput | 40618.2 records/s | 14656.6 records/s | 8608.4 records/s | 65340.7 records/s | 52213.3 records/s | 50640.4 records/s |
| Physical throughput | 27.5 MiB/s | 9.9 MiB/s | 5.8 MiB/s | 44.3 MiB/s | 35.4 MiB/s | 40.7 MiB/s |
| Output bytes | 2430876.0 B | 1586326.0 B | 1586326.0 B | 1881658.0 B | 1881658.0 B | 1881658.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
