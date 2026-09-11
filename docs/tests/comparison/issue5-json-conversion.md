---
type: Report
title: "JSON round trip"
description: "Measures converting metadata to JSON text and parsing it back."
workload: benchmark.issue5-json-conversion
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# JSON round trip

## What this measures

The query serializes `.metadata` with `tojson`, parses it with `fromjson`, and
returns the resulting object's length. It includes both text conversion steps.

## Why it matters

Applications cross JSON text boundaries when they log, cache, or hand data to
another process. A round trip can cost more than an in-memory field read.

## Input and output

The input is a metadata object from a natural snapshot. The output is one
integer for the number of members after the round trip, not the JSON text.

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
| Wall (ms) | 23.317 | 23.724 | 25.037 | 22.749 | 23.264 | 23.171 |
| Wall dispersion (MAD / p95 / range) | 0.371 / 24.128 / 21.940-24.128 | 0.452 / 24.400 / 22.552-24.400 | 0.276 / 25.657 / 24.480-25.657 | 0.325 / 23.423 / 22.173-23.423 | 0.329 / 24.008 / 22.611-24.008 | 0.480 / 24.046 / 22.341-24.046 |
| First output (ms) | 23.302 | 23.708 | 24.768 | 22.735 | 23.250 | 23.155 |
| First output dispersion (MAD / p95 / range) | 0.373 / 24.111 / 21.925-24.111 | 0.453 / 24.388 / 22.536-24.388 | 0.657 / 25.639 / 18.445-25.639 | 0.326 / 23.410 / 22.156-23.410 | 0.328 / 23.990 / 22.600-23.990 | 0.482 / 24.028 / 22.325-24.028 |
| User CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 5.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.40 (gnu-time-v) | 18.39 (gnu-time-v) | 32.77 (gnu-time-v) | 9.20 (gnu-time-v) | 9.73 (gnu-time-v) | 9.15 (gnu-time-v) |
| Logical throughput (records/s) | 8319.931 | 8177.201 | 7748.687 | 8527.847 | 8339.065 | 8372.535 |
| Physical throughput (MiB/s) | 5.696 | 5.598 | 5.298 | 5.838 | 5.701 | 6.777 |
| Output bytes | 2 | 2 | 2 | 2 | 2 | 2 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.343 | 23.232 | 23.363 | 23.384 | 23.067 | 23.193 |
| Wall dispersion (MAD / p95 / range) | 0.296 / 24.125 / 22.233-24.146 | 0.263 / 23.851 / 21.660-24.044 | 0.323 / 24.687 / 22.598-24.738 | 0.253 / 24.230 / 22.336-24.336 | 0.378 / 24.395 / 21.996-25.057 | 0.328 / 23.922 / 22.554-24.223 |
| First output (ms) | 23.330 | 23.218 | 23.348 | 23.369 | 23.051 | 23.178 |
| First output dispersion (MAD / p95 / range) | 0.296 / 24.107 / 22.218-24.130 | 0.262 / 23.833 / 21.644-24.032 | 0.323 / 24.674 / 22.583-24.725 | 0.253 / 24.219 / 22.326-24.322 | 0.376 / 24.376 / 21.981-25.044 | 0.328 / 23.904 / 22.539-24.205 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.02 (gnu-time-v) | 12.14 (gnu-time-v) | 12.02 (gnu-time-v) | 8.29 (gnu-time-v) | 8.40 (gnu-time-v) | 8.02 (gnu-time-v) |
| Logical throughput (records/s) | 85.677 | 86.088 | 85.604 | 85.530 | 86.706 | 86.233 |
| Physical throughput (MiB/s) | 0.073 | 0.073 | 0.072 | 0.072 | 0.073 | 0.084 |
| Output bytes | 2 | 2 | 2 | 2 | 2 | 2 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 100.061 | 258.304 | 932.736 | 136.829 | 174.562 | 174.827 |
| Wall dispersion (MAD / p95 / range) | 0.761 / 103.979 / 97.898-103.979 | 1.588 / 296.140 / 255.386-296.140 | 23.218 / 968.176 / 852.653-968.176 | 2.279 / 139.642 / 133.860-139.642 | 1.179 / 177.892 / 172.677-177.892 | 1.290 / 176.194 / 138.597-176.194 |
| First output (ms) | 92.015 | 240.237 | 848.826 | 120.069 | 161.042 | 135.398 |
| First output dispersion (MAD / p95 / range) | 0.434 / 93.043 / 90.934-93.043 | 1.167 / 248.020 / 237.710-248.020 | 9.701 / 868.517 / 784.530-868.517 | 1.118 / 121.748 / 118.413-121.748 | 0.927 / 164.463 / 158.139-164.463 | 0.697 / 136.756 / 134.699-136.756 |
| User CPU (ms) | 70.000 | 230.000 | 1170.000 | 90.000 | 120.000 | 105.000 |
| System CPU (ms) | 20.000 | 100.000 | 465.000 | 20.000 | 30.000 | 25.000 |
| Peak RSS (MiB, source) | 60.18 (gnu-time-v) | 293.32 (gnu-time-v) | 1302.04 (gnu-time-v) | 69.39 (gnu-time-v) | 89.40 (gnu-time-v) | 78.12 (gnu-time-v) |
| Logical throughput (records/s) | 112671.834 | 43646.246 | 12087.021 | 82394.814 | 64584.318 | 64486.422 |
| Physical throughput (MiB/s) | 76.373 | 29.585 | 8.182 | 55.851 | 43.716 | 51.773 |
| Output bytes | 2 | 2 | 2 | 2 | 2 | 2 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.859 | 63.801 | 181.271 | 61.496 | 62.970 | 59.880 |
| Wall dispersion (MAD / p95 / range) | 0.321 / 24.847 / 22.774-24.847 | 0.764 / 65.594 / 62.817-65.594 | 0.589 / 183.574 / 180.065-183.574 | 1.319 / 64.435 / 60.142-64.435 | 1.181 / 66.800 / 60.049-66.800 | 0.621 / 61.777 / 58.428-61.777 |
| First output (ms) | 20.190 | 53.029 | 159.694 | 26.388 | 32.706 | 28.378 |
| First output dispersion (MAD / p95 / range) | 0.398 / 23.947 / 19.311-23.947 | 0.193 / 53.811 / 51.728-53.811 | 2.130 / 164.076 / 147.550-164.076 | 0.563 / 64.416 / 25.434-64.416 | 0.819 / 36.569 / 31.648-36.569 | 0.346 / 29.952 / 27.935-29.952 |
| User CPU (ms) | 10.000 | 50.000 | 200.000 | 20.000 | 20.000 | 20.000 |
| System CPU (ms) | 0.000 | 20.000 | 80.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.60 (gnu-time-v) | 68.39 (gnu-time-v) | 234.27 (gnu-time-v) | 20.34 (gnu-time-v) | 24.22 (gnu-time-v) | 21.77 (gnu-time-v) |
| Logical throughput (records/s) | 93256.214 | 34873.788 | 12274.440 | 36181.215 | 35334.286 | 37157.959 |
| Physical throughput (MiB/s) | 63.190 | 23.630 | 8.305 | 24.516 | 23.909 | 29.821 |
| Output bytes | 2 | 2 | 2 | 2 | 2 | 2 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
