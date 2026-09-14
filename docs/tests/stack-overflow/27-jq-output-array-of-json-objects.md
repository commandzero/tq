---
type: Report
title: "jq: output array of json objects"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 27."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 27: jq: output array of json objects

[Stack Overflow question](https://stackoverflow.com/questions/38061346/jq-output-array-of-json-objects) · [selected answer](https://stackoverflow.com/a/38061430)

Rank: 27 · Question score: 134

Fixture: [source data](../../../tests/stack-overflow/27-jq-output-array-of-json-objects.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `map({"name": .name, "email": .email})`

```json
[
  {
    "name": "John",
    "email": "john@company.com",
    "extra": 1
  },
  {
    "name": "Brad",
    "email": "brad@company.com",
    "extra": 2
  }
]
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
| jq | JSON | timed | 3.132 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.448 | +73.9% | 15.69 (darwin-wait4) | +487.1% | 30 (+1 warmup) |
| tq | TOON | timed | 2.599 | -17.0% | 4.34 (darwin-wait4) | +62.6% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c 'map({"name": .name, "email": .email})'`

```text
[{"name":"John","email":"john@company.com"},{"name":"Brad","email":"brad@company.com"}]
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq 'map({"name": .name, "email": .email})'`

```text
[
  {
    "name": "John",
    "email": "john@company.com"
  },
  {
    "name": "Brad",
    "email": "brad@company.com"
  }
]
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json 'map({"name": .name, "email": .email})'`

```text
[2]{name,email}:
  John,john@company.com
  Brad,brad@company.com
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 26 | 24 | -2 | -7.69% |
| `o200k_base` | Expanded | 45 | 24 | -21 | -46.67% |
| `cl100k_base` | Compact | 26 | 24 | -2 | -7.69% |
| `cl100k_base` | Expanded | 45 | 24 | -21 | -46.67% |
<!-- STACK_OVERFLOW_RESULTS_END -->
