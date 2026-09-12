---
type: Report
title: "jq manual: streaming coverage review"
description: "Recorded review of jq manual: streaming coverage review."
generated: { by: codex/gpt-6, at: 2026-09-10T02:12:04Z }
---

# jq manual: streaming coverage review

Source: [pinned jq manual source inventory](../../../tests/compatibility/reviews/jq-manual/source-examples.toon), jq 1.8. The machine-readable ledger is [streaming.toon](../../../tests/compatibility/reviews/jq-manual/streaming.toon). The three table queries and inputs are copied exactly. The `--stream` and `--stream-errors` prose capabilities use executable adaptations; grammar placeholders are recorded as non-executable.

Inventory: 13 records (3 tables, 7 prose/examples, 3 non-executable grammar placeholders): 4 new, 3 adapted, 3 covered by exact cases, and 3 non-executable. The stream-option case also covers the concrete streamed representation and its array end markers. The error-form literal and malformed-input cases cover both the concrete placeholder and its executable witness. The final table covers the tostream/fromstream roundtrip.

| Line | Kind | Query / input | Case ID(s) | Disposition / reason |
| ---: | --- | --- | --- | --- |
| 16 | prose option | `.` / `["a",["b"]]` | `manual.streaming.stream-option` | adapted — representative small fixture for the documented `--stream` behavior |
| 18 | prose syntax | `[<path>, <leaf-value>]` | — | non-executable — placeholders |
| 22 | prose example | `.` / `["a",["b"]]` | `manual.streaming.stream-option` | new — emits the exact illustrated stream records |
| 24 | prose syntax | `[<path>, <leaf-value>]` | — | non-executable — placeholders |
| 24 | prose syntax | `[<path>]` | — | non-executable — placeholder |
| 24 | prose example | malformed JSON / `[1, bad, 2]` | `manual.streaming.stream-errors-form` | adapted — executable witness for generic `["error message"]` form |
| 24 | inline | `["error message"]` / `null` | `manual.streaming.stream-error-literal` | new — exact concrete error-form representation |
| 28 | prose definition | truncate stream / `1` | `manual.streaming.truncate-stream` | covered — exact table behavior |
| 30 | table | `truncate_stream([[0],"a"],[[1,0],"b"],[[1,0]],[[1]])` / `1` | `manual.streaming.truncate-stream` | new — exact query/input |
| 39 | prose definition | fromstream / `null` | `manual.streaming.fromstream-truncate` | covered — exact table behavior |
| 41 | table | `fromstream(1\|truncate_stream(...))` / `null` | `manual.streaming.fromstream-truncate` | new — exact query/input |
| 49 | prose definition | tostream roundtrip / `[0,[1,{"a":1},{"b":2}]]` | `manual.streaming.tostream-roundtrip` | covered — exact roundtrip behavior |
| 51 | table | `. as $dot\|fromstream($dot\|tostream)\|.==$dot` / roundtrip fixture | `manual.streaming.tostream-roundtrip` | new — exact query/input |

<!-- tq-manual-compare:begin section=streaming -->
## Results

[Case collection](../../../tests/compatibility/reviews/jq-manual/streaming.toon)


| Verdict | Cases |
| --- | ---: |
| match | 7 |

Independent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.

| Output campaign | Matches | Cases |
| --- | ---: | ---: |
| compact_json | 7 | 7 |
| toon | 7 | 7 |

A match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from matches. Historical expected-difference labels do not pass either gate.

Missing features and unaccepted mismatches remain failures. Reference discrepancies describe errors in the imported manual, not successful compatibility.

JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.

### Output size

7 eligible examples. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with default LF-terminated `-o toon` results; explicitly requested `--seq -o toon` output is shown in the cases but excluded from size totals. Diff is TOON tokens minus JSON tokens. % is the signed percent difference `(TOON - JSON) / JSON`, so savings are negative and growth is positive.

| Tokenizer | JSON tokens | TOON tokens | Diff | % |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 175 | 176 | 1 | +0.57% |
| `cl100k_base` | 175 | 176 | 1 | +0.57% |

