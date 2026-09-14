---
type: Report
title: "Discarding a stream"
description: "Measures parsing and stream control when a query emits no values."
workload: benchmark.parse-discard
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Discarding a stream

## What this measures

The `empty` query reads each natural input and emits nothing. The workload
isolates input handling and the cost of a filter that discards its result.

## Why it matters

Log and validation pipelines often reject or discard records early. This case
shows the cost of doing that without paying to serialize an output document.

## Input and output

The input is a natural JSON, YAML, or TOON snapshot at each catalog size. The
output is an empty value sequence, not an empty array. That distinction keeps
parsing cost separate from output construction.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
