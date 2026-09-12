---
type: Report
title: "jq manual: advanced-features coverage review"
description: "Recorded review of jq manual: advanced-features coverage review."
generated: { by: codex/gpt-6, at: 2026-09-10T02:12:04Z }
---

# jq manual: advanced-features coverage review

Source: [pinned jq manual source inventory](../../../tests/compatibility/reviews/jq-manual/source-examples.toon), jq 1.8. The machine-readable ledger is [advanced-features.toon](../../../tests/compatibility/reviews/jq-manual/advanced-features.toon). Table queries and inputs are copied exactly; all MVP cases keep jq and tq enabled so incompatibilities remain observable. yq is marked unsupported because these are jq-target manual examples.

Inventory: 63 records (25 tables, 25 fenced blocks, 7 runnable prose examples, 6 signatures/conceptual fragments): 43 new, 5 covered by exact cases, and 15 non-executable. Fenced `source_line` values are the opening fence lines from the source inventory. Fixture-only and output-only blocks link to the executable cases that consume or produce them.

| Line | Kind | Query / input | Case ID(s) | Disposition / reason |
| ---: | --- | --- | --- | --- |
| 99 | table | `.bar as $x \| .foo \| . + $x` / `{"foo":10, "bar":200}` | `manual.advanced.table-001` | new — exact query/input |
| 105 | table | `. as $i\|[(.*2\|. as $i\| $i), $i]` / `5` | `manual.advanced.table-002` | new — exact query/input |
| 111 | table | `. as [$a, $b, {c: $c}] \| $a + $b + $c` / `[2, 3, {"c": 4, "d": 5}]` | `manual.advanced.table-003` | new — exact query/input |
| 117 | table | `.[] as [$a, $b] \| {a: $a, b: $b}` / `[[0], [0, 1], [2, 1, 0]]` | `manual.advanced.table-004` | new — exact query/input |
| 160 | table | destructuring `?//` / two object forms | `manual.advanced.table-005` | new — exact query/input |
| 167 | table | destructuring `?//` with alternate variables / two object forms | `manual.advanced.table-006` | new — exact query/input |
| 174 | table | error-retrying `?//` / `[[3]]` | `manual.advanced.table-007` | new — exact query/input; downstream error retries |
| 219 | table | `def addvalue(f): . + [f]; map(addvalue(.[0]))` / `[[1,2],[10,20]]` | `manual.advanced.table-008` | new — exact query/input |
| 225 | table | value-like `addvalue` / `[[1,2],[10,20]]` | `manual.advanced.table-009` | new — exact query/input |
| 251 | table | `isempty(empty)` / `null` | `manual.advanced.table-010` | new — exact query/input |
| 257 | table | `isempty(.[])` / `[]` | `manual.advanced.table-011` | new — exact query/input |
| 263 | table | `isempty(.[])` / `[1,2,3]` | `manual.advanced.table-012` | new — exact query/input |
| 273 | table | `[limit(3; .[])]` / `[0,1,2,3,4,5,6,7,8,9]` | `manual.advanced.table-013` | new — exact query/input |
| 283 | table | `[skip(3; .[])]` / `[0,1,2,3,4,5,6,7,8,9]` | `manual.advanced.table-014` | new — exact query/input |
| 295 | table | first/last/nth generator / `10` | `manual.advanced.table-015` | new — exact query/input |
| 301 | table | first/last/nth empty generator / `null` | `manual.advanced.table-016` | new — exact query/input |
| 313 | table | array first/last/nth / `10` | `manual.advanced.table-017` | new — exact query/input |
| 335 | table | reduce sum / `[1,2,3,4,5]` | `manual.advanced.table-018` | new — exact query/input |
| 341 | table | reduce destructured pairs / `[[1,2],[3,4],[5,6]]` | `manual.advanced.table-019` | new — exact query/input |
| 347 | table | reduce destructured objects / three `{x,y}` objects | `manual.advanced.table-020` | new — exact query/input |
| 373 | table | foreach accumulator / `[1,2,3,4,5]` | `manual.advanced.table-021` | new — exact query/input |
| 383 | table | foreach extraction / `[1,2,3,4,5]` | `manual.advanced.table-022` | new — exact query/input |
| 393 | table | foreach index / `["foo", "bar", "baz"]` | `manual.advanced.table-023` | new — exact query/input |
| 433 | table | recursive range generator / `null` | `manual.advanced.table-024` | new — exact query/input |
| 442 | table | recursive while generator / `1` | `manual.advanced.table-025` | new — exact query/input |
| 38 | fenced | `length as $array_length \| add / $array_length` / synthesized `[10,20,30]` | `manual.advanced.fence-average-variable` | new — runnable filter; synthesized numeric fixture |
| 46 | fenced fixture | posts/realnames object | `manual.advanced.fence-post-author-lookup`, `manual.advanced.fence-post-author-lookup-parenthesized`, `manual.advanced.fence-post-author-lookup-out-of-scope` | covered — input-only fixture |
| 55 | fenced output | two named-author objects | `manual.advanced.fence-post-author-lookup` | covered — output of preceding query |
| 62 | fenced | `.realnames as $names \| .posts[] \| {title, author: $names[.author]}` / posts object | `manual.advanced.fence-post-author-lookup` | new — exact runnable query/fixture |
| 72 | fenced | ellipsis placeholder | — | non-executable — incomplete placeholder |
| 77 | fenced | binding with no following expression | — | non-executable — incomplete syntax fragment |
| 85 | fenced | parenthesized author lookup / posts object | `manual.advanced.fence-post-author-lookup-parenthesized` | new — exact runnable query/fixture |
| 91 | fenced | out-of-scope author lookup / posts object | `manual.advanced.fence-post-author-lookup-out-of-scope` | new — expected compile error preserved |
| 131 | fenced fixture | resources/events object | `manual.advanced.fence-destructuring-alternative`, `manual.advanced.fence-destructuring-alternative-variables` | covered — input-only fixture |
| 138 | fenced | resource destructuring `?//` / resources object | `manual.advanced.fence-destructuring-alternative` | new — exact runnable query/fixture |
| 144 | fenced | ellipsis placeholder | — | non-executable — incomplete placeholder |
| 150 | fenced | alternate variable destructuring / resources object | `manual.advanced.fence-destructuring-alternative-variables` | new — exact runnable query/fixture |
| 156 | fenced | error-retrying `?//` / `[[3]]` | `manual.advanced.table-007` | covered — exact table case |
| 184 | fenced | synthesized `def increment` invocation / `null` | `manual.advanced.fence-function-increment` | new — declaration made runnable with invocation |
| 190 | fenced | synthesized `def map` invocation / `[1,2,3]` | `manual.advanced.fence-function-map` | new — declaration made runnable with invocation |
| 196 | fenced | `def foo(f): f\|f; 5\|foo(.*2)` / `null` | `manual.advanced.fence-function-filter-argument` | new — exact runnable block |
| 205 | fenced | `def addvalue(f): f as $f \| map(. + $f);` | `manual.advanced.table-009` | covered — definition exercised by exact table case |
| 211 | fenced | `def addvalue($f): ...;` | — | non-executable — function-body placeholder |
| 236 | fenced | scoping example with ellipses | — | non-executable — incomplete illustration |
| 241 | fenced | scoped scoping example with ellipses | — | non-executable — incomplete illustration |
| 323 | fenced | `reduce .[] as $item (0; . + $item)` / `[1,2,3,4,5]` | `manual.advanced.table-018` | covered — exact table case |
| 329 | fenced | expanded reduce pipeline / synthesized `null` | `manual.advanced.fence-reduce-expanded` | new — exact runnable expansion |
| 359 | fenced | `foreach .[] as $item (0; . + $item; [$item, . * 2])` / `[1,2,3,4,5]` | `manual.advanced.table-022` | covered — exact table case |
| 365 | fenced | expanded foreach pipeline / synthesized `null` | `manual.advanced.fence-foreach-expanded` | new — exact runnable expansion |
| 409 | fenced | recursive helper library plus synthesized identity invocation / `null` | `manual.advanced.fence-recursion-library` | new — declarations made runnable; unbounded `repeat` is defined but not invoked |

