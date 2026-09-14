---
type: Report
title: "Array construction"
description: "Measures building one array of compact objects from a feature collection."
workload: benchmark.array-construction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Array construction

## What this measures

The query maps every feature to `{id, mag}` and wraps the results in one array.
It measures collection growth and object creation in a blocking pipeline.

## Why it matters

APIs and batch jobs often need a compact array for one downstream request. The
cost includes retaining all projected values until the array is complete.

## Input and output

The input is a natural feature snapshot. The output is one array with one
object per feature, containing only `id` and `mag`, rather than a stream of
individual objects.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
