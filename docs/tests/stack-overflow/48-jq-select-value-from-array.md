---
type: Report
title: "jq select value from array"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 48."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 48: jq select value from array

[Stack Overflow question](https://stackoverflow.com/questions/37563691/jq-select-value-from-array) · [selected answer](https://stackoverflow.com/a/37567004)

Rank: 48 · Question score: 82

Fixture: [source data](../../../tests/stack-overflow/48-jq-select-value-from-array.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[] | select(.name == "foo")`

```json
[
  {
    "name": "foo",
    "id": 1
  },
  {
    "name": "bar",
    "id": 2
  },
  {
    "name": "foo",
    "id": 3
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
| jq | JSON | timed | 99.203 | +0.0% | 2.59 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 105.602 | +6.5% | 15.81 (bsd-time-l) | +509.6% | 30 (+1 warmup) |
| tq | TOON | timed | 95.143 | -4.1% | 4.48 (bsd-time-l) | +72.9% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"name":"foo","id":1}
{"name":"foo","id":3}
```

#### Expanded JSON (`jq`)

```text
{
  "name": "foo",
  "id": 1
}
{
  "name": "foo",
  "id": 3
}
```

#### TOON (`tq -o toon`)

```text
name: foo
id: 1
name: foo
id: 3
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 18 | 18 | +0 | +0.00% |
| `o200k_base` | Expanded | 32 | 18 | -14 | -43.75% |
| `cl100k_base` | Compact | 18 | 18 | +0 | +0.00% |
| `cl100k_base` | Expanded | 32 | 18 | -14 | -43.75% |
<!-- STACK_OVERFLOW_RESULTS_END -->
