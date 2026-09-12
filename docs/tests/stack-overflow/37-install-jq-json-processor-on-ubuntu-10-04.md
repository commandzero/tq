---
type: Report
title: "Install jq JSON processor on Ubuntu 10.04"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 37."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 37: Install jq JSON processor on Ubuntu 10.04

[Stack Overflow question](https://stackoverflow.com/questions/33184780/install-jq-json-processor-on-ubuntu-10-04) · [selected answer](https://stackoverflow.com/a/44546590)

Rank: 37 · Question score: 112

Fixture: [source data](../../../tests/stack-overflow/37-install-jq-json-processor-on-ubuntu-10-04.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.`

```json
{
  "name": "John",
  "age": 31,
  "city": "New York"
}
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
| jq | JSON | timed | 214.139 | +0.0% | 2.59 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 139.869 | -34.7% | 15.84 (bsd-time-l) | +510.8% | 30 (+1 warmup) |
| tq | TOON | timed | 125.695 | -41.3% | 3.69 (bsd-time-l) | +42.2% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"name":"John","age":31,"city":"New York"}
```

#### Expanded JSON (`jq`)

```text
{
  "name": "John",
  "age": 31,
  "city": "New York"
}
```

#### TOON (`tq -o toon`)

```text
name: John
age: 31
city: New York
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 14 | 14 | +0 | +0.00% |
| `o200k_base` | Expanded | 24 | 14 | -10 | -41.67% |
| `cl100k_base` | Compact | 14 | 14 | +0 | +0.00% |
| `cl100k_base` | Expanded | 24 | 14 | -10 | -41.67% |
<!-- STACK_OVERFLOW_RESULTS_END -->
