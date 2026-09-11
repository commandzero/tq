---
type: Report
title: "Conditionals and comparisons manual audit"
description: "Recorded review of Conditionals and comparisons manual audit."
generated: { by: codex/gpt-6, at: 2026-09-10T02:12:04Z }
---

# Conditionals and comparisons manual audit

Source: [pinned jq manual source inventory](../../../tests/compatibility/reviews/jq-manual/source-examples.toon), jq 1.8 manual. The machine-readable ledger is [conditionals-and-comparisons.toon](../../../tests/compatibility/reviews/jq-manual/conditionals-and-comparisons.toon). `new` means an executable case was added to [manual-conditionals-and-comparisons.jsonl](../../../tests/compatibility/cases/manual-conditionals-and-comparisons.jsonl); `covered` reuses an exact case; `non-executable` marks symbolic or incomplete source text.

Counts: 25 case records, covering 19 exact tables, 4 executable fences including the negative `break` fence, and 2 synthesized prose witnesses. The inventory has 40 examples: 25 new, 2 covered, and 13 non-executable. There are no unresolved gaps. The exact table queries and inputs are preserved. The `not` fence, `.foo // 1`, and generic `a // b` fence use synthesized inputs or literal substitutions where the manual gives no input.

| Line | Kind | Query | Input | Case IDs | Disposition |
| ---: | --- | --- | --- | --- | --- |
| 22 | table | `. == false` | `null` | `manual.condcomp.eq-false` | new |
| 28 | table | `. == {"b": {"d": (4 + 1e-20), "c": 3}, "a":1}` | `{"a":1, "b": {"c": 3, "d": 4}}` | `manual.condcomp.eq-object` | new |
| 34 | table | `.[] == 1` | `[1, 1.0, "1", "banana"]` | `manual.condcomp.eq-stream` | new |
| 55 | table | `if . == 0 then ... elif ... end` | `2` | `manual.condcomp.if-elif` | new |
| 67 | table | `. < 5` | `2` | `manual.condcomp.compare-lt` | new |
| 80 | fenced | `.foo and .bar \| not` | synthesized object | `manual.condcomp.fence-not` | new |
| 88–110 | table | Boolean operators and `not` | `null` | `manual.condcomp.and-string`, `manual.condcomp.or-stream`, `manual.condcomp.and-stream`, `manual.condcomp.not-array` | new |
| 121 | fenced | `1 // 2` | `null` | `manual.condcomp.prose-default` | new, symbolic operands replaced |
| 127–130 | prose/fenced | Alternative defaults and precedence | `null` / `{}` | `manual.condcomp.prose-default`, `manual.condcomp.prose-precedence`, `manual.condcomp.fence-precedence` | new |
| 136–160 | table | Alternative operator examples | `null` / object inputs | `manual.condcomp.alt-empty`, `manual.condcomp.alt-present`, `manual.condcomp.alt-missing`, `manual.condcomp.alt-generator`, `manual.condcomp.alt-pipe` | new |
| 174–186 | table | `try`/`catch` examples | `true` / array | `manual.condcomp.try-catch-type`, `manual.condcomp.try-catch-array`, `manual.condcomp.try-catch-error` | new |
| 198–217 | fenced | Repeat/label/reduce sketches | — | — | non-executable, incomplete placeholders |
| 223 | fenced | `break $out` | `null` | `manual.condcomp.fence-break-error` | new, compile error |
| 233–239 | table | Optional field and conversion operators | arrays | `manual.condcomp.optional-field`, `manual.condcomp.optional-tonumber` | new |

The remaining prose entries are recorded in the JSON ledger. They are semantic templates such as `if A then B else C end`, `try EXP`, and `EXP?`, so they remain non-executable until the manual supplies concrete operands and branches. The five fenced examples with ellipses or undefined symbolic functions are handled the same way.

<!-- tq-manual-compare:begin section=conditionals-and-comparisons -->
## Results

[Case collection](../../../tests/compatibility/reviews/jq-manual/conditionals-and-comparisons.toon)


| Verdict | Cases |
| --- | ---: |
| match | 25 |

Independent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.

| Output campaign | Matches | Cases |
| --- | ---: | ---: |
| compact_json | 25 | 25 |
| toon | 25 | 25 |

A match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from matches. Historical expected-difference labels do not pass either gate.

Missing features and unaccepted mismatches remain failures. Reference discrepancies describe errors in the imported manual, not successful compatibility.

JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.

### Output size

24 eligible examples. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with default LF-terminated `-o toon` results; explicitly requested `--seq -o toon` output is shown in the cases but excluded from size totals. Diff is TOON tokens minus JSON tokens. % is the signed percent difference `(TOON - JSON) / JSON`, so savings are negative and growth is positive.

| Tokenizer | JSON tokens | TOON tokens | Diff | % |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 109 | 97 | -12 | -11.01% |
| `cl100k_base` | 109 | 97 | -12 | -11.01% |

