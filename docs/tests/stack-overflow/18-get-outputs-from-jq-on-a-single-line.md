---
type: Report
title: "Get outputs from jq on a single line"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 18."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 18: Get outputs from jq on a single line

[Stack Overflow question](https://stackoverflow.com/questions/40396445/get-outputs-from-jq-on-a-single-line) · [selected answer](https://stackoverflow.com/a/40396556)

Rank: 18 · Question score: 187

Fixture: [source data](../../../tests/stack-overflow/18-get-outputs-from-jq-on-a-single-line.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.issues[] | {"key": .key, "status": .fields.status.name, "assignee": .fields.assignee.emailAddress}`

```json
{
  "issues": [
    {
      "key": "SEA-739",
      "fields": {
        "status": {
          "name": "Open"
        },
        "assignee": {
          "emailAddress": null
        }
      }
    },
    {
      "key": "SEA-738",
      "fields": {
        "status": {
          "name": "Resolved"
        },
        "assignee": {
          "emailAddress": "user2@mycompany.com"
        }
      }
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
| jq | JSON | timed | 129.857 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 119.055 | -8.3% | 16.19 (bsd-time-l) | +520.4% | 30 (+1 warmup) |
| tq | TOON | timed | 104.525 | -19.5% | 4.44 (bsd-time-l) | +70.1% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"key":"SEA-739","status":"Open","assignee":null}
{"key":"SEA-738","status":"Resolved","assignee":"user2@mycompany.com"}
```

#### Expanded JSON (`jq`)

```text
{
  "key": "SEA-739",
  "status": "Open",
  "assignee": null
}
{
  "key": "SEA-738",
  "status": "Resolved",
  "assignee": "user2@mycompany.com"
}
```

#### TOON (`tq -o toon`)

```text
key: SEA-739
status: Open
assignee: null
key: SEA-738
status: Resolved
assignee: user2@mycompany.com
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 37 | 36 | -1 | -2.70% |
| `o200k_base` | Expanded | 56 | 36 | -20 | -35.71% |
| `cl100k_base` | Compact | 37 | 36 | -1 | -2.70% |
| `cl100k_base` | Expanded | 56 | 36 | -20 | -35.71% |
<!-- STACK_OVERFLOW_RESULTS_END -->
