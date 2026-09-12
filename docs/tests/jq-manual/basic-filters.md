---
type: Report
title: "Basic filters manual audit"
description: "Recorded review of Basic filters manual audit."
generated: { by: codex/gpt-6, at: 2026-09-10T02:12:04Z }
---

# Basic filters manual audit

Source: [pinned jq manual source inventory](../../../tests/compatibility/reviews/jq-manual/source-examples.toon), jq 1.8 manual. The machine-readable ledger is [basic-filters.toon](../../../tests/compatibility/reviews/jq-manual/basic-filters.toon). `new` means an exact executable query/input case was added to [manual-basic-filters.jsonl](../../../tests/compatibility/cases/manual-basic-filters.jsonl); `non-executable` means the prose or syntax fragment supplies no concrete fixture/output; `gap` means the fragment is runnable but the source supplies no fixture.

Counts: 51 new, 4 covered, 0 gaps, 3 non-executable, 58 examples inventoried. No pre-existing case covered a table example; 4 prose examples reuse exact table cases. The `have_decnum` cases keep `tq` enabled so incompatibilities remain visible. The reference binary is a local jq 1.8.1 download in the ignored build directory.

## Identity: `.` (lines 16–82)

| Line | Query | Input | Case ID | Status / reason |
| ---: | --- | --- | --- | --- |
| 18 | `.` | `null` (synthesized) | `manual.basic.prose-identity` | new — synthesized fixture for runnable prose filter |
| 20 | `.` | `{\"hello\":\"world\"}` (synthesized) | `manual.basic.prose-pretty-print` | new — synthesized fixture for runnable prose filter |
| 24 | `1E1234567890 \| .` | `null` (synthesized) | `manual.basic.identity-large-exponent` | new — concrete fenced example |
| 42 | `.` | `"Hello, world!"` | `manual.basic.identity-hello` | new — exact table query/input |
| 48 | `.` | `0.12345678901234567890123456789` | `manual.basic.identity-decimal` | new — exact table query/input |
| 54 | `[., tojson] == if have_decnum then [12345678909876543212345,"12345678909876543212345"] else [12345678909876543000000,"12345678909876543000000"] end` | `12345678909876543212345` | `manual.basic.identity-have-decnum-positive` | new — exact table query/input |
| 60 | `[1234567890987654321,-1234567890987654321 \| tojson] == if have_decnum then ["1234567890987654321","-1234567890987654321"] else ["1234567890987654400","-1234567890987654400"] end` | `null` | `manual.basic.identity-have-decnum-negative` | new — exact table query/input |
| 66 | `. < 0.12345678901234567890123456788` | `0.12345678901234567890123456789` | `manual.basic.identity-precision-compare` | new — exact table query/input |
| 72 | `map([., . == 1]) \| tojson == if have_decnum then "[[1,true],[1.000,true],[1.0,true],[1.00,true]]" else "[[1,true],[1,true],[1,true],[1,true]]" end` | `[1, 1.000, 1.0, 100e-2]` | `manual.basic.identity-decimal-forms` | new — exact table query/input |
| 78 | `. as $big \| [$big, $big + 1] \| map(. > 10000000000000000000000000000000) \| . == if have_decnum then [true, false] else [false, false] end` | `10000000000000000000000000000001` | `manual.basic.identity-big-comparison` | new — exact table query/input |

## Object Identifier-Index: `.foo`, `.foo.bar` (lines 84–112)

