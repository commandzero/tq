---
type: Report
title: "Recursive scalar traversal"
description: "Measures depth-first traversal that emits every scalar value."
workload: benchmark.recursive-scalars
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Recursive scalar traversal

## What this measures

The query `.. | scalars` walks nested arrays and objects, then emits each
number, string, boolean, or null in traversal order. It combines recursion with
scalar filtering.

## Why it matters

Schema discovery, indexing, and redaction tools often need to inspect every
leaf in an unknown document. Depth and nesting affect the amount of work.

## Input and output

The input is a nested natural snapshot. The output is a depth-first sequence of
scalar leaves, not the containers that held them and not one flattened array.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
