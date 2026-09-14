---
type: Report
title: "Enumerating paths"
description: "Measures listing all paths inside the metadata object."
workload: benchmark.issue5-paths
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Enumerating paths

## What this measures

The query enumerates `paths` below `.metadata` and counts them. It traverses
the metadata tree and materializes the path list before producing its length.

## Why it matters

Path inventories help schema tools, redactors, and diagnostics understand an
object without knowing its fields ahead of time.

## Input and output

The input is the metadata object from a natural snapshot. The output is one
integer containing the number of discovered paths, not the path arrays.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
