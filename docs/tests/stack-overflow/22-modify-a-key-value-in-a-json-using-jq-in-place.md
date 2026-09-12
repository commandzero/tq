---
type: Report
title: "Modify a key-value in a json using jq in-place"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 22."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 22: Modify a key-value in a json using jq in-place

[Stack Overflow question](https://stackoverflow.com/questions/42716734/modify-a-key-value-in-a-json-using-jq-in-place) · [selected answer](https://stackoverflow.com/a/42717073)

Rank: 22 · Question score: 172

Fixture: [source data](../../../tests/stack-overflow/22-modify-a-key-value-in-a-json-using-jq-in-place.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.address = "abcde"`

```json
{
  "address": "old",
  "city": "Boston"
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
| jq | JSON | timed | 122.034 | +0.0% | 2.59 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 150.822 | +23.6% | 15.89 (bsd-time-l) | +512.7% | 30 (+1 warmup) |
| tq | TOON | timed | 123.709 | +1.4% | 4.11 (bsd-time-l) | +58.4% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"address":"abcde","city":"Boston"}
```

#### Expanded JSON (`jq`)

```text
{
  "address": "abcde",
  "city": "Boston"
}
```

#### TOON (`tq -o toon`)

```text
address: abcde
city: Boston
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 10 | 9 | -1 | -10.00% |
| `o200k_base` | Expanded | 17 | 9 | -8 | -47.06% |
| `cl100k_base` | Compact | 10 | 9 | -1 | -10.00% |
| `cl100k_base` | Expanded | 17 | 9 | -8 | -47.06% |
<!-- STACK_OVERFLOW_RESULTS_END -->