| Line | Kind | Query / input | Case ID(s) | Disposition / reason |
| ---: | --- | --- | --- | --- |
| 34 | prose | `add / length` / synthesized `[10,20,30]` | `manual.advanced.prose-average-direct` | new — runnable prose filter |
| 68 | prose | `{foo}` / synthesized `{"foo":42}` | `manual.advanced.prose-object-shorthand` | new — runnable shorthand filter |
| 68 | prose | `.foo as $foo \| {$foo}` / synthesized `{"foo":42}` | `manual.advanced.prose-variable-shorthand` | new — binding context synthesized for variable shorthand |
| 215 | prose | documented `addvalue(.foo)` call / synthesized object array | `manual.advanced.prose-addvalue-field` | new — definition and fixture supplied |
| 215 | prose | documented `addvalue(.[])` call / synthesized `[1,2]` | `manual.advanced.prose-addvalue-iterator` | new — definition and fixture supplied |
| 425 | prose | `.[]` / `[1,2,3]` | `manual.basic.iterator-prose-array` | covered — exact existing generator case |
| 425 | prose | `range(0; 10)` / synthesized `null` | `manual.advanced.prose-range-generator` | new — runnable generator prose filter |
| 30 | prose signature | `... as $identifier \| ...` | — | non-executable — ellipsis/signature placeholder |
| 32 | prose conceptual | `a + b` discussion | — | non-executable — unnamed conceptual placeholders |
| 36 | prose signature | `expression as $variable` | — | non-executable — parameterized syntax signature |
| 66 | prose signature | `exp as $x \| ...` | — | non-executable — ellipsis/signature placeholder |
| 321 | prose signature | `reduce EXP as $var (INIT; UPDATE)` | — | non-executable — parameterized syntax signature |
| 357 | prose signature | `foreach EXP as $var (INIT; UPDATE; EXTRACT)` | — | non-executable — parameterized syntax signature |

