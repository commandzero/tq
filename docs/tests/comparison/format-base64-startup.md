---
type: Report
title: "Base64 startup formatting"
description: "Measures startup cost for a small Base64 formatting query."
workload: benchmark.format-base64-startup
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Base64 startup formatting

## What this measures

The query applies `@base64` to the small synthetic startup helper input. It
combines a short-lived invocation with one formatting operation.

## Why it matters

Encoding a small token or payload is a common CLI task. This page separates
formatting startup cost from the larger streaming and document workloads.

## Input and output

The input is the catalog's small synthetic helper value. The output is one
Base64-encoded string, rather than the original value or a JSON object.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
