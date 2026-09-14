---
type: Report
title: "User function mapping"
description: "Measures mapping a named user filter across a feature collection."
workload: benchmark.user-filter-map
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# User function mapping

## What this measures

The query defines a no-argument `magnitude` filter and applies it with `map`.
It combines user-function lookup with array construction over all features.

## Why it matters

Named mapping functions are common in reusable transformations. The case tests
the cost and semantics of invoking a filter once for every array member.

## Input and output

The input is a natural snapshot with a feature array. The output is one array
of magnitudes, in feature order, rather than a stream of separate numbers.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
