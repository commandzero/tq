---
type: Report
title: "Regular-expression testing"
description: "Measures Unicode-aware pattern testing over feature place names."
workload: benchmark.regex-test
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Regular-expression testing

## What this measures

The query tests each string place value against `^[A-Z]` and counts the
matches. It combines string filtering, a regular expression, and a numeric
reduction.

## Why it matters

Pattern checks sit inside log filters, validation rules, and text cleanup jobs.
This case measures the work of testing every candidate rather than finding one
match and stopping.

## Input and output

The input is a natural feature snapshot with place strings and other values.
The output is one integer containing the number of strings that start with an
ASCII uppercase letter.

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
| Wall (ms) | 23.598 | not measured | not measured | 25.375 | 23.317 | 23.509 |
| Wall dispersion (MAD / p95 / range) | 0.154 / 24.514 / 21.518-24.514 | not measured | not measured | 0.585 / 26.374 / 22.827-26.374 | 0.558 / 24.519 / 22.276-24.519 | 0.315 / 24.251 / 23.068-24.251 |
| First output (ms) | 23.584 | not measured | not measured | 25.356 | 23.303 | 23.491 |
| First output dispersion (MAD / p95 / range) | 0.154 / 24.503 / 21.505-24.503 | not measured | not measured | 0.584 / 26.356 / 22.809-26.356 | 0.560 / 24.501 / 22.261-24.501 | 0.315 / 24.236 / 23.052-24.236 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.48 (gnu-time-v) | not measured | not measured | 10.19 (gnu-time-v) | 10.39 (gnu-time-v) | 9.96 (gnu-time-v) |
| Logical throughput (records/s) | 8221.210 | not measured | not measured | 7645.471 | 8319.931 | 8252.334 |
| Physical throughput (MiB/s) | 5.628 | not measured | not measured | 5.234 | 5.688 | 6.680 |
| Output bytes | 4 | not measured | not measured | 4 | 4 | 4 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq strings/test expression. | yq rejects the catalog jq strings/test expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.660 | not measured | not measured | 23.647 | 23.454 | 23.083 |
| Wall dispersion (MAD / p95 / range) | 0.481 / 25.627 / 22.374-25.789 | not measured | not measured | 0.348 / 24.303 / 22.403-24.480 | 0.464 / 24.077 / 21.787-24.128 | 0.394 / 24.495 / 21.844-24.598 |
| First output (ms) | 23.644 | not measured | not measured | 23.634 | 23.438 | 23.069 |
| First output dispersion (MAD / p95 / range) | 0.482 / 25.614 / 22.359-25.771 | not measured | not measured | 0.350 / 24.282 / 22.387-24.469 | 0.463 / 24.063 / 21.771-24.114 | 0.393 / 24.477 / 21.829-24.587 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.14 (gnu-time-v) | not measured | not measured | 9.04 (gnu-time-v) | 9.24 (gnu-time-v) | 8.90 (gnu-time-v) |
| Logical throughput (records/s) | 84.529 | not measured | not measured | 84.577 | 85.271 | 86.644 |
| Physical throughput (MiB/s) | 0.072 | not measured | not measured | 0.072 | 0.072 | 0.084 |
| Output bytes | 2 | not measured | not measured | 2 | 2 | 2 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq strings/test expression. | yq rejects the catalog jq strings/test expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 136.136 | not measured | not measured | 249.018 | 288.128 | 249.071 |
| Wall dispersion (MAD / p95 / range) | 0.749 / 139.069 / 135.213-139.069 | not measured | not measured | 1.085 / 250.383 / 246.114-250.383 | 1.230 / 292.105 / 284.007-292.105 | 1.449 / 252.081 / 245.562-252.081 |
| First output (ms) | 111.986 | not measured | not measured | 220.512 | 265.039 | 237.630 |
| First output dispersion (MAD / p95 / range) | 1.199 / 113.846 / 110.094-113.846 | not measured | not measured | 0.655 / 221.984 / 219.099-221.984 | 1.738 / 269.276 / 262.067-269.276 | 0.967 / 242.134 / 235.818-242.134 |
| User CPU (ms) | 80.000 | not captured | not captured | 190.000 | 230.000 | 210.000 |
| System CPU (ms) | 20.000 | not captured | not captured | 20.000 | 30.000 | 20.000 |
| Peak RSS (MiB, source) | 60.73 (gnu-time-v) | not measured | not measured | 71.22 (gnu-time-v) | 91.10 (gnu-time-v) | 79.79 (gnu-time-v) |
| Logical throughput (records/s) | 82814.245 | not measured | not measured | 45273.927 | 39128.443 | 45264.293 |
| Physical throughput (MiB/s) | 56.135 | not measured | not measured | 30.688 | 26.486 | 36.340 |
| Output bytes | 6 | not measured | not measured | 6 | 6 | 6 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq strings/test expression. | yq rejects the catalog jq strings/test expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 41.917 | not measured | not measured | 60.934 | 62.498 | 61.678 |
| Wall dispersion (MAD / p95 / range) | 17.959 / 60.788 / 23.352-60.788 | not measured | not measured | 0.662 / 61.746 / 59.364-61.746 | 0.573 / 63.689 / 61.211-63.689 | 0.648 / 62.954 / 60.800-62.954 |
| First output (ms) | 23.536 | not measured | not measured | 46.008 | 52.980 | 48.906 |
| First output dispersion (MAD / p95 / range) | 0.758 / 60.543 / 21.926-60.543 | not measured | not measured | 0.727 / 48.053 / 44.885-48.053 | 0.293 / 56.321 / 52.155-56.321 | 0.519 / 51.507 / 48.351-51.507 |
| User CPU (ms) | 10.000 | not captured | not captured | 30.000 | 40.000 | 40.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.85 (gnu-time-v) | not measured | not measured | 21.20 (gnu-time-v) | 25.12 (gnu-time-v) | 22.90 (gnu-time-v) |
| Logical throughput (records/s) | 53081.089 | not measured | not measured | 36514.918 | 35600.854 | 36074.159 |
| Physical throughput (MiB/s) | 35.967 | not measured | not measured | 24.742 | 24.089 | 28.951 |
| Output bytes | 5 | not measured | not measured | 5 | 5 | 5 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq strings/test expression. | yq rejects the catalog jq strings/test expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
