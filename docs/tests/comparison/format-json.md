---
type: Report
title: "JSON text formatting"
description: "Measures serializing each feature projection as compact JSON text."
workload: benchmark.format-json
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# JSON text formatting

## What this measures

The query projects `[id, properties]` for each feature and applies `@json`.
It measures repeated conversion from values to JSON text.

## Why it matters

Small JSON fragments are common message and logging boundaries. Serialization
cost can dominate when the query emits many compact records.

## Input and output

The input is a natural feature snapshot. The output is one JSON string per
feature containing its ID and properties, not an array of value pairs.

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
| Wall (ms) | 23.296 | 25.154 | 63.147 | 23.773 | 23.321 | 23.152 |
| Wall dispersion (MAD / p95 / range) | 0.233 / 24.012 / 22.990-24.012 | 0.315 / 26.102 / 24.462-26.102 | 0.760 / 65.601 / 62.360-65.601 | 0.269 / 24.658 / 23.186-24.658 | 0.228 / 23.690 / 22.548-23.690 | 0.350 / 23.788 / 22.385-23.788 |
| First output (ms) | 23.280 | 19.210 | 26.105 | 23.757 | 23.304 | 23.135 |
| First output dispersion (MAD / p95 / range) | 0.234 / 24.001 / 22.975-24.001 | 0.242 / 19.696 / 18.300-19.696 | 0.292 / 27.754 / 25.734-27.754 | 0.266 / 24.645 / 23.170-24.645 | 0.228 / 23.675 / 22.533-23.675 | 0.348 / 23.771 / 22.368-23.771 |
| User CPU (ms) | 0.000 | 20.000 | 20.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.30 (gnu-time-v) | 27.39 (gnu-time-v) | 35.02 (gnu-time-v) | 9.32 (gnu-time-v) | 9.81 (gnu-time-v) | 9.23 (gnu-time-v) |
| Logical throughput (records/s) | 8327.789 | 7712.491 | 3072.197 | 8160.518 | 8318.683 | 8379.406 |
| Physical throughput (MiB/s) | 5.701 | 5.280 | 2.100 | 5.587 | 5.687 | 6.783 |
| Output bytes | 132463 | 132463 | 132463 | 132463 | 132463 | 132463 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.340 | 23.638 | 23.703 | 24.024 | 23.530 | 23.371 |
| Wall dispersion (MAD / p95 / range) | 0.267 / 24.037 / 22.469-24.542 | 0.370 / 24.616 / 22.151-24.693 | 0.415 / 25.263 / 22.573-26.223 | 0.449 / 24.779 / 22.604-24.905 | 0.362 / 24.224 / 22.410-24.272 | 0.290 / 24.241 / 22.456-24.300 |
| First output (ms) | 23.326 | 23.623 | 23.691 | 23.997 | 23.517 | 23.350 |
| First output dispersion (MAD / p95 / range) | 0.266 / 24.021 / 22.452-24.525 | 0.371 / 24.605 / 22.136-24.678 | 0.416 / 25.251 / 22.556-26.208 | 0.443 / 24.764 / 22.588-24.890 | 0.360 / 24.210 / 22.397-24.257 | 0.286 / 24.223 / 22.446-24.289 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | 12.02 (gnu-time-v) | 12.27 (gnu-time-v) | 8.23 (gnu-time-v) | 8.53 (gnu-time-v) | 8.06 (gnu-time-v) |
| Logical throughput (records/s) | 85.690 | 84.610 | 84.376 | 83.248 | 84.998 | 85.576 |
| Physical throughput (MiB/s) | 0.073 | 0.072 | 0.071 | 0.070 | 0.072 | 0.083 |
| Output bytes | 1353 | 1353 | 1353 | 1353 | 1353 | 1353 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 251.400 | 997.104 | 1526.081 | 288.026 | 329.652 | 287.628 |
| Wall dispersion (MAD / p95 / range) | 2.047 / 255.147 / 245.507-255.147 | 2.571 / 1025.289 / 991.947-1025.289 | 18.590 / 1589.837 / 1507.407-1589.837 | 1.448 / 298.863 / 282.810-298.863 | 1.992 / 333.821 / 326.082-333.821 | 1.302 / 290.134 / 285.125-290.134 |
| First output (ms) | 80.718 | 828.972 | 1323.011 | 105.388 | 147.311 | 121.443 |
| First output dispersion (MAD / p95 / range) | 0.449 / 81.746 / 79.502-81.746 | 2.277 / 837.654 / 822.355-837.654 | 30.875 / 1379.989 / 1288.279-1379.989 | 0.827 / 108.688 / 103.866-108.688 | 0.749 / 150.009 / 145.590-150.009 | 0.708 / 127.815 / 119.601-127.815 |
| User CPU (ms) | 190.000 | 1025.000 | 2020.000 | 200.000 | 230.000 | 205.000 |
| System CPU (ms) | 30.000 | 315.000 | 545.000 | 50.000 | 60.000 | 60.000 |
| Peak RSS (MiB, source) | 63.67 (gnu-time-v) | 832.21 (gnu-time-v) | 1528.47 (gnu-time-v) | 69.53 (gnu-time-v) | 89.61 (gnu-time-v) | 78.13 (gnu-time-v) |
| Logical throughput (records/s) | 44844.869 | 11306.750 | 7387.550 | 39142.232 | 34199.702 | 39196.462 |
| Physical throughput (MiB/s) | 30.398 | 7.664 | 5.001 | 26.532 | 23.149 | 31.469 |
| Output bytes | 7670840 | 7670840 | 7670840 | 7670840 | 7670840 | 7670840 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 61.322 | 219.189 | 299.565 | 62.523 | 63.478 | 61.987 |
| Wall dispersion (MAD / p95 / range) | 0.398 / 62.720 / 60.090-62.720 | 0.861 / 222.399 / 217.339-222.399 | 1.192 / 337.606 / 296.050-337.606 | 0.867 / 63.972 / 60.538-63.972 | 0.605 / 64.534 / 62.208-64.534 | 0.416 / 63.174 / 61.322-63.174 |
| First output (ms) | 18.223 | 164.770 | 255.880 | 24.762 | 32.160 | 27.209 |
| First output dispersion (MAD / p95 / range) | 0.342 / 18.749 / 17.121-18.749 | 0.954 / 167.992 / 162.030-167.992 | 1.450 / 262.287 / 251.966-262.287 | 0.369 / 26.461 / 23.909-26.461 | 0.460 / 33.329 / 31.277-33.329 | 0.595 / 28.738 / 26.144-28.738 |
| User CPU (ms) | 30.000 | 190.000 | 340.000 | 30.000 | 40.000 | 40.000 |
| System CPU (ms) | 0.000 | 60.000 | 110.000 | 10.000 | 10.000 | 10.000 |
| Peak RSS (MiB, source) | 15.23 (gnu-time-v) | 164.02 (gnu-time-v) | 267.39 (gnu-time-v) | 20.41 (gnu-time-v) | 24.40 (gnu-time-v) | 21.58 (gnu-time-v) |
| Logical throughput (records/s) | 36283.879 | 10151.080 | 7427.424 | 35586.619 | 35051.514 | 35894.623 |
| Physical throughput (MiB/s) | 24.586 | 6.878 | 5.026 | 24.113 | 23.717 | 28.807 |
| Output bytes | 1512411 | 1512411 | 1512411 | 1512411 | 1512411 | 1512411 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