| Line | Query | Input | Case ID | Status / reason |
| ---: | --- | --- | --- | --- |
| 86 | `.foo` | `{\"foo\":42}` (synthesized) | `manual.basic.prose-object-field` | new — synthesized fixture for runnable prose filter |
| 88 | `.foo.bar` | `{\"foo\":{\"bar\":42}}` (synthesized) | `manual.basic.prose-object-chain` | new — synthesized fixture for runnable prose filter |
| 88 | `.foo \| .bar` | `{\"foo\":{\"bar\":42}}` (synthesized) | `manual.basic.prose-object-chain-pipe` | new — synthesized fixture for runnable prose filter |
| 92 | `.\"foo$\"` | `{\"foo$\":42}` (synthesized) | `manual.basic.prose-special-key-identifier` | new — synthesized fixture for runnable prose filter |
| 92 | `.[\"foo$\"]` | `{\"foo$\":42}` (synthesized) | `manual.basic.prose-special-key-index` | new — synthesized fixture for runnable prose filter |
| 94 | `.[\"foo::bar\"]` | `{\"foo::bar\":42}` (synthesized) | `manual.basic.prose-colon-key` | new — synthesized fixture for runnable prose filter |
| 94 | `.[\"foo.bar\"]` | `{\"foo.bar\":42}` (synthesized) | `manual.basic.prose-dot-key` | new — synthesized fixture for runnable prose filter |
| 94 | `.foo::bar` | `{"foo::bar":42}` | `manual.basic.invalid-colon-key` | new, compile-error case with a synthesized fixture |
| 96 | `.foo` | `{\"foo\": 42, \"bar\": \"less interesting data\"}` | `manual.basic.object-field-present` | new — exact table query/input |
| 102 | `.foo` | `{\"notfoo\": true, \"alsonotfoo\": false}` | `manual.basic.object-field-missing` | new — exact table query/input |
| 108 | `.[\"foo\"]` | `{\"foo\": 42}` | `manual.basic.object-computed-field` | new — exact table query/input |

## Optional Object Identifier-Index: `.foo?` (lines 114–140)

| Line | Query | Input | Case ID | Status / reason |
| ---: | --- | --- | --- | --- |
| 118 | `.foo?` | `{\"foo\": 42, \"bar\": \"less interesting data\"}` | `manual.basic.optional-field-present` | new — exact table query/input |
| 124 | `.foo?` | `{\"notfoo\": true, \"alsonotfoo\": false}` | `manual.basic.optional-field-missing` | new — exact table query/input |
| 130 | `.[\"foo\"]?` | `{\"foo\": 42}` | `manual.basic.optional-computed-field` | new — exact table query/input |
| 136 | `[.foo?]` | `[1,2]` | `manual.basic.optional-array-field` | new — exact table query/input |

## Object Index / Array Index (lines 142–168)

| Line | Query | Input | Case ID | Status / reason |
| ---: | --- | --- | --- | --- |
| 144 | `.[\"foo\"]` | `{\"foo\":42}` (synthesized) | `manual.basic.object-computed-field` | covered — exact query/input is in table case |
| 148 | `.[<number>]` | — | — | non-executable — placeholder syntax is not runnable |
| 148 | `.[2]` | `[0,1,2]` (synthesized) | `manual.basic.prose-array-index-third` | new — synthesized fixture for runnable prose filter |
| 150 | `.[-1]` | `[1,2,3]` (synthesized) | `manual.basic.prose-array-index-last` | new — synthesized fixture for runnable prose filter |
| 150 | `.[-2]` | `[1,2,3]` (synthesized) | `manual.basic.array-index-negative` | covered — exact query/input is in table case |
| 152 | `.[0]` | `[{\"name\":\"JSON\", \"good\":true}, {\"name\":\"XML\", \"good\":false}]` | `manual.basic.array-index-zero` | new — exact table query/input |
| 158 | `.[2]` | `[{\"name\":\"JSON\", \"good\":true}, {\"name\":\"XML\", \"good\":false}]` | `manual.basic.array-index-out-of-range` | new — exact table query/input |
| 164 | `.[-2]` | `[1,2,3]` | `manual.basic.array-index-negative` | new — exact table query/input |

## Array/String Slice (lines 170–196)

