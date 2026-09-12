---
type: Report
title: "getting all the values of an array with jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 49."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 49: getting all the values of an array with jq

[Stack Overflow question](https://stackoverflow.com/questions/45523425/getting-all-the-values-of-an-array-with-jq) · [selected answer](https://stackoverflow.com/a/45524015)

Rank: 49 · Question score: 80

Fixture: [source data](../../../tests/stack-overflow/49-getting-all-the-values-of-an-array-with-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.response[] | select(has("text")) | .text`

```json
{
  "response": [
    {
      "text": "first"
    },
    {
      "status": 200
    },
    {
      "text": "second"
    }
  ]
}
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
| jq | JSON | timed | 97.556 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 103.172 | +5.8% | 15.78 (bsd-time-l) | +504.8% | 30 (+1 warmup) |
| tq | TOON | timed | 104.933 | +7.6% | 4.45 (bsd-time-l) | +70.7% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"first"
"second"
```

#### Expanded JSON (`jq`)

```text
"first"
"second"
```

#### TOON (`tq -o toon`)

```text
first
second
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 6 | 4 | -2 | -33.33% |
| `o200k_base` | Expanded | 6 | 4 | -2 | -33.33% |
| `cl100k_base` | Compact | 6 | 4 | -2 | -33.33% |
| `cl100k_base` | Expanded | 6 | 4 | -2 | -33.33% |
<!-- STACK_OVERFLOW_RESULTS_END -->
