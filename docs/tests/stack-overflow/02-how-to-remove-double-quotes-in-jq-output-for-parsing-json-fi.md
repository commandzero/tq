---
type: Report
title: "How to remove double-quotes in jq output for parsing json files in bash?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 02."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 02: How to remove double-quotes in jq output for parsing json files in bash?

[Stack Overflow question](https://stackoverflow.com/questions/44656515/how-to-remove-double-quotes-in-jq-output-for-parsing-json-files-in-bash) · [selected answer](https://stackoverflow.com/a/44656583)

Rank: 2 · Question score: 797

Fixture: [source data](../../../tests/stack-overflow/02-how-to-remove-double-quotes-in-jq-output-for-parsing-json-fi.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.name`

```json
{
  "name": "Google",
  "id": 1
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
| jq | JSON | timed | 184.611 | +0.0% | 2.58 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 172.964 | -6.3% | 15.42 (bsd-time-l) | +498.2% | 30 (+1 warmup) |
| tq | TOON | timed | 178.718 | -3.2% | 3.95 (bsd-time-l) | +53.3% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"Google"
```

#### Expanded JSON (`jq`)

```text
"Google"
```

#### TOON (`tq -o toon`)

```text
Google
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 3 | 2 | -1 | -33.33% |
| `o200k_base` | Expanded | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | Compact | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | Expanded | 3 | 2 | -1 | -33.33% |
<!-- STACK_OVERFLOW_RESULTS_END -->
