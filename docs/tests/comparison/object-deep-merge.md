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
Last updated: 2026-09-11
Profile: `smoke` | Status: `passed`

Tools: `jq` (jq-1.8.1), `tq` (tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, 16 logical CPUs, compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Captured values show their source when present; missing values are `not captured`, never estimates.

Compare columns with the same input format to isolate tool differences. Missing adapters are marked `not recorded`.

Timing rows show numeric medians followed by compact MAD / p95 / range rows. CPU, throughput, and output cells use the captured summary values.

### deep-merge

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 63.316 | 9995.119 | 9438.673 | 62.849 | 101.203 | 63.615 |
| Wall dispersion (MAD / p95 / range) | 0.617 / 64.934 / 61.344-65.032 | 34.057 / 10129.668 / 9880.376-10155.135 | 63.106 / 9549.777 / 9267.133-9558.469 | 0.512 / 64.607 / 61.087-64.658 | 0.746 / 102.902 / 99.108-103.248 | 0.486 / 64.570 / 61.903-64.841 |
| First output (ms) | 26.669 | 9968.150 | 9392.370 | 38.500 | 75.352 | 35.995 |
| First output dispersion (MAD / p95 / range) | 0.384 / 27.664 / 25.389-27.925 | 32.935 / 10100.958 / 9858.194-10107.746 | 60.107 / 9520.396 / 9236.199-9528.590 | 0.823 / 39.557 / 37.116-39.909 | 0.595 / 76.623 / 73.051-77.502 | 0.597 / 37.651 / 34.104-38.679 |
| User CPU (ms) | 30.000 | 10330.000 | 9690.000 | 30.000 | 60.000 | 30.000 |
| System CPU (ms) | 5.000 | 90.000 | 110.000 | 10.000 | 20.000 | 10.000 |
| Peak RSS (MiB, source) | 26.73 (gnu-time-v) | 203.39 (gnu-time-v) | 277.20 (gnu-time-v) | 36.90 (gnu-time-v) | 56.65 (gnu-time-v) | 38.40 (gnu-time-v) |
| Logical throughput (records/s) | 31.587 | 0.200 | 0.212 | 31.822 | 19.762 | 31.439 |
| Physical throughput (MiB/s) | 21.773 | 0.138 | 0.168 | 21.935 | 15.695 | 27.967 |
| Output bytes | 1565563 | 1025562 | 1025562 | 1075560 | 1075560 | 1075560 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | parser-specific |
<!-- benchmark-results:end -->
