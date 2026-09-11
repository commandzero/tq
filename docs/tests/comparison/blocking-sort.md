---
type: Report
title: "Blocking sort"
description: "Measures collecting and sorting all feature magnitudes."
workload: benchmark.blocking-sort
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Blocking sort

## What this measures

The query collects `.properties.mag` from every feature and applies `sort`. It
must consume the collection before it can emit the sorted array.

## Why it matters

Ranking and threshold preparation are common jobs where input size affects both
memory and time. This is a clear contrast with streaming projection.

## Input and output

The input is a natural feature snapshot. The output is one array of magnitudes
in ascending order, including the collected values rather than the original
feature objects.

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
| Wall (ms) | 23.348 | 24.462 | 25.633 | 23.723 | 23.355 | 23.820 |
| Wall dispersion (MAD / p95 / range) | 0.275 / 24.001 / 22.785-24.001 | 0.431 / 25.035 / 22.600-25.035 | 0.270 / 27.224 / 24.868-27.224 | 0.211 / 24.201 / 22.990-24.201 | 0.292 / 24.230 / 22.177-24.230 | 0.174 / 24.334 / 22.448-24.334 |
| First output (ms) | 23.335 | 24.448 | 25.321 | 23.707 | 23.342 | 23.802 |
| First output dispersion (MAD / p95 / range) | 0.276 / 23.986 / 22.768-23.986 | 0.429 / 25.022 / 22.586-25.022 | 0.481 / 27.201 / 17.955-27.201 | 0.213 / 24.188 / 22.974-24.188 | 0.292 / 24.215 / 22.161-24.215 | 0.178 / 24.319 / 22.429-24.319 |
| User CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 5.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.40 (gnu-time-v) | 18.77 (gnu-time-v) | 32.52 (gnu-time-v) | 8.41 (gnu-time-v) | 9.78 (gnu-time-v) | 7.98 (gnu-time-v) |
| Logical throughput (records/s) | 8309.063 | 7930.830 | 7568.517 | 8177.718 | 8306.750 | 8144.416 |
| Physical throughput (MiB/s) | 5.688 | 5.430 | 5.174 | 5.599 | 5.679 | 6.593 |
| Output bytes | 1424 | 841 | 841 | 846 | 846 | 846 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.256 | 23.465 | 23.221 | 23.203 | 23.377 | 23.340 |
| Wall dispersion (MAD / p95 / range) | 0.429 / 24.354 / 22.337-25.192 | 0.330 / 24.166 / 21.997-24.471 | 0.385 / 24.325 / 22.306-24.352 | 0.507 / 24.325 / 21.585-24.417 | 0.379 / 24.505 / 21.694-25.282 | 0.297 / 24.616 / 21.740-26.409 |
| First output (ms) | 23.239 | 23.450 | 23.207 | 23.189 | 23.364 | 23.328 |
| First output dispersion (MAD / p95 / range) | 0.427 / 24.342 / 22.320-25.177 | 0.329 / 24.151 / 21.982-24.460 | 0.383 / 24.310 / 22.289-24.341 | 0.507 / 24.312 / 21.570-24.402 | 0.379 / 24.486 / 21.679-25.271 | 0.299 / 24.605 / 21.727-26.393 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | 11.89 (gnu-time-v) | 12.02 (gnu-time-v) | 8.36 (gnu-time-v) | 8.53 (gnu-time-v) | 8.02 (gnu-time-v) |
| Logical throughput (records/s) | 85.999 | 85.232 | 86.129 | 86.194 | 85.554 | 85.692 |
| Physical throughput (MiB/s) | 0.073 | 0.072 | 0.073 | 0.073 | 0.072 | 0.083 |
| Output bytes | 19 | 12 | 12 | 15 | 15 | 15 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 139.322 | 337.389 | 944.942 | 172.518 | 215.201 | 170.535 |
| Wall dispersion (MAD / p95 / range) | 1.238 / 141.843 / 136.646-141.843 | 1.067 / 340.177 / 335.238-340.177 | 23.450 / 1007.794 / 919.337-1007.794 | 0.952 / 174.025 / 170.081-174.025 | 2.395 / 218.302 / 211.173-218.302 | 0.555 / 171.885 / 169.270-171.885 |
| First output (ms) | 101.385 | 302.993 | 866.403 | 172.502 | 181.224 | 170.409 |
| First output dispersion (MAD / p95 / range) | 0.549 / 103.569 / 100.396-103.569 | 3.172 / 312.218 / 299.391-312.218 | 32.949 / 916.581 / 828.987-916.581 | 0.953 / 174.008 / 170.064-174.008 | 0.499 / 184.669 / 179.326-184.669 | 1.097 / 171.865 / 143.180-171.865 |
| User CPU (ms) | 75.000 | 355.000 | 1270.000 | 140.000 | 150.000 | 140.000 |
| System CPU (ms) | 20.000 | 120.000 | 425.000 | 0.000 | 30.000 | 0.000 |
| Peak RSS (MiB, source) | 60.90 (gnu-time-v) | 332.33 (gnu-time-v) | 1299.50 (gnu-time-v) | 9.30 (gnu-time-v) | 90.80 (gnu-time-v) | 8.69 (gnu-time-v) |
| Logical throughput (records/s) | 80920.458 | 33415.385 | 11930.897 | 65349.513 | 52388.232 | 66109.790 |
| Physical throughput (MiB/s) | 54.851 | 22.650 | 8.076 | 44.297 | 35.461 | 53.076 |
| Output bytes | 84626 | 50803 | 50803 | 50810 | 50810 | 50810 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 24.195 | 102.688 | 182.570 | 60.048 | 61.434 | 59.580 |
| Wall dispersion (MAD / p95 / range) | 0.200 / 24.785 / 23.142-24.785 | 0.560 / 104.013 / 64.764-104.013 | 0.797 / 220.965 / 180.403-220.965 | 0.383 / 60.536 / 58.231-60.536 | 0.766 / 62.978 / 60.062-62.978 | 0.590 / 61.725 / 58.114-61.725 |
| First output (ms) | 21.215 | 62.196 | 167.638 | 60.032 | 36.186 | 59.565 |
| First output dispersion (MAD / p95 / range) | 0.318 / 24.051 / 20.865-24.051 | 0.507 / 65.550 / 60.341-65.550 | 2.433 / 174.228 / 153.949-174.228 | 0.384 / 60.520 / 58.215-60.520 | 0.706 / 37.445 / 34.605-37.445 | 0.628 / 61.708 / 34.276-61.708 |
| User CPU (ms) | 10.000 | 70.000 | 200.000 | 30.000 | 20.000 | 20.000 |
| System CPU (ms) | 0.000 | 20.000 | 90.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.79 (gnu-time-v) | 75.27 (gnu-time-v) | 235.14 (gnu-time-v) | 8.48 (gnu-time-v) | 24.77 (gnu-time-v) | 8.10 (gnu-time-v) |
| Logical throughput (records/s) | 91959.249 | 21667.681 | 12187.106 | 37053.690 | 36218.024 | 37345.060 |
| Physical throughput (MiB/s) | 62.311 | 14.682 | 8.246 | 25.107 | 24.507 | 29.971 |
| Output bytes | 16581 | 9905 | 9905 | 9911 | 9911 | 9911 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
