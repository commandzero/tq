---
type: Report
title: "Grouping a collection"
description: "Measures grouping features by magnitude and counting the groups."
workload: benchmark.issue5-collection
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Grouping a collection

## What this measures

The query groups the feature collection by `.properties.mag` and returns the
number of groups. It exercises sorting and collection grouping even though the
final result is only a count.

## Why it matters

Grouping supports summaries such as counts by severity or category. It must
keep enough information to form complete groups before reporting the count.

## Input and output

The input is a natural feature snapshot. The output is one integer for the
number of distinct magnitude groups, not the grouped arrays themselves.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
