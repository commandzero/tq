---
type: Report
title: "Sort by multiple keys"
description: "Measures sorting feature objects by magnitude and then ID."
workload: benchmark.comma-generator-sort
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Sort by multiple keys

## What this measures

The query sorts the feature objects by `.properties.mag` and `.id`. The comma
generator supplies a second key for ties, so the whole collection is retained
until ordering is known.

## Why it matters

Stable tie-breaking makes reports and pagination repeatable. It also costs more
than sorting a single scalar because the result keeps complete feature objects.

## Input and output

The input is a natural feature snapshot. The output is one array of the
original feature objects, ordered by magnitude and then ID, rather than an
array of just the sort keys.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