Only successful jq/JSON/TOON-equivalent results enter the totals. A negative `Diff` means TOON uses fewer tokens; `%` is negative for savings and positive for growth. The manual is a correctness corpus, not a representative workload benchmark.

### Reviewed disparities and historical differences

| Case | Reason |
| --- | --- |

### Cases

Each case shows the original jq invocation, then the complete jq, tq JSON, and tq TOON output. Control bytes use `\xNN` escapes so record separators remain visible. This section is generated by the separate `tq-manual-compare` command.

#### builtin.tostream

```
# input
jq tostream

# jq
[
  [
    "a",
    0
  ],
  1
]
[
  [
    "a",
    0
  ]
]
[
  [
    "a"
  ]
]

# tq -o json
[
  [
    "a",
    0
  ],
  1
]
[
  [
    "a",
    0
  ]
]
[
  [
    "a"
  ]
]

# tq
[2]:
  - [2]: a,0
  - 1
[1]:
  - [2]: a,0
[1]:
  - [1]: a
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 42 | 42 | 39 | -3 | -7.14% |
| `cl100k_base` | 42 | 42 | 39 | -3 | -7.14% |

#### manual.streaming.fromstream-truncate

```
# input
jq 'fromstream(1|truncate_stream([[0],"a"],[[1,0],"b"],[[1,0]],[[1]]))'

# jq
[
  "b"
]

# tq -o json
[
  "b"
]

# tq
[1]: b
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 | -16.67% |
| `cl100k_base` | 6 | 6 | 5 | -1 | -16.67% |

#### manual.streaming.stream-error-literal

```
# input
jq '["error message"]'

# jq
[
  "error message"
]

# tq -o json
[
  "error message"
]

# tq
[1]: error message
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 7 | 7 | 6 | -1 | -14.29% |
| `cl100k_base` | 7 | 7 | 6 | -1 | -14.29% |

#### manual.streaming.stream-errors-form

```
# input
jq --stream-errors .

# jq
[
  [
    0
  ],
  1
]
[
  "Invalid numeric literal at line 1, column 8",
  [
    1
  ]
]

# tq -o json
[
  [
    0
  ],
  1
]
[
  "Invalid numeric literal at line 1, column 8",
  [
    1
  ]
]

# tq
[2]:
  - [1]: 0
  - 1
[2]:
  - "Invalid numeric literal at line 1, column 8"
  - [1]: 1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 38 | 38 | 42 | +4 | +10.53% |
| `cl100k_base` | 38 | 38 | 42 | +4 | +10.53% |

#### manual.streaming.stream-option

```
# input
jq --stream .

# jq
[
  [
    0
  ],
  "a"
]
[
  [
    1,
    0
  ],
  "b"
]
[
  [
    1,
    0
  ]
]
[
  [
    1
  ]
]

# tq -o json
[
  [
    0
  ],
  "a"
]
[
  [
    1,
    0
  ],
  "b"
]
[
  [
    1,
    0
  ]
]
[
  [
    1
  ]
]

# tq
[2]:
  - [1]: 0
  - a
[2]:
  - [2]: 1,0
  - b
[1]:
  - [2]: 1,0
[1]:
  - [1]: 1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 56 | 56 | 56 | 0 | +0% |
| `cl100k_base` | 56 | 56 | 56 | 0 | +0% |

#### manual.streaming.tostream-roundtrip

```
# input
jq '. as $dot|fromstream($dot|tostream)|.==$dot'

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.streaming.truncate-stream

```
# input
jq 'truncate_stream([[0],"a"],[[1,0],"b"],[[1,0]],[[1]])'

# jq
[
  [
    0
  ],
  "b"
]
[
  [
    0
  ]
]

# tq -o json
[
  [
    0
  ],
  "b"
]
[
  [
    0
  ]
]

# tq
[2]:
  - [1]: 0
  - b
[1]:
  - [1]: 0
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 24 | 24 | 26 | +2 | +8.33% |
| `cl100k_base` | 24 | 24 | 26 | +2 | +8.33% |

<!-- tq-manual-compare:end -->
