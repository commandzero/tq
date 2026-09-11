---
type: Report
title: "Numeric reduction"
description: "Measures a blocking sum across all feature magnitudes."
workload: benchmark.numeric-reduction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Numeric reduction

## What this measures

The query reduces non-null feature magnitudes into one sum. It keeps an
accumulator while consuming the complete feature stream, so this is a blocking
reduction rather than an early-result projection.

## Why it matters

Totals, scores, and counters are common data-processing jobs. Their cost grows
with the number of values even when the final output is only one number.

## Input and output

The input is a natural snapshot whose features may have missing magnitudes. The
output is one numeric sum, with null magnitudes omitted. It is not one output
per feature.

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
| Wall (ms) | 22.904 | 24.177 | 25.455 | 23.122 | 23.147 | 24.157 |
| Wall dispersion (MAD / p95 / range) | 0.270 / 24.247 / 22.420-24.247 | 0.508 / 25.535 / 22.899-25.535 | 0.567 / 27.254 / 24.692-27.254 | 0.365 / 24.056 / 22.594-24.056 | 0.401 / 24.054 / 22.219-24.054 | 0.226 / 25.002 / 23.028-25.002 |
| First output (ms) | 22.889 | 24.157 | 25.436 | 23.107 | 23.133 | 24.142 |
| First output dispersion (MAD / p95 / range) | 0.272 / 24.230 / 22.401-24.230 | 0.507 / 25.519 / 22.884-25.519 | 0.566 / 27.234 / 18.798-27.234 | 0.363 / 24.041 / 22.577-24.041 | 0.402 / 24.037 / 22.206-24.037 | 0.228 / 24.987 / 23.011-24.987 |
| User CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.43 (gnu-time-v) | 19.07 (gnu-time-v) | 34.02 (gnu-time-v) | 9.10 (gnu-time-v) | 9.66 (gnu-time-v) | 9.27 (gnu-time-v) |
| Logical throughput (records/s) | 8470.321 | 8023.989 | 7621.292 | 8390.096 | 8381.216 | 8030.799 |
| Physical throughput (MiB/s) | 5.799 | 5.493 | 5.210 | 5.744 | 5.730 | 6.501 |
| Output bytes | 19 | 19 | 19 | 19 | 19 | 19 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.151 | 23.227 | 22.971 | 23.069 | 23.383 | 23.916 |
| Wall dispersion (MAD / p95 / range) | 0.359 / 23.799 / 21.694-23.999 | 0.538 / 24.341 / 22.100-24.380 | 0.305 / 24.208 / 22.101-24.289 | 0.344 / 24.201 / 22.096-24.521 | 0.506 / 25.223 / 21.582-26.232 | 0.369 / 25.150 / 22.539-26.226 |
| First output (ms) | 23.137 | 23.212 | 22.954 | 23.054 | 23.363 | 23.904 |
| First output dispersion (MAD / p95 / range) | 0.359 / 23.784 / 21.679-23.984 | 0.537 / 24.326 / 22.086-24.362 | 0.305 / 24.193 / 22.084-24.276 | 0.345 / 24.191 / 22.080-24.509 | 0.505 / 25.206 / 21.567-26.186 | 0.366 / 25.133 / 22.524-26.213 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | 12.02 (gnu-time-v) | 12.14 (gnu-time-v) | 8.16 (gnu-time-v) | 8.34 (gnu-time-v) | 8.10 (gnu-time-v) |
| Logical throughput (records/s) | 86.389 | 86.105 | 87.068 | 86.698 | 85.534 | 83.626 |
| Physical throughput (MiB/s) | 0.073 | 0.073 | 0.074 | 0.073 | 0.072 | 0.081 |
| Output bytes | 5 | 5 | 5 | 5 | 5 | 5 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 136.095 | 336.009 | 926.003 | 157.481 | 212.229 | 176.177 |
| Wall dispersion (MAD / p95 / range) | 1.727 / 139.483 / 101.796-139.483 | 0.928 / 337.509 / 333.740-337.509 | 6.024 / 1006.912 / 897.981-1006.912 | 18.161 / 177.003 / 136.599-177.003 | 3.067 / 217.228 / 178.318-217.228 | 1.095 / 180.969 / 171.402-180.969 |
| First output (ms) | 99.665 | 304.483 | 845.279 | 134.285 | 175.952 | 151.133 |
| First output dispersion (MAD / p95 / range) | 0.863 / 101.396 / 96.922-101.396 | 4.434 / 312.447 / 299.073-312.447 | 9.134 / 916.742 / 835.905-916.742 | 1.038 / 137.899 / 132.566-137.899 | 1.728 / 181.429 / 173.228-181.429 | 0.802 / 152.711 / 149.722-152.711 |
| User CPU (ms) | 70.000 | 360.000 | 1280.000 | 100.000 | 140.000 | 120.000 |
| System CPU (ms) | 20.000 | 110.000 | 395.000 | 20.000 | 30.000 | 20.000 |
| Peak RSS (MiB, source) | 60.29 (gnu-time-v) | 344.36 (gnu-time-v) | 1296.59 (gnu-time-v) | 69.21 (gnu-time-v) | 89.41 (gnu-time-v) | 78.05 (gnu-time-v) |
| Logical throughput (records/s) | 82839.498 | 33552.723 | 12174.907 | 71589.589 | 53121.864 | 63992.462 |
| Physical throughput (MiB/s) | 56.152 | 22.743 | 8.241 | 48.526 | 35.957 | 51.376 |
| Output bytes | 19 | 19 | 19 | 19 | 19 | 19 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.799 | 102.197 | 184.648 | 60.035 | 61.282 | 60.385 |
| Wall dispersion (MAD / p95 / range) | 0.403 / 24.472 / 22.564-24.472 | 0.124 / 103.354 / 101.252-103.354 | 3.022 / 220.683 / 181.164-220.683 | 0.309 / 60.778 / 59.314-60.778 | 0.650 / 62.616 / 60.192-62.616 | 0.676 / 61.145 / 59.088-61.145 |
| First output (ms) | 21.207 | 63.816 | 167.148 | 28.858 | 35.305 | 31.654 |
| First output dispersion (MAD / p95 / range) | 0.614 / 23.785 / 19.922-23.785 | 1.204 / 68.324 / 62.381-68.324 | 3.032 / 175.754 / 154.908-175.754 | 0.454 / 29.394 / 27.690-29.394 | 0.867 / 36.950 / 33.886-36.950 | 0.589 / 61.079 / 30.823-61.079 |
| User CPU (ms) | 10.000 | 70.000 | 200.000 | 20.000 | 20.000 | 20.000 |
| System CPU (ms) | 0.000 | 20.000 | 80.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.61 (gnu-time-v) | 80.02 (gnu-time-v) | 235.20 (gnu-time-v) | 20.14 (gnu-time-v) | 24.27 (gnu-time-v) | 21.89 (gnu-time-v) |
| Logical throughput (records/s) | 93489.359 | 21771.676 | 12049.955 | 37061.405 | 36307.266 | 36846.899 |
| Physical throughput (MiB/s) | 63.348 | 14.752 | 8.153 | 25.112 | 24.567 | 29.571 |
| Output bytes | 18 | 18 | 18 | 18 | 18 | 18 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
