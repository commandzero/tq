---
type: Report
title: "HTML and URI escaping"
description: "Measures two text-escaping passes over feature place names."
workload: benchmark.format-uri-html
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# HTML and URI escaping

## What this measures

The query takes each place string through `@html` and then `@uri`. It exercises
escaping rules and repeated text allocation for every feature.

## Why it matters

Data often crosses an HTML or URL boundary after it leaves a JSON document.
Applying the transformations in sequence is common in report and request
generation.

## Input and output

The input is a natural feature snapshot. The output is one URI-encoded string
per place after HTML escaping, not the original place text or full feature.

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
| Wall (ms) | 23.358 | not measured | not measured | 23.366 | 23.177 | 23.618 |
| Wall dispersion (MAD / p95 / range) | 0.410 / 23.839 / 21.666-23.839 | not measured | not measured | 0.350 / 24.199 / 22.273-24.199 | 0.413 / 23.932 / 21.917-23.932 | 0.549 / 25.140 / 22.996-25.140 |
| First output (ms) | 23.342 | not measured | not measured | 23.352 | 23.162 | 23.605 |
| First output dispersion (MAD / p95 / range) | 0.409 / 23.826 / 21.652-23.826 | not measured | not measured | 0.350 / 24.189 / 22.257-24.189 | 0.412 / 23.917 / 21.903-23.917 | 0.552 / 25.129 / 22.982-25.129 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.40 (gnu-time-v) | not measured | not measured | 8.65 (gnu-time-v) | 9.95 (gnu-time-v) | 8.20 (gnu-time-v) |
| Logical throughput (records/s) | 8305.506 | not measured | not measured | 8302.484 | 8370.548 | 8214.074 |
| Physical throughput (MiB/s) | 5.686 | not measured | not measured | 5.684 | 5.723 | 6.649 |
| Output bytes | 8635 | not measured | not measured | 8247 | 8247 | 8247 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @html/@uri expression. | yq rejects the catalog jq @html/@uri expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.474 | not measured | not measured | 23.428 | 23.456 | 23.203 |
| Wall dispersion (MAD / p95 / range) | 0.406 / 24.309 / 22.430-24.355 | not measured | not measured | 0.261 / 24.163 / 22.582-24.534 | 0.407 / 24.254 / 22.463-24.819 | 0.305 / 24.212 / 22.211-24.241 |
| First output (ms) | 23.461 | not measured | not measured | 23.413 | 23.443 | 23.186 |
| First output dispersion (MAD / p95 / range) | 0.406 / 24.296 / 22.414-24.339 | not measured | not measured | 0.260 / 24.147 / 22.568-24.521 | 0.405 / 24.238 / 22.445-24.805 | 0.307 / 24.193 / 22.196-24.226 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | not measured | not measured | 8.79 (gnu-time-v) | 8.46 (gnu-time-v) | 8.20 (gnu-time-v) |
| Logical throughput (records/s) | 85.202 | not measured | not measured | 85.366 | 85.266 | 86.196 |
| Physical throughput (MiB/s) | 0.072 | not measured | not measured | 0.072 | 0.072 | 0.084 |
| Output bytes | 88 | not measured | not measured | 84 | 84 | 84 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @html/@uri expression. | yq rejects the catalog jq @html/@uri expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 135.571 | not measured | not measured | 393.517 | 287.263 | 211.285 |
| Wall dispersion (MAD / p95 / range) | 1.284 / 138.292 / 133.237-138.292 | not measured | not measured | 1.659 / 397.571 / 390.136-397.571 | 2.021 / 291.237 / 282.713-291.237 | 1.387 / 244.884 / 209.758-244.884 |
| First output (ms) | 81.093 | not measured | not measured | 309.772 | 159.310 | 32.657 |
| First output dispersion (MAD / p95 / range) | 0.627 / 82.999 / 79.186-82.999 | not measured | not measured | 2.780 / 313.657 / 305.340-313.657 | 0.881 / 163.385 / 158.113-163.385 | 0.323 / 33.192 / 31.573-33.192 |
| User CPU (ms) | 90.000 | not captured | not captured | 350.000 | 185.000 | 200.000 |
| System CPU (ms) | 20.000 | not captured | not captured | 20.000 | 60.000 | 0.000 |
| Peak RSS (MiB, source) | 60.30 (gnu-time-v) | not measured | not measured | 11.64 (gnu-time-v) | 89.31 (gnu-time-v) | 8.27 (gnu-time-v) |
| Logical throughput (records/s) | 83159.071 | not measured | not measured | 28649.370 | 39246.266 | 53359.207 |
| Physical throughput (MiB/s) | 56.369 | not measured | not measured | 19.420 | 26.565 | 42.839 |
| Output bytes | 492984 | not measured | not measured | 470436 | 470436 | 470436 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @html/@uri expression. | yq rejects the catalog jq @html/@uri expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 60.217 | not measured | not measured | 96.635 | 61.724 | 59.355 |
| Wall dispersion (MAD / p95 / range) | 0.739 / 61.114 / 23.971-61.114 | not measured | not measured | 0.730 / 98.154 / 95.017-98.154 | 0.410 / 62.769 / 59.937-62.769 | 0.513 / 60.614 / 58.511-60.614 |
| First output (ms) | 18.128 | not measured | not measured | 67.722 | 42.421 | 32.139 |
| First output dispersion (MAD / p95 / range) | 0.255 / 18.829 / 17.357-18.829 | not measured | not measured | 0.507 / 68.379 / 66.006-68.379 | 0.637 / 43.758 / 41.486-43.758 | 0.617 / 33.247 / 30.986-33.247 |
| User CPU (ms) | 10.000 | not captured | not captured | 60.000 | 30.000 | 40.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 10.000 | 0.000 |
| Peak RSS (MiB, source) | 14.61 (gnu-time-v) | not measured | not measured | 9.89 (gnu-time-v) | 24.40 (gnu-time-v) | 8.30 (gnu-time-v) |
| Logical throughput (records/s) | 36949.699 | not measured | not measured | 23024.784 | 36047.567 | 37486.627 |
| Physical throughput (MiB/s) | 25.037 | not measured | not measured | 15.601 | 24.391 | 30.085 |
| Output bytes | 97260 | not measured | not measured | 92810 | 92810 | 92810 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @html/@uri expression. | yq rejects the catalog jq @html/@uri expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
