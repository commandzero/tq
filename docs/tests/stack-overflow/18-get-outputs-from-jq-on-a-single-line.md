---
type: Report
title: "Get outputs from jq on a single line"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 18."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
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
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `structured`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 3.097 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.576 | +80.0% | 15.83 (darwin-wait4) | +492.4% | 30 (+1 warmup) |
| tq | TOON | timed | 2.782 | -10.2% | 4.58 (darwin-wait4) | +71.3% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.issues[] | {"key": .key, "status": .fields.status.name, "assignee": .fields.assignee.emailAddress}'`

```text
{"key":"SEA-739","status":"Open","assignee":null}
{"key":"SEA-738","status":"Resolved","assignee":"user2@mycompany.com"}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.issues[] | {"key": .key, "status": .fields.status.name, "assignee": .fields.assignee.emailAddress}'`

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

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.issues[] | {"key": .key, "status": .fields.status.name, "assignee": .fields.assignee.emailAddress}'`

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
