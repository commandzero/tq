---
type: Report
title: "Exclude column from jq json output"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 44."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 44: Exclude column from jq json output

[Stack Overflow question](https://stackoverflow.com/questions/33895076/exclude-column-from-jq-json-output) · [selected answer](https://stackoverflow.com/a/33895231)

Rank: 44 · Question score: 85

Fixture: [source data](../../../tests/stack-overflow/44-exclude-column-from-jq-json-output.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark removes only `timestamp`, preserving the remaining fields and object structure. It does not measure the selected answer's separate CSV projection.

## Benchmark input

Query: `del(.[].timestamp)`

Output mode: structured, using `tq`'s default TOON output.

```json
[
  {
    "timestamp": 1448369447295,
    "group": "employees",
    "uid": "elgalu"
  },
  {
    "timestamp": 1448369447296,
    "group": "employees",
    "uid": "mike"
  },
  {
    "timestamp": 1448369786667,
    "group": "services",
    "uid": "pacts"
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
| jq | JSON | timed | 3.075 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.303 | +72.5% | 15.55 (darwin-wait4) | +478.5% | 30 (+1 warmup) |
| tq | TOON | timed | 2.932 | -4.7% | 4.39 (darwin-wait4) | +63.4% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c 'del(.[].timestamp)'`

```text
[{"group":"employees","uid":"elgalu"},{"group":"employees","uid":"mike"},{"group":"services","uid":"pacts"}]
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq 'del(.[].timestamp)'`

```text
[
  {
    "group": "employees",
    "uid": "elgalu"
  },
  {
    "group": "employees",
    "uid": "mike"
  },
  {
    "group": "services",
    "uid": "pacts"
  }
]
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json 'del(.[].timestamp)'`

```text
[3]{group,uid}:
  employees,elgalu
  employees,mike
  services,pacts
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 31 | 25 | -6 | -19.35% |
| `o200k_base` | Expanded | 60 | 25 | -35 | -58.33% |
| `cl100k_base` | Compact | 31 | 25 | -6 | -19.35% |
| `cl100k_base` | Expanded | 60 | 25 | -35 | -58.33% |
<!-- STACK_OVERFLOW_RESULTS_END -->