Execution is intentionally left to the parent compatibility build/suite; this audit records source-to-case coverage and does not claim jq/tq parity.

<!-- tq-manual-compare:begin section=advanced-features -->
## Results

[Case collection](../../../tests/compatibility/reviews/jq-manual/advanced-features.toon)


| Verdict | Cases |
| --- | ---: |
| match | 45 |

Independent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.

| Output campaign | Matches | Cases |
| --- | ---: | ---: |
| compact_json | 45 | 45 |
| toon | 45 | 45 |

A match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from matches. Historical expected-difference labels do not pass either gate.

Missing features and unaccepted mismatches remain failures. Reference discrepancies describe errors in the imported manual, not successful compatibility.

JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.

### Output size

44 eligible examples. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with default LF-terminated `-o toon` results; explicitly requested `--seq -o toon` output is shown in the cases but excluded from size totals. Diff is TOON tokens minus JSON tokens. % is the signed percent difference `(TOON - JSON) / JSON`, so savings are negative and growth is positive.

| Tokenizer | JSON tokens | TOON tokens | Diff | % |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 903 | 636 | -267 | -29.57% |
| `cl100k_base` | 903 | 636 | -267 | -29.57% |

Only successful jq/JSON/TOON-equivalent results enter the totals. A negative `Diff` means TOON uses fewer tokens; `%` is negative for savings and positive for growth. The manual is a correctness corpus, not a representative workload benchmark.

### Reviewed disparities and historical differences

| Case | Reason |
| --- | --- |

### Cases

Each case shows the original jq invocation, then the complete jq, tq JSON, and tq TOON output. Control bytes use `\xNN` escapes so record separators remain visible. This section is generated by the separate `tq-manual-compare` command.

#### builtin.limit

```
# input
jq 'limit(2; range(0;5))'

# jq
0
1

# tq -o json
0
1

# tq
0
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### manual.advanced.fence-average-variable

```
# input
jq 'length as $array_length | add / $array_length'

# jq
20

# tq -o json
20

# tq
20
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.advanced.fence-destructuring-alternative