Only successful jq/JSON/TOON-equivalent results enter the totals. A negative `Diff` means TOON uses fewer tokens; `%` is negative for savings and positive for growth. The manual is a correctness corpus, not a representative workload benchmark.

### Reviewed disparities and historical differences

| Case | Reason |
| --- | --- |

### Cases

Each case shows the original jq invocation, then the complete jq, tq JSON, and tq TOON output. Control bytes use `\xNN` escapes so record separators remain visible. This section is generated by the separate `tq-manual-compare` command.

#### manual.condcomp.alt-empty

```
# input
jq 'empty // 42'

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

#### manual.condcomp.alt-generator

```
# input
jq '(false, null, 1) // 42'

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

#### manual.condcomp.alt-missing

```
# input
jq '.foo // 42'

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

#### manual.condcomp.alt-pipe

```
# input
jq '(false, null, 1) | . // 42'

# jq
42
42
1

# tq -o json
42
42
1

# tq
42
42
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 | +0% |
| `cl100k_base` | 6 | 6 | 6 | 0 | +0% |

#### manual.condcomp.alt-present

```
# input
jq '.foo // 42'

# jq
19

# tq -o json
19

# tq
19
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.condcomp.and-stream

```
# input
jq '(true, true) and (true, false)'

# jq
true
false
true
false

# tq -o json
true
false
true
false

# tq
true
false
true
false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 8 | 0 | +0% |
| `cl100k_base` | 8 | 8 | 8 | 0 | +0% |

#### manual.condcomp.and-string

```
# input
jq '42 and "a string"'

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

#### manual.condcomp.compare-lt

```
# input
jq '. < 5'

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

#### manual.condcomp.eq-false

```
# input
jq '. == false'

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

#### manual.condcomp.eq-object

```
# input
jq '. == {"b": {"d": (4 + 1e-20), "c": 3}, "a":1}'

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

#### manual.condcomp.eq-stream

```
# input
jq '.[] == 1'

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

#### manual.condcomp.fence-break-error

```
# input
jq 'break $out'

# jq


[stderr]
jq: error: $*label-out is not defined at <top-level>, line 1, column 1:
    break $out
    ^^^^^^^^^^
jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-RESOLVE-LABEL-001: unknown label $out

# tq


[stderr]
tq: query compilation failed: TQ-RESOLVE-LABEL-001: unknown label $out
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 | n/a |
| `cl100k_base` | 0 | 0 | 0 | 0 | n/a |

#### manual.condcomp.fence-generic

```
# input
jq '1
// 2'

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

#### manual.condcomp.fence-not

```
# input
jq '.foo and .bar |
not'

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

#### manual.condcomp.fence-precedence

```
# input
jq 'false,
(1 // 2)'

# jq
false
1

# tq -o json
false
1

# tq
false
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### manual.condcomp.if-elif

```
# input
jq 'if . == 0 then
  "zero"
elif . == 1 then
  "one"
else
  "many"
end'

# jq
"many"

# tq -o json
"many"

# tq
many
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | 3 | 3 | 2 | -1 | -33.33% |

#### manual.condcomp.not-array

```
# input
jq '[true, false | not]'

# jq
[
  false,
  true
]

# tq -o json
[
  false,
  true
]

# tq
[2]: false,true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 6 | -2 | -25% |
| `cl100k_base` | 8 | 8 | 6 | -2 | -25% |

#### manual.condcomp.optional-field

```
# input
jq '[.[] | .a?]'

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

#### manual.condcomp.optional-tonumber

```
# input
jq '[.[] | tonumber?]'

# jq
[
  1,
  3,
  4
]

# tq -o json
[
  1,
  3,
  4
]

# tq
[3]: 1,3,4
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual.condcomp.or-stream

```
# input
jq '(true, false) or false'

# jq
true
false

# tq -o json
true
false

# tq
true
false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### manual.condcomp.prose-default

```
# input
jq '.foo // 1'

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

#### manual.condcomp.prose-precedence

```
# input
jq 'false, 1 // 2'

# jq
false
1

# tq -o json
false
1

# tq
false
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### manual.condcomp.try-catch-array

```
# input
jq '[.[]|try .a]'

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

#### manual.condcomp.try-catch-error

```
# input
jq 'try error("some exception") catch .'

# jq
"some exception"

# tq -o json
"some exception"

# tq
some exception
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 3 | -1 | -25% |
| `cl100k_base` | 4 | 4 | 3 | -1 | -25% |

#### manual.condcomp.try-catch-type

```
# input
jq 'try .a catch ". is not an object"'

# jq
". is not an object"

# tq -o json
". is not an object"

# tq
. is not an object
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 | +0% |
| `cl100k_base` | 6 | 6 | 6 | 0 | +0% |

<!-- tq-manual-compare:end -->
