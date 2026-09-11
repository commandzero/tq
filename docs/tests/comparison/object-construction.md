---
type: Report
title: "Object construction"
description: "Measures building a lookup object with computed feature IDs as keys."
workload: benchmark.object-construction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Object construction

## What this measures

The query turns the feature collection into an object whose keys come from
`.id` and whose values are magnitudes. It exercises computed keys, repeated
merges, and a blocking object result.

## Why it matters

Lookup maps are a common boundary between raw records and application code.
They also stress a different memory pattern from an output array.

## Input and output

The input is a natural feature snapshot with IDs and magnitudes. The output is
one object mapping each ID to its magnitude. It is not a list of key-value
records.

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
| Wall (ms) | 22.738 | 25.143 | 63.438 | 23.339 | 23.375 | 23.095 |
| Wall dispersion (MAD / p95 / range) | 0.268 / 23.754 / 22.359-23.754 | 0.468 / 27.686 / 24.202-27.686 | 0.507 / 64.234 / 62.415-64.234 | 0.381 / 24.267 / 21.948-24.267 | 0.236 / 25.958 / 21.848-25.958 | 0.213 / 23.573 / 22.435-23.573 |
| First output (ms) | 22.724 | 25.125 | 63.152 | 23.323 | 23.360 | 23.082 |
| First output dispersion (MAD / p95 / range) | 0.264 / 23.738 / 22.343-23.738 | 0.636 / 27.667 / 18.206-27.667 | 0.747 / 64.215 / 26.217-64.215 | 0.380 / 24.250 / 21.934-24.250 | 0.234 / 25.942 / 21.834-25.942 | 0.210 / 23.562 / 22.424-23.562 |
| User CPU (ms) | 0.000 | 20.000 | 20.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.56 (gnu-time-v) | 24.64 (gnu-time-v) | 38.52 (gnu-time-v) | 9.23 (gnu-time-v) | 9.78 (gnu-time-v) | 9.27 (gnu-time-v) |
| Logical throughput (records/s) | 8531.785 | 7715.712 | 3058.128 | 8312.267 | 8299.465 | 8400.087 |
| Physical throughput (MiB/s) | 5.841 | 5.282 | 2.091 | 5.691 | 5.674 | 6.800 |
| Output bytes | 4324 | 3547 | 3547 | 3351 | 3351 | 3351 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.391 | 23.431 | 23.248 | 23.234 | 23.607 | 23.383 |
| Wall dispersion (MAD / p95 / range) | 0.345 / 24.258 / 22.068-24.780 | 0.446 / 24.308 / 21.994-24.369 | 0.343 / 24.336 / 22.163-24.429 | 0.335 / 23.995 / 22.061-24.736 | 0.353 / 24.279 / 22.512-24.324 | 0.315 / 24.128 / 21.703-24.480 |
| First output (ms) | 23.377 | 23.417 | 23.233 | 23.219 | 23.596 | 23.368 |
| First output dispersion (MAD / p95 / range) | 0.343 / 24.243 / 22.053-24.763 | 0.444 / 24.291 / 21.984-24.351 | 0.341 / 24.325 / 22.146-24.415 | 0.335 / 23.977 / 22.045-24.720 | 0.350 / 24.263 / 22.497-24.308 | 0.315 / 24.113 / 21.691-24.465 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | 12.02 (gnu-time-v) | 12.27 (gnu-time-v) | 8.09 (gnu-time-v) | 8.57 (gnu-time-v) | 7.95 (gnu-time-v) |
| Logical throughput (records/s) | 85.503 | 85.355 | 86.029 | 86.081 | 84.721 | 85.532 |
| Physical throughput (MiB/s) | 0.072 | 0.072 | 0.073 | 0.073 | 0.072 | 0.083 |
| Output bytes | 47 | 38 | 38 | 34 | 34 | 34 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 138.439 | 24622.290 | 23476.393 | not measured | not measured | not measured |
| Wall dispersion (MAD / p95 / range) | 1.844 / 142.543 / 134.413-142.543 | 36.373 / 24720.131 / 24545.205-24720.131 | 95.745 / 23713.697 / 23271.108-23713.697 | not measured | not measured | not measured |
| First output (ms) | 107.597 | 24562.186 | 23372.218 | not measured | not measured | not measured |
| First output dispersion (MAD / p95 / range) | 0.557 / 112.204 / 106.292-112.204 | 40.865 / 24668.050 / 24510.235-24668.050 | 89.207 / 23598.949 / 23163.985-23598.949 | not measured | not measured | not measured |
| User CPU (ms) | 80.000 | 55355.000 | 52810.000 | not captured | not captured | not captured |
| System CPU (ms) | 20.000 | 325.000 | 635.000 | not captured | not captured | not captured |
| Peak RSS (MiB, source) | 65.82 (gnu-time-v) | 490.74 (gnu-time-v) | 1475.91 (gnu-time-v) | not measured | not measured | not measured |
| Logical throughput (records/s) | 81436.884 | 457.878 | 480.227 | not measured | not measured | not measured |
| Physical throughput (MiB/s) | 55.201 | 0.310 | 0.325 | not measured | not measured | not measured |
| Output bytes | 251352 | 206255 | 206255 | not measured | not measured | not measured |
| Outcome | timed | timed | timed | resource-limit | resource-limit | resource-limit |
| Details | none | none | none | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | not timed | not timed | not timed |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 24.887 | 783.399 | 858.529 | 135.679 | 135.833 | 135.089 |
| Wall dispersion (MAD / p95 / range) | 0.365 / 60.381 / 24.459-60.381 | 1.784 / 863.556 / 778.944-863.556 | 3.205 / 862.215 / 822.027-862.215 | 0.618 / 136.703 / 133.397-136.703 | 0.513 / 137.335 / 135.083-137.335 | 0.758 / 136.590 / 134.101-136.590 |
| First output (ms) | 22.073 | 746.051 | 811.159 | 104.180 | 111.359 | 106.813 |
| First output dispersion (MAD / p95 / range) | 0.642 / 23.801 / 21.234-23.801 | 4.327 / 846.905 / 741.231-846.905 | 2.896 / 814.169 / 802.592-814.169 | 0.372 / 105.777 / 102.741-105.777 | 0.485 / 112.142 / 110.164-112.142 | 1.190 / 109.299 / 105.117-109.299 |
| User CPU (ms) | 10.000 | 2110.000 | 2100.000 | 90.000 | 100.000 | 100.000 |
| System CPU (ms) | 0.000 | 50.000 | 130.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 15.73 (gnu-time-v) | 111.27 (gnu-time-v) | 283.89 (gnu-time-v) | 21.46 (gnu-time-v) | 25.65 (gnu-time-v) | 22.97 (gnu-time-v) |
| Logical throughput (records/s) | 89404.107 | 2840.187 | 2591.642 | 16398.940 | 16380.469 | 16470.623 |
| Physical throughput (MiB/s) | 60.579 | 1.924 | 1.754 | 11.112 | 11.084 | 13.218 |
| Output bytes | 49953 | 41052 | 41052 | 38825 | 38825 | 38825 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
