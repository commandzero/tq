---
type: Report
title: "Using jq or alternative command line tools to compare JSON files"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 17."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 17: Using jq or alternative command line tools to compare JSON files

[Stack Overflow question](https://stackoverflow.com/questions/31930041/using-jq-or-alternative-command-line-tools-to-compare-json-files) · [selected answer](https://stackoverflow.com/a/31933234)

Rank: 17 · Question score: 214

Fixture: [source data](../../../tests/stack-overflow/17-using-jq-or-alternative-command-line-tools-to-compare-json-f.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[0].City == .[1].City`

```json
[
  {
    "People": [
      "John",
      "Bryan"
    ],
    "City": "Boston",
    "State": "MA"
  },
  {
    "People": [
      "Bryan",
      "John"
    ],
    "State": "MA",
    "City": "Boston"
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
| jq | JSON | timed | 3.414 | +0.0% | 2.70 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.000 | +46.5% | 15.23 (darwin-wait4) | +463.6% | 30 (+1 warmup) |
| tq | TOON | timed | 2.566 | -24.8% | 4.17 (darwin-wait4) | +54.3% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.[0].City == .[1].City'`

```text
true
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.[0].City == .[1].City'`

```text
true
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.[0].City == .[1].City'`

```text
true
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 2 | 2 | +0 | +0.00% |
| `o200k_base` | Expanded | 2 | 2 | +0 | +0.00% |
| `cl100k_base` | Compact | 2 | 2 | +0 | +0.00% |
| `cl100k_base` | Expanded | 2 | 2 | +0 | +0.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