| Line | Query | Input | Case ID | Status / reason |
| ---: | --- | --- | --- | --- |
| 172 | `.[<number>:<number>]` | — | — | non-executable — placeholder syntax is not runnable |
| 172 | `.[10:15]` | `[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19]` (synthesized) | `manual.basic.prose-slice-example` | new — synthesized fixture for runnable prose filter |
| 174 | `.[2:4]` | `[\"a\",\"b\",\"c\",\"d\",\"e\"]` | `manual.basic.slice-array-middle` | new — exact table query/input |
| 180 | `.[2:4]` | `\"abcdefghi\"` | `manual.basic.slice-string-middle` | new — exact table query/input |
| 186 | `.[:3]` | `[\"a\",\"b\",\"c\",\"d\",\"e\"]` | `manual.basic.slice-array-prefix` | new — exact table query/input |
| 192 | `.[-2:]` | `[\"a\",\"b\",\"c\",\"d\",\"e\"]` | `manual.basic.slice-array-suffix` | new — exact table query/input |

## Array/Object Value Iterator: `.[]` (lines 198–236)

| Line | Query | Input | Case ID | Status / reason |
| ---: | --- | --- | --- | --- |
| 200 | `.[]` | `[1,2,3]` | `manual.basic.iterator-prose-array` | new — concrete prose example |
| 200 | `.foo[]` | `{\"foo\":[1,2,3]}` (synthesized) | `manual.basic.iterator-field-array` | covered — exact query/input is in table case |
| 202 | `.[]` | `{\"a\": 1, \"b\": 1}` (synthesized) | `manual.basic.iterator-object` | covered — exact query/input is in table case |
| 206 | `.[]` | `[{\"name\":\"JSON\", \"good\":true}, {\"name\":\"XML\", \"good\":false}]` | `manual.basic.iterator-array` | new — exact table query/input |
| 213 | `.[]` | `[]` | `manual.basic.iterator-empty-array` | new — exact table query/input |
| 219 | `.foo[]` | `{\"foo\":[1,2,3]}` | `manual.basic.iterator-field-array` | new — exact table query/input |
| 227 | `.[]` | `{\"a\": 1, \"b\": 1}` | `manual.basic.iterator-object` | new — exact table query/input |

## Comma: `,` (lines 238–269)

| Line | Query | Input | Case ID | Status / reason |
| ---: | --- | --- | --- | --- |
| 241 | `.foo,\\n.bar` | `{\"foo\":1,\"bar\":2}` (synthesized) | `manual.basic.prose-comma-code` | new — synthesized fixture for runnable fenced fragment |
| 249 | `.foo, .bar` | `{\"foo\": 42, \"bar\": \"something else\", \"baz\": true}` | `manual.basic.comma-fields` | new — exact table query/input |
| 256 | `.user, .projects[]` | `{\"user\":\"stedolan\", \"projects\": [\"jq\", \"wikiflow\"]}` | `manual.basic.comma-user-projects` | new — exact table query/input |
| 264 | `.[4,2]` | `[\"a\",\"b\",\"c\",\"d\",\"e\"]` | `manual.basic.comma-indexes` | new — exact table query/input |

## Pipe: `|` (lines 271–286)

| Line | Query | Input | Case ID | Status / reason |
| ---: | --- | --- | --- | --- |
| 275 | `.[] \| .foo` | `[{\"foo\":1},{\"foo\":2}]` (synthesized) | `manual.basic.prose-pipe-field` | new — synthesized fixture for runnable prose filter |
| 277 | `.a.b.c` | `{\"a\":{\"b\":{\"c\":42}}}` (synthesized) | `manual.basic.prose-pipe-chain` | new — synthesized fixture for runnable prose filter |
| 277 | `.a \| .b \| .c` | `{\"a\":{\"b\":{\"c\":42}}}` (synthesized) | `manual.basic.prose-pipe-chain-expanded` | new — synthesized fixture for runnable prose filter |
| 279 | `.a \| . \| .b` | `{\"a\":{\"b\":42}}` (synthesized) | `manual.basic.prose-pipe-identity` | new — synthesized fixture for runnable prose filter |
| 279 | `.a.b` | `{\"a\":{\"b\":42}}` (synthesized) | `manual.basic.prose-pipe-identity-equivalent` | new — synthesized fixture for runnable prose filter |
| 281 | `.[] \| .name` | `[{\"name\":\"JSON\", \"good\":true}, {\"name\":\"XML\", \"good\":false}]` | `manual.basic.pipe-name` | new — exact table query/input |

