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
Last updated: 2026-09-11
Profile: `smoke` | Status: `passed`

Tools: `jq` (jq-1.8.1), `tq` (tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, 16 logical CPUs, compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Captured values show their source when present; missing values are `not captured`, never estimates.

Compare columns with the same input format to isolate tool differences. Missing adapters are marked `not recorded`.

Timing rows show numeric medians followed by compact MAD / p95 / range rows. CPU, throughput, and output cells use the captured summary values.

### startup

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.499 | 23.326 | 23.645 | 23.221 | 23.274 | 23.500 |
| Wall dispersion (MAD / p95 / range) | 0.403 / 24.293 / 22.107-24.781 | 0.268 / 24.533 / 22.132-24.574 | 0.475 / 24.620 / 22.776-24.769 | 0.323 / 23.892 / 22.050-24.184 | 0.475 / 24.469 / 22.279-24.487 | 0.463 / 25.777 / 21.535-26.005 |
| First output (ms) | 23.485 | 23.311 | 23.633 | 23.206 | 23.258 | 23.486 |
| First output dispersion (MAD / p95 / range) | 0.403 / 24.282 / 22.093-24.770 | 0.266 / 24.472 / 22.117-24.561 | 0.477 / 24.609 / 22.761-24.754 | 0.324 / 23.881 / 22.035-24.166 | 0.472 / 24.453 / 22.266-24.475 | 0.458 / 25.762 / 21.520-25.988 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | 11.64 (gnu-time-v) | 11.64 (gnu-time-v) | 7.83 (gnu-time-v) | 8.04 (gnu-time-v) | 7.76 (gnu-time-v) |
| Logical throughput (records/s) | 42.555 | 42.870 | 42.292 | 43.064 | 42.966 | 42.554 |
| Physical throughput (MiB/s) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Output bytes | 11 | 11 | 11 | 9 | 9 | 9 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
