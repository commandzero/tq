---
type: Report
title: "jq: how to filter an array of objects based on values in an inner array?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 05."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
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
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `structured`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 3.135 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.130 | +63.7% | 15.31 (darwin-wait4) | +469.8% | 30 (+1 warmup) |
| tq | TOON | timed | 2.942 | -6.1% | 4.64 (darwin-wait4) | +72.7% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.[] | select(.Names[] == "foo_data") | .Id'`

```text
"a2"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.[] | select(.Names[] == "foo_data") | .Id'`

```text
"a2"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.[] | select(.Names[] == "foo_data") | .Id'`

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
