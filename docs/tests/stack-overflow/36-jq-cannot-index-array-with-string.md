---
type: Report
title: "jq: Cannot index array with string"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 36."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
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
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `structured`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 3.245 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.577 | +71.8% | 15.20 (darwin-wait4) | +469.0% | 30 (+1 warmup) |
| tq | TOON | timed | 2.584 | -20.4% | 4.58 (darwin-wait4) | +71.3% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.[] | .aux[] | .["def"]'`

```text
"I want this"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.[] | .aux[] | .["def"]'`

```text
"I want this"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.[] | .aux[] | .["def"]'`

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
