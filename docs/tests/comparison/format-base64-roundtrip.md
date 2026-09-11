---
type: Report
title: "Base64 round trip"
description: "Measures Base64 encoding followed by decoding for each place string."
workload: benchmark.format-base64-roundtrip
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Base64 round trip

## What this measures

The query extracts each place string, applies `@base64`, and decodes it with
`@base64d`. It includes both conversions for every feature.

## Why it matters

Encoding is common when text crosses a binary-safe transport or storage field.
A round trip shows the cost of conversion even when the final value is
unchanged.

## Input and output

The input is a natural feature snapshot. The output is one decoded place string
per feature, so it should have the same semantic text as the selected input.

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
| Wall (ms) | 23.659 | 24.619 | 26.587 | 23.811 | 24.212 | 23.198 |
| Wall dispersion (MAD / p95 / range) | 0.198 / 24.240 / 22.896-24.240 | 0.509 / 25.849 / 23.733-25.849 | 0.577 / 27.743 / 24.718-27.743 | 0.371 / 24.363 / 23.085-24.363 | 0.748 / 26.466 / 22.614-26.466 | 0.299 / 24.185 / 22.617-24.185 |
| First output (ms) | 23.638 | 24.603 | 20.669 | 23.796 | 24.197 | 23.181 |
| First output dispersion (MAD / p95 / range) | 0.203 / 24.223 / 22.886-24.223 | 0.544 / 25.832 / 12.845-25.832 | 0.263 / 21.562 / 19.686-21.562 | 0.370 / 24.348 / 23.068-24.348 | 0.746 / 26.450 / 22.599-26.450 | 0.299 / 24.168 / 22.602-24.168 |
| User CPU (ms) | 0.000 | 10.000 | 20.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.43 (gnu-time-v) | 21.83 (gnu-time-v) | 33.77 (gnu-time-v) | 8.74 (gnu-time-v) | 9.84 (gnu-time-v) | 8.20 (gnu-time-v) |
| Logical throughput (records/s) | 8200.013 | 7879.933 | 7296.662 | 8147.495 | 8012.390 | 8362.790 |
| Physical throughput (MiB/s) | 5.614 | 5.395 | 4.988 | 5.578 | 5.478 | 6.770 |
| Output bytes | 6025 | 6025 | 6025 | 6019 | 6019 | 6019 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.506 | 23.895 | 23.654 | 23.779 | 23.468 | 23.520 |
| Wall dispersion (MAD / p95 / range) | 0.393 / 24.689 / 21.506-24.969 | 0.338 / 24.709 / 22.526-24.783 | 0.390 / 24.531 / 22.440-24.554 | 0.407 / 24.850 / 22.920-24.875 | 0.326 / 24.413 / 22.795-24.776 | 0.256 / 24.181 / 22.747-24.442 |
| First output (ms) | 23.492 | 23.883 | 23.638 | 23.763 | 23.452 | 23.503 |
| First output dispersion (MAD / p95 / range) | 0.391 / 24.677 / 21.494-24.959 | 0.338 / 24.694 / 22.511-24.770 | 0.390 / 24.515 / 22.426-24.543 | 0.404 / 24.837 / 22.905-24.859 | 0.327 / 24.397 / 22.783-24.760 | 0.261 / 24.168 / 22.733-24.424 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | 11.89 (gnu-time-v) | 12.02 (gnu-time-v) | 8.81 (gnu-time-v) | 8.61 (gnu-time-v) | 8.20 (gnu-time-v) |
| Logical throughput (records/s) | 85.086 | 83.698 | 84.554 | 84.106 | 85.222 | 85.032 |
| Physical throughput (MiB/s) | 0.072 | 0.071 | 0.072 | 0.071 | 0.072 | 0.083 |
| Output bytes | 58 | 58 | 58 | 58 | 58 | 58 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 138.752 | 492.760 | 1089.118 | 395.119 | 250.638 | 209.940 |
| Wall dispersion (MAD / p95 / range) | 0.671 / 149.346 / 137.419-149.346 | 1.127 / 532.270 / 489.329-532.270 | 33.334 / 1133.755 / 1054.896-1133.755 | 2.561 / 402.875 / 389.214-402.875 | 1.800 / 289.784 / 248.139-289.784 | 1.616 / 244.065 / 206.884-244.065 |
| First output (ms) | 81.252 | 378.665 | 928.128 | 307.094 | 161.117 | 41.712 |
| First output dispersion (MAD / p95 / range) | 0.625 / 82.202 / 79.011-82.202 | 2.796 / 388.136 / 375.563-388.136 | 15.259 / 960.934 / 882.277-960.934 | 1.813 / 315.783 / 304.399-315.783 | 1.149 / 162.428 / 158.126-162.428 | 0.291 / 42.294 / 40.922-42.294 |
| User CPU (ms) | 80.000 | 540.000 | 1625.000 | 330.000 | 180.000 | 190.000 |
| System CPU (ms) | 20.000 | 175.000 | 455.000 | 20.000 | 60.000 | 0.000 |
| Peak RSS (MiB, source) | 60.31 (gnu-time-v) | 453.43 (gnu-time-v) | 1305.14 (gnu-time-v) | 11.70 (gnu-time-v) | 89.42 (gnu-time-v) | 8.30 (gnu-time-v) |
| Logical throughput (records/s) | 81252.590 | 22879.269 | 10351.495 | 28533.176 | 44981.118 | 53701.185 |
| Physical throughput (MiB/s) | 55.076 | 15.508 | 7.007 | 19.341 | 30.447 | 43.114 |
| Output bytes | 347435 | 347435 | 347435 | 347161 | 347161 | 347161 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 24.433 | 104.677 | 222.623 | 96.977 | 62.118 | 59.930 |
| Wall dispersion (MAD / p95 / range) | 0.269 / 25.137 / 23.862-25.137 | 0.556 / 105.932 / 102.335-105.932 | 0.680 / 224.555 / 220.498-224.555 | 0.312 / 99.628 / 96.587-99.628 | 0.430 / 62.961 / 61.392-62.961 | 0.731 / 61.251 / 58.547-61.251 |
| First output (ms) | 18.120 | 78.166 | 180.357 | 69.866 | 45.514 | 41.622 |
| First output dispersion (MAD / p95 / range) | 0.200 / 18.434 / 17.681-18.434 | 0.860 / 80.254 / 77.042-80.254 | 2.421 / 183.946 / 174.948-183.946 | 0.271 / 71.135 / 69.050-71.135 | 0.275 / 47.158 / 44.222-47.158 | 0.551 / 42.903 / 39.954-42.903 |
| User CPU (ms) | 10.000 | 100.000 | 275.000 | 60.000 | 30.000 | 40.000 |
| System CPU (ms) | 0.000 | 40.000 | 90.000 | 0.000 | 10.000 | 0.000 |
| Peak RSS (MiB, source) | 14.65 (gnu-time-v) | 99.39 (gnu-time-v) | 236.02 (gnu-time-v) | 9.84 (gnu-time-v) | 24.47 (gnu-time-v) | 8.31 (gnu-time-v) |
| Logical throughput (records/s) | 91067.226 | 21255.863 | 9994.475 | 22943.585 | 35818.925 | 37126.648 |
| Physical throughput (MiB/s) | 61.706 | 14.403 | 6.763 | 15.546 | 24.236 | 29.796 |
| Output bytes | 68494 | 68494 | 68494 | 68450 | 68450 | 68450 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
