---
type: Report
title: "jq: how to filter an array of objects based on values in an inner array?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 05."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 05: jq: how to filter an array of objects based on values in an inner array?

[Stack Overflow question](https://stackoverflow.com/questions/26701538/jq-how-to-filter-an-array-of-objects-based-on-values-in-an-inner-array) · [selected answer](https://stackoverflow.com/a/26701851)

Rank: 5 · Question score: 518

Fixture: [source data](../../../tests/stack-overflow/05-jq-how-to-filter-an-array-of-objects-based-on-values-in-an-i.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[] | select(.Names[] == "foo_data") | .Id`

```json
[
  {
    "Id": "a1",
    "Names": [
      "condescending_jones",
      "loving_hoover"
    ]
  },
  {
    "Id": "a2",
    "Names": [
      "foo_data"
    ]
  },
  {
    "Id": "a3",
    "Names": [
      "jovial_wozniak"
    ]
  },
  {
    "Id": "a4",
    "Names": [
      "bar_data"
    ]
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
| jq | JSON | timed | 182.846 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 182.848 | +0.0% | 15.78 (bsd-time-l) | +504.8% | 30 (+1 warmup) |
| tq | TOON | timed | 183.104 | +0.1% | 4.53 (bsd-time-l) | +73.7% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"a2"
```

#### Expanded JSON (`jq`)

```text
"a2"
```

#### TOON (`tq -o toon`)

```text
a2
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 3 | 3 | +0 | +0.00% |
| `o200k_base` | Expanded | 3 | 3 | +0 | +0.00% |
| `cl100k_base` | Compact | 3 | 3 | +0 | +0.00% |
| `cl100k_base` | Expanded | 3 | 3 | +0 | +0.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
