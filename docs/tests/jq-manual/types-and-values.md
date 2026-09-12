---
type: Report
title: "jq manual: types-and-values coverage review"
description: "Recorded review of jq manual: types-and-values coverage review."
generated: { by: codex/gpt-6, at: 2026-09-10T02:12:04Z }
---

# jq manual: types-and-values coverage review

Source: [pinned jq manual source inventory](../../../tests/compatibility/reviews/jq-manual/source-examples.toon), jq 1.8. The machine-readable ledger is [types-and-values.toon](../../../tests/compatibility/reviews/jq-manual/types-and-values.toon). Table queries and inputs are copied exactly; runnable fenced and inline examples retain their exact query text. Synthesized fixtures are called out where the prose gives only a shape or no concrete input. All executable MVP cases keep jq and tq enabled; yq is marked unsupported because these are jq-target manual examples.

Inventory: 37 records (5 tables, 9 fenced blocks, 23 inline/prose examples): 15 new, 8 adapted, 10 covered by exact cases, and 4 non-executable placeholders. Every fenced opening line is recorded with its source line. Output-only and fixture-only blocks link to the executable cases that produce or consume them.

| Line | Kind | Query / input | Case ID(s) | Disposition / reason |
| ---: | --- | --- | --- | --- |
| 32 | table | `[.user, .projects[]]` / `{"user":"stedolan", "projects": ["jq", "wikiflow"]}` | `manual.types.table-001` | new — exact query/input |
| 38 | table | `[ .[] \| . * 2]` / `[1, 2, 3]` | `manual.types.table-002` | new — exact query/input |
| 107 | table | `{user, title: .titles[]}` / titles fixture | `manual.types.table-003` | new — exact query/input |
| 114 | table | `{(.user): .titles}` / titles fixture | `manual.types.table-004` | new — exact query/input |
| 126 | table | `.. \| .a?` / `[[{"a":1}]]` | `manual.types.table-005` | new — exact query/input |
| 52 | fenced | `{foo: .bar}` / `{"bar":42,"baz":43}` | `manual.types.fence-object-field` | new — exact query; prose fixture |
| 58 | fenced | `{user: .user, title: .title}` / representative user/title/id/content object | `manual.types.fence-object-selection` | adapted — exact query; fixture synthesized from prose fields |
| 66 | fenced fixture | titles input object | `manual.types.table-003`, `manual.types.table-004` | covered — reused by following queries |
| 72 | fenced | `{user, title: .titles[]}` / titles fixture | `manual.types.table-003` | covered — exact table case |
| 78 | fenced output | two constructed title objects | `manual.types.table-003` | covered — output of preceding query |
| 85 | fenced | `{(.user): .titles}` / titles fixture | `manual.types.table-004` | covered — exact table case |
| 91 | fenced output | computed-key object | `manual.types.table-004` | covered — output of preceding query |
| 97 | fenced | variable-key query / `null` | `manual.types.fence-variable-key` | new — exact query; input irrelevant |
| 103 | fenced output | variable-key object | `manual.types.fence-variable-key` | covered — output of preceding query |
| 18 | inline | `42` / `null` | `manual.types.inline-literal-number` | new — input-independent literal |
| 24 | inline | `[]` / `null` | `manual.types.inline-empty-array` | new — representative input |
| 26 | inline | `[1,2,3]` / `null` | `manual.types.inline-array-literal` | new — representative input |
| 26 | inline | `[]` / `null` | `manual.types.inline-empty-array` | covered — repeated constructor syntax |
| 26 | inline | `[.foo, .bar, .baz]` / representative object | `manual.types.inline-array-fields` | adapted — fixture synthesized |
| 26 | inline | `[.items[].name]` / representative items object | `manual.types.inline-array-names` | adapted — fixture synthesized |
| 28 | inline | `[1,2,3]` / `null` | `manual.types.inline-array-literal` | covered — repeated expression |
| 30 | inline | `[X]` | — | non-executable — `X` is an incomplete placeholder |
| 44 | inline | `{}` / `null` | `manual.types.inline-empty-object` | new — representative input |
| 46 | inline | `{}` / `null` | `manual.types.inline-empty-object` | covered — repeated constructor syntax |
| 46 | inline | `{"a": 42, "b": 17}` / `null` | `manual.types.inline-object-literal` | new — representative input |
| 48 | inline | `{a:42, b:17}` / `null` | `manual.types.inline-object-identifier-keys` | new — representative input |
| 48 | inline | `{("a"+"b"):59}` / `null` | `manual.types.inline-object-expression-key` | new — representative input |
| 62 | inline | `{user, title}` / representative object | `manual.types.inline-object-shorthand` | adapted — fixture synthesized |
| 120 | inline | `..` / representative nested object | `manual.types.inline-recursive-descent` | adapted — fixture synthesized |
| 122 | inline | `.` / representative object | `manual.types.inline-current-value` | adapted — fixture synthesized |
| 122 | inline | `recurse` / representative object | `manual.types.inline-recurse-builtin` | adapted — fixture synthesized |
| 122 | inline | `..a` / nested array/object | `manual.types.inline-invalid-recursive` | new — documented compile-error example |
| 122 | inline | `.. \| .a` / `null` | `manual.types.inline-recursive-pipe` | adapted — exact replacement query; representative input |
| 122 | inline | `.. \| .a?` / `[[{"a":1}]]` | `manual.types.table-005` | covered — exact table case |
| 122 | inline | `//` | — | non-executable — XPath comparison notation |
| 124 | inline | `path(EXP)` | — | non-executable — `EXP` is a placeholder |
| 124 | inline | `?` | — | non-executable — operator shown without an operand |

