---
type: Report
title: "How do I select multiple fields in jq?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 08."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 08: How do I select multiple fields in jq?

[Stack Overflow question](https://stackoverflow.com/questions/34834519/how-do-i-select-multiple-fields-in-jq) · [selected answer](https://stackoverflow.com/a/34835208)

Rank: 8 · Question score: 314

Fixture: [source data](../../../tests/stack-overflow/08-how-do-i-select-multiple-fields-in-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `{"login": .login, "id": .id}`

```json
{
  "login": "dmaxfield",
  "id": 7449977,
  "email": "d@example.test"
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
| jq | JSON | timed | 68.257 | +0.0% | 2.58 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 96.867 | +41.9% | 16.14 (bsd-time-l) | +526.1% | 30 (+1 warmup) |
| tq | TOON | timed | 111.841 | +63.9% | 4.11 (bsd-time-l) | +59.4% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"login":"dmaxfield","id":7449977}
```

#### Expanded JSON (`jq`)

```text
{
  "login": "dmaxfield",
  "id": 7449977
}
```

#### TOON (`tq -o toon`)

```text
login: dmaxfield
id: 7449977
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 13 | 13 | +0 | +0.00% |
| `o200k_base` | Expanded | 20 | 13 | -7 | -35.00% |
| `cl100k_base` | Compact | 13 | 13 | +0 | +0.00% |
| `cl100k_base` | Expanded | 20 | 13 | -7 | -35.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
