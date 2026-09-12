---
type: Report
title: "jq not working on tag name with dashes and numbers"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 39."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 39: jq not working on tag name with dashes and numbers

[Stack Overflow question](https://stackoverflow.com/questions/37344329/jq-not-working-on-tag-name-with-dashes-and-numbers) · [selected answer](https://stackoverflow.com/a/37344498)

Rank: 39 · Question score: 102

Fixture: [source data](../../../tests/stack-overflow/39-jq-not-working-on-tag-name-with-dashes-and-numbers.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.status`

```json
{
  "status": "ok",
  "hostname": "0b0b495a46db",
  "component-status": [
    {
      "status-code": 200,
      "component": "Service1",
      "status": "OK"
    },
    {
      "status-code": 200,
      "component": "Service2",
      "status": "OK"
    }
  ]
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
| jq | JSON | timed | 189.757 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 167.353 | -11.8% | 15.52 (bsd-time-l) | +494.6% | 30 (+1 warmup) |
| tq | TOON | timed | 139.279 | -26.6% | 3.95 (bsd-time-l) | +51.5% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"ok"
```

#### Expanded JSON (`jq`)

```text
"ok"
```

#### TOON (`tq -o toon`)

```text
ok
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 3 | 2 | -1 | -33.33% |
| `o200k_base` | Expanded | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | Compact | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | Expanded | 3 | 2 | -1 | -33.33% |
<!-- STACK_OVERFLOW_RESULTS_END -->
