---
type: Report
title: "Reading additional inputs"
description: "Measures consuming the current value and following input values."
workload: benchmark.issue5-inputs
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Reading additional inputs

## What this measures

The query builds `[., inputs]` and counts the values. It exercises the CLI input
sequence, including the difference between the current value and values read by
`inputs`.

## Why it matters

Filters that combine multiple JSON roots are useful for batch files and stream
processors. They also expose buffering and input-boundary behavior that a
single-document query cannot show.

## Input and output

The input is the medium issue-sequence stream, with one current value followed
by additional values. The output is one integer equal to the number of values
consumed, not the values themselves.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
