---
type: Report
title: "How to filter array of objects by element property values using jq?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 34."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 34: How to filter array of objects by element property values using jq?

[Stack Overflow question](https://stackoverflow.com/questions/38121740/how-to-filter-array-of-objects-by-element-property-values-using-jq) · [selected answer](https://stackoverflow.com/a/38122029)

Rank: 34 · Question score: 120

Fixture: [source data](../../../tests/stack-overflow/34-how-to-filter-array-of-objects-by-element-property-values-us.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.theList[] | select(.id == 2 or .id == 4)`

```json
{
  "theList": [
    {
      "id": 1,
      "name": "Horst"
    },
    {
      "id": 2,
      "name": "Fritz"
    },
    {
      "id": 3,
      "name": "Walter"
    },
    {
      "id": 4,
      "name": "Gerhart"
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
| jq | JSON | timed | 99.588 | +0.0% | 2.58 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 99.376 | -0.2% | 16.00 (bsd-time-l) | +520.6% | 30 (+1 warmup) |
| tq | TOON | timed | 103.511 | +3.9% | 4.58 (bsd-time-l) | +77.6% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"id":2,"name":"Fritz"}
{"id":4,"name":"Gerhart"}
```

#### Expanded JSON (`jq`)

```text
{
  "id": 2,
  "name": "Fritz"
}
{
  "id": 4,
  "name": "Gerhart"
}
```

#### TOON (`tq -o toon`)

```text
id: 2
name: Fritz
id: 4
name: Gerhart
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 20 | 19 | -1 | -5.00% |
| `o200k_base` | Expanded | 34 | 19 | -15 | -44.12% |
| `cl100k_base` | Compact | 20 | 19 | -1 | -5.00% |
| `cl100k_base` | Expanded | 34 | 19 | -15 | -44.12% |
<!-- STACK_OVERFLOW_RESULTS_END -->
