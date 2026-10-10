---
type: Report
title: Native TSV record parsing
description: Measures native TSV parsing and ordered ID emission.
workload: benchmark.native-tsv
generated:
  by: codex
  at: 2026-10-08T15:30:56.147Z
benchmark_runs:
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

# Native TSV record parsing

## What this measures

yq reads TSV with `.[] | .id`; tq reads TSV records with `.id` and raw output.

These are native-format workloads, not the CSV/TSV query-formatting operators
on the natural feature snapshots and not CPU/RSS calibration controls.

## Input and output

The retained campaign uses deterministic TSV fixtures with 8 and
131,072 records. Output is one ID per input record.
The harness checks the ordered result sequence before timing each adapter.

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

### native-rows-131072

| Metric | yq TSV | tq TSV |
| --- | ---: | ---: |
| Wall time | 2256.7 ms | 5059.8 ms |
| Wall time dispersion | 2.7 ms / 2259.4 ms / 2250.2-2259.4 ms | 5.2 ms / 5087.6 ms / 5054.6-5087.6 ms |
| First output | 1676.8 ms | 496.0 ms |
| First output dispersion | 0.4 ms / 1682.4 ms / 1676.3-1682.4 ms | 0.0 ms / 496.0 ms / 492.8-496.0 ms |
| User CPU | 5143.2 ms | 1878.0 ms |
| System CPU | 366.8 ms | 3499.5 ms |
| Peak RSS | 601.6 MiB | 15.0 MiB |
| Logical throughput | 58081.6 records/s | 25904.5 records/s |
| Physical throughput | 1.6 MiB/s | 0.7 MiB/s |
| Output bytes | 806394.0 B | 806394.0 B |
| Outcome | timed | timed |
| Details | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input |

### native-rows-8

| Metric | yq TSV | tq TSV |
| --- | ---: | ---: |
| Wall time | 2.8 ms | 2.1 ms |
| Wall time dispersion | 0.1 ms / 2.9 ms / 2.7-2.9 ms | 0.0 ms / 2.2 ms / 2.1-2.2 ms |
| First output | 2.8 ms | 2.1 ms |
| First output dispersion | 0.1 ms / 2.9 ms / 2.7-2.9 ms | 0.0 ms / 2.2 ms / 2.1-2.2 ms |
| User CPU | 0.0 ms | 1.1 ms |
| System CPU | 2.8 ms | 1.1 ms |
| Peak RSS | 12.4 MiB | 14.9 MiB |
| Logical throughput | 2842.9 records/s | 3800.5 records/s |
| Physical throughput | 0.1 MiB/s | 0.1 MiB/s |
| Output bytes | 16.0 B | 16.0 B |
| Outcome | timed | timed |
| Details | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input |
<!-- benchmark-results:end -->
