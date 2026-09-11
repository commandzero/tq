---
type: Report
title: "Enumerating paths"
description: "Measures listing all paths inside the metadata object."
workload: benchmark.issue5-paths
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Enumerating paths

## What this measures

The query enumerates `paths` below `.metadata` and counts them. It traverses
the metadata tree and materializes the path list before producing its length.

## Why it matters

Path inventories help schema tools, redactors, and diagnostics understand an
object without knowing its fields ahead of time.

## Input and output

The input is the metadata object from a natural snapshot. The output is one
integer containing the number of discovered paths, not the path arrays.

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
| Wall (ms) | 23.497 | not measured | not measured | 23.311 | 23.274 | 23.867 |
| Wall dispersion (MAD / p95 / range) | 0.302 / 25.387 / 22.868-25.387 | not measured | not measured | 0.285 / 24.389 / 22.797-24.389 | 0.308 / 23.952 / 21.985-23.952 | 0.519 / 24.999 / 22.883-24.999 |
| First output (ms) | 23.480 | not measured | not measured | 23.296 | 23.259 | 23.854 |
| First output dispersion (MAD / p95 / range) | 0.302 / 25.368 / 22.857-25.368 | not measured | not measured | 0.284 / 24.376 / 22.782-24.376 | 0.308 / 23.936 / 21.970-23.936 | 0.517 / 24.987 / 22.864-24.987 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.29 (gnu-time-v) | not measured | not measured | 9.10 (gnu-time-v) | 9.54 (gnu-time-v) | 8.93 (gnu-time-v) |
| Logical throughput (records/s) | 8256.197 | not measured | not measured | 8322.073 | 8335.482 | 8128.378 |
| Physical throughput (MiB/s) | 5.652 | not measured | not measured | 5.697 | 5.699 | 6.580 |
| Output bytes | 2 | not measured | not measured | 2 | 2 | 2 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq paths expression. | yq rejects the catalog jq paths expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.273 | not measured | not measured | 23.669 | 23.270 | 23.439 |
| Wall dispersion (MAD / p95 / range) | 0.443 / 24.301 / 21.862-24.646 | not measured | not measured | 0.281 / 24.335 / 21.783-24.983 | 0.411 / 24.322 / 22.005-24.408 | 0.311 / 24.475 / 21.957-24.588 |
| First output (ms) | 23.256 | not measured | not measured | 23.653 | 23.255 | 23.421 |
| First output dispersion (MAD / p95 / range) | 0.444 / 24.286 / 21.849-24.633 | not measured | not measured | 0.281 / 24.320 / 21.769-24.970 | 0.410 / 24.309 / 21.991-24.397 | 0.307 / 24.461 / 21.942-24.570 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.02 (gnu-time-v) | not measured | not measured | 7.96 (gnu-time-v) | 8.45 (gnu-time-v) | 7.91 (gnu-time-v) |
| Logical throughput (records/s) | 85.938 | not measured | not measured | 84.497 | 85.948 | 85.330 |
| Physical throughput (MiB/s) | 0.073 | not measured | not measured | 0.072 | 0.073 | 0.083 |
| Output bytes | 2 | not measured | not measured | 2 | 2 | 2 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq paths expression. | yq rejects the catalog jq paths expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 99.451 | not measured | not measured | 136.736 | 177.092 | 173.982 |
| Wall dispersion (MAD / p95 / range) | 1.069 / 103.192 / 97.186-103.192 | not measured | not measured | 1.123 / 139.127 / 133.570-139.127 | 2.620 / 182.684 / 172.090-182.684 | 2.166 / 178.185 / 141.422-178.185 |
| First output (ms) | 91.996 | not measured | not measured | 121.788 | 162.302 | 137.685 |
| First output dispersion (MAD / p95 / range) | 0.229 / 94.438 / 90.851-94.438 | not measured | not measured | 1.413 / 127.324 / 119.181-127.324 | 0.936 / 167.251 / 159.552-167.251 | 0.748 / 139.357 / 135.497-139.357 |
| User CPU (ms) | 65.000 | not captured | not captured | 90.000 | 130.000 | 100.000 |
| System CPU (ms) | 20.000 | not captured | not captured | 20.000 | 25.000 | 25.000 |
| Peak RSS (MiB, source) | 60.27 (gnu-time-v) | not measured | not measured | 69.27 (gnu-time-v) | 89.31 (gnu-time-v) | 78.15 (gnu-time-v) |
| Logical throughput (records/s) | 113362.929 | not measured | not measured | 82450.854 | 63661.825 | 64799.807 |
| Physical throughput (MiB/s) | 76.842 | not measured | not measured | 55.888 | 43.092 | 52.024 |
| Output bytes | 2 | not measured | not measured | 2 | 2 | 2 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq paths expression. | yq rejects the catalog jq paths expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.818 | not measured | not measured | 60.173 | 61.415 | 60.551 |
| Wall dispersion (MAD / p95 / range) | 0.455 / 25.410 / 22.865-25.410 | not measured | not measured | 0.655 / 61.914 / 58.847-61.914 | 0.595 / 62.781 / 60.374-62.781 | 0.523 / 62.187 / 59.135-62.187 |
| First output (ms) | 20.454 | not measured | not measured | 26.094 | 33.614 | 28.963 |
| First output dispersion (MAD / p95 / range) | 0.345 / 25.233 / 19.435-25.233 | not measured | not measured | 0.454 / 27.703 / 24.795-27.703 | 0.189 / 34.241 / 31.725-34.241 | 0.641 / 30.563 / 27.958-30.563 |
| User CPU (ms) | 10.000 | not captured | not captured | 20.000 | 20.000 | 20.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.65 (gnu-time-v) | not measured | not measured | 20.10 (gnu-time-v) | 24.21 (gnu-time-v) | 21.71 (gnu-time-v) |
| Logical throughput (records/s) | 93416.744 | not measured | not measured | 36976.410 | 36228.934 | 36745.884 |
| Physical throughput (MiB/s) | 63.298 | not measured | not measured | 25.055 | 24.514 | 29.490 |
| Output bytes | 2 | not measured | not measured | 2 | 2 | 2 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq paths expression. | yq rejects the catalog jq paths expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