## Parenthesis (lines 288–295)

| Line | Query | Input | Case ID | Status / reason |
| ---: | --- | --- | --- | --- |
| 290 | grouping operator | — | — | non-executable — conceptual description has no concrete query/input |
| 292 | `(. + 2) * 5` | `1` | `manual.basic.parentheses-arithmetic` | new — exact table query/input |

## Harness limitations

The source tables are executable jq 1.8 examples, but the repository’s default `/usr/bin/jq` is Apple jq 1.7.1 and does not provide `have_decnum`; use `target/reference-build/jq/jq` (jq 1.8.1) for source baselines. The `have_decnum` cases mark `tq` supported so a compile/runtime difference is observable rather than hidden by an adapter skip. This audit does not claim that `tq` passes: the cases require a compatibility campaign execution by the parent suite.

I directly executed all 50 cases with `target/reference-build/jq/jq`, `yq` 4.53.2, and `target/debug/tq` using JSON input/output adapters. jq completed 50/50; yq completed 45/46 (the large-exponent identity overflows yq’s parser; four `have_decnum` cases are intentionally adapter-unsupported); tq completed 43/50. The seven tq errors are observable divergences: the large exponent exceeds tq’s configured exponent limit; four `have_decnum` queries report an unknown builtin; `.[4,2]` is rejected by tq’s index parser; and `.\"foo$\"` is rejected by tq’s parser. These are execution observations, not claims of parity.

<!-- tq-manual-compare:begin section=basic-filters -->
## Results

[Case collection](../../../tests/compatibility/reviews/jq-manual/basic-filters.toon)


| Verdict | Cases |
| --- | ---: |
| match | 51 |

Independent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.

| Output campaign | Matches | Cases |
| --- | ---: | ---: |
| compact_json | 51 | 51 |
| toon | 51 | 51 |

A match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from matches. Historical expected-difference labels do not pass either gate.

Missing features and unaccepted mismatches remain failures. Reference discrepancies describe errors in the imported manual, not successful compatibility.

JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.

### Output size

50 eligible examples. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with default LF-terminated `-o toon` results; explicitly requested `--seq -o toon` output is shown in the cases but excluded from size totals. Diff is TOON tokens minus JSON tokens. % is the signed percent difference `(TOON - JSON) / JSON`, so savings are negative and growth is positive.

| Tokenizer | JSON tokens | TOON tokens | Diff | % |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 254 | 290 | 36 | +14.17% |
| `cl100k_base` | 253 | 290 | 37 | +14.62% |

Only successful jq/JSON/TOON-equivalent results enter the totals. A negative `Diff` means TOON uses fewer tokens; `%` is negative for savings and positive for growth. The manual is a correctness corpus, not a representative workload benchmark.

### Reviewed disparities and historical differences

| Case | Reason |
| --- | --- |

### Cases

Each case shows the original jq invocation, then the complete jq, tq JSON, and tq TOON output. Control bytes use `\xNN` escapes so record separators remain visible. This section is generated by the separate `tq-manual-compare` command.

#### manual.basic.array-index-negative

```
# input
jq '.[-2]'

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

#### manual.basic.array-index-out-of-range

```
# input
jq '.[2]'

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

#### manual.basic.array-index-zero

```
# input
jq '.[0]'

# jq
{
  "name": "JSON",
  "good": true
}

# tq -o json
{
  "name": "JSON",
  "good": true
}

# tq
name: JSON
good: true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 15 | 15 | 8 | -7 | -46.67% |
| `cl100k_base` | 15 | 15 | 8 | -7 | -46.67% |

#### manual.basic.comma-fields

```
# input
jq '.foo, .bar'

# jq
42
"something else"

# tq -o json
42
"something else"

# tq
42
something else
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 | -16.67% |
| `cl100k_base` | 6 | 6 | 5 | -1 | -16.67% |

#### manual.basic.comma-indexes

```
# input
jq '.[4,2]'

# jq
"e"
"c"

# tq -o json
"e"
"c"

# tq
e
c
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 4 | -2 | -33.33% |
| `cl100k_base` | 5 | 5 | 4 | -1 | -20% |

#### manual.basic.comma-user-projects

```
# input
jq '.user, .projects[]'

