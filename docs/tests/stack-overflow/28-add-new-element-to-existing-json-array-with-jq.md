---
type: Report
title: "Add new element to existing JSON array with jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 28."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
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
Last updated: 2026-09-11
Status: `passed`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.1; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)  
Memory evidence: authoritative peak RSS from the recorded time implementation; sampled-group RSS is not used as a substitute.  

| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 124.769 | +0.0% | 2.64 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 190.307 | +52.5% | 16.34 (bsd-time-l) | +518.9% | 30 (+1 warmup) |
| tq | TOON | timed | 103.594 | -17.0% | 4.25 (bsd-time-l) | +60.9% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"report":"1.0","data":{"date":"2010-01-07","messages":[{"date":"2010-01-07T19:58:42.949Z","xml":"old.xml","status":"OK","message":"loaded"},{"date":"2010-01-07T20:22:46.949Z","xml":"old2.xml","status":"NOK","message":"duplicated"},{"date":"2010-01-07T19:55:99.999Z","xml":"new.xml","status":"OKKK","message":"added"}]}}
```

#### Expanded JSON (`jq`)

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

#### TOON (`tq -o toon`)

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
