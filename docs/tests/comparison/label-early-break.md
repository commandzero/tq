---
type: Report
title: "Early break with a label"
description: "Measures stopping a feature traversal after the first result."
workload: benchmark.label-early-break
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Early break with a label

## What this measures

The labeled query emits a feature ID and then breaks to the outer label. It
tests non-local control flow and whether later features are left unread.

## Why it matters

First-match queries should stop work when the answer is known. Labels provide a
way to express that exit when a simple predicate is not enough.

## Input and output

The input is a natural feature snapshot. The output is the early result
sequence, normally the first feature ID, rather than every feature ID in the
document.

## Results

<!-- benchmark-results:start -->
Last updated: 2026-09-11
Profile: `standard` | Status: `observed-failures`

Tools: `jq` (jq-1.8.1), `tq` (tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, 16 logical CPUs, compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Captured values show their source when present; missing values are `not captured`, never estimates.

Compare columns with the same input format to isolate tool differences. Missing adapters are marked `not recorded`.

Timing rows show numeric medians followed by compact MAD / p95 / range rows. CPU, throughput, and output cells use the captured summary values.

### usgs-all-day

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.606 | not measured | not measured | 23.363 | 23.066 | 23.596 |
| Wall dispersion (MAD / p95 / range) | 0.328 / 24.332 / 22.930-24.332 | not measured | not measured | 0.181 / 23.802 / 22.960-23.802 | 0.410 / 24.042 / 22.225-24.042 | 0.241 / 24.561 / 22.952-24.561 |
| First output (ms) | 23.593 | not measured | not measured | 23.349 | 23.055 | 23.581 |
| First output dispersion (MAD / p95 / range) | 0.325 / 24.321 / 22.915-24.321 | not measured | not measured | 0.182 / 23.788 / 22.942-23.788 | 0.407 / 24.024 / 22.206-24.024 | 0.241 / 24.549 / 22.936-24.549 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.25 (gnu-time-v) | not measured | not measured | 9.14 (gnu-time-v) | 9.61 (gnu-time-v) | 8.98 (gnu-time-v) |
| Logical throughput (records/s) | 8218.250 | not measured | not measured | 8303.550 | 8410.648 | 8221.732 |
| Physical throughput (MiB/s) | 5.626 | not measured | not measured | 5.685 | 5.750 | 6.655 |
| Output bytes | 13 | not measured | not measured | 11 | 11 | 11 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq label/break expression. | yq rejects the catalog jq label/break expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.505 | not measured | not measured | 23.355 | 23.273 | 23.534 |
| Wall dispersion (MAD / p95 / range) | 0.435 / 24.312 / 21.967-24.323 | not measured | not measured | 0.342 / 24.277 / 22.292-24.482 | 0.302 / 24.496 / 22.628-24.520 | 0.330 / 23.995 / 22.383-24.082 |
| First output (ms) | 23.491 | not measured | not measured | 23.338 | 23.258 | 23.520 |
| First output dispersion (MAD / p95 / range) | 0.432 / 24.297 / 21.951-24.312 | not measured | not measured | 0.340 / 24.266 / 22.274-24.468 | 0.300 / 24.481 / 22.612-24.504 | 0.330 / 23.979 / 22.368-24.065 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | not measured | not measured | 8.07 (gnu-time-v) | 8.40 (gnu-time-v) | 8.06 (gnu-time-v) |
| Logical throughput (records/s) | 85.088 | not measured | not measured | 85.635 | 85.935 | 84.985 |
| Physical throughput (MiB/s) | 0.072 | not measured | not measured | 0.072 | 0.073 | 0.083 |
| Output bytes | 13 | not measured | not measured | 11 | 11 | 11 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq label/break expression. | yq rejects the catalog jq label/break expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 100.436 | not measured | not measured | 135.022 | 174.734 | 174.181 |
| Wall dispersion (MAD / p95 / range) | 1.756 / 103.578 / 98.467-103.578 | not measured | not measured | 0.800 / 138.996 / 133.531-138.996 | 1.252 / 180.405 / 171.246-180.405 | 1.762 / 178.004 / 169.515-178.004 |
| First output (ms) | 92.152 | not measured | not measured | 120.584 | 162.662 | 137.352 |
| First output dispersion (MAD / p95 / range) | 0.559 / 92.947 / 91.254-92.947 | not measured | not measured | 0.952 / 121.968 / 118.826-121.968 | 0.744 / 164.598 / 161.855-164.598 | 0.477 / 138.503 / 136.248-138.503 |
| User CPU (ms) | 60.000 | not captured | not captured | 90.000 | 130.000 | 110.000 |
| System CPU (ms) | 20.000 | not captured | not captured | 20.000 | 30.000 | 20.000 |
| Peak RSS (MiB, source) | 60.29 (gnu-time-v) | not measured | not measured | 69.35 (gnu-time-v) | 89.38 (gnu-time-v) | 78.10 (gnu-time-v) |
| Logical throughput (records/s) | 112250.029 | not measured | not measured | 83497.813 | 64520.929 | 64725.774 |
| Physical throughput (MiB/s) | 76.088 | not measured | not measured | 56.598 | 43.673 | 51.965 |
| Output bytes | 13 | not measured | not measured | 11 | 11 | 11 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq label/break expression. | yq rejects the catalog jq label/break expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.784 | not measured | not measured | 60.398 | 61.483 | 60.269 |
| Wall dispersion (MAD / p95 / range) | 0.315 / 25.032 / 22.843-25.032 | not measured | not measured | 0.268 / 62.333 / 59.584-62.333 | 0.356 / 62.767 / 60.723-62.767 | 0.470 / 61.273 / 59.504-61.273 |
| First output (ms) | 20.092 | not measured | not measured | 25.634 | 33.589 | 28.893 |
| First output dispersion (MAD / p95 / range) | 0.533 / 23.905 / 19.311-23.905 | not measured | not measured | 0.528 / 62.320 / 24.894-62.320 | 0.228 / 34.477 / 32.228-34.477 | 0.798 / 30.051 / 27.612-30.051 |
| User CPU (ms) | 10.000 | not captured | not captured | 10.000 | 20.000 | 20.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.36 (gnu-time-v) | not measured | not measured | 20.32 (gnu-time-v) | 24.27 (gnu-time-v) | 21.71 (gnu-time-v) |
| Logical throughput (records/s) | 93550.286 | not measured | not measured | 36838.968 | 36188.571 | 36917.818 |
| Physical throughput (MiB/s) | 63.389 | not measured | not measured | 24.962 | 24.487 | 29.628 |
| Output bytes | 13 | not measured | not measured | 11 | 11 | 11 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq label/break expression. | yq rejects the catalog jq label/break expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