<!-- tq-manual-compare:begin section=types-and-values -->
## Results

[Case collection](../../../tests/compatibility/reviews/jq-manual/types-and-values.toon)


| Verdict | Cases |
| --- | ---: |
| match | 23 |

Independent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.

| Output campaign | Matches | Cases |
| --- | ---: | ---: |
| compact_json | 23 | 23 |
| toon | 23 | 23 |

A match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from matches. Historical expected-difference labels do not pass either gate.

Missing features and unaccepted mismatches remain failures. Reference discrepancies describe errors in the imported manual, not successful compatibility.

JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.

### Output size

22 eligible examples. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with default LF-terminated `-o toon` results; explicitly requested `--seq -o toon` output is shown in the cases but excluded from size totals. Diff is TOON tokens minus JSON tokens. % is the signed percent difference `(TOON - JSON) / JSON`, so savings are negative and growth is positive.

| Tokenizer | JSON tokens | TOON tokens | Diff | % |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 288 | 188 | -100 | -34.72% |
| `cl100k_base` | 288 | 193 | -95 | -32.99% |

Only successful jq/JSON/TOON-equivalent results enter the totals. A negative `Diff` means TOON uses fewer tokens; `%` is negative for savings and positive for growth. The manual is a correctness corpus, not a representative workload benchmark.

### Reviewed disparities and historical differences

| Case | Reason |
| --- | --- |

### Cases

Each case shows the original jq invocation, then the complete jq, tq JSON, and tq TOON output. Control bytes use `\xNN` escapes so record separators remain visible. This section is generated by the separate `tq-manual-compare` command.

#### manual.types.fence-object-field

```
# input
jq '{foo: .bar}'

# jq
{
  "foo": 42
}

# tq -o json
{
  "foo": 42
}

# tq
foo: 42
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 5 | -4 | -44.44% |
| `cl100k_base` | 9 | 9 | 5 | -4 | -44.44% |

#### manual.types.fence-object-selection

```
# input
jq '{user: .user, title: .title}'

# jq
{
  "user": "stedolan",
  "title": "JQ Primer"
}

# tq -o json
{
  "user": "stedolan",
  "title": "JQ Primer"
}

# tq
user: stedolan
title: JQ Primer
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 19 | 19 | 11 | -8 | -42.11% |
| `cl100k_base` | 19 | 19 | 12 | -7 | -36.84% |

#### manual.types.fence-variable-key

```
# input
jq '"f o o" as $foo | "b a r" as $bar | {$foo, $bar:$foo}'

# jq
{
  "foo": "f o o",
  "b a r": "f o o"
}

# tq -o json
{
  "foo": "f o o",
  "b a r": "f o o"
}

# tq
foo: f o o
"b a r": f o o
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 15 | -7 | -31.82% |
| `cl100k_base` | 22 | 22 | 15 | -7 | -31.82% |

#### manual.types.inline-array-fields

```
# input
jq '[.foo, .bar, .baz]'

# jq
[
  1,
  2,
  3
]

# tq -o json
[
  1,
  2,
  3
]

# tq
[3]: 1,2,3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual.types.inline-array-literal

```
# input
jq '[1,2,3]'

# jq
[
  1,
  2,
  3
]

# tq -o json
[
  1,
  2,
  3
]

# tq
[3]: 1,2,3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual.types.inline-array-names

```
# input
jq '[.items[].name]'

# jq
[
  "alpha",
  "beta"
]

# tq -o json
[
  "alpha",
  "beta"
]

# tq
[2]: alpha,beta
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 | -30% |
| `cl100k_base` | 10 | 10 | 7 | -3 | -30% |

#### manual.types.inline-current-value

```
# input
jq .

# jq
{
  "a": 1
}

# tq -o json
{
  "a": 1
}

# tq
a: 1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 5 | -4 | -44.44% |
| `cl100k_base` | 9 | 9 | 5 | -4 | -44.44% |

#### manual.types.inline-empty-array

```
# input
jq '[]'

# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 | +200% |
| `cl100k_base` | 1 | 1 | 3 | +2 | +200% |

#### manual.types.inline-empty-object

