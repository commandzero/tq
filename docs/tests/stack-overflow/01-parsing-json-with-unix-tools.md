---
type: Report
title: "Parsing JSON with Unix tools"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 01."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 01: Parsing JSON with Unix tools

[Stack Overflow question](https://stackoverflow.com/questions/1955505/parsing-json-with-unix-tools) · [selected answer](https://stackoverflow.com/a/1955555)

Rank: 1 · Question score: 1321

Fixture: [source data](../../../tests/stack-overflow/01-parsing-json-with-unix-tools.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.name`

```json
{
  "name": "lambda",
  "id": 1,
  "nested": {
    "text": "My status"
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
| jq | JSON | timed | 240.053 | +0.0% | 2.62 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 183.630 | -23.5% | 15.62 (bsd-time-l) | +495.2% | 30 (+1 warmup) |
| tq | TOON | timed | 182.676 | -23.9% | 3.95 (bsd-time-l) | +50.6% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"lambda"
```

#### Expanded JSON (`jq`)

```text
"lambda"
```

#### TOON (`tq -o toon`)

```text
lambda
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 3 | 2 | -1 | -33.33% |
| `o200k_base` | Expanded | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | Compact | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | Expanded | 3 | 2 | -1 | -33.33% |
<!-- STACK_OVERFLOW_RESULTS_END -->
