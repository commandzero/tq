---
type: Report
title: "Convert string to json in jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 40."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 40: Convert string to json in jq

[Stack Overflow question](https://stackoverflow.com/questions/34340549/convert-string-to-json-in-jq) · [selected answer](https://stackoverflow.com/a/34340787)

Rank: 40 · Question score: 100

Fixture: [source data](../../../tests/stack-overflow/40-convert-string-to-json-in-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.response.text`

```json
{
  "requestType": "POST",
  "response": {
    "size": 78,
    "text": "{\"recordID\":123,\"title\":\"Hello World\",\"content\":\"Lorem ipsum...\"}"
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
| jq | JSON | timed | 128.470 | +0.0% | 2.58 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 141.851 | +10.4% | 15.73 (bsd-time-l) | +510.3% | 30 (+1 warmup) |
| tq | TOON | timed | 132.828 | +3.4% | 3.94 (bsd-time-l) | +52.7% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"{\"recordID\":123,\"title\":\"Hello World\",\"content\":\"Lorem ipsum...\"}"
```

#### Expanded JSON (`jq`)

```text
"{\"recordID\":123,\"title\":\"Hello World\",\"content\":\"Lorem ipsum...\"}"
```

#### TOON (`tq -o toon`)

```text
"{\"recordID\":123,\"title\":\"Hello World\",\"content\":\"Lorem ipsum...\"}"
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 19 | 19 | +0 | +0.00% |
| `o200k_base` | Expanded | 19 | 19 | +0 | +0.00% |
| `cl100k_base` | Compact | 19 | 19 | +0 | +0.00% |
| `cl100k_base` | Expanded | 19 | 19 | +0 | +0.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
