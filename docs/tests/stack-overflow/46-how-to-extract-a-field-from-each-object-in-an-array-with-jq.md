---
type: Report
title: "How to extract a field from each object in an array with jq?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 46."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 46: How to extract a field from each object in an array with jq?

[Stack Overflow question](https://stackoverflow.com/questions/48426052/how-to-extract-a-field-from-each-object-in-an-array-with-jq) · [selected answer](https://stackoverflow.com/a/48426543)

Rank: 46 · Question score: 84

Fixture: [source data](../../../tests/stack-overflow/46-how-to-extract-a-field-from-each-object-in-an-array-with-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[].username`

```json
[
  {
    "username": "jdoe",
    "name": "John Doe"
  },
  {
    "username": "jadoe",
    "name": "Jane Doe"
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
| jq | JSON | timed | 3.228 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.160 | +59.8% | 15.17 (darwin-wait4) | +467.8% | 30 (+1 warmup) |
| tq | TOON | timed | 2.572 | -20.3% | 3.95 (darwin-wait4) | +48.0% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.[].username'`

```text
"jdoe"
"jadoe"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.[].username'`

```text
"jdoe"
"jadoe"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.[].username'`

```text
jdoe
jadoe
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 9 | 7 | -2 | -22.22% |
| `o200k_base` | Expanded | 9 | 7 | -2 | -22.22% |
| `cl100k_base` | Compact | 10 | 8 | -2 | -20.00% |
| `cl100k_base` | Expanded | 10 | 8 | -2 | -20.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
