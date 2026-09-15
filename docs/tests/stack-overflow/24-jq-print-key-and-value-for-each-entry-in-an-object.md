---
type: Report
title: "jq: print key and value for each entry in an object"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 24."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 24: jq: print key and value for each entry in an object

[Stack Overflow question](https://stackoverflow.com/questions/34226370/jq-print-key-and-value-for-each-entry-in-an-object) · [selected answer](https://stackoverflow.com/a/34227629)

Rank: 24 · Question score: 164

Fixture: [source data](../../../tests/stack-overflow/24-jq-print-key-and-value-for-each-entry-in-an-object.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark uses the selected answer's `to_entries[]` alternative, preserving each object entry's key and value.

## Benchmark input

Query: `to_entries[] | "\(.key), \(.value | .ip)"`

Output mode: `raw`, so each key and IP pair is written as one line.

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
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `raw`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | raw | timed | 3.118 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | raw | timed | 5.114 | +64.0% | 14.70 (darwin-wait4) | +447.1% | 30 (+1 warmup) |
| tq | raw | timed | 2.607 | -16.4% | 4.28 (darwin-wait4) | +59.3% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `raw`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### jq

Status: `exited-0`.
Command: `jq -r 'to_entries[] | "\(.key), \(.value | .ip)"'`

```text
host1, 10.1.2.3
host2, 10.1.2.2
host3, 10.1.18.1
```

#### yq

Status: `exited-0`.
Command: `yq -p=json -o=json -I=0 -r 'to_entries[] | "\(.key), \(.value | .ip)"'`

```text
host1, 10.1.2.3
host2, 10.1.2.2
host3, 10.1.18.1
```

#### tq

Status: `exited-0`.
Command: `tq --input-format json -r 'to_entries[] | "\(.key), \(.value | .ip)"'`

```text
host1, 10.1.2.3
host2, 10.1.2.2
host3, 10.1.18.1
```

Token comparison is not applicable to raw or color output profiles.
<!-- STACK_OVERFLOW_RESULTS_END -->
