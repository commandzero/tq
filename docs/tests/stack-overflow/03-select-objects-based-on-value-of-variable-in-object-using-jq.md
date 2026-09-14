---
type: Report
title: "Select objects based on value of variable in object using jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 03."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 03: Select objects based on value of variable in object using jq

[Stack Overflow question](https://stackoverflow.com/questions/18592173/select-objects-based-on-value-of-variable-in-object-using-jq) · [selected answer](https://stackoverflow.com/a/18608100)

Rank: 3 · Question score: 556

Fixture: [source data](../../../tests/stack-overflow/03-select-objects-based-on-value-of-variable-in-object-using-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[] | select(.location == "Stockholm") | .name`

```json
{
  "FOO": {
    "name": "Donald",
    "location": "Stockholm"
  },
  "BAR": {
    "name": "Walt",
    "location": "Stockholm"
  },
  "BAZ": {
    "name": "Jack",
    "location": "Whereever"
  }
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
| jq | JSON | timed | 3.095 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.460 | +76.4% | 15.22 (darwin-wait4) | +466.3% | 30 (+1 warmup) |
| tq | TOON | timed | 2.723 | -12.0% | 4.61 (darwin-wait4) | +71.5% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.[] | select(.location == "Stockholm") | .name'`

```text
"Donald"
"Walt"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.[] | select(.location == "Stockholm") | .name'`

```text
"Donald"
"Walt"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.[] | select(.location == "Stockholm") | .name'`

```text
Donald
Walt
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 6 | 5 | -1 | -16.67% |
| `o200k_base` | Expanded | 6 | 5 | -1 | -16.67% |
| `cl100k_base` | Compact | 6 | 5 | -1 | -16.67% |
| `cl100k_base` | Expanded | 6 | 5 | -1 | -16.67% |
<!-- STACK_OVERFLOW_RESULTS_END -->
