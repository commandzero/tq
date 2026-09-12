---
type: Report
title: "Iterating through JSON array in Shell script"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 32."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
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
Last updated: 2026-09-11
Status: `passed`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.1; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)  
Memory evidence: authoritative peak RSS from the recorded time implementation; sampled-group RSS is not used as a substitute.  

| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 74.227 | +0.0% | 2.59 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 72.323 | -2.6% | 15.83 (bsd-time-l) | +510.2% | 30 (+1 warmup) |
| tq | TOON | timed | 94.064 | +26.7% | 4.02 (bsd-time-l) | +54.8% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"original_name":"pdf_convert","changed_name":"pdf_convert_1"}
{"original_name":"video_encode","changed_name":"video_encode_1"}
{"original_name":"video_transcode","changed_name":"video_transcode_1"}
```

#### Expanded JSON (`jq`)

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

#### TOON (`tq -o toon`)

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
