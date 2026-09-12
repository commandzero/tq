---
type: Report
title: "Base64 startup formatting"
description: "Measures startup cost for a small Base64 formatting query."
workload: benchmark.format-base64-startup
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Base64 startup formatting

## What this measures

The query applies `@base64` to the small synthetic startup helper input. It
combines a short-lived invocation with one formatting operation.

## Why it matters

Encoding a small token or payload is a common CLI task. This page separates
formatting startup cost from the larger streaming and document workloads.

## Input and output

The input is the catalog's small synthetic helper value. The output is one
Base64-encoded string, rather than the original value or a JSON object.

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

### startup

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 2.9 ms | 2.9 ms | 1.2 ms | 1.2 ms | 1.2 ms |
| Wall time dispersion | 0.1 ms / 1.7 ms / 1.3-1.7 ms | 0.1 ms / 3.2 ms / 2.7-3.3 ms | 0.1 ms / 3.2 ms / 2.7-3.2 ms | 0.0 ms / 1.4 ms / 1.1-1.4 ms | 0.0 ms / 1.5 ms / 1.2-1.5 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms |
| First output | 1.3 ms | 2.6 ms | 2.5 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.5 ms / 1.2-1.7 ms | 0.1 ms / 2.9 ms / 2.4-2.9 ms | 0.1 ms / 2.9 ms / 2.4-2.9 ms | 0.0 ms / 1.2 ms / 1.0-1.2 ms | 0.1 ms / 1.3 ms / 0.9-1.4 ms | 0.1 ms / 1.2 ms / 0.9-1.2 ms |
| User CPU | 0.7 ms | 1.0 ms | 1.0 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | 1.9 ms | 2.0 ms | 1.1 ms | 1.0 ms | 1.0 ms |
| Peak RSS | 4.1 MiB | 11.8 MiB | 11.6 MiB | 7.8 MiB | 8.0 MiB | 7.7 MiB |
| Logical throughput | 675.4 records/s | 343.6 records/s | 344.8 records/s | 850.7 records/s | 842.1 records/s | 850.7 records/s |
| Physical throughput | 0.0 MiB/s | 0.0 MiB/s | 0.0 MiB/s | 0.0 MiB/s | 0.0 MiB/s | 0.0 MiB/s |
| Output bytes | 11.0 B | 11.0 B | 11.0 B | 9.0 B | 9.0 B | 9.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
---

### Host: linux / x86_64 / standard

No row was recorded for `benchmark.format-base64-startup` on this host; all measurements are unmeasured (`-`).
<!-- benchmark-results:end -->
