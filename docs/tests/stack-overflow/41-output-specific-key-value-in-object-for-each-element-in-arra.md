---
type: Report
title: "Output specific key value in object for each element in array with jq for JSON"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 41."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 41: Output specific key value in object for each element in array with jq for JSON

[Stack Overflow question](https://stackoverflow.com/questions/35677309/output-specific-key-value-in-object-for-each-element-in-array-with-jq-for-json) · [selected answer](https://stackoverflow.com/a/35677443)

Rank: 41 · Question score: 98

Fixture: [source data](../../../tests/stack-overflow/41-output-specific-key-value-in-object-for-each-element-in-arra.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[].AssetId`

```json
[
  {
    "AssetId": 14462955,
    "Name": "Cultural Item"
  },
  {
    "AssetId": 114385498,
    "Name": "Redspybot"
  },
  {
    "AssetId": 29715011,
    "Name": "American Cowboy"
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
| jq | JSON | timed | 3.120 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.018 | +60.8% | 15.20 (darwin-wait4) | +465.7% | 30 (+1 warmup) |
| tq | TOON | timed | 2.732 | -12.4% | 3.98 (darwin-wait4) | +48.3% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.[].AssetId'`

```text
14462955
114385498
29715011
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.[].AssetId'`

```text
14462955
114385498
29715011
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.[].AssetId'`

```text
14462955
114385498
29715011
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 12 | 12 | +0 | +0.00% |
| `o200k_base` | Expanded | 12 | 12 | +0 | +0.00% |
| `cl100k_base` | Compact | 12 | 12 | +0 | +0.00% |
| `cl100k_base` | Expanded | 12 | 12 | +0 | +0.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
