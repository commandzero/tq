---
type: Report
title: "User function calls"
description: "Measures passing a value through a user-defined filter function."
workload: benchmark.user-filter-call
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# User function calls

## What this measures

The query defines `magnitude(f)` and calls it for each feature's magnitude.
This isolates function definition, argument passing, and repeated invocation.

## Why it matters

Reusable filters make production queries easier to maintain. A function call
should keep the same stream behavior as the equivalent inline expression.

## Input and output

The input is a natural feature snapshot. The output is one magnitude per
feature in input order, emitted as a sequence after passing through the user
function.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
