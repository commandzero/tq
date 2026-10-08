---
type: Report
title: Native CSV record parsing
description: Measures native CSV parsing and ordered ID emission.
workload: benchmark.native-csv
generated:
  by: codex
  at: 2026-10-08T15:30:56.139Z
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

# Native CSV record parsing

## What this measures

yq reads CSV with `.[] | .id`; tq reads CSV records with `.id` and raw output.

These are native-format workloads, not the CSV/TSV query-formatting operators
on the natural feature snapshots and not CPU/RSS calibration controls.

## Input and output

The retained campaign uses deterministic CSV fixtures with 8 and
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

| Metric | yq CSV | tq CSV |
| --- | ---: | ---: |
| Wall time | 2257.3 ms | 5049.8 ms |
| Wall time dispersion | 0.3 ms / 2257.6 ms / 2247.0-2257.6 ms | 16.0 ms / 5077.5 ms / 5033.8-5077.5 ms |
| First output | 1682.3 ms | 492.6 ms |
| First output dispersion | 0.3 ms / 1682.5 ms / 1674.7-1682.5 ms | 5.4 ms / 498.0 ms / 485.0-498.0 ms |
| User CPU | 5083.9 ms | 1833.5 ms |
| System CPU | 357.8 ms | 3525.4 ms |
| Peak RSS | 598.5 MiB | 15.3 MiB |
| Logical throughput | 58065.7 records/s | 25956.1 records/s |
| Physical throughput | 1.6 MiB/s | 0.7 MiB/s |
| Output bytes | 806394.0 B | 806394.0 B |
| Outcome | timed | timed |
| Details | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input |

### native-rows-8

| Metric | yq CSV | tq CSV |
| --- | ---: | ---: |
| Wall time | 2.7 ms | 2.0 ms |
| Wall time dispersion | 0.1 ms / 3.0 ms / 2.6-3.0 ms | 0.1 ms / 2.1 ms / 2.0-2.1 ms |
| First output | 2.7 ms | 2.0 ms |
| First output dispersion | 0.1 ms / 3.0 ms / 2.6-3.0 ms | 0.1 ms / 2.1 ms / 2.0-2.1 ms |
| User CPU | 1.3 ms | 1.0 ms |
| System CPU | 1.3 ms | 1.0 ms |
| Peak RSS | 12.8 MiB | 14.8 MiB |
| Logical throughput | 2994.0 records/s | 3937.0 records/s |
| Physical throughput | 0.1 MiB/s | 0.1 MiB/s |
| Output bytes | 16.0 B | 16.0 B |
| Outcome | timed | timed |
| Details | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input |
<!-- benchmark-results:end -->
