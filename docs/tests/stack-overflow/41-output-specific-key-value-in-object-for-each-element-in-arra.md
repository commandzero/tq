---
type: Report
title: "Output specific key value in object for each element in array with jq for JSON"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 41."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
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
Last updated: 2026-09-11
Status: `passed`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.1; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)  
Memory evidence: authoritative peak RSS from the recorded time implementation; sampled-group RSS is not used as a substitute.  

| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 131.976 | +0.0% | 2.58 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 126.169 | -4.4% | 15.73 (bsd-time-l) | +510.3% | 30 (+1 warmup) |
| tq | TOON | timed | 125.183 | -5.1% | 3.94 (bsd-time-l) | +52.7% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
14462955
114385498
29715011
```

#### Expanded JSON (`jq`)

```text
14462955
114385498
29715011
```

#### TOON (`tq -o toon`)

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
