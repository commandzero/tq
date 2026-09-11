---
type: Report
title: "Structural walking"
description: "Measures a post-order walk that increments every numeric value."
workload: benchmark.walk-structural
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Structural walking

## What this measures

The `walk` query visits the whole document and adds one to every value whose
type is `number`. It combines recursive traversal, type checks, and rebuilding
the original structure.

## Why it matters

Tree-wide edits support migrations and normalization when the fields are not
known in advance. The result must preserve shape while changing selected
leaves.

## Input and output

The input is a natural nested snapshot. The output is one complete document
with every numeric leaf incremented; strings, booleans, arrays, and object keys
keep their roles.

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
| Wall (ms) | 23.433 | not measured | not measured | 23.337 | 23.367 | 22.968 |
| Wall dispersion (MAD / p95 / range) | 0.288 / 24.801 / 22.831-24.801 | not measured | not measured | 0.382 / 25.160 / 22.370-25.160 | 0.572 / 24.185 / 22.064-24.185 | 0.302 / 23.507 / 22.257-23.507 |
| First output (ms) | 15.998 | not measured | not measured | 14.100 | 14.442 | 13.838 |
| First output dispersion (MAD / p95 / range) | 0.470 / 16.760 / 14.934-16.760 | not measured | not measured | 0.490 / 14.764 / 13.204-14.764 | 0.361 / 15.453 / 13.375-15.453 | 0.299 / 15.296 / 13.508-15.296 |
| User CPU (ms) | 10.000 | not captured | not captured | 10.000 | 10.000 | 10.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.90 (gnu-time-v) | not measured | not measured | 9.79 (gnu-time-v) | 10.62 (gnu-time-v) | 9.94 (gnu-time-v) |
| Logical throughput (records/s) | 8279.100 | not measured | not measured | 8312.979 | 8302.129 | 8446.718 |
| Physical throughput (MiB/s) | 5.668 | not measured | not measured | 5.691 | 5.676 | 6.838 |
| Output bytes | 213177 | not measured | not measured | 165349 | 165349 | 165349 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq walk expression. | yq rejects the catalog jq walk expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.451 | not measured | not measured | 23.349 | 23.343 | 23.646 |
| Wall dispersion (MAD / p95 / range) | 0.423 / 24.583 / 22.462-24.859 | not measured | not measured | 0.346 / 23.930 / 22.357-24.190 | 0.379 / 24.392 / 22.079-24.653 | 0.605 / 26.929 / 22.379-27.413 |
| First output (ms) | 23.438 | not measured | not measured | 23.337 | 23.329 | 23.631 |
| First output dispersion (MAD / p95 / range) | 0.422 / 24.572 / 22.448-24.847 | not measured | not measured | 0.348 / 23.912 / 22.344-24.175 | 0.385 / 24.381 / 22.063-24.643 | 0.606 / 26.906 / 22.364-27.394 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | not measured | not measured | 8.41 (gnu-time-v) | 8.61 (gnu-time-v) | 8.16 (gnu-time-v) |
| Logical throughput (records/s) | 85.282 | not measured | not measured | 85.657 | 85.681 | 84.581 |
| Physical throughput (MiB/s) | 0.072 | not measured | not measured | 0.072 | 0.073 | 0.082 |
| Output bytes | 2623 | not measured | not measured | 2037 | 2037 | 2037 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq walk expression. | yq rejects the catalog jq walk expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 952.231 | not measured | not measured | 695.054 | 740.195 | 734.899 |
| Wall dispersion (MAD / p95 / range) | 8.035 / 964.146 / 920.190-964.146 | not measured | not measured | 3.686 / 739.840 / 688.656-739.840 | 5.923 / 771.164 / 729.928-771.164 | 2.886 / 738.896 / 701.005-738.896 |
| First output (ms) | 792.380 | not measured | not measured | 593.344 | 638.347 | 610.541 |
| First output dispersion (MAD / p95 / range) | 7.179 / 825.010 / 781.583-825.010 | not measured | not measured | 3.592 / 648.880 / 589.660-648.880 | 2.586 / 652.305 / 634.417-652.305 | 0.500 / 617.238 / 605.034-617.238 |
| User CPU (ms) | 870.000 | not captured | not captured | 625.000 | 660.000 | 640.000 |
| System CPU (ms) | 40.000 | not captured | not captured | 50.000 | 50.000 | 50.000 |
| Peak RSS (MiB, source) | 86.61 (gnu-time-v) | not measured | not measured | 107.47 (gnu-time-v) | 127.68 (gnu-time-v) | 116.25 (gnu-time-v) |
| Logical throughput (records/s) | 11839.564 | not measured | not measured | 16220.322 | 15231.122 | 15340.884 |
| Physical throughput (MiB/s) | 8.025 | not measured | not measured | 10.995 | 10.310 | 12.316 |
| Output bytes | 12307507 | not measured | not measured | 9534923 | 9534923 | 9534923 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq walk expression. | yq rejects the catalog jq walk expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 209.498 | not measured | not measured | 137.941 | 174.998 | 173.410 |
| Wall dispersion (MAD / p95 / range) | 0.617 / 210.601 / 207.322-210.601 | not measured | not measured | 0.600 / 174.961 / 136.314-174.961 | 1.472 / 182.256 / 171.102-182.256 | 0.573 / 174.664 / 170.983-174.664 |
| First output (ms) | 156.089 | not measured | not measured | 120.141 | 127.409 | 123.885 |
| First output dispersion (MAD / p95 / range) | 0.514 / 158.313 / 152.977-158.313 | not measured | not measured | 0.898 / 122.915 / 119.016-122.915 | 1.202 / 130.501 / 125.814-130.501 | 1.058 / 126.657 / 121.915-126.657 |
| User CPU (ms) | 160.000 | not captured | not captured | 120.000 | 125.000 | 120.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 10.000 | 10.000 | 10.000 |
| Peak RSS (MiB, source) | 19.86 (gnu-time-v) | not measured | not measured | 27.86 (gnu-time-v) | 31.90 (gnu-time-v) | 29.30 (gnu-time-v) |
| Logical throughput (records/s) | 10620.626 | not measured | not measured | 16130.085 | 12714.467 | 12830.863 |
| Physical throughput (MiB/s) | 7.196 | not measured | not measured | 10.930 | 8.603 | 10.297 |
| Output bytes | 2427690 | not measured | not measured | 1880294 | 1880294 | 1880294 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq walk expression. | yq rejects the catalog jq walk expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
