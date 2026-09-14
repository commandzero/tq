---
type: Report
title: "How to check for presence of 'key' in jq before iterating over the values"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 33."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
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
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `structured`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 3.118 | +0.0% | 2.70 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.208 | +67.1% | 15.38 (darwin-wait4) | +468.8% | 30 (+1 warmup) |
| tq | TOON | timed | 2.570 | -17.6% | 4.38 (darwin-wait4) | +61.8% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.result | select(.property_history != null) | .property_history | map(select(.event_name == "Sold"))[0].date'`

```text
"2020-02-02"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.result | select(.property_history != null) | .property_history | map(select(.event_name == "Sold"))[0].date'`

```text
"2020-02-02"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.result | select(.property_history != null) | .property_history | map(select(.event_name == "Sold"))[0].date'`

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
