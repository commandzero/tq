---
type: Report
title: "Identity re-encoding"
description: "Measures reading and writing a complete natural document unchanged."
workload: benchmark.identity-reencode
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Identity re-encoding

## What this measures

The query is `.` on natural snapshots. Unlike the startup identity case, the
documents span the catalog's small, medium, and large tiers, exposing full
parse and output costs.

## Why it matters

Format conversion and pass-through filters are useful baselines for pipelines
that do not change the data. They show the cost of carrying a document through
the tool.

## Input and output

The input is a complete natural snapshot in the selected format. The output is
the same semantic document, including its structure and object members, not a
single field or a stream of projected values.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