```
# input
jq '.resources[] as {$id, $kind, events: {$user_id, $ts}} ?// {$id, $kind, events: [{$user_id, $ts}]} | {$user_id, $kind, $id, $ts}'

# jq
{
  "user_id": 1,
  "kind": "widget",
  "id": 1,
  "ts": 13
}
{
  "user_id": 1,
  "kind": "widget",
  "id": 2,
  "ts": 14
}

# tq -o json
{
  "user_id": 1,
  "kind": "widget",
  "id": 1,
  "ts": 13
}
{
  "user_id": 1,
  "kind": "widget",
  "id": 2,
  "ts": 14
}

# tq
user_id: 1
kind: widget
id: 1
ts: 13
user_id: 1
kind: widget
id: 2
ts: 14
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 62 | 62 | 40 | -22 | -35.48% |
| `cl100k_base` | 62 | 62 | 40 | -22 | -35.48% |

#### manual.advanced.fence-destructuring-alternative-variables

```
# input
jq '.resources[] as {$id, $kind, events: {$user_id, $ts}} ?// {$id, $kind, events: [{$first_user_id, $first_ts}]} | {$user_id, $first_user_id, $kind, $id, $ts, $first_ts}'

# jq
{
  "user_id": 1,
  "first_user_id": null,
  "kind": "widget",
  "id": 1,
  "ts": 13,
  "first_ts": null
}
{
  "user_id": null,
  "first_user_id": null,
  "kind": "widget",
  "id": 2,
  "ts": null,
  "first_ts": null
}

# tq -o json
{
  "user_id": 1,
  "first_user_id": null,
  "kind": "widget",
  "id": 1,
  "ts": 13,
  "first_ts": null
}
{
  "user_id": null,
  "first_user_id": null,
  "kind": "widget",
  "id": 2,
  "ts": null,
  "first_ts": null
}

# tq
user_id: 1
first_user_id: null
kind: widget
id: 1
ts: 13
first_ts: null
user_id: null
first_user_id: null
kind: widget
id: 2
ts: null
first_ts: null
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 90 | 90 | 60 | -30 | -33.33% |
| `cl100k_base` | 90 | 90 | 60 | -30 | -33.33% |

#### manual.advanced.fence-foreach-expanded

```
# input
jq '0 | 1 as $item | . + $item | [$item, . * 2],
    2 as $item | . + $item | [$item, . * 2],
    3 as $item | . + $item | [$item, . * 2]'

# jq
[
  1,
  2
]
[
  2,
  6
]
[
  3,
  12
]

# tq -o json
[
  1,
  2
]
[
  2,
  6
]
[
  3,
  12
]

# tq
[2]: 1,2
[2]: 2,6
[2]: 3,12
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 24 | -6 | -20% |
| `cl100k_base` | 30 | 30 | 24 | -6 | -20% |

#### manual.advanced.fence-function-filter-argument

```
# input
jq 'def foo(f): f|f; 5|foo(.*2)'

# jq
20

# tq -o json
20

# tq
20
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.advanced.fence-function-increment

```
# input
jq 'def increment: . + 1; 1 | increment'

# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.advanced.fence-function-map

```
# input
jq 'def map(f): [.[] | f]; map(.+1)'

# jq
[
  2,
  3,
  4
]

# tq -o json
[
  2,
  3,
  4
]

# tq
[3]: 2,3,4
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual.advanced.fence-post-author-lookup

```
# input
jq '.realnames as $names | .posts[] | {title, author: $names[.author]}'

# jq
{
  "title": "First post",
  "author": "Anonymous Coward"
}
{
  "title": "A well-written article",
  "author": "Person McPherson"
}

# tq -o json
{
  "title": "First post",
  "author": "Anonymous Coward"
}
{
  "title": "A well-written article",
  "author": "Person McPherson"
}

# tq
title: First post
author: Anonymous Coward
title: A well-written article
author: Person McPherson
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 41 | 41 | 25 | -16 | -39.02% |
| `cl100k_base` | 41 | 41 | 25 | -16 | -39.02% |

#### manual.advanced.fence-post-author-lookup-out-of-scope

```
# input
jq '(.realnames as $names | .posts[]) | {title, author: $names[.author]}'

# jq


[stderr]
jq: error: $names is not defined at <top-level>, line 1, column 53:
    (.realnames as $names | .posts[]) | {title, author: $names[.author]}
                                                        ^^^^^^
jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-RESOLVE-VARIABLE-001: unknown variable $names

# tq


[stderr]
tq: query compilation failed: TQ-RESOLVE-VARIABLE-001: unknown variable $names
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 | n/a |
| `cl100k_base` | 0 | 0 | 0 | 0 | n/a |

#### manual.advanced.fence-post-author-lookup-parenthesized

