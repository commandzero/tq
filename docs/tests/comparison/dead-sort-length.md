---
type: Report
title: "Sort before counting"
description: "Measures work whose sorted intermediate does not affect the final count."
workload: benchmark.dead-sort-length
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Sort before counting

## What this measures

The query collects the `release` values, sorts them, and then asks for the
length. The final count does not depend on the order, so the case exposes the
cost of doing unnecessary intermediate work.

## Why it matters

Real queries sometimes carry an expensive step that a later operation does not
use. This is a useful stress case for planning and optimization decisions.

## Input and output

The input is a natural feature snapshot. The output is one integer, the number
of collected values. It contains no sorted values.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
