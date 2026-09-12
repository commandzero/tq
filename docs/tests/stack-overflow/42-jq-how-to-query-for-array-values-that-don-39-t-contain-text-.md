---
type: Report
title: "jq: how to query for array values that don't contain text \"foo\"?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 42."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 42: jq: how to query for array values that don't contain text "foo"?

[Stack Overflow question](https://stackoverflow.com/questions/42746828/jq-how-to-query-for-array-values-that-dont-contain-text-foo) · [selected answer](https://stackoverflow.com/a/42747910)

Rank: 42 · Question score: 92

Fixture: [source data](../../../tests/stack-overflow/42-jq-how-to-query-for-array-values-that-don-39-t-contain-text-.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[] | .name`

```json
[
  {
    "name": "one",
    "imageTags": [
      "latest",
      "stable"
    ]
  },
  {
    "name": "two",
    "imageTags": [
      "foo",
      "latest"
    ]
  },
  {
    "name": "three",
    "imageTags": []
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
| jq | JSON | timed | 137.188 | +0.0% | 2.59 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 132.101 | -3.7% | 15.72 (bsd-time-l) | +506.0% | 30 (+1 warmup) |
| tq | TOON | timed | 134.951 | -1.6% | 3.92 (bsd-time-l) | +51.2% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"one"
"two"
"three"
```

#### Expanded JSON (`jq`)

```text
"one"
"two"
"three"
```

#### TOON (`tq -o toon`)

```text
one
two
three
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 9 | 6 | -3 | -33.33% |
| `o200k_base` | Expanded | 9 | 6 | -3 | -33.33% |
| `cl100k_base` | Compact | 9 | 6 | -3 | -33.33% |
| `cl100k_base` | Expanded | 9 | 6 | -3 | -33.33% |
<!-- STACK_OVERFLOW_RESULTS_END -->
