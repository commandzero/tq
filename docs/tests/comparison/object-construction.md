---
type: Report
title: Object construction
description: Measures building a lookup object with computed feature IDs as keys.
workload: benchmark.object-construction
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

# Object construction

## What this measures

The query turns the feature collection into an object whose keys come from
`.id` and whose values are magnitudes. It exercises computed keys, repeated
merges, and a blocking object result.

## Why it matters

Lookup maps are a common boundary between raw records and application code.
They also stress a different memory pattern from an output array.

## Input and output

The input is a natural feature snapshot with IDs and magnitudes. The output is
one object mapping each ID to its magnitude. It is not a list of key-value
records.

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
| Wall time | 3.3 ms | 22.0 ms | 36.4 ms | 5.3 ms | 6.1 ms | 6.4 ms |
| Wall time dispersion | 0.1 ms / 3.5 ms / 3.2-3.5 ms | 0.1 ms / 22.1 ms / 21.8-22.1 ms | 0.9 ms / 37.4 ms / 35.5-37.4 ms | 0.0 ms / 5.3 ms / 5.0-5.3 ms | 0.1 ms / 6.4 ms / 6.0-6.4 ms | 0.0 ms / 6.5 ms / 6.4-6.5 ms |
| First output | 3.3 ms | 22.0 ms | 33.9 ms | 5.3 ms | 6.1 ms | 6.4 ms |
| First output dispersion | 0.1 ms / 3.3 ms / 3.2-3.3 ms | 0.1 ms / 22.1 ms / 21.8-22.1 ms | 0.9 ms / 34.8 ms / 32.8-34.8 ms | 0.0 ms / 5.3 ms / 5.0-5.3 ms | 0.1 ms / 6.4 ms / 6.0-6.4 ms | 0.0 ms / 6.5 ms / 6.4-6.5 ms |
| User CPU | 0.8 ms | 28.1 ms | 39.0 ms | 3.0 ms | 3.9 ms | 4.2 ms |
| System CPU | 2.3 ms | 10.2 ms | 19.5 ms | 2.0 ms | 2.0 ms | 2.1 ms |
| Peak RSS | 3.4 MiB | 28.0 MiB | 45.5 MiB | 17.2 MiB | 19.9 MiB | 19.6 MiB |
| Logical throughput | 72966.5 records/s | 11109.1 records/s | 6695.6 records/s | 45735.7 records/s | 40177.8 records/s | 38041.8 records/s |
| Physical throughput | 49.7 MiB/s | 7.6 MiB/s | 4.6 MiB/s | 31.2 MiB/s | 27.3 MiB/s | 30.7 MiB/s |
| Output bytes | 5463.0 B | 4486.0 B | 4486.0 B | 4240.0 B | 4240.0 B | 4240.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.3 ms | 3.1 ms | 3.4 ms | 1.9 ms | 2.3 ms | 2.0 ms |
| Wall time dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | 0.1 ms / 3.4 ms / 3.0-3.4 ms | 0.1 ms / 3.6 ms / 3.3-3.6 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 2.3 ms / 2.0-2.3 ms | 0.0 ms / 2.0 ms / 2.0-2.0 ms |
| First output | 1.3 ms | 3.1 ms | 3.3 ms | 1.9 ms | 2.3 ms | 2.0 ms |
| First output dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | 0.1 ms / 3.4 ms / 3.0-3.4 ms | 0.0 ms / 3.4 ms / 3.3-3.4 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 2.3 ms / 2.0-2.3 ms | 0.0 ms / 2.0 ms / 2.0-2.0 ms |
| User CPU | 0.0 ms | 1.1 ms | 2.3 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 1.1 ms | 2.2 ms | 1.1 ms | 1.8 ms | 1.9 ms | 1.9 ms |
| Peak RSS | 2.7 MiB | 13.6 MiB | 14.0 MiB | 15.0 MiB | 15.9 MiB | 15.4 MiB |
| Logical throughput | 5430.6 records/s | 2229.3 records/s | 2082.1 records/s | 3640.1 records/s | 3102.8 records/s | 3586.1 records/s |
| Physical throughput | 3.9 MiB/s | 1.6 MiB/s | 1.5 MiB/s | 2.6 MiB/s | 2.2 MiB/s | 3.0 MiB/s |
| Output bytes | 160.0 B | 131.0 B | 131.0 B | 122.0 B | 122.0 B | 122.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `process exited with classified error Resource (exit status 5)`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 124.2 ms | 21428.0 ms | 20881.0 ms | - | - | - |
| Wall time dispersion | 0.1 ms / 124.5 ms / 124.2-124.5 ms | 0.2 ms / 21548.9 ms / 21427.9-21548.9 ms | 72.3 ms / 21132.0 ms / 20808.8-21132.0 ms | - | - | - |
| First output | 117.0 ms | 21390.8 ms | 20776.2 ms | - | - | - |
| First output dispersion | 0.0 ms / 118.0 ms / 117.0-118.0 ms | 0.1 ms / 21511.8 ms / 21390.7-21511.8 ms | 71.7 ms / 21028.0 ms / 20704.5-21028.0 ms | - | - | - |
| User CPU | 92.9 ms | 49883.4 ms | 48873.1 ms | - | - | - |
| System CPU | 30.0 ms | 320.3 ms | 734.5 ms | - | - | - |
| Peak RSS | 62.1 MiB | 470.4 MiB | 1310.8 MiB | - | - | - |
| Logical throughput | 86861.3 records/s | 503.6 records/s | 516.8 records/s | - | - | - |
| Physical throughput | 58.8 MiB/s | 0.3 MiB/s | 0.3 MiB/s | - | - | - |
| Output bytes | 241015.0 B | 197846.0 B | 197846.0 B | - | - | - |
| Outcome | timed | timed | timed | resource-limit | resource-limit | resource-limit |
| Details | none | none | none | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | not timed | not timed | not timed |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 22.6 ms | 742.6 ms | 842.4 ms | 108.3 ms | 117.2 ms | 118.2 ms |
| Wall time dispersion | 1.1 ms / 23.6 ms / 20.8-23.6 ms | 3.9 ms / 755.3 ms / 738.8-755.3 ms | 0.1 ms / 842.5 ms / 836.2-842.5 ms | 0.2 ms / 108.6 ms / 107.7-108.6 ms | 1.2 ms / 118.9 ms / 116.0-118.9 ms | 0.5 ms / 120.5 ms / 117.7-120.5 ms |
| First output | 21.2 ms | 733.8 ms | 822.0 ms | 108.3 ms | 116.9 ms | 118.2 ms |
| First output dispersion | 1.1 ms / 23.3 ms / 20.1-23.3 ms | 3.2 ms / 749.4 ms / 730.6-749.4 ms | 0.8 ms / 822.7 ms / 815.7-822.7 ms | 0.2 ms / 108.6 ms / 107.7-108.6 ms | 1.0 ms / 118.1 ms / 116.0-118.1 ms | 0.5 ms / 120.1 ms / 117.7-120.1 ms |
| User CPU | 16.3 ms | 2131.0 ms | 2165.6 ms | 105.0 ms | 115.4 ms | 115.8 ms |
| System CPU | 5.9 ms | 59.1 ms | 140.8 ms | 3.0 ms | 1.0 ms | 2.0 ms |
| Peak RSS | 14.6 MiB | 111.5 MiB | 285.5 MiB | 38.9 MiB | 39.8 MiB | 39.3 MiB |
| Logical throughput | 99051.2 records/s | 3008.3 records/s | 2651.9 records/s | 20625.2 records/s | 19058.7 records/s | 18906.1 records/s |
| Physical throughput | 67.2 MiB/s | 2.0 MiB/s | 1.8 MiB/s | 14.0 MiB/s | 12.9 MiB/s | 15.2 MiB/s |
| Output bytes | 49938.0 B | 41001.0 B | 41001.0 B | 38765.0 B | 38765.0 B | 38765.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
