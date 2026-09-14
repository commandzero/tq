---
type: Report
title: "User function sorting"
description: "Measures sorting with a named key filter and projecting IDs."
workload: benchmark.user-filter-sort-by
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# User function sorting

## What this measures

The query defines `magnitude`, uses it as the `sort_by` key, and maps the
ordered features to IDs. It exercises a user filter in a sorting callback.

## Why it matters

Sorting by a reusable key function is a practical pattern for reports and
ranking. It also combines callback overhead with a blocking operation.

## Input and output

The input is a natural feature snapshot. The output is one array of IDs ordered
by each feature's magnitude, not the sorted feature objects themselves.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
