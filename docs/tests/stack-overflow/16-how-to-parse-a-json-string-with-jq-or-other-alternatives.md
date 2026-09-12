---
type: Report
title: "how to parse a JSON String with jq (or other alternatives)?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 16."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 16: how to parse a JSON String with jq (or other alternatives)?

[Stack Overflow question](https://stackoverflow.com/questions/35154684/how-to-parse-a-json-string-with-jq-or-other-alternatives) · [selected answer](https://stackoverflow.com/a/35155249)

Rank: 16 · Question score: 217

Fixture: [source data](../../../tests/stack-overflow/16-how-to-parse-a-json-string-with-jq-or-other-alternatives.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.c`

```json
{
  "a": 1,
  "b": 2,
  "c": "{\"id\":\"9ee\",\"parent\":\"abc\"}"
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
| jq | JSON | timed | 98.425 | +0.0% | 2.62 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 108.025 | +9.8% | 15.48 (bsd-time-l) | +489.9% | 30 (+1 warmup) |
| tq | TOON | timed | 83.087 | -15.6% | 3.95 (bsd-time-l) | +50.6% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"{\"id\":\"9ee\",\"parent\":\"abc\"}"
```

#### Expanded JSON (`jq`)

```text
"{\"id\":\"9ee\",\"parent\":\"abc\"}"
```

#### TOON (`tq -o toon`)

```text
"{\"id\":\"9ee\",\"parent\":\"abc\"}"
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 12 | 12 | +0 | +0.00% |
| `o200k_base` | Expanded | 12 | 12 | +0 | +0.00% |
| `cl100k_base` | Compact | 12 | 12 | +0 | +0.00% |
| `cl100k_base` | Expanded | 12 | 12 | +0 | +0.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