# jq
"stedolan"
"jq"
"wikiflow"

# tq -o json
"stedolan"
"jq"
"wikiflow"

# tq
stedolan
jq
wikiflow
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 9 | -3 | -25% |
| `cl100k_base` | 12 | 12 | 9 | -3 | -25% |

#### manual.basic.identity-big-comparison

```
# input
jq '. as $big | [$big, $big + 1] | map(. > 10000000000000000000000000000000) | . == if have_decnum then [true, false] else [false, false] end'

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

#### manual.basic.identity-decimal

```
# input
jq .

# jq
0.12345678901234567890123456789

# tq -o json
0.12345678901234567890123456789

# tq
0.12345678901234567890123456789
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 13 | 13 | 13 | 0 | +0% |
| `cl100k_base` | 13 | 13 | 13 | 0 | +0% |

#### manual.basic.identity-decimal-forms

```
# input
jq 'map([., . == 1]) | tojson == if have_decnum then "[[1,true],[1.000,true],[1.0,true],[1.00,true]]" else "[[1,true],[1,true],[1,true],[1,true]]" end'

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

#### manual.basic.identity-have-decnum-negative

```
# input
jq '[1234567890987654321,-1234567890987654321 | tojson] == if have_decnum then ["1234567890987654321","-1234567890987654321"] else ["1234567890987654400","-1234567890987654400"] end'

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

#### manual.basic.identity-have-decnum-positive

```
# input
jq '[., tojson] == if have_decnum then [12345678909876543212345,"12345678909876543212345"] else [12345678909876543000000,"12345678909876543000000"] end'

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

#### manual.basic.identity-hello

```
# input
jq .

# jq
"Hello, world!"

# tq -o json
"Hello, world!"

# tq
"Hello, world!"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 5 | 0 | +0% |
| `cl100k_base` | 5 | 5 | 5 | 0 | +0% |

#### manual.basic.identity-large-exponent

```
# input
jq '1E1234567890 | .'

# jq
1.7976931348623157e+308

# tq -o json
1.7976931348623157e+308

# tq
179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 104 | +92 | +766.67% |
| `cl100k_base` | 12 | 12 | 104 | +92 | +766.67% |

#### manual.basic.identity-precision-compare

```
# input
jq '. < 0.12345678901234567890123456788'

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

#### manual.basic.invalid-colon-key

```
# input
jq .foo::bar

# jq


[stderr]
jq: error: syntax error, unexpected ':', expecting end of file at <top-level>, line 1, column 5:
    .foo::bar
        ^
jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-PARSE-FIELD-001: field names cannot use namespace separators

# tq


[stderr]
tq: query compilation failed: TQ-PARSE-FIELD-001: field names cannot use namespace separators
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 | n/a |
| `cl100k_base` | 0 | 0 | 0 | 0 | n/a |

#### manual.basic.iterator-array

```
# input
jq '.[]'

# jq
{
  "name": "JSON",
  "good": true
}
{
  "name": "XML",
  "good": false
}

# tq -o json
{
  "name": "JSON",
  "good": true
}
{
  "name": "XML",
  "good": false
}

# tq
name: JSON
good: true
name: XML
good: false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 16 | -14 | -46.67% |
| `cl100k_base` | 30 | 30 | 16 | -14 | -46.67% |

#### manual.basic.iterator-empty-array

```
# input
jq '.[]'

# jq

# tq -o json

# tq
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 | n/a |
| `cl100k_base` | 0 | 0 | 0 | 0 | n/a |

#### manual.basic.iterator-field-array

```
# input
jq '.foo[]'

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

#### manual.basic.iterator-object

```
# input
jq '.[]'

# jq
1
1

# tq -o json
1
1

# tq
1
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

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

#### manual.basic.object-computed-field

```
# input
jq '.["foo"]'

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

#### manual.basic.object-field-missing

```
# input
jq .foo

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

#### manual.basic.object-field-present

```
# input
jq .foo

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