```
# input
jq '{}'

# jq
{}

# tq -o json
{}

# tq

```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 1 | 0 | +0% |
| `cl100k_base` | 1 | 1 | 1 | 0 | +0% |

#### manual.types.inline-invalid-recursive

```
# input
jq ..a

# jq


[stderr]
jq: error: syntax error, unexpected IDENT, expecting end of file at <top-level>, line 1, column 3:
    ..a
      ^
jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-PARSE-UNEXPECTED-001: expected end of query

# tq


[stderr]
tq: query compilation failed: TQ-PARSE-UNEXPECTED-001: expected end of query
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 | n/a |
| `cl100k_base` | 0 | 0 | 0 | 0 | n/a |

#### manual.types.inline-literal-number

```
# input
jq 42

# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.types.inline-object-expression-key

```
# input
jq '{("a"+"b"):59}'

# jq
{
  "ab": 59
}

# tq -o json
{
  "ab": 59
}

# tq
ab: 59
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 5 | -4 | -44.44% |
| `cl100k_base` | 9 | 9 | 5 | -4 | -44.44% |

#### manual.types.inline-object-identifier-keys

```
# input
jq '{a:42, b:17}'

# jq
{
  "a": 42,
  "b": 17
}

# tq -o json
{
  "a": 42,
  "b": 17
}

# tq
a: 42
b: 17
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 10 | -6 | -37.5% |
| `cl100k_base` | 16 | 16 | 10 | -6 | -37.5% |

#### manual.types.inline-object-literal

```
# input
jq '{"a": 42, "b": 17}'

# jq
{
  "a": 42,
  "b": 17
}

# tq -o json
{
  "a": 42,
  "b": 17
}

# tq
a: 42
b: 17
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 10 | -6 | -37.5% |
| `cl100k_base` | 16 | 16 | 10 | -6 | -37.5% |

#### manual.types.inline-object-shorthand

```
# input
jq '{user, title}'

# jq
{
  "user": "stedolan",
  "title": "JQ Primer"
}

# tq -o json
{
  "user": "stedolan",
  "title": "JQ Primer"
}

# tq
user: stedolan
title: JQ Primer
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 19 | 19 | 11 | -8 | -42.11% |
| `cl100k_base` | 19 | 19 | 12 | -7 | -36.84% |

#### manual.types.inline-recurse-builtin

```
# input
jq recurse

# jq
{
  "a": 1
}
1

# tq -o json
{
  "a": 1
}
1

# tq
a: 1
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 11 | 11 | 7 | -4 | -36.36% |
| `cl100k_base` | 11 | 11 | 7 | -4 | -36.36% |

#### manual.types.inline-recursive-descent

```
# input
jq ..

# jq
{
  "a": [
    1
  ]
}
[
  1
]
1

# tq -o json
{
  "a": [
    1
  ]
}
[
  1
]
1

# tq
a[1]: 1
[1]: 1
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 21 | 21 | 15 | -6 | -28.57% |
| `cl100k_base` | 21 | 21 | 15 | -6 | -28.57% |

#### manual.types.inline-recursive-pipe

```
# input
jq '.. | .a'

# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.types.table-001

```
# input
jq '[.user, .projects[]]'

# jq
[
  "stedolan",
  "jq",
  "wikiflow"
]

# tq -o json
[
  "stedolan",
  "jq",
  "wikiflow"
]

# tq
[3]: stedolan,jq,wikiflow
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 12 | -5 | -29.41% |
| `cl100k_base` | 17 | 17 | 13 | -4 | -23.53% |

#### manual.types.table-002

```
# input
jq '[ .[] | . * 2]'

# jq
[
  2,
  4,
  6
]

# tq -o json
[
  2,
  4,
  6
]

# tq
[3]: 2,4,6
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual.types.table-003

```
# input
jq '{user, title: .titles[]}'

# jq
{
  "user": "stedolan",
  "title": "JQ Primer"
}
{
  "user": "stedolan",
  "title": "More JQ"
}

# tq -o json
{
  "user": "stedolan",
  "title": "JQ Primer"
}
{
  "user": "stedolan",
  "title": "More JQ"
}

# tq
user: stedolan
title: JQ Primer
user: stedolan
title: More JQ
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 38 | 38 | 22 | -16 | -42.11% |
| `cl100k_base` | 38 | 38 | 24 | -14 | -36.84% |

#### manual.types.table-004

```
# input
jq '{(.user): .titles}'

# jq
{
  "stedolan": [
    "JQ Primer",
    "More JQ"
  ]
}

# tq -o json
{
  "stedolan": [
    "JQ Primer",
    "More JQ"
  ]
}

# tq
stedolan[2]: JQ Primer,More JQ
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 13 | -9 | -40.91% |
| `cl100k_base` | 22 | 22 | 13 | -9 | -40.91% |

#### manual.types.table-005

```
# input
jq '.. | .a?'

# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

<!-- tq-manual-compare:end -->
