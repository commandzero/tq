---
type: Report
title: Base64 round trip
description: Measures Base64 encoding followed by decoding for each place string.
workload: benchmark.format-base64-roundtrip
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

# Base64 round trip

## What this measures

The query extracts each place string, applies `@base64`, and decodes it with
`@base64d`. It includes both conversions for every feature.

## Why it matters

Encoding is common when text crosses a binary-safe transport or storage field.
A round trip shows the cost of conversion even when the final value is
unchanged.

## Input and output

The input is a natural feature snapshot. The output is one decoded place string
per feature, so it should have the same semantic text as the selected input.

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
| Wall time | 3.2 ms | 14.0 ms | 25.0 ms | 8.4 ms | 6.4 ms | 6.6 ms |
| Wall time dispersion | 0.1 ms / 3.3 ms / 3.0-3.3 ms | 0.2 ms / 14.2 ms / 13.7-14.2 ms | 0.0 ms / 25.5 ms / 24.9-25.5 ms | 0.0 ms / 8.8 ms / 8.4-8.8 ms | 0.1 ms / 6.5 ms / 6.1-6.5 ms | 0.0 ms / 7.3 ms / 6.6-7.3 ms |
| First output | 3.2 ms | 12.7 ms | 24.4 ms | 8.4 ms | 6.4 ms | 6.6 ms |
| First output dispersion | 0.1 ms / 3.3 ms / 3.0-3.3 ms | 0.0 ms / 12.7 ms / 12.7-12.7 ms | 0.0 ms / 24.4 ms / 24.3-24.4 ms | 0.0 ms / 8.5 ms / 8.4-8.5 ms | 0.1 ms / 6.5 ms / 6.1-6.5 ms | 0.2 ms / 7.3 ms / 6.4-7.3 ms |
| User CPU | 1.1 ms | 11.9 ms | 27.7 ms | 6.6 ms | 3.6 ms | 4.8 ms |
| System CPU | 2.0 ms | 7.6 ms | 11.3 ms | 2.2 ms | 2.4 ms | 1.8 ms |
| Peak RSS | 3.3 MiB | 25.5 MiB | 37.3 MiB | 18.0 MiB | 17.8 MiB | 16.1 MiB |
| Logical throughput | 76585.1 records/s | 17434.8 records/s | 9774.9 records/s | 29075.3 records/s | 38226.5 records/s | 37003.3 records/s |
| Physical throughput | 52.2 MiB/s | 11.9 MiB/s | 6.7 MiB/s | 19.8 MiB/s | 26.0 MiB/s | 29.8 MiB/s |
| Output bytes | 7535.0 B | 7535.0 B | 7535.0 B | 7533.0 B | 7533.0 B | 7533.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.3 ms | 3.2 ms | 3.3 ms | 2.1 ms | 1.9 ms | 1.9 ms |
| Wall time dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | 0.0 ms / 3.6 ms / 3.1-3.6 ms | 0.1 ms / 3.6 ms / 3.3-3.6 ms | 0.1 ms / 2.1 ms / 2.0-2.1 ms | 0.0 ms / 2.4 ms / 1.9-2.4 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms |
| First output | 1.2 ms | 3.2 ms | 3.3 ms | 2.1 ms | 1.9 ms | 1.9 ms |
| First output dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | 0.0 ms / 3.3 ms / 3.1-3.3 ms | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.1 ms / 2.1 ms / 2.0-2.1 ms | 0.0 ms / 2.4 ms / 1.9-2.4 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms |
| User CPU | 1.1 ms | 1.1 ms | 2.3 ms | 1.0 ms | 0.9 ms | 0.0 ms |
| System CPU | 0.0 ms | 2.2 ms | 1.1 ms | 1.0 ms | 1.2 ms | 1.8 ms |
| Peak RSS | 2.7 MiB | 13.5 MiB | 14.0 MiB | 16.3 MiB | 15.7 MiB | 16.1 MiB |
| Logical throughput | 5335.4 records/s | 2215.9 records/s | 2090.8 records/s | 3378.4 records/s | 3597.1 records/s | 3591.6 records/s |
| Physical throughput | 3.9 MiB/s | 1.6 MiB/s | 1.5 MiB/s | 2.4 MiB/s | 2.6 MiB/s | 3.1 MiB/s |
| Output bytes | 222.0 B | 222.0 B | 222.0 B | 222.0 B | 222.0 B | 222.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 100.5 ms | 466.6 ms | 1028.8 ms | 272.4 ms | 189.9 ms | 207.1 ms |
| Wall time dispersion | 0.2 ms / 100.7 ms / 99.9-100.7 ms | 1.4 ms / 468.2 ms / 465.3-468.2 ms | 32.7 ms / 1061.6 ms / 926.5-1061.6 ms | 0.3 ms / 272.7 ms / 270.6-272.7 ms | 0.3 ms / 190.2 ms / 187.5-190.2 ms | 0.9 ms / 209.0 ms / 206.2-209.0 ms |
| First output | 78.0 ms | 373.6 ms | 901.7 ms | 222.2 ms | 124.3 ms | 43.4 ms |
| First output dispersion | 0.0 ms / 79.2 ms / 78.0-79.2 ms | 0.9 ms / 378.7 ms / 372.7-378.7 ms | 0.3 ms / 901.9 ms / 813.8-901.9 ms | 0.0 ms / 222.2 ms / 220.2-222.2 ms | 0.9 ms / 126.5 ms / 123.4-126.5 ms | 1.1 ms / 45.4 ms / 42.3-45.4 ms |
| User CPU | 76.2 ms | 519.4 ms | 1422.4 ms | 267.9 ms | 148.3 ms | 205.5 ms |
| System CPU | 24.1 ms | 183.1 ms | 480.5 ms | 4.0 ms | 40.3 ms | 1.0 ms |
| Peak RSS | 56.6 MiB | 437.3 MiB | 1170.2 MiB | 30.2 MiB | 117.7 MiB | 16.3 MiB |
| Logical throughput | 107354.2 records/s | 23127.4 records/s | 10489.6 records/s | 39624.6 records/s | 56833.2 records/s | 52113.1 records/s |
| Physical throughput | 72.7 MiB/s | 15.7 MiB/s | 7.1 MiB/s | 26.8 MiB/s | 38.4 MiB/s | 41.8 MiB/s |
| Output bytes | 331643.0 B | 331643.0 B | 331643.0 B | 331365.0 B | 331365.0 B | 331365.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.6 ms | 95.0 ms | 211.1 ms | 58.8 ms | 39.8 ms | 44.6 ms |
| Wall time dispersion | 0.4 ms / 21.6 ms / 20.2-21.6 ms | 0.4 ms / 99.4 ms / 94.6-99.4 ms | 11.1 ms / 222.2 ms / 198.1-222.2 ms | 0.4 ms / 59.8 ms / 58.4-59.8 ms | 0.8 ms / 40.7 ms / 39.0-40.7 ms | 0.7 ms / 45.3 ms / 44.0-45.3 ms |
| First output | 18.0 ms | 79.2 ms | 183.9 ms | 58.1 ms | 38.1 ms | 42.3 ms |
| First output dispersion | 0.1 ms / 18.0 ms / 17.0-18.0 ms | 0.0 ms / 79.2 ms / 77.1-79.2 ms | 9.4 ms / 194.7 ms / 174.5-194.7 ms | 0.0 ms / 59.1 ms / 58.0-59.1 ms | 0.2 ms / 39.1 ms / 37.9-39.1 ms | 1.1 ms / 43.3 ms / 41.2-43.3 ms |
| User CPU | 17.4 ms | 110.0 ms | 282.9 ms | 56.5 ms | 30.8 ms | 42.3 ms |
| System CPU | 3.9 ms | 35.1 ms | 108.6 ms | 2.0 ms | 8.9 ms | 2.1 ms |
| Peak RSS | 13.4 MiB | 100.5 MiB | 238.4 MiB | 22.0 MiB | 33.9 MiB | 16.1 MiB |
| Logical throughput | 108299.4 records/s | 23510.8 records/s | 10585.0 records/s | 38010.0 records/s | 56091.2 records/s | 50038.1 records/s |
| Physical throughput | 73.4 MiB/s | 15.9 MiB/s | 7.2 MiB/s | 25.8 MiB/s | 38.0 MiB/s | 40.2 MiB/s |
| Output bytes | 69527.0 B | 69527.0 B | 69527.0 B | 69483.0 B | 69483.0 B | 69483.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
