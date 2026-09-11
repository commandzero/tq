---
type: Report
title: "Array construction"
description: "Measures building one array of compact objects from a feature collection."
workload: benchmark.array-construction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Array construction

## What this measures

The query maps every feature to `{id, mag}` and wraps the results in one array.
It measures collection growth and object creation in a blocking pipeline.

## Why it matters

APIs and batch jobs often need a compact array for one downstream request. The
cost includes retaining all projected values until the array is complete.

## Input and output

The input is a natural feature snapshot. The output is one array with one
object per feature, containing only `id` and `mag`, rather than a stream of
individual objects.

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
| Wall (ms) | 26.744 | 24.547 | 26.316 | 22.971 | 23.235 | 23.256 |
| Wall dispersion (MAD / p95 / range) | 0.197 / 27.662 / 23.342-27.662 | 0.469 / 26.164 / 23.805-26.164 | 0.509 / 27.872 / 25.251-27.872 | 0.402 / 24.522 / 21.772-24.522 | 0.389 / 23.903 / 22.665-23.903 | 0.351 / 23.863 / 22.704-23.863 |
| First output (ms) | 26.730 | 24.530 | 26.155 | 22.951 | 23.219 | 23.238 |
| First output dispersion (MAD / p95 / range) | 0.195 / 27.648 / 23.329-27.648 | 0.469 / 26.147 / 23.788-26.147 | 0.425 / 27.851 / 25.034-27.851 | 0.401 / 24.507 / 21.757-24.507 | 0.389 / 23.891 / 22.650-23.891 | 0.352 / 23.853 / 22.687-23.853 |
| User CPU (ms) | 0.000 | 10.000 | 20.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.36 (gnu-time-v) | 22.64 (gnu-time-v) | 33.64 (gnu-time-v) | 9.29 (gnu-time-v) | 9.90 (gnu-time-v) | 9.40 (gnu-time-v) |
| Logical throughput (records/s) | 7253.828 | 7903.367 | 7371.941 | 8445.431 | 8349.473 | 8341.933 |
| Physical throughput (MiB/s) | 4.966 | 5.411 | 5.040 | 5.782 | 5.708 | 6.753 |
| Output bytes | 9562 | 6069 | 6069 | 3560 | 3560 | 3560 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.291 | 23.382 | 23.196 | 23.749 | 23.078 | 23.212 |
| Wall dispersion (MAD / p95 / range) | 0.290 / 24.217 / 21.765-24.299 | 0.377 / 24.522 / 22.502-24.579 | 0.442 / 24.620 / 22.259-24.931 | 0.369 / 24.541 / 22.565-24.545 | 0.374 / 24.487 / 22.007-25.152 | 0.277 / 24.101 / 22.085-24.132 |
| First output (ms) | 23.279 | 23.364 | 23.183 | 23.733 | 23.063 | 23.198 |
| First output dispersion (MAD / p95 / range) | 0.286 / 24.202 / 21.750-24.281 | 0.378 / 24.506 / 22.488-24.563 | 0.444 / 24.606 / 22.242-24.916 | 0.371 / 24.523 / 22.549-24.534 | 0.375 / 24.476 / 21.992-25.135 | 0.277 / 24.087 / 22.069-24.115 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | 12.02 (gnu-time-v) | 12.27 (gnu-time-v) | 8.29 (gnu-time-v) | 8.49 (gnu-time-v) | 7.99 (gnu-time-v) |
| Logical throughput (records/s) | 85.872 | 85.536 | 86.222 | 84.214 | 86.663 | 86.164 |
| Physical throughput (MiB/s) | 0.073 | 0.072 | 0.073 | 0.071 | 0.073 | 0.084 |
| Output bytes | 101 | 64 | 64 | 49 | 49 | 49 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 138.626 | 569.176 | 1119.989 | 176.037 | 215.935 | 175.361 |
| Wall dispersion (MAD / p95 / range) | 0.549 / 139.540 / 136.503-139.540 | 1.143 / 607.350 / 566.673-607.350 | 4.987 / 1192.097 / 1083.344-1192.097 | 0.722 / 179.722 / 174.058-179.722 | 1.538 / 221.616 / 210.691-221.616 | 0.365 / 177.229 / 174.046-177.229 |
| First output (ms) | 104.936 | 532.577 | 1034.692 | 129.426 | 172.523 | 144.398 |
| First output dispersion (MAD / p95 / range) | 0.319 / 105.737 / 102.762-105.737 | 4.822 / 539.467 / 524.243-539.467 | 3.881 / 1105.459 / 1021.605-1105.459 | 0.438 / 131.657 / 127.710-131.657 | 1.196 / 180.271 / 171.248-180.271 | 0.825 / 146.784 / 143.204-146.784 |
| User CPU (ms) | 80.000 | 650.000 | 1730.000 | 120.000 | 160.000 | 130.000 |
| System CPU (ms) | 20.000 | 170.000 | 405.000 | 20.000 | 30.000 | 30.000 |
| Peak RSS (MiB, source) | 64.67 (gnu-time-v) | 474.99 (gnu-time-v) | 1306.37 (gnu-time-v) | 74.12 (gnu-time-v) | 94.23 (gnu-time-v) | 83.05 (gnu-time-v) |
| Logical throughput (records/s) | 81326.442 | 19807.581 | 10066.166 | 64043.355 | 52210.156 | 64290.236 |
| Physical throughput (MiB/s) | 55.126 | 13.426 | 6.814 | 43.411 | 35.340 | 51.615 |
| Output bytes | 555750 | 352817 | 352817 | 206270 | 206270 | 206270 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 24.311 | 141.043 | 223.144 | 61.907 | 61.733 | 60.880 |
| Wall dispersion (MAD / p95 / range) | 0.619 / 59.021 / 23.296-59.021 | 0.997 / 143.061 / 139.974-143.061 | 1.177 / 260.295 / 221.066-260.295 | 0.848 / 64.776 / 61.041-64.776 | 0.391 / 63.227 / 60.497-63.227 | 0.439 / 61.945 / 59.411-61.945 |
| First output (ms) | 21.822 | 109.534 | 206.054 | 31.816 | 37.980 | 34.350 |
| First output dispersion (MAD / p95 / range) | 0.672 / 23.629 / 19.987-23.629 | 1.169 / 111.795 / 106.026-111.795 | 2.006 / 215.117 / 199.808-215.117 | 0.654 / 33.553 / 31.078-33.553 | 0.736 / 38.947 / 35.514-38.947 | 0.235 / 35.901 / 32.550-35.901 |
| User CPU (ms) | 10.000 | 125.000 | 280.000 | 20.000 | 30.000 | 20.000 |
| System CPU (ms) | 0.000 | 30.000 | 90.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 15.45 (gnu-time-v) | 112.27 (gnu-time-v) | 238.28 (gnu-time-v) | 21.28 (gnu-time-v) | 25.40 (gnu-time-v) | 22.71 (gnu-time-v) |
| Logical throughput (records/s) | 91520.474 | 15775.275 | 9971.117 | 35941.299 | 36042.311 | 36547.006 |
| Physical throughput (MiB/s) | 62.013 | 10.689 | 6.747 | 24.354 | 24.388 | 29.331 |
| Output bytes | 110028 | 69977 | 69977 | 41066 | 41066 | 41066 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