```
# input
jq '.realnames as $names | (.posts[] | {title, author: $names[.author]})'

# jq
{
  "title": "First post",
  "author": "Anonymous Coward"
}
{
  "title": "A well-written article",
  "author": "Person McPherson"
}

# tq -o json
{
  "title": "First post",
  "author": "Anonymous Coward"
}
{
  "title": "A well-written article",
  "author": "Person McPherson"
}

# tq
title: First post
author: Anonymous Coward
title: A well-written article
author: Person McPherson
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 41 | 41 | 25 | -16 | -39.02% |
| `cl100k_base` | 41 | 41 | 25 | -16 | -39.02% |

#### manual.advanced.fence-recursion-library

```
# input
jq 'def recurse(f): def r: ., (f | select(. != null) | r); r;

def while(cond; update):
  def _while:
    if cond then ., (update | _while) else empty end;
  _while;

def repeat(exp):
  def _repeat:
    exp, _repeat;
  _repeat;
.'

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

#### manual.advanced.fence-reduce-expanded

```
# input
jq '0 | 1 as $item | . + $item |
    2 as $item | . + $item |
    3 as $item | . + $item'

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

#### manual.advanced.prose-addvalue-field

```
# input
jq 'def addvalue(f): f as $f | map(. + $f); map(addvalue(.foo))'

# jq
[
  [
    20
  ],
  [
    40
  ]
]

# tq -o json
[
  [
    20
  ],
  [
    40
  ]
]

# tq
[2]:
  - [1]: 20
  - [1]: 40
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 19 | +1 | +5.56% |
| `cl100k_base` | 18 | 18 | 19 | +1 | +5.56% |

#### manual.advanced.prose-addvalue-iterator

```
# input
jq 'def addvalue(f): f as $f | map(. + $f); addvalue(.[])'

# jq
[
  2,
  3
]
[
  3,
  4
]

# tq -o json
[
  2,
  3
]
[
  3,
  4
]

# tq
[2]: 2,3
[2]: 3,4
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 16 | -4 | -20% |
| `cl100k_base` | 20 | 20 | 16 | -4 | -20% |

#### manual.advanced.prose-average-direct

```
# input
jq 'add / length'

# jq
20

# tq -o json
20

# tq
20
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.advanced.prose-object-shorthand

```
# input
jq '{foo}'

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

#### manual.advanced.prose-range-generator

```
# input
jq 'range(0; 10)'

# jq
0
1
2
3
4
5
6
7
8
9

# tq -o json
0
1
2
3
4
5
6
7
8
9

# tq
0
1
2
3
4
5
6
7
8
9
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 20 | 0 | +0% |
| `cl100k_base` | 20 | 20 | 20 | 0 | +0% |

#### manual.advanced.prose-variable-shorthand

```
# input
jq '.foo as $foo | {$foo}'

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

#### manual.advanced.table-001

```
# input
jq '.bar as $x | .foo | . + $x'

# jq
210

# tq -o json
210

# tq
210
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.advanced.table-002

```
# input
jq '. as $i|[(.*2|. as $i| $i), $i]'

# jq
[
  10,
  5
]

# tq -o json
[
  10,
  5
]

# tq
[2]: 10,5
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 8 | -2 | -20% |
| `cl100k_base` | 10 | 10 | 8 | -2 | -20% |

#### manual.advanced.table-003

```
# input
jq '. as [$a, $b, {c: $c}] | $a + $b + $c'

# jq
9

# tq -o json
9

# tq
9
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.advanced.table-004

```
# input
jq '.[] as [$a, $b] | {a: $a, b: $b}'

# jq
{
  "a": 0,
  "b": null
}
{
  "a": 0,
  "b": 1
}
{
  "a": 2,
  "b": 1
}

# tq -o json
{
  "a": 0,
  "b": null
}
{
  "a": 0,
  "b": 1
}
{
  "a": 2,
  "b": 1
}

# tq
a: 0
b: null
a: 0
b: 1
a: 2
b: 1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 47 | 47 | 29 | -18 | -38.3% |
| `cl100k_base` | 47 | 47 | 29 | -18 | -38.3% |

#### manual.advanced.table-005

```
# input
jq '.[] as {$a, $b, c: {$d, $e}} ?// {$a, $b, c: [{$d, $e}]} | {$a, $b, $d, $e}'

