---
type: Report
title: "jq not working on tag name with dashes and numbers"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 39."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 39: jq not working on tag name with dashes and numbers

[Stack Overflow question](https://stackoverflow.com/questions/37344329/jq-not-working-on-tag-name-with-dashes-and-numbers) · [selected answer](https://stackoverflow.com/a/37344498)

Rank: 39 · Question score: 102

Fixture: [source data](../../../tests/stack-overflow/39-jq-not-working-on-tag-name-with-dashes-and-numbers.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark selects the field whose name contains a dash by using bracket notation.

## Benchmark input

Query: `.["component-status"]`

Output mode: structured, using `tq`'s default TOON output.

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
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `structured`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 3.135 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.317 | +69.6% | 15.48 (darwin-wait4) | +479.5% | 30 (+1 warmup) |
| tq | TOON | timed | 2.583 | -17.6% | 4.31 (darwin-wait4) | +61.4% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.["component-status"]'`

```text
[{"status-code":200,"component":"Service1","status":"OK"},{"status-code":200,"component":"Service2","status":"OK"}]
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.["component-status"]'`

```text
[
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
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.["component-status"]'`

```text
[2]{"status-code",component,status}:
  200,Service1,OK
  200,Service2,OK
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 31 | 29 | -2 | -6.45% |
| `o200k_base` | Expanded | 56 | 29 | -27 | -48.21% |
| `cl100k_base` | Compact | 31 | 29 | -2 | -6.45% |
| `cl100k_base` | Expanded | 56 | 29 | -27 | -48.21% |
<!-- STACK_OVERFLOW_RESULTS_END -->
