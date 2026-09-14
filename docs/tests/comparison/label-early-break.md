---
type: Report
title: "Early break with a label"
description: "Measures stopping a feature traversal after the first result."
workload: benchmark.label-early-break
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Early break with a label

## What this measures

The labeled query emits a feature ID and then breaks to the outer label. It
tests non-local control flow and whether later features are left unread.

## Why it matters

First-match queries should stop work when the answer is known. Labels provide a
way to express that exit when a simple predicate is not enough.

## Input and output

The input is a natural feature snapshot. The output is the early result
sequence, normally the first feature ID, rather than every feature ID in the
document.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
