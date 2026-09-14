---
type: Report
title: "Bounded recursion"
description: "Measures recursive traversal with a fixed output limit."
workload: benchmark.recurse-bounded
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Bounded recursion

## What this measures

The query recursively visits child values with `recurse(.[]?)` and keeps at
most 2,048 results. It tests traversal while putting a clear bound on work and
output.

## Why it matters

Recursive queries are useful for irregular documents, but an unbounded walk can
run away on large or cyclic-looking data. A fixed limit is a practical guard.

## Input and output

The input is a natural nested snapshot. The output is the depth-first sequence
of visited values up to the limit, including containers and scalars as the
recursion produces them.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
