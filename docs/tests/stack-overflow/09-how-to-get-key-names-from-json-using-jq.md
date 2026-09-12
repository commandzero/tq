---
type: Report
title: "How to get key names from JSON using jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 09."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 09: How to get key names from JSON using jq

[Stack Overflow question](https://stackoverflow.com/questions/23118341/how-to-get-key-names-from-json-using-jq) · [selected answer](https://stackoverflow.com/a/23118607)

Rank: 9 · Question score: 314

Fixture: [source data](../../../tests/stack-overflow/09-how-to-get-key-names-from-json-using-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `keys | sort | .[]`

```json
{
  "email": "madireddy@test.com",
  "id": "2323",
  "name": "test",
  "date": "02-03-2014-13:41",
  "type": "application"
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
| jq | JSON | timed | 108.962 | +0.0% | 2.62 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 114.282 | +4.9% | 15.66 (bsd-time-l) | +496.4% | 30 (+1 warmup) |
| tq | TOON | timed | 105.275 | -3.4% | 4.11 (bsd-time-l) | +56.5% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"date"
"email"
"id"
"name"
"type"
```

#### Expanded JSON (`jq`)

```text
"date"
"email"
"id"
"name"
"type"
```

#### TOON (`tq -o toon`)

```text
date
email
id
name
type
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 12 | 10 | -2 | -16.67% |
| `o200k_base` | Expanded | 12 | 10 | -2 | -16.67% |
| `cl100k_base` | Compact | 12 | 10 | -2 | -16.67% |
| `cl100k_base` | Expanded | 12 | 10 | -2 | -16.67% |
<!-- STACK_OVERFLOW_RESULTS_END -->
