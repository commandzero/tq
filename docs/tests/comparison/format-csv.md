---
type: Report
title: "CSV formatting"
description: "Measures formatting selected feature fields as CSV rows."
workload: benchmark.format-csv
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# CSV formatting

## What this measures

The query selects `id`, `place`, and `mag` for each feature and applies `@csv`.
It measures quoting, delimiter handling, and text output for repeated rows.

## Why it matters

CSV remains a practical interchange format for spreadsheets and simple data
loads. Correct quoting matters when fields contain commas or special text.

## Input and output

The input is a natural feature snapshot. The output is one CSV string per
feature containing the three selected fields, not JSON arrays or feature
objects.

The yq adapter forces quoted string fields to match jq CSV text. It emits an empty magnitude field for null values.

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
| Wall (ms) | 23.846 | 61.665 | 63.452 | 23.155 | 23.421 | 23.181 |
| Wall dispersion (MAD / p95 / range) | 0.448 / 24.958 / 22.829-24.958 | 0.671 / 62.974 / 24.818-62.974 | 0.286 / 64.600 / 62.461-64.600 | 0.380 / 24.363 / 22.439-24.363 | 0.330 / 24.093 / 22.533-24.093 | 1.175 / 24.769 / 21.809-24.769 |
| First output (ms) | 23.830 | 24.657 | 33.170 | 23.139 | 23.407 | 23.168 |
| First output dispersion (MAD / p95 / range) | 0.450 / 24.940 / 22.815-24.940 | 1.032 / 62.955 / 22.933-62.955 | 0.443 / 34.136 / 31.637-34.136 | 0.381 / 24.348 / 22.426-24.348 | 0.329 / 24.077 / 22.518-24.077 | 1.172 / 24.753 / 21.794-24.753 |
| User CPU (ms) | 0.000 | 20.000 | 30.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.43 (gnu-time-v) | 24.27 (gnu-time-v) | 37.64 (gnu-time-v) | 9.09 (gnu-time-v) | 9.66 (gnu-time-v) | 8.96 (gnu-time-v) |
| Logical throughput (records/s) | 8135.707 | 3146.031 | 3057.453 | 8378.139 | 8283.342 | 8368.742 |
| Physical throughput (MiB/s) | 5.570 | 2.154 | 2.090 | 5.736 | 5.663 | 6.774 |
| Output bytes | 10734 | 10734 | 10734 | 10734 | 10734 | 10734 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.597 | 23.595 | 23.656 | 23.953 | 24.040 | 23.869 |
| Wall dispersion (MAD / p95 / range) | 0.485 / 24.666 / 21.864-25.022 | 0.414 / 24.769 / 22.122-24.870 | 0.408 / 24.991 / 22.419-25.008 | 0.399 / 24.884 / 22.849-25.198 | 0.373 / 25.076 / 22.832-25.102 | 0.394 / 24.856 / 23.009-25.211 |
| First output (ms) | 23.582 | 23.579 | 23.640 | 23.933 | 24.024 | 23.854 |
| First output dispersion (MAD / p95 / range) | 0.483 / 24.652 / 21.849-25.007 | 0.415 / 24.751 / 22.110-24.859 | 0.406 / 24.980 / 22.405-24.990 | 0.398 / 24.871 / 22.832-25.182 | 0.375 / 25.061 / 22.814-25.085 | 0.396 / 24.837 / 22.995-25.197 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | 12.02 (gnu-time-v) | 12.14 (gnu-time-v) | 8.01 (gnu-time-v) | 8.52 (gnu-time-v) | 7.90 (gnu-time-v) |
| Logical throughput (records/s) | 84.757 | 84.764 | 84.545 | 83.497 | 83.195 | 83.791 |
| Physical throughput (MiB/s) | 0.072 | 0.072 | 0.072 | 0.071 | 0.070 | 0.081 |
| Output bytes | 106 | 106 | 106 | 106 | 106 | 106 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 137.877 | 1120.377 | 1749.505 | 248.434 | 289.065 | 250.909 |
| Wall dispersion (MAD / p95 / range) | 0.636 / 140.292 / 135.940-140.292 | 3.135 / 1130.760 / 1114.556-1130.760 | 2.146 / 1758.223 / 1741.927-1758.223 | 1.671 / 251.121 / 245.157-251.121 | 1.410 / 294.782 / 286.617-294.782 | 1.548 / 254.050 / 247.341-254.050 |
| First output (ms) | 80.811 | 997.755 | 1573.591 | 114.180 | 156.523 | 130.583 |
| First output dispersion (MAD / p95 / range) | 0.843 / 82.085 / 78.559-82.085 | 1.923 / 1005.425 / 994.202-1005.425 | 10.172 / 1586.192 / 1559.183-1586.192 | 0.907 / 116.304 / 112.134-116.304 | 0.603 / 161.277 / 154.354-161.277 | 0.764 / 134.225 / 126.410-134.225 |
| User CPU (ms) | 90.000 | 1560.000 | 2460.000 | 150.000 | 200.000 | 170.000 |
| System CPU (ms) | 20.000 | 200.000 | 540.000 | 60.000 | 60.000 | 55.000 |
| Peak RSS (MiB, source) | 60.68 (gnu-time-v) | 478.52 (gnu-time-v) | 1440.90 (gnu-time-v) | 69.40 (gnu-time-v) | 89.43 (gnu-time-v) | 78.07 (gnu-time-v) |
| Logical throughput (records/s) | 81768.533 | 10062.684 | 6444.108 | 45380.353 | 39001.609 | 44932.715 |
| Physical throughput (MiB/s) | 55.426 | 6.821 | 4.362 | 30.761 | 26.400 | 36.074 |
| Output bytes | 621328 | 621328 | 621328 | 621328 | 621328 | 621328 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 59.328 | 223.684 | 377.770 | 61.555 | 62.771 | 61.246 |
| Wall dispersion (MAD / p95 / range) | 1.819 / 61.185 / 24.616-61.185 | 1.141 / 258.511 / 222.200-258.511 | 1.189 / 382.951 / 339.967-382.951 | 0.448 / 62.155 / 60.449-62.155 | 0.712 / 63.779 / 61.371-63.779 | 0.372 / 62.032 / 59.337-62.032 |
| First output (ms) | 18.110 | 200.703 | 312.293 | 33.163 | 40.441 | 35.906 |
| First output dispersion (MAD / p95 / range) | 0.548 / 19.163 / 17.172-19.163 | 1.198 / 202.637 / 199.038-202.637 | 3.982 / 318.717 / 306.726-318.717 | 0.618 / 34.618 / 31.127-34.618 | 0.727 / 41.980 / 39.214-41.980 | 0.315 / 36.279 / 34.407-36.279 |
| User CPU (ms) | 10.000 | 305.000 | 480.000 | 30.000 | 30.000 | 30.000 |
| System CPU (ms) | 0.000 | 40.000 | 120.000 | 10.000 | 10.000 | 10.000 |
| Peak RSS (MiB, source) | 14.77 (gnu-time-v) | 105.89 (gnu-time-v) | 277.36 (gnu-time-v) | 20.14 (gnu-time-v) | 24.22 (gnu-time-v) | 21.77 (gnu-time-v) |
| Logical throughput (records/s) | 37503.687 | 9947.090 | 5889.827 | 36146.536 | 35446.305 | 36328.607 |
| Physical throughput (MiB/s) | 25.412 | 6.740 | 3.985 | 24.493 | 23.984 | 29.155 |
| Output bytes | 122894 | 122894 | 122894 | 122894 | 122894 | 122894 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
