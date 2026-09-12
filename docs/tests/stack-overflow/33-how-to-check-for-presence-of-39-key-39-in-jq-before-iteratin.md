---
type: Report
title: "How to check for presence of 'key' in jq before iterating over the values"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 33."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 33: How to check for presence of 'key' in jq before iterating over the values

[Stack Overflow question](https://stackoverflow.com/questions/42097410/how-to-check-for-presence-of-key-in-jq-before-iterating-over-the-values) · [selected answer](https://stackoverflow.com/a/42099242)

Rank: 33 · Question score: 124

Fixture: [source data](../../../tests/stack-overflow/33-how-to-check-for-presence-of-39-key-39-in-jq-before-iteratin.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.result | select(.property_history != null) | .property_history | map(select(.event_name == "Sold"))[0].date`

```json
{
  "result": {
    "property_history": [
      {
        "event_name": "Listed",
        "date": "2020-01-01"
      },
      {
        "event_name": "Sold",
        "date": "2020-02-02"
      }
    ]
  }
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
| jq | JSON | timed | 93.794 | +0.0% | 2.64 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 94.624 | +0.9% | 15.97 (bsd-time-l) | +504.7% | 30 (+1 warmup) |
| tq | TOON | timed | 108.974 | +16.2% | 4.28 (bsd-time-l) | +62.1% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"2020-02-02"
```

#### Expanded JSON (`jq`)

```text
"2020-02-02"
```

#### TOON (`tq -o toon`)

```text
2020-02-02
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 8 | 7 | -1 | -12.50% |
| `o200k_base` | Expanded | 8 | 7 | -1 | -12.50% |
| `cl100k_base` | Compact | 8 | 7 | -1 | -12.50% |
| `cl100k_base` | Expanded | 8 | 7 | -1 | -12.50% |
<!-- STACK_OVERFLOW_RESULTS_END -->
