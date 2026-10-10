---
type: Report
title: "I/O coverage audit"
description: "Recorded review of I/O coverage audit."
generated: { by: codex/gpt-6, at: 2026-09-10T02:12:04Z }
benchmark_runs: [{"platform":"macos","target":"aarch64-macos","campaign_id":null,"captured_at":null,"binaries":{"jq":{"version":"jq-1.8.2","sha256":"2d75340ba57a4b4b4c8708a21c2dc8e958a48aaa8bba13b27f77f6e4c0eca07e","identity_status":"measured"},"tq":{"version":"tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)","sha256":"8a0ba42cf81881829b93ee950db0d4d3f808a22af574b210484be7833b238d00","identity_status":"measured"}}},{"platform":"linux","target":"x86_64-linux","campaign_id":null,"captured_at":null,"binaries":{"jq":{"version":"jq-1.8.2","sha256":"b1c22172dd303f3be49e935aa56aa48a8b7a46e0bc838b4997d3bb451495870f","identity_status":"measured"},"tq":{"version":"tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)","sha256":"474625b11f0ec3a1fa125bbeb4e55008efa50ec713b0c398822ada48f0289b91","identity_status":"measured"}}},{"platform":"windows","target":"x86_64-windows","campaign_id":null,"captured_at":null,"binaries":{"jq":{"version":"jq-1.8.2","sha256":"a6fc67fedaf9128a3309a1e2ebb8b986aeccf70122ee46d2cb4849e423f0c627","identity_status":"measured"},"tq":{"version":"tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)","sha256":"a1cec37e4c6e70bcd06647a5b5b5617ece0b88e41fe8a1ad9590bafc00992428","identity_status":"measured"}}}]
---

# I/O coverage audit

Source: [jq manual, I/O](https://jqlang.org/manual/#io). The audit accounts for all five fenced blocks and the runnable prose examples in the section. jq and tq adapters remain enabled for every MVP case so missing tq `input`, `debug`, and `stderr` support, plus line-number differences, are visible in compatibility output. Debug/stderr cases compare raw stderr bytes.

| Source | Example | Cases | Disposition |
| ---: | --- | --- | --- |
| 16 | input prose | `manual.io.input` | covered |
| 16 | inputs prose | `manual.io.inputs` | covered |
| 18 | stdout/stderr prose | `manual.io.debug`, `manual.io.stderr` | covered |
| 26 | `-n` input note | `manual.io.input-null-input` | new |
| 28 | fenced `[., input]` shell example | `manual.io.input` | new |
| 38 | fenced `reduce inputs` shell example | `manual.io.inputs` | new |
| 44 | `debug` prose | `manual.io.debug` | new |
| 48 | debug output placeholder | `manual.io.debug` | covered |
| 54 | `debug(msgs)` definition | `manual.io.debug-msgs` | new |
| 58 | fenced debug expression | `manual.io.debug-example` | new |
| 64 | fenced debug stderr output | `manual.io.debug-example` | covered |
| 71 | `stderr` prose | `manual.io.stderr` | new |
| 75 | `input_filename` prose | `manual.io.input-filename` | new |
| 77 | `input_line_number` prose | `manual.io.input-line-number` | new |

The output placeholder at line 48 is intentionally linked to its exercising case; it is not executable on its own. The direct-process harness adapts shell pipelines to stdin bytes; the process CLI admits platform metadata by default.

<!-- tq-manual-compare:begin section=io -->
## Results

[Case collection](../../../tests/compatibility/reviews/jq-manual/io.toon)


| Verdict | Cases |
| --- | ---: |
| Exact match | 9 |

Independent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.

| Output campaign | Exact matches | Cases |
| --- | ---: | ---: |
| compact_json | 9 | 9 |
| toon | 9 | 9 |

An exact match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from exact matches. Historical expected-difference labels do not pass either gate.

Differences include missing features and unaccepted mismatches; these still fail the strict and completion gates. Reference discrepancies describe errors in the imported manual, not successful compatibility.

JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.

### Output size

9 eligible examples. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with default LF-terminated `-o toon` results; TOON sequence captures, when available, are shown in the cases but excluded from size totals. Diff is TOON tokens minus JSON tokens. % is the signed percent difference `(TOON - JSON) / JSON`, so savings are negative and growth is positive.

| Tokenizer | JSON tokens | TOON tokens | Diff | % |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 47 | 40 | -7 | -14.89% |
| `cl100k_base` | 47 | 40 | -7 | -14.89% |

Only successful jq/JSON/TOON-equivalent results enter the totals. A negative `Diff` means TOON uses fewer tokens; `%` is negative for savings and positive for growth. The manual is a correctness corpus, not a representative workload benchmark.

### Differences

No differences.

### Cases

Each case shows the original jq invocation and the available jq, tq JSON, and tq TOON output captures. Missing captures are marked `<not run>`; these placeholders are not execution evidence. Control bytes use `\xNN` escapes so record separators remain visible. This section is generated by the separate `tq-manual-compare` command.

#### manual.io.debug

```
# input
jq debug

# jq
42


[stderr]
["DEBUG:",42]

# tq -o json
42


[stderr]
["DEBUG:",42]

# tq
42


[stderr]
["DEBUG:",42]
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.io.debug-example

```
# input
jq '1 as $x | 2 | debug("Entering function foo with $x == \($x)", .) | (.+1)'

# jq
3


[stderr]
["DEBUG:","Entering function foo with $x == 1"]
["DEBUG:",2]

# tq -o json
3


[stderr]
["DEBUG:","Entering function foo with $x == 1"]
["DEBUG:",2]

# tq
3


[stderr]
["DEBUG:","Entering function foo with $x == 1"]
["DEBUG:",2]
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.io.debug-msgs

```
# input
jq 'debug("message")'

# jq
42


[stderr]
["DEBUG:","message"]

# tq -o json
42


[stderr]
["DEBUG:","message"]

# tq
42


[stderr]
["DEBUG:","message"]
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.io.input

```
# input
jq '[., input]'

# jq
[
  1,
  2
]
[
  3,
  4
]

# tq -o json
[
  1,
  2
]
[
  3,
  4
]

# tq
[2]: 1,2
[2]: 3,4
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 16 | -4 | -20% |
| `cl100k_base` | 20 | 20 | 16 | -4 | -20% |

#### manual.io.input-filename

```
# input
jq input_filename

# jq
"<stdin>"

# tq -o json
"<stdin>"

# tq
<stdin>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 3 | 0 | +0% |
| `cl100k_base` | 3 | 3 | 3 | 0 | +0% |

#### manual.io.input-line-number

```
# input
jq input_line_number

# jq
1
2

# tq -o json
1
2

# tq
1
2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### manual.io.input-null-input

```
# input
jq -n '[., input]'

# jq
[
  null,
  1
]

# tq -o json
[
  null,
  1
]

# tq
[2]: null,1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 7 | -2 | -22.22% |
| `cl100k_base` | 9 | 9 | 7 | -2 | -22.22% |

#### manual.io.inputs

```
# input
jq -n 'reduce inputs as $i (0; . + $i)'

# jq
6

# tq -o json
6

# tq
6
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.io.stderr

```
# input
jq stderr

# jq
"hello"


[stderr]
hello

# tq -o json
"hello"


[stderr]
hello

# tq
hello


[stderr]
hello
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | 3 | 3 | 2 | -1 | -33.33% |

<!-- tq-manual-compare:end -->
