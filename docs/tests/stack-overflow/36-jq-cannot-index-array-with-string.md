---
type: Report
title: "jq: Cannot index array with string"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 36."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 36: jq: Cannot index array with string

[Stack Overflow question](https://stackoverflow.com/questions/34543829/jq-cannot-index-array-with-string) · [selected answer](https://stackoverflow.com/a/34544406)

Rank: 36 · Question score: 115

Fixture: [source data](../../../tests/stack-overflow/36-jq-cannot-index-array-with-string.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[] | .aux[] | .["def"]`

```json
[
  {
    "id": 123,
    "name": "John",
    "aux": [
      {
        "abc": "random",
        "def": "I want this"
      }
    ],
    "blah": 23.11
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
| jq | JSON | timed | 104.365 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 100.417 | -3.8% | 15.67 (bsd-time-l) | +500.6% | 30 (+1 warmup) |
| tq | TOON | timed | 106.959 | +2.5% | 4.42 (bsd-time-l) | +69.5% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"I want this"
```

#### Expanded JSON (`jq`)

```text
"I want this"
```

#### TOON (`tq -o toon`)

```text
I want this
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 4 | 4 | +0 | +0.00% |
| `o200k_base` | Expanded | 4 | 4 | +0 | +0.00% |
| `cl100k_base` | Compact | 4 | 4 | +0 | +0.00% |
| `cl100k_base` | Expanded | 4 | 4 | +0 | +0.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
