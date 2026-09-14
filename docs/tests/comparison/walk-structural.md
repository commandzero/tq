---
type: Report
title: "Structural walking"
description: "Measures a post-order walk that increments every numeric value."
workload: benchmark.walk-structural
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Structural walking

## What this measures

The `walk` query visits the whole document and adds one to every value whose
type is `number`. It combines recursive traversal, type checks, and rebuilding
the original structure.

## Why it matters

Tree-wide edits support migrations and normalization when the fields are not
known in advance. The result must preserve shape while changing selected
leaves.

## Input and output

The input is a natural nested snapshot. The output is one complete document
with every numeric leaf incremented; strings, booleans, arrays, and object keys
keep their roles.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
