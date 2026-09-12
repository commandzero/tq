---
type: Report
title: "Startup identity"
description: "Measures the fixed cost of launching a query that returns its input."
workload: benchmark.startup
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Startup identity

## What this measures

The query is `.` on a small synthetic helper input. It measures the startup
path and an identity query, including the cost of launching a short-lived
command rather than processing a large document.

## Why it matters

Many scripts invoke a JSON tool for one small value. Startup cost can dominate
that kind of use, so this is a useful baseline for CLI latency.

## Input and output

The input is one small JSON value. The output is the same semantic value and
object order. No filtering or transformation is part of this workload.

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
| Wall time | 1.3 ms | 2.9 ms | 2.8 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| Wall time dispersion | 0.0 ms / 1.7 ms / 1.3-1.7 ms | 0.1 ms / 3.1 ms / 2.7-3.3 ms | 0.1 ms / 3.1 ms / 2.6-3.2 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.2 ms / 1.0-1.3 ms |
| First output | 1.3 ms | 2.4 ms | 2.4 ms | 0.9 ms | 0.9 ms | 0.9 ms |
| First output dispersion | 0.1 ms / 1.5 ms / 1.2-1.5 ms | 0.1 ms / 2.8 ms / 2.2-2.8 ms | 0.1 ms / 2.7 ms / 2.2-2.8 ms | 0.0 ms / 1.2 ms / 0.8-1.2 ms | 0.0 ms / 1.2 ms / 0.8-1.2 ms | 0.0 ms / 1.0 ms / 0.8-1.2 ms |
| User CPU | 1.2 ms | 1.0 ms | 1.4 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | 1.9 ms | 1.5 ms | 0.9 ms | 0.9 ms | 0.9 ms |
| Peak RSS | 4.1 MiB | 10.8 MiB | 11.0 MiB | 7.0 MiB | 7.4 MiB | 7.0 MiB |
| Logical throughput | 741.3 records/s | 346.7 records/s | 362.6 records/s | 970.9 records/s | 976.6 records/s | 984.3 records/s |
| Physical throughput | 0.0 MiB/s | 0.0 MiB/s | 0.0 MiB/s | 0.0 MiB/s | 0.0 MiB/s | 0.0 MiB/s |
| Output bytes | 5.0 B | 5.0 B | 5.0 B | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
---

### Host: linux / x86_64 / standard

No row was recorded for `benchmark.startup` on this host; all measurements are unmeasured (`-`).
<!-- benchmark-results:end -->
