---
type: Report
title: "Blocking sort"
description: "Measures collecting and sorting all feature magnitudes."
workload: benchmark.blocking-sort
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Blocking sort

## What this measures

The query collects `.properties.mag` from every feature and applies `sort`. It
must consume the collection before it can emit the sorted array.

## Why it matters

Ranking and threshold preparation are common jobs where input size affects both
memory and time. This is a clear contrast with streaming projection.

## Input and output

The input is a natural feature snapshot. The output is one array of magnitudes
in ascending order, including the collected values rather than the original
feature objects.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
