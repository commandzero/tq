---
type: Report
title: "Startup identity"
description: "Measures the fixed cost of launching a query that returns its input."
workload: benchmark.startup
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Startup identity

## What this measures

The query is `.` on a small synthetic helper input. It measures the startup
path and an identity query, including the cost of launching a short-lived
command rather than processing a large document.

## Why it matters

Many scripts invoke a JSON tool for one small value. Startup cost can dominate
that kind of use, so this is a useful baseline for CLI latency.

## Input and output

The input is one small JSON value. The output is the same semantic value and
object order. No filtering or transformation is part of this workload.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
