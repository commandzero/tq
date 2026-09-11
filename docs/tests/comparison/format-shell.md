---
type: Report
title: "Shell quoting"
description: "Measures shell-safe formatting of selected feature fields."
workload: benchmark.format-shell
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Shell quoting

## What this measures

The query selects `id`, `place`, and `mag` for each feature and applies `@sh`.
It exercises quoting and escaping for values that may contain shell-sensitive
characters.

## Why it matters

Generated command arguments need quoting that preserves data instead of letting
the shell interpret it. This is formatting work, not permission to execute the
result.

## Input and output

The input is a natural feature snapshot. The output is one shell-quoted string
per feature containing the selected fields, not a command and not the original
array.

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
| Wall (ms) | 23.420 | not measured | not measured | 23.570 | 24.101 | 23.723 |
| Wall dispersion (MAD / p95 / range) | 0.406 / 24.719 / 22.798-24.719 | not measured | not measured | 0.386 / 24.379 / 22.088-24.379 | 0.547 / 25.134 / 23.277-25.134 | 0.349 / 24.503 / 22.662-24.503 |
| First output (ms) | 23.408 | not measured | not measured | 23.556 | 24.090 | 23.708 |
| First output dispersion (MAD / p95 / range) | 0.406 / 24.703 / 22.779-24.703 | not measured | not measured | 0.385 / 24.363 / 22.073-24.363 | 0.547 / 25.118 / 23.266-25.118 | 0.348 / 24.487 / 22.648-24.487 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.41 (gnu-time-v) | not measured | not measured | 9.12 (gnu-time-v) | 9.65 (gnu-time-v) | 9.00 (gnu-time-v) |
| Logical throughput (records/s) | 8283.518 | not measured | not measured | 8230.976 | 8049.459 | 8177.890 |
| Physical throughput (MiB/s) | 5.671 | not measured | not measured | 5.635 | 5.503 | 6.620 |
| Output bytes | 9958 | not measured | not measured | 9952 | 9952 | 9952 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @sh expression for this array. | yq rejects the catalog jq @sh expression for this array. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.486 | not measured | not measured | 23.919 | 23.842 | 23.497 |
| Wall dispersion (MAD / p95 / range) | 0.612 / 24.769 / 22.591-25.083 | not measured | not measured | 0.344 / 24.791 / 22.609-25.334 | 0.295 / 24.555 / 22.881-24.792 | 0.272 / 24.248 / 22.848-24.558 |
| First output (ms) | 23.472 | not measured | not measured | 23.903 | 23.828 | 23.483 |
| First output dispersion (MAD / p95 / range) | 0.614 / 24.754 / 22.578-25.070 | not measured | not measured | 0.344 / 24.780 / 22.595-25.319 | 0.295 / 24.537 / 22.869-24.766 | 0.273 / 24.231 / 22.834-24.543 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | not measured | not measured | 8.15 (gnu-time-v) | 8.49 (gnu-time-v) | 8.07 (gnu-time-v) |
| Logical throughput (records/s) | 85.155 | not measured | not measured | 83.616 | 83.886 | 85.115 |
| Physical throughput (MiB/s) | 0.072 | not measured | not measured | 0.071 | 0.071 | 0.083 |
| Output bytes | 98 | not measured | not measured | 98 | 98 | 98 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @sh expression for this array. | yq rejects the catalog jq @sh expression for this array. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 137.446 | not measured | not measured | 249.396 | 287.150 | 250.443 |
| Wall dispersion (MAD / p95 / range) | 0.768 / 140.811 / 134.523-140.811 | not measured | not measured | 0.855 / 251.378 / 245.936-251.378 | 1.220 / 296.167 / 284.159-296.167 | 1.214 / 257.475 / 245.716-257.475 |
| First output (ms) | 80.745 | not measured | not measured | 115.174 | 158.125 | 130.976 |
| First output dispersion (MAD / p95 / range) | 0.690 / 83.062 / 79.423-83.062 | not measured | not measured | 1.626 / 117.304 / 111.847-117.304 | 1.036 / 160.886 / 155.694-160.886 | 1.486 / 141.209 / 129.296-141.209 |
| User CPU (ms) | 85.000 | not captured | not captured | 160.000 | 195.000 | 170.000 |
| System CPU (ms) | 20.000 | not captured | not captured | 50.000 | 60.000 | 60.000 |
| Peak RSS (MiB, source) | 60.68 (gnu-time-v) | not measured | not measured | 69.35 (gnu-time-v) | 89.54 (gnu-time-v) | 78.18 (gnu-time-v) |
| Logical throughput (records/s) | 82025.239 | not measured | not measured | 45205.216 | 39261.710 | 45016.231 |
| Physical throughput (MiB/s) | 55.600 | not measured | not measured | 30.642 | 26.576 | 36.141 |
| Output bytes | 576240 | not measured | not measured | 575966 | 575966 | 575966 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @sh expression for this array. | yq rejects the catalog jq @sh expression for this array. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 60.047 | not measured | not measured | 61.187 | 64.272 | 60.937 |
| Wall dispersion (MAD / p95 / range) | 1.228 / 61.466 / 23.604-61.466 | not measured | not measured | 0.694 / 62.186 / 59.675-62.186 | 0.878 / 65.760 / 63.012-65.760 | 0.616 / 62.899 / 59.851-62.899 |
| First output (ms) | 17.957 | not measured | not measured | 34.033 | 42.317 | 37.873 |
| First output dispersion (MAD / p95 / range) | 0.414 / 19.356 / 17.349-19.356 | not measured | not measured | 0.528 / 35.562 / 32.909-35.562 | 0.824 / 44.322 / 40.397-44.322 | 0.583 / 38.672 / 34.707-38.672 |
| User CPU (ms) | 10.000 | not captured | not captured | 30.000 | 30.000 | 30.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 10.000 | 10.000 | 10.000 |
| Peak RSS (MiB, source) | 14.71 (gnu-time-v) | not measured | not measured | 20.21 (gnu-time-v) | 24.52 (gnu-time-v) | 21.96 (gnu-time-v) |
| Logical throughput (records/s) | 37054.307 | not measured | not measured | 36364.231 | 34618.227 | 36513.120 |
| Physical throughput (MiB/s) | 25.108 | not measured | not measured | 24.640 | 23.424 | 29.303 |
| Output bytes | 113994 | not measured | not measured | 113950 | 113950 | 113950 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @sh expression for this array. | yq rejects the catalog jq @sh expression for this array. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
