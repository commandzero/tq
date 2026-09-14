---
type: Report
title: "Deep object merge"
description: "Measures recursive merge semantics on a pair of helper objects."
workload: benchmark.object-deep-merge
generated: { by: codex/gpt-5.6-luna, at: 2026-09-12T17:42:14Z }
---

# Deep object merge

## What this measures

The query `.[0] * .[1]` multiplies two helper objects using jq's deep-merge
operator. The workload is document-sized and focuses on recursive object
combination rather than a large corpus scan.

## Why it matters

Configuration overlays and defaults often merge nested objects. A shallow
replacement can silently lose fields, so this case checks the deeper behavior.

## Input and output

The input is a generated two-element array containing two 10,000-entry objects
to merge. The output is one merged object with nested values combined according
to the operator's semantics.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
