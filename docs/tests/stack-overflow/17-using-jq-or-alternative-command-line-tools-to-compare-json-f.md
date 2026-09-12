---
type: Report
title: "Using jq or alternative command line tools to compare JSON files"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 17."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 17: Using jq or alternative command line tools to compare JSON files

[Stack Overflow question](https://stackoverflow.com/questions/31930041/using-jq-or-alternative-command-line-tools-to-compare-json-files) · [selected answer](https://stackoverflow.com/a/31933234)

Rank: 17 · Question score: 214

Fixture: [source data](../../../tests/stack-overflow/17-using-jq-or-alternative-command-line-tools-to-compare-json-f.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[0].City == .[1].City`

```json
[
  {
    "People": [
      "John",
      "Bryan"
    ],
    "City": "Boston",
    "State": "MA"
  },
  {
    "People": [
      "Bryan",
      "John"
    ],
    "State": "MA",
    "City": "Boston"
  }
]
```

Scenario source registry: [jq questions sorted by votes](https://stackoverflow.com/questions/tagged/jq?tab=votes&pagesize=50)

## Results
<!-- STACK_OVERFLOW_RESULTS_START -->
Last updated: 2026-09-11
Status: `passed`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.1; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)  
Memory evidence: authoritative peak RSS from the recorded time implementation; sampled-group RSS is not used as a substitute.  

| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 97.302 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 112.037 | +15.1% | 15.75 (bsd-time-l) | +503.6% | 30 (+1 warmup) |
| tq | TOON | timed | 88.639 | -8.9% | 4.06 (bsd-time-l) | +55.7% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
true
```

#### Expanded JSON (`jq`)

```text
true
```

#### TOON (`tq -o toon`)

```text
true
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 2 | 2 | +0 | +0.00% |
| `o200k_base` | Expanded | 2 | 2 | +0 | +0.00% |
| `cl100k_base` | Compact | 2 | 2 | +0 | +0.00% |
| `cl100k_base` | Expanded | 2 | 2 | +0 | +0.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