# jq
{
  "a": 1,
  "b": 2,
  "d": 3,
  "e": 4
}
{
  "a": 1,
  "b": 2,
  "d": 3,
  "e": 4
}

# tq -o json
{
  "a": 1,
  "b": 2,
  "d": 3,
  "e": 4
}
{
  "a": 1,
  "b": 2,
  "d": 3,
  "e": 4
}

# tq
a: 1
b: 2
d: 3
e: 4
a: 1
b: 2
d: 3
e: 4
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 60 | 60 | 40 | -20 | -33.33% |
| `cl100k_base` | 60 | 60 | 40 | -20 | -33.33% |

#### manual.advanced.table-006

```
# input
jq '.[] as {$a, $b, c: {$d}} ?// {$a, $b, c: [{$e}]} | {$a, $b, $d, $e}'

# jq
{
  "a": 1,
  "b": 2,
  "d": 3,
  "e": null
}
{
  "a": 1,
  "b": 2,
  "d": null,
  "e": 4
}

# tq -o json
{
  "a": 1,
  "b": 2,
  "d": 3,
  "e": null
}
{
  "a": 1,
  "b": 2,
  "d": null,
  "e": 4
}

# tq
a: 1
b: 2
d: 3
e: null
a: 1
b: 2
d: null
e: 4
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 58 | 58 | 38 | -20 | -34.48% |
| `cl100k_base` | 58 | 58 | 38 | -20 | -34.48% |

#### manual.advanced.table-007

```
# input
jq '.[] as [$a] ?// [$b] | if $a != null then error("err: \($a)") else {$a,$b} end'

# jq
{
  "a": null,
  "b": 3
}

# tq -o json
{
  "a": null,
  "b": 3
}

# tq
a: null
b: 3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 15 | 15 | 9 | -6 | -40% |
| `cl100k_base` | 15 | 15 | 9 | -6 | -40% |

#### manual.advanced.table-008

```
# input
jq 'def addvalue(f): . + [f]; map(addvalue(.[0]))'

# jq
[
  [
    1,
    2,
    1
  ],
  [
    10,
    20,
    10
  ]
]

# tq -o json
[
  [
    1,
    2,
    1
  ],
  [
    10,
    20,
    10
  ]
]

# tq
[2]:
  - [3]: 1,2,1
  - [3]: 10,20,10
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 34 | 34 | 27 | -7 | -20.59% |
| `cl100k_base` | 34 | 34 | 27 | -7 | -20.59% |

#### manual.advanced.table-009

```
# input
jq 'def addvalue(f): f as $x | map(. + $x); addvalue(.[0])'

# jq
[
  [
    1,
    2,
    1,
    2
  ],
  [
    10,
    20,
    1,
    2
  ]
]

# tq -o json
[
  [
    1,
    2,
    1,
    2
  ],
  [
    10,
    20,
    1,
    2
  ]
]

# tq
[2]:
  - [4]: 1,2,1,2
  - [4]: 10,20,1,2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 42 | 42 | 31 | -11 | -26.19% |
| `cl100k_base` | 42 | 42 | 31 | -11 | -26.19% |

#### manual.advanced.table-010

```
# input
jq 'isempty(empty)'

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

#### manual.advanced.table-011

```
# input
jq 'isempty(.[])'

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

#### manual.advanced.table-012

```
# input
jq 'isempty(.[])'

# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.advanced.table-013

```
# input
jq '[limit(3; .[])]'

# jq
[
  0,
  1,
  2
]

# tq -o json
[
  0,
  1,
  2
]

# tq
[3]: 0,1,2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual.advanced.table-014

```
# input
jq '[skip(3; .[])]'

# jq
[
  3,
  4,
  5,
  6,
  7,
  8,
  9
]

# tq -o json
[
  3,
  4,
  5,
  6,
  7,
  8,
  9
]

# tq
[7]: 3,4,5,6,7,8,9
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 18 | -12 | -40% |
| `cl100k_base` | 30 | 30 | 18 | -12 | -40% |

#### manual.advanced.table-015

```
# input
jq '[first(range(.)), last(range(.)), nth(5; range(.))]'

# jq
[
  0,
  9,
  5
]

# tq -o json
[
  0,
  9,
  5
]