#### manual.basic.optional-array-field

```
# input
jq '[.foo?]'

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

#### manual.basic.optional-computed-field

```
# input
jq '.["foo"]?'

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

#### manual.basic.optional-field-missing

```
# input
jq '.foo?'

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

#### manual.basic.optional-field-present

```
# input
jq '.foo?'

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

#### manual.basic.parentheses-arithmetic

```
# input
jq '(. + 2) * 5'

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

#### manual.basic.pipe-name

```
# input
jq '.[] | .name'

# jq
"JSON"
"XML"

# tq -o json
"JSON"
"XML"

# tq
JSON
XML
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 4 | -2 | -33.33% |
| `cl100k_base` | 6 | 6 | 4 | -2 | -33.33% |

#### manual.basic.prose-array-index-last

```
# input
jq '.[-1]'

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

#### manual.basic.prose-array-index-third

```
# input
jq '.[2]'

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

#### manual.basic.prose-colon-key

```
# input
jq '.["foo::bar"]'

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

#### manual.basic.prose-comma-code

```
# input
jq '.foo,
.bar'

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

#### manual.basic.prose-dot-key

```
# input
jq '.["foo.bar"]'

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

#### manual.basic.prose-identity

```
# input
jq .

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

#### manual.basic.prose-object-chain

```
# input
jq .foo.bar

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

#### manual.basic.prose-object-chain-pipe

```
# input
jq '.foo | .bar'

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

#### manual.basic.prose-object-field

```
# input
jq .foo

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

#### manual.basic.prose-pipe-chain

```
# input
jq .a.b.c

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

#### manual.basic.prose-pipe-chain-expanded

```
# input
jq '.a | .b | .c'

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

#### manual.basic.prose-pipe-field

```
# input
jq '.[] | .foo'

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

#### manual.basic.prose-pipe-identity

```
# input
jq '.a | . | .b'

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

#### manual.basic.prose-pipe-identity-equivalent

```
# input
jq .a.b

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

#### manual.basic.prose-pretty-print

```
# input
jq .

# jq
{
  "hello": "world"
}

# tq -o json
{
  "hello": "world"
}

# tq
hello: world
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 | -55.56% |
| `cl100k_base` | 9 | 9 | 4 | -5 | -55.56% |

#### manual.basic.prose-slice-example

```
# input
jq '.[10:15]'

# jq
[
  10,
  11,
  12,
  13,
  14
]

# tq -o json
[
  10,
  11,
  12,
  13,
  14
]

# tq
[5]: 10,11,12,13,14
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 14 | -8 | -36.36% |
| `cl100k_base` | 22 | 22 | 14 | -8 | -36.36% |

#### manual.basic.prose-special-key-identifier

```
# input
jq '."foo$"'

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

#### manual.basic.prose-special-key-index

```
# input
jq '.["foo$"]'

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

#### manual.basic.slice-array-middle

```
# input
jq '.[2:4]'

# jq
[
  "c",
  "d"
]

# tq -o json
[
  "c",
  "d"
]

# tq
[2]: c,d
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 6 | -4 | -40% |
| `cl100k_base` | 10 | 10 | 6 | -4 | -40% |

#### manual.basic.slice-array-prefix

```
# input
jq '.[:3]'

# jq
[
  "a",
  "b",
  "c"
]

# tq -o json
[
  "a",
  "b",
  "c"
]

# tq
[3]: a,b,c
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 7 | -7 | -50% |
| `cl100k_base` | 14 | 14 | 7 | -7 | -50% |

#### manual.basic.slice-array-suffix

```
# input
jq '.[-2:]'

# jq
[
  "d",
  "e"
]

# tq -o json
[
  "d",
  "e"
]

# tq
[2]: d,e
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 6 | -4 | -40% |
| `cl100k_base` | 10 | 10 | 6 | -4 | -40% |

#### manual.basic.slice-string-middle

```
# input
jq '.[2:4]'

# jq
"cd"

# tq -o json
"cd"

# tq
cd
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | 3 | 3 | 2 | -1 | -33.33% |

<!-- tq-manual-compare:end -->
