---
type: Report
title: "How to check if element exists in array with jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 47."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 47: How to check if element exists in array with jq

[Stack Overflow question](https://stackoverflow.com/questions/43259563/how-to-check-if-element-exists-in-array-with-jq) · [selected answer](https://stackoverflow.com/a/43269105)

Rank: 47 · Question score: 82

Fixture: [source data](../../../tests/stack-overflow/47-how-to-check-if-element-exists-in-array-with-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.fruit[] | select(. == "orange")`

```json
{
  "fruit": [
    "apple",
    "orange",
    "pomegranate",
    "apricot",
    "mango"
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
| jq | JSON | timed | 91.188 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 113.095 | +24.0% | 15.53 (bsd-time-l) | +495.2% | 30 (+1 warmup) |
| tq | TOON | timed | 100.578 | +10.3% | 4.44 (bsd-time-l) | +70.1% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"orange"
```

#### Expanded JSON (`jq`)

```text
"orange"
```

#### TOON (`tq -o toon`)

```text
orange
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 3 | 2 | -1 | -33.33% |
| `o200k_base` | Expanded | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | Compact | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | Expanded | 3 | 2 | -1 | -33.33% |
<!-- STACK_OVERFLOW_RESULTS_END -->