# tq
[3]: 0,9,5
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual.advanced.table-016

```
# input
jq '[first(empty), last(empty), nth(5; empty)]'

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

#### manual.advanced.table-017

```
# input
jq '[range(.)]|[first, last, nth(5)]'

# jq
[
  0,
  9,
  5
]

# tq -o json
[
  0,
  9,
  5
]

# tq
[3]: 0,9,5
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual.advanced.table-018

```
# input
jq 'reduce .[] as $item (0; . + $item)'

# jq
15

# tq -o json
15

# tq
15
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.advanced.table-019

```
# input
jq 'reduce .[] as [$i,$j] (0; . + $i * $j)'

# jq
44

# tq -o json
44

# tq
44
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.advanced.table-020

```
# input
jq 'reduce .[] as {$x,$y} (null; .x += $x | .y += [$y])'

# jq
{
  "x": "abc",
  "y": [
    1,
    2,
    3
  ]
}

# tq -o json
{
  "x": "abc",
  "y": [
    1,
    2,
    3
  ]
}

# tq
x: abc
y[3]: 1,2,3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 28 | 28 | 15 | -13 | -46.43% |
| `cl100k_base` | 28 | 28 | 15 | -13 | -46.43% |

#### manual.advanced.table-021

```
# input
jq 'foreach .[] as $item (0; . + $item)'

# jq
1
3
6
10
15

# tq -o json
1
3
6
10
15

# tq
1
3
6
10
15
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 10 | 0 | +0% |
| `cl100k_base` | 10 | 10 | 10 | 0 | +0% |

#### manual.advanced.table-022

```
# input
jq 'foreach .[] as $item (0; . + $item; [$item, . * 2])'

# jq
[
  1,
  2
]
[
  2,
  6
]
[
  3,
  12
]
[
  4,
  20
]
[
  5,
  30
]

# tq -o json
[
  1,
  2
]
[
  2,
  6
]
[
  3,
  12
]
[
  4,
  20
]
[
  5,
  30
]

# tq
[2]: 1,2
[2]: 2,6
[2]: 3,12
[2]: 4,20
[2]: 5,30
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 50 | 50 | 40 | -10 | -20% |
| `cl100k_base` | 50 | 50 | 40 | -10 | -20% |

#### manual.advanced.table-023

```
# input
jq 'foreach .[] as $item (0; . + 1; {index: ., $item})'

# jq
{
  "index": 1,
  "item": "foo"
}
{
  "index": 2,
  "item": "bar"
}
{
  "index": 3,
  "item": "baz"
}

# tq -o json
{
  "index": 1,
  "item": "foo"
}
{
  "index": 2,
  "item": "bar"
}
{
  "index": 3,
  "item": "baz"
}

# tq
index: 1
item: foo
index: 2
item: bar
index: 3
item: baz
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 48 | 48 | 27 | -21 | -43.75% |
| `cl100k_base` | 48 | 48 | 27 | -21 | -43.75% |

#### manual.advanced.table-024

```
# input
jq 'def range(init; upto; by): def _range: if (by > 0 and . < upto) or (by < 0 and . > upto) then ., ((.+by)|_range) else empty end; if init == upto then empty elif by == 0 then init else init|_range end; range(0; 10; 3)'

# jq
0
3
6
9

# tq -o json
0
3
6
9

# tq
0
3
6
9
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 8 | 0 | +0% |
| `cl100k_base` | 8 | 8 | 8 | 0 | +0% |

#### manual.advanced.table-025

```
# input
jq 'def while(cond; update): def _while: if cond then ., (update | _while) else empty end; _while; [while(.<100; .*2)]'

# jq
[
  1,
  2,
  4,
  8,
  16,
  32,
  64
]

# tq -o json
[
  1,
  2,
  4,
  8,
  16,
  32,
  64
]

# tq
[7]: 1,2,4,8,16,32,64
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 18 | -12 | -40% |
| `cl100k_base` | 30 | 30 | 18 | -12 | -40% |

#### manual.basic.iterator-prose-array

```
# input
jq '.[]'

# jq
1
2
3

# tq -o json
1
2
3

# tq
1
2
3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 | +0% |
| `cl100k_base` | 6 | 6 | 6 | 0 | +0% |

<!-- tq-manual-compare:end -->
