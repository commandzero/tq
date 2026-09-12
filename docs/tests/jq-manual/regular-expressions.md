---
type: Report
title: "Regular expressions coverage audit"
description: "Recorded review of Regular expressions coverage audit."
generated: { by: codex/gpt-6, at: 2026-09-10T02:12:04Z }
---

# Regular expressions coverage audit

Source: [jq manual, Regular expressions](https://jqlang.org/manual/#regular-expressions). See the [machine-readable inventory](../../../tests/compatibility/reviews/jq-manual/regular-expressions.toon).

All 17 tables are covered by exact `mvp` cases. Two of the three fences are runnable and have cases; the first fence is grammar-only and remains non-executable. Every runnable case marks jq and tq as supported so any result or diagnostic difference remains visible to the compatibility runner.

8 additional representative cases exercise the m/p/s/l flags and array argument
forms for test, match, and capture. The grammar fence links to those witnesses.
The JSON ledger records their inputs and source locations.

| Source line | Example | Query / input | Case ID | Disposition |
| ---: | --- | --- | --- | --- |
| 66 | `regex.table-001` | `test("foo")` / `"foo"` | `manual.regex.table-001` | new |
| 72 | `regex.table-002` | `.[] \| test("a b c # spaces are ignored"; "ix")` / array input | `manual.regex.table-002` | new |
| 97 | `regex.table-003` | `match("(abc)+"; "g")` / `"abc abc"` | `manual.regex.table-003` | new |
| 104 | `regex.table-004` | `match("foo")` / `"foo bar foo"` | `manual.regex.table-004` | new |
| 110 | `regex.table-005` | `match(["foo", "ig"])` / `"foo bar FOO"` | `manual.regex.table-005` | new |
| 117 | `regex.table-006` | optional capture / input retains two spaces | `manual.regex.table-006` | new |
| 124 | `regex.table-007` | `[ match("."; "g")] \| length` / `"abc"` | `manual.regex.table-007` | new |
| 134 | `regex.table-008` | `capture(...)` / `"xyzzy-14"` | `manual.regex.table-008` | new |
| 144 | `regex.table-009` | `scan("c")` / `"abcdefabc"` | `manual.regex.table-009` | new |
| 151 | `regex.table-010` | `scan("(a+)(b+)")` / `"abaabbaaabbb"` | `manual.regex.table-010` | new |
| 165 | `regex.table-011` | `split(", *"; null)` / `"ab,cd, ef"` | `manual.regex.table-011` | new |
| 175 | `regex.table-012` | `splits(", *")` / `"ab,cd,   ef, gh"` | `manual.regex.table-012` | new |
| 184 | `regex.table-013` | `splits(",? *"; "n")` / `"ab,cd ef,  gh"` | `manual.regex.table-013` | new |
| 197 | `regex.table-014` | `sub(...)` / `"123abc456def"` | `manual.regex.table-014` | new |
| 203 | `regex.table-015` | substitution stream / `"aB"` | `manual.regex.table-015` | new |
| 213 | `regex.table-016` | `gsub(...)` / `"Abcabc"` | `manual.regex.table-016` | new |
| 219 | `regex.table-017` | substitution stream / `"p"` | `manual.regex.table-017` | new |
| 22 | `regex.fence-pattern-shapes` | `STRING \| FILTER(...)` placeholders | — | non-executable |
| 50 | `regex.fence-whitespace-extended` | `"a b" \| test("a\\sb"; "x")` / null input | `manual.regex.fence-whitespace-extended` | new |
| 56 | `regex.fence-inline-flags` | inline flag scope / null input | `manual.regex.fence-inline-flags` | new |

The inventory is source coverage, not a claim that tq already matches jq. Parent-level build, schema, and campaign checks remain to be run.

<!-- tq-manual-compare:begin section=regular-expressions -->
## Results

[Case collection](../../../tests/compatibility/reviews/jq-manual/regular-expressions.toon)


| Verdict | Cases |
| --- | ---: |
| failure | 1 |
| match | 27 |

Independent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.

| Output campaign | Matches | Cases |
| --- | ---: | ---: |
| compact_json | 27 | 28 |
| toon | 28 | 28 |

A match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from matches. Historical expected-difference labels do not pass either gate.

Missing features and unaccepted mismatches remain failures. Reference discrepancies describe errors in the imported manual, not successful compatibility.

JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.

### Output size

27 eligible examples. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with default LF-terminated `-o toon` results; explicitly requested `--seq -o toon` output is shown in the cases but excluded from size totals. Diff is TOON tokens minus JSON tokens. % is the signed percent difference `(TOON - JSON) / JSON`, so savings are negative and growth is positive.

| Tokenizer | JSON tokens | TOON tokens | Diff | % |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 542 | 335 | -207 | -38.19% |
| `cl100k_base` | 540 | 335 | -205 | -37.96% |

Only successful jq/JSON/TOON-equivalent results enter the totals. A negative `Diff` means TOON uses fewer tokens; `%` is negative for savings and positive for growth. The manual is a correctness corpus, not a representative workload benchmark.

### Reviewed disparities and historical differences

| Case | Reason |
| --- | --- |

### Cases

Each case shows the original jq invocation, then the complete jq, tq JSON, and tq TOON output. Control bytes use `\xNN` escapes so record separators remain visible. This section is generated by the separate `tq-manual-compare` command.

#### manual.audit.regex.gsub-three-arguments

```
# input
jq 'gsub("p"; "x"; "g")'

# jq
"x"

# tq -o json
"x"

# tq
x
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.regex.array-capture

```
# input
jq 'capture(["(?<x>foo)"])'

# jq
{
  "x": "foo"
}

# tq -o json
{
  "x": "foo"
}

# tq
x: foo
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 | -55.56% |
| `cl100k_base` | 9 | 9 | 4 | -5 | -55.56% |

#### manual.regex.array-capture-flags

```
# input
jq 'capture(["(?<x>foo)", "i"])'

# jq
{
  "x": "FOO"
}

# tq -o json
{
  "x": "FOO"
}

# tq
x: FOO
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 5 | -5 | -50% |
| `cl100k_base` | 10 | 10 | 5 | -5 | -50% |

#### manual.regex.array-match

```
# input
jq 'match(["foo"])'

# jq
{
  "offset": 0,
  "length": 3,
  "string": "foo",
  "captures": []
}

# tq -o json
{
  "offset": 0,
  "length": 3,
  "string": "foo",
  "captures": []
}

# tq
offset: 0
length: 3
string: foo
captures[0]:
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 28 | 28 | 18 | -10 | -35.71% |
| `cl100k_base` | 28 | 28 | 18 | -10 | -35.71% |

#### manual.regex.array-test

```
# input
jq 'test(["foo"])'

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

#### manual.regex.fence-inline-flags

```
# input
jq -n '("test", "TEst", "teST", "TEST") | test("(?i)te(?-i)st")'

# jq
true
true
false
false

# tq -o json
true
true
false
false

# tq
true
true
false
false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 8 | 0 | +0% |
| `cl100k_base` | 8 | 8 | 8 | 0 | +0% |

#### manual.regex.fence-whitespace-extended

```
# input
jq -n '"a b" | test("a\\sb"; "x")'

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

#### manual.regex.flag-l

```
# input
jq 'match("a|ab"; "l")'

# jq
{
  "offset": 0,
  "length": 2,
  "string": "ab",
  "captures": []
}

# tq -o json


[stderr]
tq: bytecode operation is not executable in this language wave: regex flag 'l' (longest-match mode)

# tq


[stderr]
tq: bytecode operation is not executable in this language wave: regex flag 'l' (longest-match mode)
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 28 | 0 | 0 | 0 | n/a |
| `cl100k_base` | 28 | 0 | 0 | 0 | n/a |

#### manual.regex.flag-m

```
# input
jq 'test("a.b"; "m")'

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

#### manual.regex.flag-p

```
# input
jq '[test("a.b"; "p"), test("^b"; "p")]'

# jq
[
  true,
  false
]

# tq -o json
[
  true,
  false
]

# tq
[2]: true,false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 6 | -2 | -25% |
| `cl100k_base` | 8 | 8 | 6 | -2 | -25% |

#### manual.regex.flag-s

```
# input
jq 'test("^b"; "s")'

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

#### manual.regex.table-001

```
# input
jq 'test("foo")'

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

#### manual.regex.table-002

```
# input
jq '.[] | test("a b c # spaces are ignored"; "ix")'

# jq
true
true

# tq -o json
true
true

# tq
true
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### manual.regex.table-003

```
# input
jq 'match("(abc)+"; "g")'

# jq
{
  "offset": 0,
  "length": 3,
  "string": "abc",
  "captures": [
    {
      "offset": 0,
      "length": 3,
      "string": "abc",
      "name": null
    }
  ]
}
{
  "offset": 4,
  "length": 3,
  "string": "abc",
  "captures": [
    {
      "offset": 4,
      "length": 3,
      "string": "abc",
      "name": null
    }
  ]
}

# tq -o json
{
  "offset": 0,
  "length": 3,
  "string": "abc",
  "captures": [
    {
      "offset": 0,
      "length": 3,
      "string": "abc",
      "name": null
    }
  ]
}
{
  "offset": 4,
  "length": 3,
  "string": "abc",
  "captures": [
    {
      "offset": 4,
      "length": 3,
      "string": "abc",
      "name": null
    }
  ]
}

# tq
offset: 0
length: 3
string: abc
captures[1]{offset,length,string,name}:
  0,3,abc,null
offset: 4
length: 3
string: abc
captures[1]{offset,length,string,name}:
  4,3,abc,null
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 122 | 122 | 66 | -56 | -45.9% |
| `cl100k_base` | 122 | 122 | 66 | -56 | -45.9% |

#### manual.regex.table-004

```
# input
jq 'match("foo")'

# jq
{
  "offset": 0,
  "length": 3,
  "string": "foo",
  "captures": []
}

# tq -o json
{
  "offset": 0,
  "length": 3,
  "string": "foo",
  "captures": []
}

# tq
offset: 0
length: 3
string: foo
captures[0]:
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 28 | 28 | 18 | -10 | -35.71% |
| `cl100k_base` | 28 | 28 | 18 | -10 | -35.71% |

#### manual.regex.table-005

```
# input
jq 'match(["foo", "ig"])'

# jq
{
  "offset": 0,
  "length": 3,
  "string": "foo",
  "captures": []
}
{
  "offset": 8,
  "length": 3,
  "string": "FOO",
  "captures": []
}

# tq -o json
{
  "offset": 0,
  "length": 3,
  "string": "foo",
  "captures": []
}
{
  "offset": 8,
  "length": 3,
  "string": "FOO",
  "captures": []
}

# tq
offset: 0
length: 3
string: foo
captures[0]:
offset: 8
length: 3
string: FOO
captures[0]:
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 57 | 57 | 37 | -20 | -35.09% |
| `cl100k_base` | 57 | 57 | 37 | -20 | -35.09% |

#### manual.regex.table-006

```
# input
jq 'match("foo (?<bar123>bar)? foo"; "ig")'

# jq
{
  "offset": 0,
  "length": 11,
  "string": "foo bar foo",
  "captures": [
    {
      "offset": 4,
      "length": 3,
      "string": "bar",
      "name": "bar123"
    }
  ]
}
{
  "offset": 12,
  "length": 8,
  "string": "foo  foo",
  "captures": [
    {
      "offset": -1,
      "string": null,
      "length": 0,
      "name": "bar123"
    }
  ]
}

# tq -o json
{
  "offset": 0,
  "length": 11,
  "string": "foo bar foo",
  "captures": [
    {
      "offset": 4,
      "length": 3,
      "string": "bar",
      "name": "bar123"
    }
  ]
}
{
  "offset": 12,
  "length": 8,
  "string": "foo  foo",
  "captures": [
    {
      "offset": -1,
      "string": null,
      "length": 0,
      "name": "bar123"
    }
  ]
}

# tq
offset: 0
length: 11
string: foo bar foo
captures[1]{offset,length,string,name}:
  4,3,bar,bar123
offset: 12
length: 8
string: foo  foo
captures[1]{offset,string,length,name}:
  -1,null,0,bar123
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 129 | 129 | 73 | -56 | -43.41% |
| `cl100k_base` | 129 | 129 | 73 | -56 | -43.41% |

#### manual.regex.table-007

```
# input
jq '[ match("."; "g")] | length'

# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.regex.table-008

```
# input
jq 'capture("(?<a>[a-z]+)-(?<n>[0-9]+)")'

# jq
{
  "a": "xyzzy",
  "n": "14"
}

# tq -o json
{
  "a": "xyzzy",
  "n": "14"
}

# tq
a: xyzzy
n: "14"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 10 | -7 | -41.18% |
| `cl100k_base` | 17 | 17 | 10 | -7 | -41.18% |

#### manual.regex.table-009

```
# input
jq 'scan("c")'

# jq
"c"
"c"

# tq -o json
"c"
"c"

# tq
c
c
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 4 | -2 | -33.33% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### manual.regex.table-010

```
# input
jq 'scan("(a+)(b+)")'

# jq
[
  "a",
  "b"
]
[
  "aa",
  "bb"
]
[
  "aaa",
  "bbb"
]

# tq -o json
[
  "a",
  "b"
]
[
  "aa",
  "bb"
]
[
  "aaa",
  "bbb"
]

# tq
[2]: a,b
[2]: aa,bb
[2]: aaa,bbb
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 20 | -10 | -33.33% |
| `cl100k_base` | 30 | 30 | 20 | -10 | -33.33% |

#### manual.regex.table-011

```
# input
jq 'split(", *"; null)'

# jq
[
  "ab",
  "cd",
  "ef"
]

# tq -o json
[
  "ab",
  "cd",
  "ef"
]

# tq
[3]: ab,cd,ef
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 | -35.71% |
| `cl100k_base` | 14 | 14 | 9 | -5 | -35.71% |

#### manual.regex.table-012

```
# input
jq 'splits(", *")'

# jq
"ab"
"cd"
"ef"
"gh"

# tq -o json
"ab"
"cd"
"ef"
"gh"

# tq
ab
cd
ef
gh
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 8 | -4 | -33.33% |
| `cl100k_base` | 12 | 12 | 8 | -4 | -33.33% |

#### manual.regex.table-013

```
# input
jq 'splits(",? *"; "n")'

# jq
"ab"
"cd"
"ef"
"gh"

# tq -o json
"ab"
"cd"
"ef"
"gh"

# tq
ab
cd
ef
gh
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 8 | -4 | -33.33% |
| `cl100k_base` | 12 | 12 | 8 | -4 | -33.33% |

#### manual.regex.table-014

```
# input
jq 'sub("[^a-z]*(?<x>[a-z]+)"; "Z\(.x)"; "g")'

# jq
"ZabcZdef"

# tq -o json
"ZabcZdef"

# tq
ZabcZdef
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 | -16.67% |
| `cl100k_base` | 6 | 6 | 5 | -1 | -16.67% |

#### manual.regex.table-015

```
# input
jq '[sub("(?<a>.)"; "\(.a|ascii_upcase)", "\(.a|ascii_downcase)")]'

# jq
[
  "AB",
  "aB"
]

# tq -o json
[
  "AB",
  "aB"
]

# tq
[2]: AB,aB
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 11 | 11 | 7 | -4 | -36.36% |
| `cl100k_base` | 11 | 11 | 7 | -4 | -36.36% |

#### manual.regex.table-016

```
# input
jq 'gsub("(?<x>.)[^a]*"; "+\(.x)-")'

# jq
"+A-+a-"

# tq -o json
"+A-+a-"

# tq
+A-+a-
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 7 | 7 | 5 | -2 | -28.57% |
| `cl100k_base` | 7 | 7 | 5 | -2 | -28.57% |

#### manual.regex.table-017

```
# input
jq '[gsub("p"; "a", "b")]'

# jq
[
  "a",
  "b"
]

# tq -o json
[
  "a",
  "b"
]

# tq
[2]: a,b
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 6 | -4 | -40% |
| `cl100k_base` | 10 | 10 | 6 | -4 | -40% |

<!-- tq-manual-compare:end -->
