---
type: Report
title: "jq: output array of json objects"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 27."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 27: jq: output array of json objects

[Stack Overflow question](https://stackoverflow.com/questions/38061346/jq-output-array-of-json-objects) · [selected answer](https://stackoverflow.com/a/38061430)

Rank: 27 · Question score: 134

Fixture: [source data](../../../tests/stack-overflow/27-jq-output-array-of-json-objects.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `map({"name": .name, "email": .email})`

```json
[
  {
    "name": "John",
    "email": "john@company.com",
    "extra": 1
  },
  {
    "name": "Brad",
    "email": "brad@company.com",
    "extra": 2
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
| jq | JSON | timed | 128.564 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 124.728 | -3.0% | 16.23 (bsd-time-l) | +522.2% | 30 (+1 warmup) |
| tq | TOON | timed | 115.553 | -10.1% | 4.22 (bsd-time-l) | +61.7% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
[{"name":"John","email":"john@company.com"},{"name":"Brad","email":"brad@company.com"}]
```

#### Expanded JSON (`jq`)

```text
[
  {
    "name": "John",
    "email": "john@company.com"
  },
  {
    "name": "Brad",
    "email": "brad@company.com"
  }
]
```

#### TOON (`tq -o toon`)

```text
[2]{name,email}:
  John,john@company.com
  Brad,brad@company.com
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 26 | 24 | -2 | -7.69% |
| `o200k_base` | Expanded | 45 | 24 | -21 | -46.67% |
| `cl100k_base` | Compact | 26 | 24 | -2 | -7.69% |
| `cl100k_base` | Expanded | 45 | 24 | -21 | -46.67% |
<!-- STACK_OVERFLOW_RESULTS_END -->
