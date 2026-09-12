---
type: Report
title: "Deep object merge"
description: "Measures recursive merge semantics on a pair of helper objects."
workload: benchmark.object-deep-merge
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Deep object merge

## What this measures

The query `.[0] * .[1]` multiplies two helper objects using jq's deep-merge
operator. The workload is document-sized and focuses on recursive object
combination rather than a large corpus scan.

## Why it matters

Configuration overlays and defaults often merge nested objects. A shallow
replacement can silently lose fields, so this case checks the deeper behavior.

## Input and output

The input is a small two-element array containing the objects to merge. The
output is one merged object with nested values combined according to the
operator's semantics.

## Results

<!-- benchmark-results:start -->
### Host: linux / x86_64 / smoke

Last updated: 2026-09-11
Profile: `smoke` | Status: `passed`

Tools: `jq` (jq-1.8.1), `tq` (tq 0.1.0 (TOON v3; jq target 1.8.x; revision a4b4d916-worktree)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, AMD Ryzen 7 7700 8-Core Processor, 16 logical CPUs, 61.9 GiB RAM; kernel `Linux 7.2.0-ogc4.1.fc44.x86_64 #1 SMP PREEMPT_DYNAMIC Thu Aug 20 16:15:37 UTC 2026`; compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Missing or invalid values are `-`, never estimates. RSS collector provenance (outside measurement tables): `linux-wait4`.
Measurement method (outside measurement tables): `tq-bench` native measurement: RSS scope `wait4 child lifetime including pre exec waited descendants and threads`; residual RSS floor `2.5 MiB` retained, not subtracted; observed control excess `1.1 ms`; primary timing is sampler-free.

Compare columns with the same input format to isolate tool differences. Missing adapters have `-` measurement cells and `not recorded` outcome/detail cells.

Timing rows show numeric medians followed by compact MAD / p95 / range rows, with one decimal place and a unit in each measurement cell. CPU, throughput, and output cells use the captured summary values.

### deep-merge

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 42.5 ms | 9954.6 ms | 9410.9 ms | 48.4 ms | 88.1 ms | 48.4 ms |
| Wall time dispersion | 0.5 ms / 44.1 ms / 41.1-44.2 ms | 50.2 ms / 10058.5 ms / 9848.4-10108.5 ms | 58.0 ms / 9595.3 ms / 9242.5-9599.8 ms | 1.1 ms / 50.5 ms / 46.2-50.7 ms | 0.3 ms / 90.8 ms / 86.5-91.1 ms | 0.6 ms / 50.1 ms / 46.4-51.4 ms |
| First output | 25.5 ms | 9940.0 ms | 9391.2 ms | 36.2 ms | 74.5 ms | 35.1 ms |
| First output dispersion | 0.4 ms / 26.3 ms / 24.4-26.5 ms | 50.4 ms / 10044.0 ms / 9834.3-10093.7 ms | 58.2 ms / 9575.2 ms / 9222.9-9579.9 ms | 1.0 ms / 38.1 ms / 33.9-38.1 ms | 0.5 ms / 76.1 ms / 73.6-76.5 ms | 0.6 ms / 36.4 ms / 32.9-37.0 ms |
| User CPU | 31.9 ms | 10312.7 ms | 9695.9 ms | 34.5 ms | 63.1 ms | 32.4 ms |
| System CPU | 10.8 ms | 91.9 ms | 125.6 ms | 14.0 ms | 24.2 ms | 15.8 ms |
| Peak RSS | 26.7 MiB | 203.5 MiB | 277.7 MiB | 36.8 MiB | 56.5 MiB | 38.3 MiB |
| Logical throughput | 47.0 records/s | 0.2 records/s | 0.2 records/s | 41.3 records/s | 22.7 records/s | 41.3 records/s |
| Physical throughput | 32.4 MiB/s | 0.1 MiB/s | 0.2 MiB/s | 28.5 MiB/s | 18.0 MiB/s | 36.7 MiB/s |
| Output bytes | 1565563.0 B | 1025562.0 B | 1025562.0 B | 1075560.0 B | 1075560.0 B | 1075560.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | parser-specific |
---

### Host: linux / x86_64 / standard

No row was recorded for `benchmark.object-deep-merge` on this host; all measurements are unmeasured (`-`).
<!-- benchmark-results:end -->
