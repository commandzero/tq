---
type: Report
title: "String reduction"
description: "Measures concatenation of a field collected from every feature."
workload: benchmark.string-reduction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# String reduction

## What this measures

The query collects non-null place names and combines them with `add`. It
exercises string accumulation over the entire feature collection.

## Why it matters

Reports and exports often assemble text from many records. This case makes
allocation and final-output cost visible when the result is one large string.

## Input and output

The input is a natural snapshot with `properties.place` strings. The output is
one concatenated string in feature order, not a list of place names.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
