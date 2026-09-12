---
type: Report
title: "Exclude column from jq json output"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 44."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 44: Exclude column from jq json output

[Stack Overflow question](https://stackoverflow.com/questions/33895076/exclude-column-from-jq-json-output) · [selected answer](https://stackoverflow.com/a/33895231)

Rank: 44 · Question score: 85

Fixture: [source data](../../../tests/stack-overflow/44-exclude-column-from-jq-json-output.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[].group`

```json
[
  {
    "timestamp": 1448369447295,
    "group": "employees",
    "uid": "elgalu"
  },
  {
    "timestamp": 1448369447296,
    "group": "services",
    "uid": "pacts"
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
| jq | JSON | timed | 117.472 | +0.0% | 2.59 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 113.910 | -3.0% | 15.78 (bsd-time-l) | +508.4% | 30 (+1 warmup) |
| tq | TOON | timed | 112.932 | -3.9% | 3.91 (bsd-time-l) | +50.6% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"employees"
"services"
```

#### Expanded JSON (`jq`)

```text
"employees"
"services"
```

#### TOON (`tq -o toon`)

```text
employees
services
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 6 | 4 | -2 | -33.33% |
| `o200k_base` | Expanded | 6 | 4 | -2 | -33.33% |
| `cl100k_base` | Compact | 6 | 4 | -2 | -33.33% |
| `cl100k_base` | Expanded | 6 | 4 | -2 | -33.33% |
<!-- STACK_OVERFLOW_RESULTS_END -->
