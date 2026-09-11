---
type: Report
title: "Templated URI output"
description: "Measures building and URI-escaping a query-string template per feature."
workload: benchmark.format-template
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Templated URI output

## What this measures

The query builds `id=...&place=...` from each feature and applies URI escaping
to the template. It combines interpolation with text formatting.

## Why it matters

Query strings and small links are often assembled at the edge of a data
pipeline. Escaping each interpolated value is part of producing safe output.

## Input and output

The input is a natural feature snapshot. The output is one URI-escaped query
string per feature with `id` and `place` fields, not the source feature.

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
| Wall (ms) | 23.157 | not measured | not measured | 22.602 | 23.343 | 23.677 |
| Wall dispersion (MAD / p95 / range) | 0.551 / 24.160 / 21.809-24.160 | not measured | not measured | 0.734 / 24.180 / 21.774-24.180 | 0.539 / 24.021 / 22.541-24.021 | 0.292 / 24.733 / 22.866-24.733 |
| First output (ms) | 23.140 | not measured | not measured | 22.587 | 23.328 | 23.662 |
| First output dispersion (MAD / p95 / range) | 0.554 / 24.144 / 21.795-24.144 | not measured | not measured | 0.728 / 24.167 / 21.757-24.167 | 0.538 / 24.005 / 22.523-24.005 | 0.295 / 24.718 / 22.853-24.718 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.36 (gnu-time-v) | not measured | not measured | 8.77 (gnu-time-v) | 9.94 (gnu-time-v) | 8.19 (gnu-time-v) |
| Logical throughput (records/s) | 8377.777 | not measured | not measured | 8583.311 | 8310.843 | 8193.606 |
| Physical throughput (MiB/s) | 5.736 | not measured | not measured | 5.876 | 5.682 | 6.633 |
| Output bytes | 12699 | not measured | not measured | 12311 | 12311 | 12311 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq format-string expression. | yq rejects the catalog jq format-string expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.613 | not measured | not measured | 23.826 | 23.863 | 23.725 |
| Wall dispersion (MAD / p95 / range) | 0.370 / 24.639 / 22.024-24.675 | not measured | not measured | 0.394 / 25.008 / 22.846-25.099 | 0.378 / 24.655 / 22.703-24.864 | 0.403 / 24.744 / 21.796-24.909 |
| First output (ms) | 23.601 | not measured | not measured | 23.812 | 23.851 | 23.712 |
| First output dispersion (MAD / p95 / range) | 0.371 / 24.624 / 22.010-24.660 | not measured | not measured | 0.388 / 24.995 / 22.831-25.084 | 0.379 / 24.641 / 22.688-24.848 | 0.401 / 24.724 / 21.779-24.892 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | not measured | not measured | 8.70 (gnu-time-v) | 8.40 (gnu-time-v) | 8.26 (gnu-time-v) |
| Logical throughput (records/s) | 84.701 | not measured | not measured | 83.942 | 83.810 | 84.299 |
| Physical throughput (MiB/s) | 0.072 | not measured | not measured | 0.071 | 0.071 | 0.082 |
| Output bytes | 128 | not measured | not measured | 124 | 124 | 124 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq format-string expression. | yq rejects the catalog jq format-string expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 137.901 | not measured | not measured | 393.692 | 288.551 | 244.650 |
| Wall dispersion (MAD / p95 / range) | 1.492 / 141.177 / 134.908-141.177 | not measured | not measured | 1.658 / 395.853 / 388.983-395.853 | 2.198 / 293.670 / 282.548-293.670 | 1.271 / 247.601 / 214.636-247.601 |
| First output (ms) | 81.002 | not measured | not measured | 301.981 | 157.098 | 23.224 |
| First output dispersion (MAD / p95 / range) | 0.879 / 82.929 / 79.996-82.929 | not measured | not measured | 2.644 / 306.967 / 298.687-306.967 | 1.242 / 170.062 / 155.257-170.062 | 0.789 / 24.765 / 21.982-24.765 |
| User CPU (ms) | 90.000 | not captured | not captured | 340.000 | 200.000 | 210.000 |
| System CPU (ms) | 20.000 | not captured | not captured | 20.000 | 60.000 | 0.000 |
| Peak RSS (MiB, source) | 60.28 (gnu-time-v) | not measured | not measured | 11.71 (gnu-time-v) | 89.50 (gnu-time-v) | 8.24 (gnu-time-v) |
| Logical throughput (records/s) | 81754.006 | not measured | not measured | 28636.599 | 39071.015 | 46082.158 |
| Physical throughput (MiB/s) | 55.416 | not measured | not measured | 19.411 | 26.447 | 36.997 |
| Output bytes | 727347 | not measured | not measured | 704799 | 704799 | 704799 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq format-string expression. | yq rejects the catalog jq format-string expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 60.446 | not measured | not measured | 97.352 | 62.570 | 59.839 |
| Wall dispersion (MAD / p95 / range) | 0.853 / 61.562 / 58.419-61.562 | not measured | not measured | 0.519 / 98.664 / 94.910-98.664 | 0.234 / 62.923 / 61.608-62.923 | 0.380 / 60.356 / 58.132-60.356 |
| First output (ms) | 18.656 | not measured | not measured | 63.556 | 39.768 | 23.253 |
| First output dispersion (MAD / p95 / range) | 0.226 / 18.886 / 18.061-18.886 | not measured | not measured | 0.530 / 65.233 / 62.725-65.233 | 0.434 / 40.653 / 38.679-40.653 | 0.644 / 24.399 / 22.191-24.399 |
| User CPU (ms) | 10.000 | not captured | not captured | 60.000 | 30.000 | 40.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 10.000 | 0.000 |
| Peak RSS (MiB, source) | 14.68 (gnu-time-v) | not measured | not measured | 9.84 (gnu-time-v) | 24.57 (gnu-time-v) | 8.27 (gnu-time-v) |
| Logical throughput (records/s) | 36809.714 | not measured | not measured | 22855.206 | 35560.173 | 37183.108 |
| Physical throughput (MiB/s) | 24.942 | not measured | not measured | 15.486 | 24.061 | 29.841 |
| Output bytes | 143982 | not measured | not measured | 139532 | 139532 | 139532 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq format-string expression. | yq rejects the catalog jq format-string expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
