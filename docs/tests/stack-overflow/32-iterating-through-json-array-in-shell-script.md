---
type: Report
title: "Iterating through JSON array in Shell script"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 32."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 32: Iterating through JSON array in Shell script

[Stack Overflow question](https://stackoverflow.com/questions/33950596/iterating-through-json-array-in-shell-script) · [selected answer](https://stackoverflow.com/a/33952539)

Rank: 32 · Question score: 125

Fixture: [source data](../../../tests/stack-overflow/32-iterating-through-json-array-in-shell-script.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[]`

```json
[
  {
    "original_name": "pdf_convert",
    "changed_name": "pdf_convert_1"
  },
  {
    "original_name": "video_encode",
    "changed_name": "video_encode_1"
  },
  {
    "original_name": "video_transcode",
    "changed_name": "video_transcode_1"
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
| jq | JSON | timed | 3.120 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.070 | +62.5% | 15.47 (darwin-wait4) | +478.9% | 30 (+1 warmup) |
| tq | TOON | timed | 2.572 | -17.5% | 4.17 (darwin-wait4) | +56.1% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.[]'`

```text
{"original_name":"pdf_convert","changed_name":"pdf_convert_1"}
{"original_name":"video_encode","changed_name":"video_encode_1"}
{"original_name":"video_transcode","changed_name":"video_transcode_1"}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.[]'`

```text
{
  "original_name": "pdf_convert",
  "changed_name": "pdf_convert_1"
}
{
  "original_name": "video_encode",
  "changed_name": "video_encode_1"
}
{
  "original_name": "video_transcode",
  "changed_name": "video_transcode_1"
}
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.[]'`

```text
original_name: pdf_convert
changed_name: pdf_convert_1
original_name: video_encode
changed_name: video_encode_1
original_name: video_transcode
changed_name: video_transcode_1
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 47 | 44 | -3 | -6.38% |
| `o200k_base` | Expanded | 68 | 44 | -24 | -35.29% |
| `cl100k_base` | Compact | 47 | 44 | -3 | -6.38% |
| `cl100k_base` | Expanded | 68 | 44 | -24 | -35.29% |
<!-- STACK_OVERFLOW_RESULTS_END -->
