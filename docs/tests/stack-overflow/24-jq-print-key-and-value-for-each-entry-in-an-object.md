---
type: Report
title: "jq: print key and value for each entry in an object"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 24."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 24: jq: print key and value for each entry in an object

[Stack Overflow question](https://stackoverflow.com/questions/34226370/jq-print-key-and-value-for-each-entry-in-an-object) · [selected answer](https://stackoverflow.com/a/34227629)

Rank: 24 · Question score: 164

Fixture: [source data](../../../tests/stack-overflow/24-jq-print-key-and-value-for-each-entry-in-an-object.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `keys[]`

```json
{
  "host1": {
    "ip": "10.1.2.3"
  },
  "host2": {
    "ip": "10.1.2.2"
  },
  "host3": {
    "ip": "10.1.18.1"
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
| jq | JSON | timed | 146.075 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 95.103 | -34.9% | 15.53 (bsd-time-l) | +495.2% | 30 (+1 warmup) |
| tq | TOON | timed | 102.335 | -29.9% | 4.06 (bsd-time-l) | +55.7% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"host1"
"host2"
"host3"
```

#### Expanded JSON (`jq`)

```text
"host1"
"host2"
"host3"
```

#### TOON (`tq -o toon`)

```text
host1
host2
host3
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 12 | 9 | -3 | -25.00% |
| `o200k_base` | Expanded | 12 | 9 | -3 | -25.00% |
| `cl100k_base` | Compact | 12 | 9 | -3 | -25.00% |
| `cl100k_base` | Expanded | 12 | 9 | -3 | -25.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
