---
type: Report
title: "How to filter array of objects by element property values using jq?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 34."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
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
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `structured`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 3.112 | +0.0% | 2.70 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.189 | +66.7% | 15.50 (darwin-wait4) | +473.4% | 30 (+1 warmup) |
| tq | TOON | timed | 3.139 | +0.9% | 4.70 (darwin-wait4) | +74.0% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.theList[] | select(.id == 2 or .id == 4)'`

```text
{"id":2,"name":"Fritz"}
{"id":4,"name":"Gerhart"}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.theList[] | select(.id == 2 or .id == 4)'`

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

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.theList[] | select(.id == 2 or .id == 4)'`

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
