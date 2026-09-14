---
type: Report
title: "How to format a JSON string as a table using jq?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 23."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 23: How to format a JSON string as a table using jq?

[Stack Overflow question](https://stackoverflow.com/questions/39139107/how-to-format-a-json-string-as-a-table-using-jq) · [selected answer](https://stackoverflow.com/a/39139478)

Rank: 23 · Question score: 166

Fixture: [source data](../../../tests/stack-overflow/23-how-to-format-a-json-string-as-a-table-using-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark uses the first tab-separated solution from the selected answer.

## Benchmark input

Query: `.[] | "\(.id)\t\(.name)"`

Output mode: `raw`, so each tab-separated result is written as one line.

```json
[
  {
    "name": "George",
    "id": 12,
    "email": "george@domain.example"
  },
  {
    "name": "Jack",
    "id": 18,
    "email": "jack@domain.example"
  },
  {
    "name": "Joe",
    "id": 19,
    "email": "joe@domain.example"
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

Output profile: `raw`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | raw | timed | 3.281 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | raw | timed | 5.085 | +55.0% | 14.73 (darwin-wait4) | +448.3% | 30 (+1 warmup) |
| tq | raw | timed | 2.724 | -17.0% | 4.45 (darwin-wait4) | +65.7% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `raw`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### jq

Status: `exited-0`.
Command: `jq -r '.[] | "\(.id)\t\(.name)"'`

```text
12	George
18	Jack
19	Joe
```

#### yq

Status: `exited-0`.
Command: `yq -p=json -o=json -I=0 -r '.[] | "\(.id)\t\(.name)"'`

```text
12	George
18	Jack
19	Joe
```

#### tq

Status: `exited-0`.
Command: `tq --input-format json -r '.[] | "\(.id)\t\(.name)"'`

```text
12	George
18	Jack
19	Joe
```

Token comparison is not applicable to raw or color output profiles.
<!-- STACK_OVERFLOW_RESULTS_END -->
