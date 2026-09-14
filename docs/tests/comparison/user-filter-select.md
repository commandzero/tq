---
type: Report
title: "User function selection"
description: "Measures filtering with a named predicate and returning matching IDs."
workload: benchmark.user-filter-select
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# User function selection

## What this measures

The query defines `significant` as a magnitude predicate, applies it inside
`select`, and collects the matching IDs. It tests callback invocation and
conditional selection in one blocking result.

## Why it matters

Teams often turn business rules into named filters. This case checks that the
rule remains usable inside a collection operation instead of only inline.

## Input and output

The input is a natural feature snapshot. The output is one array of IDs whose
magnitudes meet the threshold. It does not return the full selected features.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
