---
type: Report
title: "How to convert arbitrary simple JSON to CSV using jq?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 13."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 13: How to convert arbitrary simple JSON to CSV using jq?

[Stack Overflow question](https://stackoverflow.com/questions/32960857/how-to-convert-arbitrary-simple-json-to-csv-using-jq) · [selected answer](https://stackoverflow.com/a/32965227)

Rank: 13 · Question score: 250

Fixture: [source data](../../../tests/stack-overflow/13-how-to-convert-arbitrary-simple-json-to-csv-using-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `map({"code": .code, "name": .name, "level": .level, "country": .country})`

```json
[
  {
    "code": "NSW",
    "name": "New South Wales",
    "level": "state",
    "country": "AU"
  },
  {
    "code": "AB",
    "name": "Alberta",
    "level": "province",
    "country": "CA"
  },
  {
    "code": "ABD",
    "name": "Aberdeenshire",
    "level": "council area",
    "country": "GB"
  },
  {
    "code": "AK",
    "name": "Alaska",
    "level": "state",
    "country": "US"
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
| jq | JSON | timed | 3.457 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.221 | +51.0% | 16.00 (darwin-wait4) | +498.8% | 30 (+1 warmup) |
| tq | TOON | timed | 2.575 | -25.5% | 4.47 (darwin-wait4) | +67.3% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c 'map({"code": .code, "name": .name, "level": .level, "country": .country})'`

```text
[{"code":"NSW","name":"New South Wales","level":"state","country":"AU"},{"code":"AB","name":"Alberta","level":"province","country":"CA"},{"code":"ABD","name":"Aberdeenshire","level":"council area","country":"GB"},{"code":"AK","name":"Alaska","level":"state","country":"US"}]
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq 'map({"code": .code, "name": .name, "level": .level, "country": .country})'`

```text
[
  {
    "code": "NSW",
    "name": "New South Wales",
    "level": "state",
    "country": "AU"
  },
  {
    "code": "AB",
    "name": "Alberta",
    "level": "province",
    "country": "CA"
  },
  {
    "code": "ABD",
    "name": "Aberdeenshire",
    "level": "council area",
    "country": "GB"
  },
  {
    "code": "AK",
    "name": "Alaska",
    "level": "state",
    "country": "US"
  }
]
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json 'map({"code": .code, "name": .name, "level": .level, "country": .country})'`

```text
[4]{code,name,level,country}:
  NSW,New South Wales,state,AU
  AB,Alberta,province,CA
  ABD,Aberdeenshire,council area,GB
  AK,Alaska,state,US
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 78 | 54 | -24 | -30.77% |
| `o200k_base` | Expanded | 141 | 54 | -87 | -61.70% |
| `cl100k_base` | Compact | 79 | 54 | -25 | -31.65% |
| `cl100k_base` | Expanded | 142 | 54 | -88 | -61.97% |
<!-- STACK_OVERFLOW_RESULTS_END -->
