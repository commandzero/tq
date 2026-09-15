---
type: Report
title: "jq select value from array"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 48."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
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
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `structured`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 3.107 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.417 | +74.3% | 15.52 (darwin-wait4) | +477.3% | 30 (+1 warmup) |
| tq | TOON | timed | 2.588 | -16.7% | 4.58 (darwin-wait4) | +70.3% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.[] | select(.name == "foo")'`

```text
{"name":"foo","id":1}
{"name":"foo","id":3}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.[] | select(.name == "foo")'`

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

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.[] | select(.name == "foo")'`

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
