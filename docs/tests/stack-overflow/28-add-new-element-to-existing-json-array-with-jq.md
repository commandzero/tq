---
type: Report
title: "Add new element to existing JSON array with jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 28."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 28: Add new element to existing JSON array with jq

[Stack Overflow question](https://stackoverflow.com/questions/42245288/add-new-element-to-existing-json-array-with-jq) · [selected answer](https://stackoverflow.com/a/42248841)

Rank: 28 · Question score: 133

Fixture: [source data](../../../tests/stack-overflow/28-add-new-element-to-existing-json-array-with-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.data.messages += [{"date":"2010-01-07T19:55:99.999Z","xml":"new.xml","status":"OKKK","message":"added"}]`

```json
{
  "report": "1.0",
  "data": {
    "date": "2010-01-07",
    "messages": [
      {
        "date": "2010-01-07T19:58:42.949Z",
        "xml": "old.xml",
        "status": "OK",
        "message": "loaded"
      },
      {
        "date": "2010-01-07T20:22:46.949Z",
        "xml": "old2.xml",
        "status": "NOK",
        "message": "duplicated"
      }
    ]
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

Output profile: `structured`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 3.240 | +0.0% | 2.70 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.268 | +62.6% | 15.91 (darwin-wait4) | +488.4% | 30 (+1 warmup) |
| tq | TOON | timed | 2.590 | -20.1% | 4.42 (darwin-wait4) | +63.6% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.data.messages += [{"date":"2010-01-07T19:55:99.999Z","xml":"new.xml","status":"OKKK","message":"added"}]'`

```text
{"report":"1.0","data":{"date":"2010-01-07","messages":[{"date":"2010-01-07T19:58:42.949Z","xml":"old.xml","status":"OK","message":"loaded"},{"date":"2010-01-07T20:22:46.949Z","xml":"old2.xml","status":"NOK","message":"duplicated"},{"date":"2010-01-07T19:55:99.999Z","xml":"new.xml","status":"OKKK","message":"added"}]}}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.data.messages += [{"date":"2010-01-07T19:55:99.999Z","xml":"new.xml","status":"OKKK","message":"added"}]'`

```text
{
  "report": "1.0",
  "data": {
    "date": "2010-01-07",
    "messages": [
      {
        "date": "2010-01-07T19:58:42.949Z",
        "xml": "old.xml",
        "status": "OK",
        "message": "loaded"
      },
      {
        "date": "2010-01-07T20:22:46.949Z",
        "xml": "old2.xml",
        "status": "NOK",
        "message": "duplicated"
      },
      {
        "date": "2010-01-07T19:55:99.999Z",
        "xml": "new.xml",
        "status": "OKKK",
        "message": "added"
      }
    ]
  }
}
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.data.messages += [{"date":"2010-01-07T19:55:99.999Z","xml":"new.xml","status":"OKKK","message":"added"}]'`

```text
report: "1.0"
data:
  date: 2010-01-07
  messages[3]{date,xml,status,message}:
    "2010-01-07T19:58:42.949Z",old.xml,OK,loaded
    "2010-01-07T20:22:46.949Z",old2.xml,NOK,duplicated
    "2010-01-07T19:55:99.999Z",new.xml,OKKK,added
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 120 | 110 | -10 | -8.33% |
| `o200k_base` | Expanded | 182 | 110 | -72 | -39.56% |
| `cl100k_base` | Compact | 119 | 109 | -10 | -8.40% |
| `cl100k_base` | Expanded | 182 | 109 | -73 | -40.11% |
<!-- STACK_OVERFLOW_RESULTS_END -->
