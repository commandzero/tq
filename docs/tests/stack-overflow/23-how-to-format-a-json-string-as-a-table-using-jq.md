---
type: Report
title: "How to format a JSON string as a table using jq?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 23."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 23: How to format a JSON string as a table using jq?

[Stack Overflow question](https://stackoverflow.com/questions/39139107/how-to-format-a-json-string-as-a-table-using-jq) · [selected answer](https://stackoverflow.com/a/39139478)

Rank: 23 · Question score: 166

Fixture: [source data](../../../tests/stack-overflow/23-how-to-format-a-json-string-as-a-table-using-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[] | [.id, .name]`

```json
[
  {
    "name": "George",
    "id": 12,
    "email": "george@domain.example"
  },
  {
    "name": "Jack",
    "id": 18,
    "email": "jack@domain.example"
  },
  {
    "name": "Joe",
    "id": 19,
    "email": "joe@domain.example"
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
| jq | JSON | timed | 106.678 | +0.0% | 2.58 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 103.266 | -3.2% | 15.92 (bsd-time-l) | +517.6% | 30 (+1 warmup) |
| tq | TOON | timed | 100.138 | -6.1% | 4.16 (bsd-time-l) | +61.2% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
[12,"George"]
[18,"Jack"]
[19,"Joe"]
```

#### Expanded JSON (`jq`)

```text
[
  12,
  "George"
]
[
  18,
  "Jack"
]
[
  19,
  "Joe"
]
```

#### TOON (`tq -o toon`)

```text
[2]: 12,George
[2]: 18,Jack
[2]: 19,Joe
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 15 | 24 | +9 | +60.00% |
| `o200k_base` | Expanded | 30 | 24 | -6 | -20.00% |
| `cl100k_base` | Compact | 15 | 24 | +9 | +60.00% |
| `cl100k_base` | Expanded | 30 | 24 | -6 | -20.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
