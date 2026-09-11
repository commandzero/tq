---
type: Report
title: "jq manual JSON compatibility and output size"
description: "Recorded review of jq manual JSON compatibility and output size."
generated: { by: codex/gpt-6, at: 2026-09-10T02:12:04Z }
---

# jq manual JSON compatibility and output size

905 cases. Original input fixture; structured cases independently compare jq with tq -o json, tq -o toon, and compact jq -c with tq -o json -c. Semantic JSON ignores presentation whitespace and object key order while preserving ordered results and exact decimal values. The compact campaign compares stdout bytes exactly; the TOON campaign uses standalone TOON for successful one-result cases and explicit --seq framing for original sequence adapters, failures or partial outputs, and references with zero or multiple results. Raw CLI cases retain their arguments and are assessed separately. Token totals use the o200k_base and cl100k_base encodings over successful one-result standalone TOON output; explicit sequence framing is retained for cardinality and error-output checks but excluded from size totals. Size totals include only successful jq/JSON/TOON-equivalent cases using default JSON encoder layouts, not compact JSON.

## Results

| Verdict | Cases |
| --- | ---: |
| disparity | 9 |
| match | 896 |

Independent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.

| Output campaign | Matches | Cases |
| --- | ---: | ---: |
| compact_json | 867 | 876 |
| toon | 876 | 876 |

A match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from matches. Historical expected-difference labels do not pass either gate.

Missing features and unaccepted mismatches remain failures. Reference discrepancies describe errors in the imported manual, not successful compatibility.

JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.

## Output size

714 eligible examples. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with standalone `-o toon` output for one result; explicit `--seq -o toon` output for zero or multiple results is shown in the cases but excluded from size totals.

| Tokenizer | JSON tokens | TOON tokens | Saved |
| --- | ---: | ---: | ---: |
| `o200k_base` | 7156 | 4677 | 2479 |
| `cl100k_base` | 7158 | 4686 | 2472 |

Only successful jq/JSON/TOON-equivalent results enter the totals. Negative savings mean TOON uses more tokens. The manual is a correctness corpus, not a representative workload benchmark.

## Reviewed disparities and historical differences

| Case | Reason |
| --- | --- |
| `manual.audit.math.exp-ulp` | Primary review: accept only x86_64 GNU/Linux exp(1), 1 binary64 ULP from pinned jq/glibc. Safe libm 0.2.16 is retained without FFI. The 92-input CLI probe observed at most 1 ULP for exp, not a global bound. Exponential results may differ in the last bit. Reconsider after implementation when dependencies, target, reference/runtime identities, or observations change. |
| `manual.audit.math.integer-scale-boundary` | Primary review: accept only this x86_64 GNU/Linux five-result ldexp/scalbln boundary witness. tq uses deterministic safe saturating exponent conversion and maps NaN to zero, instead of reproducing the C wrapper's out-of-range floating-to-integer conversion. This is not rounding: oversized and NaN exponents can produce finite maximum or 2 where this jq produces zero. Such scripts are not interchangeable. In-range finite exponents remain exact obligations. Reconsider after implementation if a portable defined contract or safe-library conversion justifies a change, and remeasure on changed targets/references. |
| `manual.audit.math.tgamma-ulp` | Primary review: accept only x86_64 GNU/Linux tgamma(0.5), 1 binary64 ULP from pinned jq/glibc. Safe libm 0.2.16 is retained without FFI. The 92-input CLI probe observed at most 2 ULP for tgamma, not a global bound or threshold. Gamma-function results may differ in the last bits. Reconsider after implementation when dependencies, target, reference/runtime identities, or observations change. |
| `manual.composition.arity.scalbln.2` | Primary review: accept only the called-definition wrapper around the same five ldexp/scalbln boundary expressions. Its jq/tq JSON, TOON, compact output, statuses, and diagnostics are identical to manual.audit.math.integer-scale-boundary in this final native campaign. This separately fingerprinted approval does not waive other composed calls. accept only this x86_64 GNU/Linux five-result ldexp/scalbln boundary witness. tq uses deterministic safe saturating exponent conversion and maps NaN to zero, instead of reproducing the C wrapper's out-of-range floating-to-integer conversion. This is not rounding: oversized and NaN exponents can produce finite maximum or 2 where this jq produces zero. Such scripts are not interchangeable. In-range finite exponents remain exact obligations. Reconsider after implementation if a portable defined contract or safe-library conversion justifies a change, and remeasure on changed targets/references. |
| `manual.composition.arity.y0.0` | Primary review: accept only the called-definition y0(1) witness. Its jq/tq JSON, TOON, compact output, statuses, and diagnostics are identical to manual.math.y0 in this final native campaign. This separately fingerprinted approval does not waive other composed calls. accept only x86_64 GNU/Linux y0(1), 1 binary64 ULP from pinned jq/glibc. Safe libm 0.2.16 is retained without FFI. The 92-input CLI probe includes 13 y0 inputs with at most 1 ULP observed, not a global bound. Bessel-function results may differ in the last bit. Reconsider after implementation when dependencies, target, reference/runtime identities, or observations change. |
| `manual.composition.arity.yn.2` | Primary review: accept only the called-definition yn(0;1) witness. Its jq/tq JSON, TOON, compact output, statuses, and diagnostics are identical to manual.math.yn in this final native campaign. This separately fingerprinted approval does not waive other composed calls. accept only x86_64 GNU/Linux yn(0;1), 1 binary64 ULP from pinned jq/glibc. Safe libm 0.2.16 is retained without FFI. The 92-input CLI probe includes 13 order-zero yn inputs with at most 1 ULP observed; no other order/input is approved. Bessel-function results may differ in the last bit. Reconsider after implementation when dependencies, target, reference/runtime identities, or observations change. |
| `manual.math.y0` | Primary review: accept only x86_64 GNU/Linux y0(1), 1 binary64 ULP from pinned jq/glibc. Safe libm 0.2.16 is retained without FFI. The 92-input CLI probe includes 13 y0 inputs with at most 1 ULP observed, not a global bound. Bessel-function results may differ in the last bit. Reconsider after implementation when dependencies, target, reference/runtime identities, or observations change. |
| `manual.math.yn` | Primary review: accept only x86_64 GNU/Linux yn(0;1), 1 binary64 ULP from pinned jq/glibc. Safe libm 0.2.16 is retained without FFI. The 92-input CLI probe includes 13 order-zero yn inputs with at most 1 ULP observed; no other order/input is approved. Bessel-function results may differ in the last bit. Reconsider after implementation when dependencies, target, reference/runtime identities, or observations change. |
| `manual.regex.flag-l` | Primary review: accept only this x86_64 GNU/Linux longest-match witness as an explicit fancy-regex 0.19.1 restriction. tq rejects the l flag with the recorded diagnostic/status rather than returning an incorrect first-alternative match. Scripts requiring longest matching cannot substitute tq. Reconsider after implementation when a safe Rust library provides longest matching with captures and bounded work; no other regex behavior is exempt. |

## Cases

Each case shows the complete jq, tq JSON, and tq TOON output. Control bytes use `\xNN` escapes so record separators remain visible.

### builtin.all

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### builtin.any

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### builtin.arrays

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### builtin.ascii-downcase

```
# jq
"abcé"

# tq -o json
"abcé"

# tq
abcé
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### builtin.booleans

```
# jq
false
true

# tq -o json
false
true

# tq --seq
\x1efalse
\x1etrue
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### builtin.empty

```
# jq

# tq -o json

# tq
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### builtin.error

```
# jq


[stderr]
jq: error (at <stdin>:0): boom

# tq -o json


[stderr]
tq: runtime error: boom

# tq


[stderr]
tq: runtime error: boom
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### builtin.explode

```
# jq
[
  65,
  233
]

# tq -o json
[
  65,
  233
]

# tq
[2]: 65,233
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### builtin.fromjson

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### builtin.getpath

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### builtin.group-by

```
# jq
[
  [
    {
      "k": 1
    }
  ],
  [
    {
      "k": 2
    },
    {
      "k": 2
    }
  ]
]

# tq -o json
[
  [
    {
      "k": 1
    }
  ],
  [
    {
      "k": 2
    },
    {
      "k": 2
    }
  ]
]

# tq
[2]:
  - [1]{k}:
    1
  - [2]{k}:
    2
    2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 43 | 43 | 30 | -13 |
| `cl100k_base` | 43 | 43 | 30 | -13 |

### builtin.has

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 5 | -3 |
| `cl100k_base` | 8 | 8 | 5 | -3 |

### builtin.implode

```
# jq
"Aé"

# tq -o json
"Aé"

# tq
Aé
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 |
| `cl100k_base` | 3 | 3 | 2 | -1 |

### builtin.in

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### builtin.iterables

```
# jq
[]
{}

# tq -o json
[]
{}

# tq --seq
\x1e[0]:
\x1e
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 6 | +4 |
| `cl100k_base` | 2 | 2 | 6 | +4 |

### builtin.keys

```
# jq
[
  "10",
  "2",
  "a",
  "z"
]

# tq -o json
[
  "10",
  "2",
  "a",
  "z"
]

# tq
[4]: "10","2",a,z
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 10 | -8 |
| `cl100k_base` | 18 | 18 | 10 | -8 |

### builtin.keys-unsorted

```
# jq
[
  "z",
  "a"
]

# tq -o json
[
  "z",
  "a"
]

# tq
[2]: z,a
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 5 | -5 |
| `cl100k_base` | 10 | 10 | 5 | -5 |

### builtin.length

```
# jq
[
  2,
  1,
  1,
  0
]

# tq -o json
[
  2,
  1,
  1,
  0
]

# tq
[4]: 2,1,1,0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 11 | -7 |
| `cl100k_base` | 18 | 18 | 11 | -7 |

### builtin.limit

```
# jq
0
1

# tq -o json
0
1

# tq --seq
\x1e0
\x1e1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### builtin.ltrimstr

```
# jq
"fix"

# tq -o json
"fix"

# tq
fix
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### builtin.map

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### builtin.map-values

```
# jq
{
  "a": 2,
  "b": 3
}

# tq -o json
{
  "a": 2,
  "b": 3
}

# tq
a: 2
b: 3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### builtin.max

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### builtin.max-by

```
# jq
{
  "k": 2
}

# tq -o json
{
  "k": 2
}

# tq
k: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### builtin.min

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### builtin.min-by

```
# jq
{
  "k": 1
}

# tq -o json
{
  "k": 1
}

# tq
k: 1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### builtin.nulls

```
# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### builtin.numbers

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### builtin.objects

```
# jq
{}

# tq -o json
{}

# tq
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 0 | -1 |
| `cl100k_base` | 1 | 1 | 0 | -1 |

### builtin.path

```
# jq
[
  "a",
  0
]

# tq -o json
[
  "a",
  0
]

# tq
[2]: a,0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 6 | -4 |
| `cl100k_base` | 10 | 10 | 6 | -4 |

### builtin.reverse

```
# jq
[
  3,
  2,
  1
]

# tq -o json
[
  3,
  2,
  1
]

# tq
[3]: 3,2,1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### builtin.scalars

```
# jq
null
false
0
"x"

# tq -o json
null
false
0
"x"

# tq --seq
\x1enull
\x1efalse
\x1e0
\x1ex
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 12 | +4 |
| `cl100k_base` | 8 | 8 | 12 | +4 |

### builtin.select

```
# jq
2
4

# tq -o json
2
4

# tq --seq
\x1e2
\x1e4
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### builtin.setpath

```
# jq
{
  "a": [
    null,
    7
  ]
}

# tq -o json
{
  "a": [
    null,
    7
  ]
}

# tq
a[2]: null,7
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 7 | -9 |
| `cl100k_base` | 16 | 16 | 7 | -9 |

### builtin.sort

```
# jq
[
  null,
  false,
  3,
  "x",
  []
]

# tq -o json
[
  null,
  false,
  3,
  "x",
  []
]

# tq
[5]:
  - null
  - false
  - 3
  - x
  - [0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 25 | +7 |
| `cl100k_base` | 18 | 18 | 25 | +7 |

### builtin.sort-by

```
# jq
[
  {
    "n": 1
  },
  {
    "n": 2
  },
  {
    "n": 2
  }
]

# tq -o json
[
  {
    "n": 1
  },
  {
    "n": 2
  },
  {
    "n": 2
  }
]

# tq
[3]{n}:
  1
  2
  2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 35 | 35 | 17 | -18 |
| `cl100k_base` | 35 | 35 | 17 | -18 |

### builtin.strings

```
# jq
"x"

# tq -o json
"x"

# tq
x
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### builtin.to-entries

```
# jq
[
  {
    "key": "z",
    "value": 1
  },
  {
    "key": "a",
    "value": 2
  }
]

# tq -o json
[
  {
    "key": "z",
    "value": 1
  },
  {
    "key": "a",
    "value": 2
  }
]

# tq
[2]{key,value}:
  z,1
  a,2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 38 | 38 | 16 | -22 |
| `cl100k_base` | 38 | 38 | 16 | -22 |

### builtin.tojson

```
# jq
"{\"a\":1}"

# tq -o json
"{\"a\":1}"

# tq
"{\"a\":1}"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 |
| `cl100k_base` | 6 | 6 | 6 | 0 |

### builtin.tonumber

```
# jq
[
  -2,
  1.5,
  1E+3
]

# tq -o json
[
  -2,
  1.5,
  1E+3
]

# tq
[3]: -2,1.5,1000
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 19 | 19 | 12 | -7 |
| `cl100k_base` | 19 | 19 | 12 | -7 |

### builtin.tostream

```
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

# tq --seq
\x1e[2]:
  - [2]: a,0
  - 1
\x1e[1]:
  - [2]: a,0
\x1e[1]:
  - [1]: a
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 42 | 42 | 42 | 0 |
| `cl100k_base` | 42 | 42 | 42 | 0 |

### builtin.tostring

```
# jq
[
  "null",
  "true",
  "1",
  "x",
  "[2]"
]

# tq -o json
[
  "null",
  "true",
  "1",
  "x",
  "[2]"
]

# tq
[5]: "null","true","1",x,"[2]"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 15 | -7 |
| `cl100k_base` | 22 | 22 | 15 | -7 |

### builtin.type

```
# jq
[
  "null",
  "boolean",
  "number",
  "string",
  "array",
  "object"
]

# tq -o json
[
  "null",
  "boolean",
  "number",
  "string",
  "array",
  "object"
]

# tq
[6]: "null",boolean,number,string,array,object
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 26 | 26 | 13 | -13 |
| `cl100k_base` | 26 | 26 | 12 | -14 |

### builtin.unique

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### builtin.unique-by-generator

```
# jq
[
  {
    "a": 1,
    "b": 1,
    "id": "first"
  },
  {
    "a": 1,
    "b": 2,
    "id": "distinct"
  }
]

# tq -o json
[
  {
    "a": 1,
    "b": 1,
    "id": "first"
  },
  {
    "a": 1,
    "b": 2,
    "id": "distinct"
  }
]

# tq
[2]{a,b,id}:
  1,1,first
  1,2,distinct
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 52 | 52 | 23 | -29 |
| `cl100k_base` | 52 | 52 | 24 | -28 |

### builtin.utf8bytelength

```
# jq
6

# tq -o json
6

# tq
6
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### builtin.values

```
# jq
false
0
"x"

# tq -o json
false
0
"x"

# tq --seq
\x1efalse
\x1e0
\x1ex
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 9 | +3 |
| `cl100k_base` | 6 | 6 | 9 | +3 |

### builtin.with-entries

```
# jq
{
  "a": 2,
  "b": 3
}

# tq -o json
{
  "a": 2,
  "b": 3
}

# tq
a: 2
b: 3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### date.strftime

```
# jq
"2015-03-05T23:51:47Z"

# tq -o json
"2015-03-05T23:51:47Z"

# tq
"2015-03-05T23:51:47Z"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 15 | 15 | 15 | 0 |
| `cl100k_base` | 15 | 15 | 15 | 0 |

### date.strptime

```
# jq
[
  2015,
  2,
  5,
  23,
  51,
  47,
  4,
  63
]

# tq -o json
[
  2015,
  2,
  5,
  23,
  51,
  47,
  4,
  63
]

# tq
[8]: 2015,2,5,23,51,47,4,63
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 35 | 35 | 20 | -15 |
| `cl100k_base` | 35 | 35 | 20 | -15 |

### date.utc-roundtrip

```
# jq
[
  "2015-03-05T23:51:47Z",
  [
    2015,
    2,
    5,
    23,
    51,
    47,
    4,
    63
  ],
  1425599507
]

# tq -o json
[
  "2015-03-05T23:51:47Z",
  [
    2015,
    2,
    5,
    23,
    51,
    47,
    4,
    63
  ],
  1425599507
]

# tq
[3]:
  - "2015-03-05T23:51:47Z"
  - [8]: 2015,2,5,23,51,47,4,63
  - 1425599507
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 62 | 62 | 50 | -12 |
| `cl100k_base` | 62 | 62 | 50 | -12 |

### manual-bof-code-format-uri

```
# jq
"https://www.google.com/search?q=what%20is%20jq%3F"

# tq -o json
"https://www.google.com/search?q=what%20is%20jq%3F"

# tq
"https://www.google.com/search?q=what%20is%20jq%3F"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 19 | 19 | 19 | 0 |
| `cl100k_base` | 19 | 19 | 19 | 0 |

### manual-bof-code-map-001

```
# jq
[
  2
]

# tq -o json
[
  2
]

# tq
[1]: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 |
| `cl100k_base` | 6 | 6 | 5 | -1 |

### manual-bof-code-map-002

```
# jq
[
  1,
  1
]

# tq -o json
[
  1,
  1
]

# tq
[2]: 1,1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual-bof-code-map-003

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### manual-bof-code-map-004

```
# jq
[
  2
]

# tq -o json
[
  2
]

# tq
[1]: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 |
| `cl100k_base` | 6 | 6 | 5 | -1 |

### manual-bof-code-map-005

```
# jq
[
  1
]

# tq -o json
[
  1
]

# tq
[1]: 1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 |
| `cl100k_base` | 6 | 6 | 5 | -1 |

### manual-bof-code-map-006

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### manual-bof-code-recurse-query

```
# jq
"/"
"/bin"
"/bin/ls"
"/bin/sh"
"/home"
"/home/stephen"
"/home/stephen/jq"

# tq -o json
"/"
"/bin"
"/bin/ls"
"/bin/sh"
"/home"
"/home/stephen"
"/home/stephen/jq"

# tq --seq
\x1e/
\x1e/bin
\x1e/bin/ls
\x1e/bin/sh
\x1e/home
\x1e/home/stephen
\x1e/home/stephen/jq
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 31 | 31 | 37 | +6 |
| `cl100k_base` | 31 | 31 | 37 | +6 |

### manual-bof-prose-halt-error

```
# jq


[stderr]
Error: something went wrong

# tq -o json


[stderr]
Error: something went wrong

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | n/a | n/a |
| `cl100k_base` | 0 | 0 | n/a | n/a |

### manual-bof-prose-map-select

```
# jq
[
  2,
  3
]

# tq -o json
[
  2,
  3
]

# tq
[2]: 2,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual-bof-synth-format-base64

```
# jq
"aGVsbG8="

# tq -o json
"aGVsbG8="

# tq
aGVsbG8=
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 |
| `cl100k_base` | 6 | 6 | 6 | 0 |

### manual-bof-synth-format-base64d

```
# jq
"hello"

# tq -o json
"hello"

# tq
hello
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual-bof-synth-format-csv

```
# jq
"\"a\",\"b\""

# tq -o json
"\"a\",\"b\""

# tq
"\"a\",\"b\""
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 7 | 7 | 6 | -1 |
| `cl100k_base` | 7 | 7 | 6 | -1 |

### manual-bof-synth-format-html

```
# jq
"&lt;&gt;&amp;&apos;&quot;"

# tq -o json
"&lt;&gt;&amp;&apos;&quot;"

# tq
&lt;&gt;&amp;&apos;&quot;
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 11 | 11 | 10 | -1 |
| `cl100k_base` | 11 | 11 | 10 | -1 |

### manual-bof-synth-format-json

```
# jq
"\"x\""

# tq -o json
"\"x\""

# tq
"\"x\""
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 4 | -1 |
| `cl100k_base` | 5 | 5 | 4 | -1 |

### manual-bof-synth-format-sh

```
# jq
"'a b'"

# tq -o json
"'a b'"

# tq
'a b'
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 3 | -1 |
| `cl100k_base` | 4 | 4 | 3 | -1 |

### manual-bof-synth-format-text

```
# jq
"1"

# tq -o json
"1"

# tq
"1"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 3 | 0 |
| `cl100k_base` | 3 | 3 | 3 | 0 |

### manual-bof-synth-format-tsv

```
# jq
"a\tb"

# tq -o json
"a\tb"

# tq
"a\tb"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 |
| `cl100k_base` | 4 | 4 | 4 | 0 |

### manual-bof-synth-format-uri

```
# jq
"a%20b"

# tq -o json
"a%20b"

# tq
a%20b
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 4 | -1 |
| `cl100k_base` | 5 | 5 | 4 | -1 |

### manual-bof-synth-format-urid

```
# jq
"a b"

# tq -o json
"a b"

# tq
a b
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 |
| `cl100k_base` | 3 | 3 | 2 | -1 |

### manual-bof-synth-path-boolean

```
# jq
[
  "a",
  0
]
[
  "a",
  1
]

# tq -o json
[
  "a",
  0
]
[
  "a",
  1
]

# tq --seq
\x1e[2]: a,0
\x1e[2]: a,1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 16 | -4 |
| `cl100k_base` | 20 | 20 | 16 | -4 |

### manual-bof-synth-recurse-bounded

```
# jq
0
1
2

# tq -o json
0
1
2

# tq --seq
\x1e0
\x1e1
\x1e2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 9 | +3 |
| `cl100k_base` | 6 | 6 | 9 | +3 |

### manual-bof-synth-recurse-default

```
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

# tq --seq
\x1ea[1]: 1
\x1e[1]: 1
\x1e1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 21 | 21 | 18 | -3 |
| `cl100k_base` | 21 | 21 | 18 | -3 |

### manual-bof-synth-sql-in1

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-synth-sql-in2

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-synth-sql-index

```
# jq
{
  "a": {
    "id": "a",
    "v": 1
  },
  "b": {
    "id": "b",
    "v": 2
  }
}

# tq -o json
{
  "a": {
    "id": "a",
    "v": 1
  },
  "b": {
    "id": "b",
    "v": 2
  }
}

# tq
a:
  id: a
  v: 1
b:
  id: b
  v: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 44 | 44 | 25 | -19 |
| `cl100k_base` | 44 | 44 | 25 | -19 |

### manual-bof-synth-sql-join2

```
# jq


[stderr]
jq: error (at <stdin>:0): Cannot index string with string "id"

# tq -o json


[stderr]
tq: runtime error: field access cannot be applied to string

# tq


[stderr]
tq: runtime error: field access cannot be applied to string
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual-bof-synth-sql-join3

```
# jq
[
  {
    "id": "a",
    "v": 1
  },
  {
    "id": "a",
    "v": 1
  }
]
[
  {
    "id": "b",
    "v": 2
  },
  {
    "id": "b",
    "v": 2
  }
]

# tq -o json
[
  {
    "id": "a",
    "v": 1
  },
  {
    "id": "a",
    "v": 1
  }
]
[
  {
    "id": "b",
    "v": 2
  },
  {
    "id": "b",
    "v": 2
  }
]

# tq --seq
\x1e[2]{id,v}:
  a,1
  a,1
\x1e[2]{id,v}:
  b,2
  b,2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 76 | 76 | 36 | -40 |
| `cl100k_base` | 76 | 76 | 36 | -40 |

### manual-bof-synth-sql-join4

```
# jq
[
  {
    "id": "a",
    "v": 1
  },
  {
    "id": "a",
    "v": 1
  }
]
[
  {
    "id": "b",
    "v": 2
  },
  {
    "id": "b",
    "v": 2
  }
]

# tq -o json
[
  {
    "id": "a",
    "v": 1
  },
  {
    "id": "a",
    "v": 1
  }
]
[
  {
    "id": "b",
    "v": 2
  },
  {
    "id": "b",
    "v": 2
  }
]

# tq --seq
\x1e[2]{id,v}:
  a,1
  a,1
\x1e[2]{id,v}:
  b,2
  b,2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 76 | 76 | 36 | -40 |
| `cl100k_base` | 76 | 76 | 36 | -40 |

### manual-bof-table-001

```
# jq
8

# tq -o json
8

# tq
8
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-002

```
# jq
[
  1,
  2,
  3,
  4
]

# tq -o json
[
  1,
  2,
  3,
  4
]

# tq
[4]: 1,2,3,4
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 11 | -7 |
| `cl100k_base` | 18 | 18 | 11 | -7 |

### manual-bof-table-003

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-004

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-005

```
# jq
{
  "a": 42,
  "b": 2,
  "c": 3
}

# tq -o json
{
  "a": 42,
  "b": 2,
  "c": 3
}

# tq
a: 42
b: 2
c: 3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 23 | 23 | 14 | -9 |
| `cl100k_base` | 23 | 23 | 14 | -9 |

### manual-bof-table-006

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-007

```
# jq
[
  "json"
]

# tq -o json
[
  "json"
]

# tq
[1]: json
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 4 | -2 |
| `cl100k_base` | 6 | 6 | 4 | -2 |

### manual-bof-table-008

```
# jq
6

# tq -o json
6

# tq
6
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-009

```
# jq
[
  "a",
  "b,c,d",
  "e"
]

# tq -o json
[
  "a",
  "b,c,d",
  "e"
]

# tq
[3]: a,"b,c,d",e
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 10 | -6 |
| `cl100k_base` | 16 | 16 | 10 | -6 |

### manual-bof-table-010

```
# jq
{
  "k": {
    "a": 0,
    "b": 2,
    "c": 3
  }
}

# tq -o json
{
  "k": {
    "a": 0,
    "b": 2,
    "c": 3
  }
}

# tq
k:
  a: 0
  b: 2
  c: 3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 19 | -11 |
| `cl100k_base` | 30 | 30 | 19 | -11 |

### manual-bof-table-011

```
# jq
1
-1

# tq -o json
1
-1

# tq --seq
\x1e1
\x1e-1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 7 | +2 |
| `cl100k_base` | 5 | 5 | 7 | +2 |

### manual-bof-table-012

```
# jq
[
  10,
  1.1,
  0.1
]

# tq -o json
[
  10,
  1.1,
  0.1
]

# tq
[3]: 10,1.1,0.1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 13 | -5 |
| `cl100k_base` | 18 | 18 | 13 | -5 |

### manual-bof-table-013

```
# jq
2
6
1
0
5

# tq -o json
2
6
1
0
5

# tq --seq
\x1e2
\x1e6
\x1e1
\x1e0
\x1e5
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 15 | +5 |
| `cl100k_base` | 10 | 10 | 15 | +5 |

### manual-bof-table-014

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-015

```
# jq
[
  "Foo",
  "abc",
  "abcd"
]

# tq -o json
[
  "Foo",
  "abc",
  "abcd"
]

# tq
[3]: Foo,abc,abcd
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 8 | -6 |
| `cl100k_base` | 14 | 14 | 8 | -6 |

### manual-bof-table-016

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual-bof-table-017

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 5 | -3 |
| `cl100k_base` | 8 | 8 | 5 | -3 |

### manual-bof-table-018

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 5 | -3 |
| `cl100k_base` | 8 | 8 | 5 | -3 |

### manual-bof-table-019

```
# jq
true
false

# tq -o json
true
false

# tq --seq
\x1etrue
\x1efalse
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual-bof-table-020

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 5 | -3 |
| `cl100k_base` | 8 | 8 | 5 | -3 |

### manual-bof-table-021

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual-bof-table-022

```
# jq
{
  "a": 2,
  "b": 3,
  "c": 4
}

# tq -o json
{
  "a": 2,
  "b": 3,
  "c": 4
}

# tq
a: 2
b: 3
c: 4
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 23 | 23 | 14 | -9 |
| `cl100k_base` | 23 | 23 | 14 | -9 |

### manual-bof-table-023

```
# jq
[
  1,
  1,
  2,
  2
]

# tq -o json
[
  1,
  1,
  2,
  2
]

# tq
[4]: 1,1,2,2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 11 | -7 |
| `cl100k_base` | 18 | 18 | 11 | -7 |

### manual-bof-table-024

```
# jq
{
  "b": true
}

# tq -o json
{
  "b": true
}

# tq
b: true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 3 | -5 |
| `cl100k_base` | 8 | 8 | 3 | -5 |

### manual-bof-table-025

```
# jq
{
  "a": 1,
  "b": {
    "c": 2
  },
  "x": null
}

# tq -o json
{
  "a": 1,
  "b": {
    "c": 2
  },
  "x": null
}

# tq
a: 1
b:
  c: 2
x: null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 29 | 29 | 16 | -13 |
| `cl100k_base` | 29 | 29 | 16 | -13 |

### manual-bof-table-026

```
# jq
[
  1,
  null,
  3
]

# tq -o json
[
  1,
  null,
  3
]

# tq
[3]: 1,null,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 13 | 13 | 8 | -5 |
| `cl100k_base` | 13 | 13 | 8 | -5 |

### manual-bof-table-027

```
# jq
[
  "a",
  0,
  "b"
]

# tq -o json
[
  "a",
  0,
  "b"
]

# tq
[3]: a,0,b
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 7 | -7 |
| `cl100k_base` | 14 | 14 | 7 | -7 |

### manual-bof-table-028

```
# jq
[
  [],
  [
    "a"
  ],
  [
    "a",
    0
  ],
  [
    "a",
    0,
    "b"
  ]
]

# tq -o json
[
  [],
  [
    "a"
  ],
  [
    "a",
    0
  ],
  [
    "a",
    0,
    "b"
  ]
]

# tq
[4]:
  - [0]:
  - [1]: a
  - [2]: a,0
  - [3]: a,0,b
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 40 | 40 | 33 | -7 |
| `cl100k_base` | 40 | 40 | 33 | -7 |

### manual-bof-table-029

```
# jq
{
  "bar": 9001,
  "baz": 42
}

# tq -o json
{
  "bar": 9001,
  "baz": 42
}

# tq
bar: 9001
baz: 42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 10 | -7 |
| `cl100k_base` | 17 | 17 | 10 | -7 |

### manual-bof-table-030

```
# jq
[
  "foo"
]

# tq -o json
[
  "foo"
]

# tq
[1]: foo
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 4 | -2 |
| `cl100k_base` | 6 | 6 | 4 | -2 |

### manual-bof-table-031

```
# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-032

```
# jq
[
  0,
  1
]

# tq -o json
[
  0,
  1
]

# tq
[2]: 0,1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual-bof-table-033

```
# jq
{
  "a": {
    "b": 1
  }
}

# tq -o json
{
  "a": {
    "b": 1
  }
}

# tq
a:
  b: 1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 7 | -9 |
| `cl100k_base` | 16 | 16 | 7 | -9 |

### manual-bof-table-034

```
# jq
{
  "a": {
    "b": 1
  }
}

# tq -o json
{
  "a": {
    "b": 1
  }
}

# tq
a:
  b: 1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 7 | -9 |
| `cl100k_base` | 16 | 16 | 7 | -9 |

### manual-bof-table-035

```
# jq
[
  {
    "a": 1
  }
]

# tq -o json
[
  {
    "a": 1
  }
]

# tq
[1]{a}:
  1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 13 | 13 | 9 | -4 |
| `cl100k_base` | 13 | 13 | 9 | -4 |

### manual-bof-table-036

```
# jq
{
  "a": {},
  "x": {
    "y": 2
  }
}

# tq -o json
{
  "a": {},
  "x": {
    "y": 2
  }
}

# tq
a:
x:
  y: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 21 | 21 | 9 | -12 |
| `cl100k_base` | 21 | 21 | 9 | -12 |

### manual-bof-table-037

```
# jq
[
  {
    "key": "a",
    "value": 1
  },
  {
    "key": "b",
    "value": 2
  }
]

# tq -o json
[
  {
    "key": "a",
    "value": 1
  },
  {
    "key": "b",
    "value": 2
  }
]

# tq
[2]{key,value}:
  a,1
  b,2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 38 | 38 | 16 | -22 |
| `cl100k_base` | 38 | 38 | 16 | -22 |

### manual-bof-table-038

```
# jq
{
  "a": 1,
  "b": 2
}

# tq -o json
{
  "a": 1,
  "b": 2
}

# tq
a: 1
b: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### manual-bof-table-039

```
# jq
{
  "KEY_a": 1,
  "KEY_b": 2
}

# tq -o json
{
  "KEY_a": 1,
  "KEY_b": 2
}

# tq
KEY_a: 1
KEY_b: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 11 | -7 |
| `cl100k_base` | 18 | 18 | 11 | -7 |

### manual-bof-table-040

```
# jq
[
  5,
  3,
  7
]

# tq -o json
[
  5,
  3,
  7
]

# tq
[3]: 5,3,7
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual-bof-table-041

```
# jq
{
  "id": "second",
  "val": 2
}

# tq -o json
{
  "id": "second",
  "val": 2
}

# tq
id: second
val: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 8 | -8 |
| `cl100k_base` | 16 | 16 | 8 | -8 |

### manual-bof-table-042

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-043

```
# jq
1
2

# tq -o json
1
2

# tq --seq
\x1e1
\x1e2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual-bof-table-044

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual-bof-table-045

```
# jq
"error message"

# tq -o json
"error message"

# tq
error message
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### manual-bof-table-046

```
# jq
"invalid value: 42"

# tq -o json
"invalid value: 42"

# tq
"invalid value: 42"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 7 | 7 | 7 | 0 |
| `cl100k_base` | 7 | 7 | 7 | 0 |

### manual-bof-table-047

```
# jq
"{\"file\":\"<top-level>\",\"line\":1}"

# tq -o json
"{\"file\":\"<top-level>\",\"line\":1}"

# tq
"{\"file\":\"<top-level>\",\"line\":1}"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 14 | 0 |
| `cl100k_base` | 14 | 14 | 14 | 0 |

### manual-bof-table-048

```
# jq
[
  [
    0
  ],
  [
    1
  ],
  [
    1,
    0
  ],
  [
    1,
    1
  ],
  [
    1,
    1,
    "a"
  ]
]

# tq -o json
[
  [
    0
  ],
  [
    1
  ],
  [
    1,
    0
  ],
  [
    1,
    1
  ],
  [
    1,
    1,
    "a"
  ]
]

# tq
[5]:
  - [1]: 0
  - [1]: 1
  - [2]: 1,0
  - [2]: 1,1
  - [3]: 1,1,a
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 58 | 58 | 49 | -9 |
| `cl100k_base` | 58 | 58 | 49 | -9 |

### manual-bof-table-049

```
# jq
[
  [
    0
  ],
  [
    1,
    1,
    "a"
  ]
]

# tq -o json
[
  [
    0
  ],
  [
    1,
    1,
    "a"
  ]
]

# tq
[2]:
  - [1]: 0
  - [3]: 1,1,a
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 26 | 26 | 21 | -5 |
| `cl100k_base` | 26 | 26 | 21 | -5 |

### manual-bof-table-050

```
# jq
"abc"

# tq -o json
"abc"

# tq
abc
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual-bof-table-051

```
# jq
6

# tq -o json
6

# tq
6
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-052

```
# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-053

```
# jq
8

# tq -o json
8

# tq
8
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-054

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-055

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-056

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-057

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-058

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-059

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-060

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual-bof-table-061

```
# jq
[
  1,
  2,
  [
    3
  ]
]

# tq -o json
[
  1,
  2,
  [
    3
  ]
]

# tq
[3]:
  - 1
  - 2
  - [1]: 3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 20 | +2 |
| `cl100k_base` | 18 | 18 | 20 | +2 |

### manual-bof-table-062

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### manual-bof-table-063

```
# jq
[
  {
    "foo": "bar"
  },
  {
    "foo": "baz"
  }
]

# tq -o json
[
  {
    "foo": "bar"
  },
  {
    "foo": "baz"
  }
]

# tq
[2]{foo}:
  bar
  baz
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 24 | 24 | 11 | -13 |
| `cl100k_base` | 24 | 24 | 11 | -13 |

### manual-bof-table-064

```
# jq
2
3

# tq -o json
2
3

# tq --seq
\x1e2
\x1e3
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual-bof-table-065

```
# jq
[
  2,
  3
]

# tq -o json
[
  2,
  3
]

# tq
[2]: 2,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual-bof-table-066

```
# jq
[
  0,
  1,
  2,
  3
]

# tq -o json
[
  0,
  1,
  2,
  3
]

# tq
[4]: 0,1,2,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 11 | -7 |
| `cl100k_base` | 18 | 18 | 11 | -7 |

### manual-bof-table-067

```
# jq
[
  0,
  3,
  6,
  9
]

# tq -o json
[
  0,
  3,
  6,
  9
]

# tq
[4]: 0,3,6,9
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 11 | -7 |
| `cl100k_base` | 18 | 18 | 11 | -7 |

### manual-bof-table-068

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### manual-bof-table-069

```
# jq
[
  0,
  -1,
  -2,
  -3,
  -4
]

# tq -o json
[
  0,
  -1,
  -2,
  -3,
  -4
]

# tq
[5]: 0,-1,-2,-3,-4
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 13 | -9 |
| `cl100k_base` | 22 | 22 | 13 | -9 |

### manual-bof-table-070

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-071

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-072

```
# jq
1
1

# tq -o json
1
1

# tq --seq
\x1e1
\x1e1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual-bof-table-073

```
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

# tq --seq
\x1etrue
\x1efalse
\x1etrue
\x1efalse
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 12 | +4 |
| `cl100k_base` | 8 | 8 | 12 | +4 |

### manual-bof-table-074

```
# jq
"1"
"1"
"[1]"

# tq -o json
"1"
"1"
"[1]"

# tq --seq
\x1e"1"
\x1e"1"
\x1e"[1]"
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 12 | +3 |
| `cl100k_base` | 9 | 9 | 12 | +3 |

### manual-bof-table-075

```
# jq
[
  "number",
  "boolean",
  "array",
  "object",
  "null",
  "string"
]

# tq -o json
[
  "number",
  "boolean",
  "array",
  "object",
  "null",
  "string"
]

# tq
[6]: number,boolean,array,object,"null",string
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 26 | 26 | 13 | -13 |
| `cl100k_base` | 26 | 26 | 13 | -13 |

### manual-bof-table-076

```
# jq
true
false

# tq -o json
true
false

# tq --seq
\x1etrue
\x1efalse
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual-bof-table-077

```
# jq
"number"
"number"

# tq -o json
"number"
"number"

# tq --seq
\x1enumber
\x1enumber
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 |
| `cl100k_base` | 6 | 6 | 6 | 0 |

### manual-bof-table-078

```
# jq
[
  null,
  3,
  6,
  8
]

# tq -o json
[
  null,
  3,
  6,
  8
]

# tq
[4]: null,3,6,8
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 10 | -7 |
| `cl100k_base` | 17 | 17 | 10 | -7 |

### manual-bof-table-079

```
# jq
[
  {
    "foo": 2,
    "bar": 1
  },
  {
    "foo": 3,
    "bar": 10
  },
  {
    "foo": 4,
    "bar": 10
  }
]

# tq -o json
[
  {
    "foo": 2,
    "bar": 1
  },
  {
    "foo": 3,
    "bar": 10
  },
  {
    "foo": 4,
    "bar": 10
  }
]

# tq
[3]{foo,bar}:
  2,1
  3,10
  4,10
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 56 | 56 | 25 | -31 |
| `cl100k_base` | 56 | 56 | 25 | -31 |

### manual-bof-table-080

```
# jq
[
  {
    "foo": 2,
    "bar": 1
  },
  {
    "foo": 3,
    "bar": 10
  },
  {
    "foo": 3,
    "bar": 20
  },
  {
    "foo": 4,
    "bar": 10
  }
]

# tq -o json
[
  {
    "foo": 2,
    "bar": 1
  },
  {
    "foo": 3,
    "bar": 10
  },
  {
    "foo": 3,
    "bar": 20
  },
  {
    "foo": 4,
    "bar": 10
  }
]

# tq
[4]{foo,bar}:
  2,1
  3,10
  3,20
  4,10
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 74 | 74 | 31 | -43 |
| `cl100k_base` | 74 | 74 | 31 | -43 |

### manual-bof-table-081

```
# jq
[
  [
    {
      "foo": 1,
      "bar": 10
    },
    {
      "foo": 1,
      "bar": 1
    }
  ],
  [
    {
      "foo": 3,
      "bar": 100
    }
  ]
]

# tq -o json
[
  [
    {
      "foo": 1,
      "bar": 10
    },
    {
      "foo": 1,
      "bar": 1
    }
  ],
  [
    {
      "foo": 3,
      "bar": 100
    }
  ]
]

# tq
[2]:
  - [2]{foo,bar}:
    1,10
    1,1
  - [1]{foo,bar}:
    3,100
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 64 | 64 | 40 | -24 |
| `cl100k_base` | 64 | 64 | 40 | -24 |

### manual-bof-table-082

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-083

```
# jq
{
  "foo": 2,
  "bar": 3
}

# tq -o json
{
  "foo": 2,
  "bar": 3
}

# tq
foo: 2
bar: 3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### manual-bof-table-084

```
# jq
[
  1,
  2,
  3,
  5
]

# tq -o json
[
  1,
  2,
  3,
  5
]

# tq
[4]: 1,2,3,5
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 11 | -7 |
| `cl100k_base` | 18 | 18 | 11 | -7 |

### manual-bof-table-085

```
# jq
[
  {
    "foo": 1,
    "bar": 2
  },
  {
    "foo": 4,
    "bar": 5
  }
]

# tq -o json
[
  {
    "foo": 1,
    "bar": 2
  },
  {
    "foo": 4,
    "bar": 5
  }
]

# tq
[2]{foo,bar}:
  1,2
  4,5
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 38 | 38 | 19 | -19 |
| `cl100k_base` | 38 | 38 | 19 | -19 |

### manual-bof-table-086

```
# jq
[
  "bacon",
  "chunky",
  "asparagus"
]

# tq -o json
[
  "bacon",
  "chunky",
  "asparagus"
]

# tq
[3]: bacon,chunky,asparagus
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 10 | -7 |
| `cl100k_base` | 17 | 17 | 10 | -7 |

### manual-bof-table-087

```
# jq
[
  4,
  3,
  2,
  1
]

# tq -o json
[
  4,
  3,
  2,
  1
]

# tq
[4]: 4,3,2,1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 11 | -7 |
| `cl100k_base` | 18 | 18 | 11 | -7 |

### manual-bof-table-088

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-089

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-090

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-091

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-092

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-093

```
# jq
[
  3,
  7,
  12
]

# tq -o json
[
  3,
  7,
  12
]

# tq
[3]: 3,7,12
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual-bof-table-094

```
# jq
[
  1,
  3,
  5
]

# tq -o json
[
  1,
  3,
  5
]

# tq
[3]: 1,3,5
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual-bof-table-095

```
# jq
[
  1,
  8
]

# tq -o json
[
  1,
  8
]

# tq
[2]: 1,8
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual-bof-table-096

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-097

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-098

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-099

```
# jq
12

# tq -o json
12

# tq
12
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-100

```
# jq
5

# tq -o json
5

# tq
5
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-101

```
# jq
8

# tq -o json
8

# tq
8
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-102

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-103

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-104

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-105

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-106

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-107

```
# jq
[
  false,
  true,
  false,
  true,
  false
]

# tq -o json
[
  false,
  true,
  false,
  true,
  false
]

# tq
[5]: false,true,false,true,false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 8 | -9 |
| `cl100k_base` | 17 | 17 | 8 | -9 |

### manual-bof-table-108

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 5 | -3 |
| `cl100k_base` | 8 | 8 | 5 | -3 |

### manual-bof-table-109

```
# jq
[
  1,
  3
]
[
  1,
  4
]
[
  2,
  3
]
[
  2,
  4
]

# tq -o json
[
  1,
  3
]
[
  1,
  4
]
[
  2,
  3
]
[
  2,
  4
]

# tq --seq
\x1e[2]: 1,3
\x1e[2]: 1,4
\x1e[2]: 2,3
\x1e[2]: 2,4
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 40 | 40 | 36 | -4 |
| `cl100k_base` | 40 | 40 | 36 | -4 |

### manual-bof-table-110

```
# jq
[
  0,
  0
]
[
  0,
  1
]
[
  1,
  0
]
[
  1,
  1
]

# tq -o json
[
  0,
  0
]
[
  0,
  1
]
[
  1,
  0
]
[
  1,
  1
]

# tq --seq
\x1e[2]: 0,0
\x1e[2]: 0,1
\x1e[2]: 1,0
\x1e[2]: 1,1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 40 | 40 | 36 | -4 |
| `cl100k_base` | 40 | 40 | 36 | -4 |

### manual-bof-table-111

```
# jq
[
  "fo",
  "",
  "barfoo",
  "bar",
  "afoo"
]

# tq -o json
[
  "fo",
  "",
  "barfoo",
  "bar",
  "afoo"
]

# tq
[5]: fo,"",barfoo,bar,afoo
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 12 | -10 |
| `cl100k_base` | 22 | 22 | 12 | -10 |

### manual-bof-table-112

```
# jq
[
  "fo",
  "",
  "bar",
  "foobar",
  "foob"
]

# tq -o json
[
  "fo",
  "",
  "bar",
  "foobar",
  "foob"
]

# tq
[5]: fo,"",bar,foobar,foob
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 21 | 21 | 12 | -9 |
| `cl100k_base` | 21 | 21 | 12 | -9 |

### manual-bof-table-113

```
# jq
[
  "fo",
  "",
  "bar",
  "bar",
  "b"
]

# tq -o json
[
  "fo",
  "",
  "bar",
  "bar",
  "b"
]

# tq
[5]: fo,"",bar,bar,b
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 10 | -10 |
| `cl100k_base` | 20 | 20 | 10 | -10 |

### manual-bof-table-114

```
# jq
"abc"
"abc "
" abc"

# tq -o json
"abc"
"abc "
" abc"

# tq --seq
\x1eabc
\x1e"abc "
\x1e" abc"
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 11 | +2 |
| `cl100k_base` | 9 | 9 | 11 | +2 |

### manual-bof-table-115

```
# jq
[
  102,
  111,
  111,
  98,
  97,
  114
]

# tq -o json
[
  102,
  111,
  111,
  98,
  97,
  114
]

# tq
[6]: 102,111,111,98,97,114
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 26 | 26 | 15 | -11 |
| `cl100k_base` | 26 | 26 | 15 | -11 |

### manual-bof-table-116

```
# jq
"ABC"

# tq -o json
"ABC"

# tq
ABC
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual-bof-table-117

```
# jq
[
  "a",
  "b,c,d",
  "e",
  ""
]

# tq -o json
[
  "a",
  "b,c,d",
  "e",
  ""
]

# tq
[4]: a,"b,c,d",e,""
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 12 | -6 |
| `cl100k_base` | 18 | 18 | 12 | -6 |

### manual-bof-table-118

```
# jq
"a, b,c,d, e"

# tq -o json
"a, b,c,d, e"

# tq
"a, b,c,d, e"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 8 | 0 |
| `cl100k_base` | 8 | 8 | 8 | 0 |

### manual-bof-table-119

```
# jq
"a 1 2.3 true  false"

# tq -o json
"a 1 2.3 true  false"

# tq
a 1 2.3 true  false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 11 | 11 | 10 | -1 |
| `cl100k_base` | 11 | 11 | 10 | -1 |

### manual-bof-table-120

```
# jq
"1, 2"

# tq -o json
"1, 2"

# tq
"1, 2"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 |
| `cl100k_base` | 6 | 6 | 6 | 0 |

### manual-bof-table-121

```
# jq
"USEFUL BUT NOT FOR é"

# tq -o json
"USEFUL BUT NOT FOR é"

# tq
USEFUL BUT NOT FOR é
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 6 | -2 |
| `cl100k_base` | 8 | 8 | 6 | -2 |

### manual-bof-table-122

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 17 | -13 |
| `cl100k_base` | 30 | 30 | 17 | -13 |

### manual-bof-table-123

```
# jq
[
  2
]

# tq -o json
[
  2
]

# tq
[1]: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 |
| `cl100k_base` | 6 | 6 | 5 | -1 |

### manual-bof-table-124

```
# jq
24

# tq -o json
24

# tq
24
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-125

```
# jq
{
  "foo": [
    {
      "foo": []
    },
    {
      "foo": [
        {
          "foo": []
        }
      ]
    }
  ]
}
{
  "foo": []
}
{
  "foo": [
    {
      "foo": []
    }
  ]
}
{
  "foo": []
}

# tq -o json
{
  "foo": [
    {
      "foo": []
    },
    {
      "foo": [
        {
          "foo": []
        }
      ]
    }
  ]
}
{
  "foo": []
}
{
  "foo": [
    {
      "foo": []
    }
  ]
}
{
  "foo": []
}

# tq --seq
\x1efoo[2]:
  - foo[0]:
  - foo[1]:
      - foo[0]:
\x1efoo[0]:
\x1efoo[1]:
  - foo[0]:
\x1efoo[0]:
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 70 | 70 | 44 | -26 |
| `cl100k_base` | 70 | 70 | 44 | -26 |

### manual-bof-table-126

```
# jq
{
  "a": 0,
  "b": [
    1
  ]
}
0
[
  1
]
1

# tq -o json
{
  "a": 0,
  "b": [
    1
  ]
}
0
[
  1
]
1

# tq --seq
\x1ea: 0
b[1]: 1
\x1e0
\x1e[1]: 1
\x1e1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 26 | -4 |
| `cl100k_base` | 30 | 30 | 26 | -4 |

### manual-bof-table-127

```
# jq
2
4
16

# tq -o json
2
4
16

# tq --seq
\x1e2
\x1e4
\x1e16
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 9 | +3 |
| `cl100k_base` | 6 | 6 | 9 | +3 |

### manual-bof-table-128

```
# jq
[
  [
    1,
    4,
    7
  ],
  [
    2,
    5,
    8
  ],
  [
    3,
    6,
    9
  ]
]

# tq -o json
[
  [
    1,
    4,
    7
  ],
  [
    2,
    5,
    8
  ],
  [
    3,
    6,
    9
  ]
]

# tq
[3]:
  - [3]: 1,4,7
  - [3]: 2,5,8
  - [3]: 3,6,9
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 50 | 50 | 38 | -12 |
| `cl100k_base` | 50 | 50 | 38 | -12 |

### manual-bof-table-129

```
# jq
[
  {
    "a": {
      "b": 2
    }
  }
]

# tq -o json
[
  {
    "a": {
      "b": 2
    }
  }
]

# tq
[1]:
  - a:
      b: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 12 | -8 |
| `cl100k_base` | 20 | 20 | 12 | -8 |

### manual-bof-table-130

```
# jq
"less"

# tq -o json
"less"

# tq
less
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual-bof-table-131

```
# jq
"less"

# tq -o json
"less"

# tq
less
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual-bof-table-132

```
# jq
[
  [
    1,
    2
  ],
  [
    null,
    3
  ]
]

# tq -o json
[
  [
    1,
    2
  ],
  [
    null,
    3
  ]
]

# tq
[2]:
  - [2]: 1,2
  - [2]: null,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 25 | 25 | 21 | -4 |
| `cl100k_base` | 25 | 25 | 21 | -4 |

### manual-bof-table-133

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual-bof-table-134

```
# jq
-1

# tq -o json
-1

# tq
-1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 |
| `cl100k_base` | 3 | 3 | 2 | -1 |

### manual-bof-table-135

```
# jq
[
  1,
  2,
  3,
  4
]

# tq -o json
[
  1,
  2,
  3,
  4
]

# tq
[4]: 1,2,3,4
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 11 | -7 |
| `cl100k_base` | 18 | 18 | 11 | -7 |

### manual-bof-table-136

```
# jq
"The input was 42, which is one less than 43"

# tq -o json
"The input was 42, which is one less than 43"

# tq
"The input was 42, which is one less than 43"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 14 | 0 |
| `cl100k_base` | 14 | 14 | 14 | 0 |

### manual-bof-table-137

```
# jq
[
  "1",
  "foo",
  "[\"foo\"]"
]

# tq -o json
[
  "1",
  "foo",
  "[\"foo\"]"
]

# tq
[3]: "1",foo,"[\"foo\"]"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 13 | -3 |
| `cl100k_base` | 16 | 16 | 13 | -3 |

### manual-bof-table-138

```
# jq
[
  "1",
  "\"foo\"",
  "[\"foo\"]"
]

# tq -o json
[
  "1",
  "\"foo\"",
  "[\"foo\"]"
]

# tq
[3]: "1","\"foo\"","[\"foo\"]"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 15 | -2 |
| `cl100k_base` | 16 | 16 | 15 | -1 |

### manual-bof-table-139

```
# jq
[
  1,
  "foo",
  [
    "foo"
  ]
]

# tq -o json
[
  1,
  "foo",
  [
    "foo"
  ]
]

# tq
[3]:
  - 1
  - foo
  - [1]: foo
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 18 | 0 |
| `cl100k_base` | 18 | 18 | 18 | 0 |

### manual-bof-table-140

```
# jq
"This works if x &lt; y"

# tq -o json
"This works if x &lt; y"

# tq
This works if x &lt; y
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 8 | -1 |
| `cl100k_base` | 9 | 9 | 8 | -1 |

### manual-bof-table-141

```
# jq
"echo 'O'\\''Hara'\\''s Ale'"

# tq -o json
"echo 'O'\\''Hara'\\''s Ale'"

# tq
"echo 'O'\\''Hara'\\''s Ale'"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 15 | 15 | 15 | 0 |
| `cl100k_base` | 15 | 15 | 15 | 0 |

### manual-bof-table-142

```
# jq
"VGhpcyBpcyBhIG1lc3NhZ2U="

# tq -o json
"VGhpcyBpcyBhIG1lc3NhZ2U="

# tq
VGhpcyBpcyBhIG1lc3NhZ2U=
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 15 | -1 |
| `cl100k_base` | 19 | 19 | 18 | -1 |

### manual-bof-table-143

```
# jq
"This is a message"

# tq -o json
"This is a message"

# tq
This is a message
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 4 | -1 |
| `cl100k_base` | 5 | 5 | 4 | -1 |

### manual-bof-table-144

```
# jq
1425599507

# tq -o json
1425599507

# tq
1425599507
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 4 | -1 |
| `cl100k_base` | 5 | 5 | 4 | -1 |

### manual-bof-table-146

```
# jq
1425599507

# tq -o json
1425599507

# tq
1425599507
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 4 | -1 |
| `cl100k_base` | 5 | 5 | 4 | -1 |

### manual.advanced.fence-average-variable

```
# jq
20

# tq -o json
20

# tq
20
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.advanced.fence-destructuring-alternative

```
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

# tq --seq
\x1euser_id: 1
kind: widget
id: 1
ts: 13
\x1euser_id: 1
kind: widget
id: 2
ts: 14
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 62 | 62 | 42 | -20 |
| `cl100k_base` | 62 | 62 | 42 | -20 |

### manual.advanced.fence-destructuring-alternative-variables

```
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

# tq --seq
\x1euser_id: 1
first_user_id: null
kind: widget
id: 1
ts: 13
first_ts: null
\x1euser_id: null
first_user_id: null
kind: widget
id: 2
ts: null
first_ts: null
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 90 | 90 | 62 | -28 |
| `cl100k_base` | 90 | 90 | 62 | -28 |

### manual.advanced.fence-foreach-expanded

```
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

# tq --seq
\x1e[2]: 1,2
\x1e[2]: 2,6
\x1e[2]: 3,12
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 27 | -3 |
| `cl100k_base` | 30 | 30 | 27 | -3 |

### manual.advanced.fence-function-filter-argument

```
# jq
20

# tq -o json
20

# tq
20
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.advanced.fence-function-increment

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.advanced.fence-function-map

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.advanced.fence-post-author-lookup

```
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

# tq --seq
\x1etitle: First post
author: Anonymous Coward
\x1etitle: A well-written article
author: Person McPherson
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 41 | 41 | 27 | -14 |
| `cl100k_base` | 41 | 41 | 27 | -14 |

### manual.advanced.fence-post-author-lookup-out-of-scope

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.advanced.fence-post-author-lookup-parenthesized

```
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

# tq --seq
\x1etitle: First post
author: Anonymous Coward
\x1etitle: A well-written article
author: Person McPherson
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 41 | 41 | 27 | -14 |
| `cl100k_base` | 41 | 41 | 27 | -14 |

### manual.advanced.fence-recursion-library

```
# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.advanced.fence-reduce-expanded

```
# jq
6

# tq -o json
6

# tq
6
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.advanced.prose-addvalue-field

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 18 | 0 |
| `cl100k_base` | 18 | 18 | 18 | 0 |

### manual.advanced.prose-addvalue-iterator

```
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

# tq --seq
\x1e[2]: 2,3
\x1e[2]: 3,4
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 18 | -2 |
| `cl100k_base` | 20 | 20 | 18 | -2 |

### manual.advanced.prose-average-direct

```
# jq
20

# tq -o json
20

# tq
20
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.advanced.prose-object-shorthand

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### manual.advanced.prose-range-generator

```
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

# tq --seq
\x1e0
\x1e1
\x1e2
\x1e3
\x1e4
\x1e5
\x1e6
\x1e7
\x1e8
\x1e9
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 30 | +10 |
| `cl100k_base` | 20 | 20 | 30 | +10 |

### manual.advanced.prose-variable-shorthand

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### manual.advanced.table-001

```
# jq
210

# tq -o json
210

# tq
210
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.advanced.table-002

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual.advanced.table-003

```
# jq
9

# tq -o json
9

# tq
9
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.advanced.table-004

```
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

# tq --seq
\x1ea: 0
b: null
\x1ea: 0
b: 1
\x1ea: 2
b: 1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 47 | 47 | 32 | -15 |
| `cl100k_base` | 47 | 47 | 32 | -15 |

### manual.advanced.table-005

```
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

# tq --seq
\x1ea: 1
b: 2
d: 3
e: 4
\x1ea: 1
b: 2
d: 3
e: 4
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 60 | 60 | 42 | -18 |
| `cl100k_base` | 60 | 60 | 42 | -18 |

### manual.advanced.table-006

```
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

# tq --seq
\x1ea: 1
b: 2
d: 3
e: null
\x1ea: 1
b: 2
d: null
e: 4
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 58 | 58 | 40 | -18 |
| `cl100k_base` | 58 | 58 | 40 | -18 |

### manual.advanced.table-007

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 15 | 15 | 8 | -7 |
| `cl100k_base` | 15 | 15 | 8 | -7 |

### manual.advanced.table-008

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 34 | 34 | 26 | -8 |
| `cl100k_base` | 34 | 34 | 26 | -8 |

### manual.advanced.table-009

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 42 | 42 | 30 | -12 |
| `cl100k_base` | 42 | 42 | 30 | -12 |

### manual.advanced.table-010

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.advanced.table-011

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.advanced.table-012

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.advanced.table-013

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.advanced.table-014

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 17 | -13 |
| `cl100k_base` | 30 | 30 | 17 | -13 |

### manual.advanced.table-015

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.advanced.table-016

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### manual.advanced.table-017

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.advanced.table-018

```
# jq
15

# tq -o json
15

# tq
15
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.advanced.table-019

```
# jq
44

# tq -o json
44

# tq
44
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.advanced.table-020

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 28 | 28 | 14 | -14 |
| `cl100k_base` | 28 | 28 | 14 | -14 |

### manual.advanced.table-021

```
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

# tq --seq
\x1e1
\x1e3
\x1e6
\x1e10
\x1e15
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 15 | +5 |
| `cl100k_base` | 10 | 10 | 15 | +5 |

### manual.advanced.table-022

```
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

# tq --seq
\x1e[2]: 1,2
\x1e[2]: 2,6
\x1e[2]: 3,12
\x1e[2]: 4,20
\x1e[2]: 5,30
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 50 | 50 | 45 | -5 |
| `cl100k_base` | 50 | 50 | 45 | -5 |

### manual.advanced.table-023

```
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

# tq --seq
\x1eindex: 1
item: foo
\x1eindex: 2
item: bar
\x1eindex: 3
item: baz
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 48 | 48 | 30 | -18 |
| `cl100k_base` | 48 | 48 | 30 | -18 |

### manual.advanced.table-024

```
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

# tq --seq
\x1e0
\x1e3
\x1e6
\x1e9
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 12 | +4 |
| `cl100k_base` | 8 | 8 | 12 | +4 |

### manual.advanced.table-025

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 17 | -13 |
| `cl100k_base` | 30 | 30 | 17 | -13 |

### manual.assignment.fence-post-comments

```
# jq
{
  "posts": [
    {
      "title": "First",
      "author": "stedolan",
      "comments": [
        "this is great"
      ]
    },
    {
      "title": "Second",
      "author": "other",
      "comments": [
        "existing",
        "this is great"
      ]
    }
  ]
}

# tq -o json
{
  "posts": [
    {
      "title": "First",
      "author": "stedolan",
      "comments": [
        "this is great"
      ]
    },
    {
      "title": "Second",
      "author": "other",
      "comments": [
        "existing",
        "this is great"
      ]
    }
  ]
}

# tq
posts[2]:
  - title: First
    author: stedolan
    comments[1]: this is great
  - title: Second
    author: other
    comments[2]: existing,this is great
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 76 | 76 | 45 | -31 |
| `cl100k_base` | 76 | 76 | 46 | -30 |

### manual.assignment.fence-post-title

```
# jq
{
  "posts": [
    {
      "title": "JQ Manual",
      "comments": []
    }
  ]
}

# tq -o json
{
  "posts": [
    {
      "title": "JQ Manual",
      "comments": []
    }
  ]
}

# tq
posts[1]:
  - title: JQ Manual
    comments[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 27 | 27 | 17 | -10 |
| `cl100k_base` | 27 | 27 | 17 | -10 |

### manual.assignment.fence-select-comment

```
# jq
{
  "posts": [
    {
      "title": "First",
      "author": "stedolan",
      "comments": [
        "terrible."
      ]
    },
    {
      "title": "Second",
      "author": "other",
      "comments": []
    }
  ]
}

# tq -o json
{
  "posts": [
    {
      "title": "First",
      "author": "stedolan",
      "comments": [
        "terrible."
      ]
    },
    {
      "title": "Second",
      "author": "other",
      "comments": []
    }
  ]
}

# tq
posts[2]:
  - title: First
    author: stedolan
    comments[1]: terrible.
  - title: Second
    author: other
    comments[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 63 | 63 | 39 | -24 |
| `cl100k_base` | 63 | 63 | 40 | -23 |

### manual.assignment.fence-select-posts

```
# jq
{
  "title": "First",
  "author": "stedolan",
  "comments": []
}

# tq -o json
{
  "title": "First",
  "author": "stedolan",
  "comments": []
}

# tq
title: First
author: stedolan
comments[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 13 | -9 |
| `cl100k_base` | 22 | 22 | 14 | -8 |

### manual.assignment.fence-var-plain-error

```
# jq


[stderr]
jq: error: $var is not defined at <top-level>, line 1, column 1:
    $var | .foo =
    ^^^^
jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-RESOLVE-VARIABLE-001: unknown variable $var

# tq


[stderr]
tq: query compilation failed: TQ-RESOLVE-VARIABLE-001: unknown variable $var
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.assignment.fence-var-update-error

```
# jq


[stderr]
jq: error: $var is not defined at <top-level>, line 1, column 1:
    $var |
    ^^^^
jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-RESOLVE-VARIABLE-001: unknown variable $var

# tq


[stderr]
tq: query compilation failed: TQ-RESOLVE-VARIABLE-001: unknown variable $var
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.assignment.prose-copy-field

```
# jq
{
  "foo": [
    1
  ],
  "bar": [
    1
  ]
}

# tq -o json
{
  "foo": [
    1
  ],
  "bar": [
    1
  ]
}

# tq
foo[1]: 1
bar[1]: 1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 24 | 24 | 13 | -11 |
| `cl100k_base` | 24 | 24 | 13 | -11 |

### manual.assignment.prose-empty

```
# jq

# tq -o json

# tq
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.assignment.prose-field

```
# jq
[
  1
]

# tq -o json
[
  1
]

# tq
[1]: 1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 |
| `cl100k_base` | 6 | 6 | 5 | -1 |

### manual.assignment.prose-grouped-plain

```
# jq
{
  "a": 0,
  "b": 0
}

# tq -o json
{
  "a": 0,
  "b": 0
}

# tq
a: 0
b: 0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### manual.assignment.prose-immutable

```
# jq
{
  "a": {
    "b": 3
  }
}
{
  "a": {
    "b": {
      "c": 1
    }
  }
}

# tq -o json
{
  "a": {
    "b": 3
  }
}
{
  "a": {
    "b": {
      "c": 1
    }
  }
}

# tq --seq
\x1ea:
  b: 3
\x1ea:
  b:
    c: 1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 39 | 39 | 21 | -18 |
| `cl100k_base` | 39 | 39 | 21 | -18 |

### manual.assignment.prose-other-field

```
# jq
[
  2
]

# tq -o json
[
  2
]

# tq
[1]: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 |
| `cl100k_base` | 6 | 6 | 5 | -1 |

### manual.assignment.prose-range-plain

```
# jq
{
  "a": 0,
  "b": 0
}
{
  "a": 1,
  "b": 1
}

# tq -o json
{
  "a": 0,
  "b": 0
}
{
  "a": 1,
  "b": 1
}

# tq --seq
\x1ea: 0
b: 0
\x1ea: 1
b: 1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 32 | 32 | 22 | -10 |
| `cl100k_base` | 32 | 32 | 22 | -10 |

### manual.assignment.prose-separate-plain

```
# jq
null
{
  "b": 0
}

# tq -o json
null
{
  "b": 0
}

# tq --seq
\x1enull
\x1eb: 0
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 11 | 11 | 9 | -2 |
| `cl100k_base` | 11 | 11 | 9 | -2 |

### manual.assignment.prose-update-both

```
# jq
{
  "foo": 1,
  "bar": 2
}

# tq -o json
{
  "foo": 1,
  "bar": 2
}

# tq
foo: 1
bar: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### manual.assignment.prose-var-path-error

```
# jq


[stderr]
jq: error: $var is not defined at <top-level>, line 1, column 1:
    $var.foo
    ^^^^
jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-RESOLVE-VARIABLE-001: unknown variable $var

# tq


[stderr]
tq: query compilation failed: TQ-RESOLVE-VARIABLE-001: unknown variable $var
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.assignment.prose-var-plain-error

```
# jq


[stderr]
jq: error: $var is not defined at <top-level>, line 1, column 1:
    $var.foo = 1
    ^^^^
jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-RESOLVE-VARIABLE-001: unknown variable $var

# tq


[stderr]
tq: query compilation failed: TQ-RESOLVE-VARIABLE-001: unknown variable $var
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.assignment.prose-var-update-error

```
# jq


[stderr]
jq: error: $var is not defined at <top-level>, line 1, column 1:
    $var.foo |= . + 1
    ^^^^
jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-RESOLVE-VARIABLE-001: unknown variable $var

# tq


[stderr]
tq: query compilation failed: TQ-RESOLVE-VARIABLE-001: unknown variable $var
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.assignment.table-001

```
# jq
[
  1,
  0,
  [
    5,
    1,
    [
      1,
      [
        0
      ]
    ],
    0
  ]
]

# tq -o json
[
  1,
  0,
  [
    5,
    1,
    [
      1,
      [
        0
      ]
    ],
    0
  ]
]

# tq
[3]:
  - 1
  - 0
  - [4]:
    - 5
    - 1
    - [2]:
      - 1
      - [1]: 0
    - 0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 42 | 42 | 50 | +8 |
| `cl100k_base` | 42 | 42 | 50 | +8 |

### manual.assignment.table-002

```
# jq
{
  "foo": 43
}

# tq -o json
{
  "foo": 43
}

# tq
foo: 43
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### manual.assignment.table-003

```
# jq
{
  "a": 20,
  "b": 20
}

# tq -o json
{
  "a": 20,
  "b": 20
}

# tq
a: 20
b: 20
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### manual.assignment.table-004

```
# jq
{
  "a": 10,
  "b": 20
}

# tq -o json
{
  "a": 10,
  "b": 20
}

# tq
a: 10
b: 20
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### manual.assignment.table-005

```
# jq
{
  "a": 0,
  "b": 0
}
{
  "a": 1,
  "b": 1
}
{
  "a": 2,
  "b": 2
}

# tq -o json
{
  "a": 0,
  "b": 0
}
{
  "a": 1,
  "b": 1
}
{
  "a": 2,
  "b": 2
}

# tq --seq
\x1ea: 0
b: 0
\x1ea: 1
b: 1
\x1ea: 2
b: 2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 48 | 48 | 33 | -15 |
| `cl100k_base` | 48 | 48 | 33 | -15 |

### manual.assignment.table-006

```
# jq
{
  "a": 0,
  "b": 0
}

# tq -o json
{
  "a": 0,
  "b": 0
}

# tq
a: 0
b: 0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### manual.audit.condcomp.boolean-short-circuit

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 5 | -3 |
| `cl100k_base` | 8 | 8 | 5 | -3 |

### manual.audit.condcomp.condition-cardinality

```
# jq
"no"
"no"
"yes"
"yes"
"yes"
"yes"

# tq -o json
"no"
"no"
"yes"
"yes"
"yes"
"yes"

# tq --seq
\x1eno
\x1eno
\x1eyes
\x1eyes
\x1eyes
\x1eyes
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 18 | 0 |
| `cl100k_base` | 18 | 18 | 18 | 0 |

### manual.audit.condcomp.empty-condition

```
# jq

# tq -o json

# tq
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.audit.condcomp.optional-else

```
# jq
[
  {
    "original": true
  },
  {
    "original": true
  }
]

# tq -o json
[
  {
    "original": true
  },
  {
    "original": true
  }
]

# tq
[2]{original}:
  true
  true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 11 | -11 |
| `cl100k_base` | 22 | 22 | 11 | -11 |

### manual.audit.condcomp.typed-errors

```
# jq
[
  {
    "code": 7
  },
  [
    1,
    2
  ]
]

# tq -o json
[
  {
    "code": 7
  },
  [
    1,
    2
  ]
]

# tq
[2]:
  - code: 7
  - [2]: 1,2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 25 | 25 | 19 | -6 |
| `cl100k_base` | 25 | 25 | 19 | -6 |

### manual.audit.documented-arities

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### manual.audit.math.abs-literal-boundary

```
# jq
[
  10,
  1.1,
  0.1,
  1.000
]

# tq -o json
[
  10,
  1.1,
  0.1,
  1.000
]

# tq
[4]: 10,1.1,0.1,1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 24 | 24 | 15 | -9 |
| `cl100k_base` | 24 | 24 | 15 | -9 |

### manual.audit.math.abs-underflow-boundary

```
# jq
[
  "-1E-400",
  "true",
  "1E-400"
]

# tq -o json
[
  "-1E-400",
  "true",
  "1E-400"
]

# tq
[3]: "-1E-400","true","1E-400"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 16 | -4 |
| `cl100k_base` | 20 | 20 | 16 | -4 |

### manual.audit.math.acos-ulp

```
# jq
1.0471975511965979

# tq -o json
1.0471975511965979

# tq
1.0471975511965979
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 8 | -1 |
| `cl100k_base` | 9 | 9 | 8 | -1 |

### manual.audit.math.erfc-ulp

```
# jq
0.004677734981047266

# tq -o json
0.004677734981047266

# tq
0.004677734981047266
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 8 | -1 |
| `cl100k_base` | 9 | 9 | 8 | -1 |

### manual.audit.math.exp-ulp

```
# jq
2.718281828459045

# tq -o json
2.7182818284590455

# tq
2.7182818284590455
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 9 | 8 | 0 |
| `cl100k_base` | 8 | 9 | 8 | 0 |

### manual.audit.math.fmin-fmax-runtime-zero

```
# jq
[
  -0,
  0,
  -0,
  0,
  -0,
  -0,
  -0,
  -0,
  -0,
  -0,
  2,
  2
]

# tq -o json
[
  -0,
  0,
  -0,
  0,
  -0,
  -0,
  -0,
  -0,
  -0,
  -0,
  2,
  2
]

# tq
[12]: 0,0,0,0,0,0,0,0,0,0,2,2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 50 | 50 | 27 | -23 |
| `cl100k_base` | 50 | 50 | 27 | -23 |

### manual.audit.math.integer-scale-boundary

```
# jq
[
  0,
  1.7976931348623157e+308,
  0,
  0,
  0
]

# tq -o json
[
  1.7976931348623157e+308,
  1.7976931348623157e+308,
  0,
  2,
  2
]

# tq
[5]: 1.7976931348623157e+308,1.7976931348623157e+308,0,2,2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 32 | 42 | 33 | +1 |
| `cl100k_base` | 32 | 42 | 33 | +1 |

### manual.audit.math.scalb-infinite-boundary

```
# jq
[
  null,
  0,
  null,
  null
]

# tq -o json
[
  null,
  0,
  null,
  null
]

# tq
[4]: null,0,null,null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 15 | 15 | 8 | -7 |
| `cl100k_base` | 15 | 15 | 8 | -7 |

### manual.audit.math.signed-zero-boundary

```
# jq
[
  [
    [
      -2.5,
      -3,
      -2,
      -2,
      -2,
      -3,
      -2
    ],
    [
      -1.5,
      -2,
      -2,
      -2,
      -1,
      -2,
      -1
    ],
    [
      -0.5,
      -1,
      -0,
      -0,
      -0,
      -1,
      -0
    ],
    [
      0,
      0,
      0,
      0,
      0,
      0,
      0
    ],
    [
      0,
      0,
      0,
      0,
      0,
      0,
      0
    ],
    [
      0.5,
      1,
      0,
      0,
      0,
      0,
      1
    ],
    [
      1.5,
      2,
      2,
      2,
      1,
      1,
      2
    ],
    [
      2.5,
      3,
      2,
      2,
      2,
      2,
      3
    ]
  ],
  [
    -0,
    0,
    0,
    -5e-324,
    5e-324,
    1,
    -1,
    -1
  ]
]

# tq -o json
[
  [
    [
      -2.5,
      -3,
      -2,
      -2,
      -2,
      -3,
      -2
    ],
    [
      -1.5,
      -2,
      -2,
      -2,
      -1,
      -2,
      -1
    ],
    [
      -0.5,
      -1,
      -0,
      -0,
      -0,
      -1,
      -0
    ],
    [
      0,
      0,
      0,
      0,
      0,
      0,
      0
    ],
    [
      0,
      0,
      0,
      0,
      0,
      0,
      0
    ],
    [
      0.5,
      1,
      0,
      0,
      0,
      0,
      1
    ],
    [
      1.5,
      2,
      2,
      2,
      1,
      1,
      2
    ],
    [
      2.5,
      3,
      2,
      2,
      2,
      2,
      3
    ]
  ],
  [
    -0,
    0,
    0,
    -5e-324,
    5e-324,
    1,
    -1,
    -1
  ]
]

# tq
[2]:
  - [8]:
    - [7]: -2.5,-3,-2,-2,-2,-3,-2
    - [7]: -1.5,-2,-2,-2,-1,-2,-1
    - [7]: -0.5,-1,0,0,0,-1,0
    - [7]: 0,0,0,0,0,0,0
    - [7]: 0,0,0,0,0,0,0
    - [7]: 0.5,1,0,0,0,0,1
    - [7]: 1.5,2,2,2,1,1,2
    - [7]: 2.5,3,2,2,2,2,3
  - [8]: 0,0,0,-5e-324,5e-324,1,-1,-1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 316 | 316 | 207 | -109 |
| `cl100k_base` | 316 | 316 | 207 | -109 |

### manual.audit.math.tgamma-ulp

```
# jq
1.772453850905516

# tq -o json
1.7724538509055159

# tq
1.7724538509055159
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 9 | 8 | 0 |
| `cl100k_base` | 8 | 9 | 8 | 0 |

### manual.audit.paths.deletion-prior-error

```
# jq
[
  2
]


[stderr]
jq: error (at <stdin>:0): later

# tq -o json
[
  2
]


[stderr]
tq: runtime error: later

# tq --seq
\x1e[1]: 2


[stderr]
tq: runtime error: later
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 7 | +1 |
| `cl100k_base` | 6 | 6 | 7 | +1 |

### manual.audit.paths.duplicate-deletion

```
# jq
[
  [
    0,
    2,
    3
  ],
  [
    0,
    2,
    3
  ]
]

# tq -o json
[
  [
    0,
    2,
    3
  ],
  [
    0,
    2,
    3
  ]
]

# tq
[2]:
  - [3]: 0,2,3
  - [3]: 0,2,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 34 | 34 | 26 | -8 |
| `cl100k_base` | 34 | 34 | 26 | -8 |

### manual.audit.paths.dynamic-index

```
# jq
[
  "a",
  1
]

# tq -o json
[
  "a",
  1
]

# tq
[2]: a,1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 6 | -4 |
| `cl100k_base` | 10 | 10 | 6 | -4 |

### manual.audit.paths.filter-cardinality

```
# jq
[
  [
    0
  ],
  [
    0
  ]
]

# tq -o json
[
  [
    0
  ],
  [
    0
  ]
]

# tq
[2]:
  - [1]: 0
  - [1]: 0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 18 | 0 |
| `cl100k_base` | 18 | 18 | 18 | 0 |

### manual.audit.paths.filter-early-stop

```
# jq
[
  0
]

# tq -o json
[
  0
]

# tq
[1]: 0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 |
| `cl100k_base` | 6 | 6 | 5 | -1 |

### manual.audit.paths.filtered

```
# jq
[
  1
]

# tq -o json
[
  1
]

# tq
[1]: 1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 |
| `cl100k_base` | 6 | 6 | 5 | -1 |

### manual.audit.paths.negative-index

```
# jq
[
  0,
  1
]

# tq -o json
[
  0,
  1
]

# tq
[2]: 0,1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual.audit.paths.path-prior-error

```
# jq
[
  0
]


[stderr]
jq: error (at <stdin>:0): later

# tq -o json
[
  0
]


[stderr]
tq: runtime error: later

# tq --seq
\x1e[1]: 0


[stderr]
tq: runtime error: later
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 7 | +1 |
| `cl100k_base` | 6 | 6 | 7 | +1 |

### manual.audit.regex.empty-replacement

```
# jq
[
  "a",
  "a"
]

# tq -o json
[
  "a",
  "a"
]

# tq
[2]: a,a
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 5 | -5 |
| `cl100k_base` | 10 | 10 | 5 | -5 |

### manual.audit.regex.gsub-three-arguments

```
# jq
"x"

# tq -o json
"x"

# tq
x
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.audit.regex.no-match-streams

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### manual.audit.regex.null-flags

```
# jq
[
  true,
  true,
  {
    "offset": 0,
    "length": 1,
    "string": "a",
    "captures": []
  },
  {
    "x": "a"
  }
]

# tq -o json
[
  true,
  true,
  {
    "offset": 0,
    "length": 1,
    "string": "a",
    "captures": []
  },
  {
    "x": "a"
  }
]

# tq
[4]:
  - true
  - true
  - offset: 0
    length: 1
    string: a
    captures[0]:
  - x: a
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 49 | 49 | 39 | -10 |
| `cl100k_base` | 49 | 49 | 39 | -10 |

### manual.audit.regex.scan-single-capture

```
# jq
[
  [
    "a"
  ],
  [
    "b"
  ]
]

# tq -o json
[
  [
    "a"
  ],
  [
    "b"
  ]
]

# tq
[2]:
  - [1]: a
  - [1]: b
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 16 | -2 |
| `cl100k_base` | 18 | 18 | 16 | -2 |

### manual.audit.regex.split-literal-overload

```
# jq
[
  [
    "a",
    "b"
  ],
  [
    "",
    "",
    "",
    ""
  ]
]

# tq -o json
[
  [
    "a",
    "b"
  ],
  [
    "",
    "",
    "",
    ""
  ]
]

# tq
[2]:
  - [2]: a,b
  - [4]: "","","",""
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 26 | 26 | 20 | -6 |
| `cl100k_base` | 26 | 26 | 20 | -6 |

### manual.audit.regex.unicode-offsets

```
# jq
{
  "offset": 1,
  "length": 2,
  "string": "é🦀",
  "captures": [
    {
      "offset": 1,
      "length": 2,
      "string": "é🦀",
      "name": "x"
    }
  ]
}

# tq -o json
{
  "offset": 1,
  "length": 2,
  "string": "é🦀",
  "captures": [
    {
      "offset": 1,
      "length": 2,
      "string": "é🦀",
      "name": "x"
    }
  ]
}

# tq
offset: 1
length: 2
string: é🦀
captures[1]{offset,length,string,name}:
  1,2,é🦀,x
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 68 | 68 | 39 | -29 |
| `cl100k_base` | 68 | 68 | 39 | -29 |

### manual.audit.streaming.multiple-reconstruction

```
# jq
1
[]
{}
[
  1,
  [
    2
  ]
]

# tq -o json
1
[]
{}
[
  1,
  [
    2
  ]
]

# tq --seq
\x1e1
\x1e[0]:
\x1e
\x1e[2]:
  - 1
  - [1]: 2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 26 | +8 |
| `cl100k_base` | 18 | 18 | 26 | +8 |

### manual.audit.streaming.scalar-empty-leaves

```
# jq
[
  [
    [],
    null
  ]
]
[
  [
    [],
    false
  ]
]
[
  [
    [],
    2
  ]
]
[
  [
    [],
    "x"
  ]
]
[
  [
    [],
    []
  ]
]
[
  [
    [],
    {}
  ]
]

# tq -o json
[
  [
    [],
    null
  ]
]
[
  [
    [],
    false
  ]
]
[
  [
    [],
    2
  ]
]
[
  [
    [],
    "x"
  ]
]
[
  [
    [],
    []
  ]
]
[
  [
    [],
    {}
  ]
]

# tq --seq
\x1e[1]:
  - [2]:
    - [0]:
    - null
\x1e[1]:
  - [2]:
    - [0]:
    - false
\x1e[1]:
  - [2]:
    - [0]:
    - 2
\x1e[1]:
  - [2]:
    - [0]:
    - x
\x1e[1]:
  - [2]:
    - [0]:
    - [0]:
\x1e[1]:
  - [2]:
    - [0]:
    -
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 66 | 66 | 108 | +42 |
| `cl100k_base` | 66 | 66 | 108 | +42 |

### manual.audit.types.array-empty-generator

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### manual.audit.types.numeric-arithmetic-projection

```
# jq
[
  "9007199254740992",
  "1"
]

# tq -o json
[
  "9007199254740992",
  "1"
]

# tq
[2]: "9007199254740992","1"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 15 | 15 | 13 | -2 |
| `cl100k_base` | 15 | 15 | 13 | -2 |

### manual.audit.types.numeric-literal-preservation

```
# jq
[
  "9007199254740993",
  "1.000",
  "1.00"
]

# tq -o json
[
  "9007199254740993",
  "1.000",
  "1.00"
]

# tq
[3]: "9007199254740993","1.000","1.00"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 23 | 23 | 19 | -4 |
| `cl100k_base` | 23 | 23 | 19 | -4 |

### manual.audit.types.object-cartesian-generators

```
# jq
{
  "a": 1,
  "b": 3
}
{
  "a": 1,
  "b": 4
}
{
  "a": 2,
  "b": 3
}
{
  "a": 2,
  "b": 4
}

# tq -o json
{
  "a": 1,
  "b": 3
}
{
  "a": 1,
  "b": 4
}
{
  "a": 2,
  "b": 3
}
{
  "a": 2,
  "b": 4
}

# tq --seq
\x1ea: 1
b: 3
\x1ea: 1
b: 4
\x1ea: 2
b: 3
\x1ea: 2
b: 4
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 64 | 64 | 44 | -20 |
| `cl100k_base` | 64 | 64 | 44 | -20 |

### manual.audit.types.object-empty-generator

```
# jq

# tq -o json

# tq
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.basic.array-index-negative

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.array-index-out-of-range

```
# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.array-index-zero

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 15 | 15 | 7 | -8 |
| `cl100k_base` | 15 | 15 | 7 | -8 |

### manual.basic.comma-fields

```
# jq
42
"something else"

# tq -o json
42
"something else"

# tq --seq
\x1e42
\x1esomething else
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 7 | +1 |
| `cl100k_base` | 6 | 6 | 7 | +1 |

### manual.basic.comma-indexes

```
# jq
"e"
"c"

# tq -o json
"e"
"c"

# tq --seq
\x1ee
\x1ec
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 |
| `cl100k_base` | 5 | 5 | 6 | +1 |

### manual.basic.comma-user-projects

```
# jq
"stedolan"
"jq"
"wikiflow"

# tq -o json
"stedolan"
"jq"
"wikiflow"

# tq --seq
\x1estedolan
\x1ejq
\x1ewikiflow
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 12 | 0 |
| `cl100k_base` | 12 | 12 | 12 | 0 |

### manual.basic.identity-big-comparison

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.identity-decimal

```
# jq
0.12345678901234567890123456789

# tq -o json
0.12345678901234567890123456789

# tq
0.12345678901234567890123456789
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 13 | 13 | 12 | -1 |
| `cl100k_base` | 13 | 13 | 12 | -1 |

### manual.basic.identity-decimal-forms

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.identity-have-decnum-negative

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.identity-have-decnum-positive

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.identity-hello

```
# jq
"Hello, world!"

# tq -o json
"Hello, world!"

# tq
"Hello, world!"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 5 | 0 |
| `cl100k_base` | 5 | 5 | 5 | 0 |

### manual.basic.identity-large-exponent

```
# jq
1.7976931348623157e+308

# tq -o json
1.7976931348623157e+308

# tq
179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 103 | +91 |
| `cl100k_base` | 12 | 12 | 103 | +91 |

### manual.basic.identity-precision-compare

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.invalid-colon-key

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.basic.iterator-array

```
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

# tq --seq
\x1ename: JSON
good: true
\x1ename: XML
good: false
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 18 | -12 |
| `cl100k_base` | 30 | 30 | 18 | -12 |

### manual.basic.iterator-empty-array

```
# jq

# tq -o json

# tq
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.basic.iterator-field-array

```
# jq
1
2
3

# tq -o json
1
2
3

# tq --seq
\x1e1
\x1e2
\x1e3
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 9 | +3 |
| `cl100k_base` | 6 | 6 | 9 | +3 |

### manual.basic.iterator-object

```
# jq
1
1

# tq -o json
1
1

# tq --seq
\x1e1
\x1e1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.basic.iterator-prose-array

```
# jq
1
2
3

# tq -o json
1
2
3

# tq --seq
\x1e1
\x1e2
\x1e3
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 9 | +3 |
| `cl100k_base` | 6 | 6 | 9 | +3 |

### manual.basic.object-computed-field

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.object-field-missing

```
# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.object-field-present

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.optional-array-field

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### manual.basic.optional-computed-field

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.optional-field-missing

```
# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.optional-field-present

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.parentheses-arithmetic

```
# jq
15

# tq -o json
15

# tq
15
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.pipe-name

```
# jq
"JSON"
"XML"

# tq -o json
"JSON"
"XML"

# tq --seq
\x1eJSON
\x1eXML
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 |
| `cl100k_base` | 6 | 6 | 6 | 0 |

### manual.basic.prose-array-index-last

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.prose-array-index-third

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.prose-colon-key

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.prose-comma-code

```
# jq
1
2

# tq -o json
1
2

# tq --seq
\x1e1
\x1e2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.basic.prose-dot-key

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.prose-identity

```
# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.prose-object-chain

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.prose-object-chain-pipe

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.prose-object-field

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.prose-pipe-chain

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.prose-pipe-chain-expanded

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.prose-pipe-field

```
# jq
1
2

# tq -o json
1
2

# tq --seq
\x1e1
\x1e2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.basic.prose-pipe-identity

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.prose-pipe-identity-equivalent

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.prose-pretty-print

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 3 | -6 |
| `cl100k_base` | 9 | 9 | 3 | -6 |

### manual.basic.prose-slice-example

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 13 | -9 |
| `cl100k_base` | 22 | 22 | 13 | -9 |

### manual.basic.prose-special-key-identifier

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.prose-special-key-index

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.basic.slice-array-middle

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 5 | -5 |
| `cl100k_base` | 10 | 10 | 5 | -5 |

### manual.basic.slice-array-prefix

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 6 | -8 |
| `cl100k_base` | 14 | 14 | 6 | -8 |

### manual.basic.slice-array-suffix

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 5 | -5 |
| `cl100k_base` | 10 | 10 | 5 | -5 |

### manual.basic.slice-string-middle

```
# jq
"cd"

# tq -o json
"cd"

# tq
cd
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual.colors.ansi-values

```
# jq
\x1b[1;36m{\x1b[0m\x1b[2;37m"null"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[1;30mnull\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"false"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[2;31mfalse\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"true"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[4;32mtrue\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"number"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[5;33m1\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"string"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[7;34m"x"\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"array"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[8;35m[]\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"object"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[1;36m{}\x1b[0m\x1b[1;36m}\x1b[0m

# tq -o json
\x1b[1;36m{\x1b[0m\x1b[2;37m"null"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[1;30mnull\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"false"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[2;31mfalse\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"true"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[4;32mtrue\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"number"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[5;33m1\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"string"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[7;34m"x"\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"array"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[8;35m[]\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"object"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[1;36m{}\x1b[0m\x1b[1;36m}\x1b[0m

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 335 | 335 | n/a | n/a |
| `cl100k_base` | 277 | 277 | n/a | n/a |

### manual.colors.custom-1-31

```
# jq
\x1b[1;31m{\x1b[0m\x1b[1;31m"null"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31mnull\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"false"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31mfalse\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"true"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31mtrue\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"number"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m1\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"string"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m"x"\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"array"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m[]\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"object"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m{}\x1b[0m\x1b[1;31m}\x1b[0m

# tq -o json
\x1b[1;31m{\x1b[0m\x1b[1;31m"null"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31mnull\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"false"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31mfalse\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"true"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31mtrue\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"number"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m1\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"string"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m"x"\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"array"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m[]\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"object"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m{}\x1b[0m\x1b[1;31m}\x1b[0m

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 335 | 335 | n/a | n/a |
| `cl100k_base` | 277 | 277 | n/a | n/a |

### manual.colors.default-palette

```
# jq
\x1b[1;39m{\x1b[0m\x1b[1;34m"null"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;90mnull\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"false"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;39mfalse\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"true"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;39mtrue\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"number"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;39m1\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"string"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;32m"x"\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"array"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[1;39m[]\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"object"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[1;39m{}\x1b[0m\x1b[1;39m}\x1b[0m

# tq -o json
\x1b[1;39m{\x1b[0m\x1b[1;34m"null"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;90mnull\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"false"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;39mfalse\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"true"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;39mtrue\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"number"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;39m1\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"string"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;32m"x"\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"array"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[1;39m[]\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"object"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[1;39m{}\x1b[0m\x1b[1;39m}\x1b[0m

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 335 | 335 | n/a | n/a |
| `cl100k_base` | 277 | 277 | n/a | n/a |

### manual.composition.arity.abs.0

```
# jq
[
  10,
  1.1,
  0.1
]

# tq -o json
[
  10,
  1.1,
  0.1
]

# tq
[3]: 10,1.1,0.1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 13 | -5 |
| `cl100k_base` | 18 | 18 | 13 | -5 |

### manual.composition.arity.acos.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.acosh.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.add.0

```
# jq
"abc"

# tq -o json
"abc"

# tq
abc
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual.composition.arity.add.1

```
# jq
8

# tq -o json
8

# tq
8
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.all.0

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.all.1

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.all.2

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.any.0

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.any.1

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.any.2

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.arrays.0

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### manual.composition.arity.ascii-downcase.0

```
# jq
"abcé"

# tq -o json
"abcé"

# tq
abcé
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### manual.composition.arity.ascii-upcase.0

```
# jq
"USEFUL BUT NOT FOR é"

# tq -o json
"USEFUL BUT NOT FOR é"

# tq
USEFUL BUT NOT FOR é
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 6 | -2 |
| `cl100k_base` | 8 | 8 | 6 | -2 |

### manual.composition.arity.asin.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.asinh.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.atan.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.atan2.2

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.atanh.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.booleans.0

```
# jq
false
true

# tq -o json
false
true

# tq --seq
\x1efalse
\x1etrue
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.composition.arity.bsearch.1

```
# jq
[
  1,
  2,
  3,
  4
]

# tq -o json
[
  1,
  2,
  3,
  4
]

# tq
[4]: 1,2,3,4
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 11 | -7 |
| `cl100k_base` | 18 | 18 | 11 | -7 |

### manual.composition.arity.builtins.0

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### manual.composition.arity.capture.1

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 10 | -7 |
| `cl100k_base` | 17 | 17 | 10 | -7 |

### manual.composition.arity.capture.2

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 3 | -6 |
| `cl100k_base` | 9 | 9 | 3 | -6 |

### manual.composition.arity.cbrt.0

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.ceil.0

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.combinations.0

```
# jq
[
  1,
  3
]
[
  1,
  4
]
[
  2,
  3
]
[
  2,
  4
]

# tq -o json
[
  1,
  3
]
[
  1,
  4
]
[
  2,
  3
]
[
  2,
  4
]

# tq --seq
\x1e[2]: 1,3
\x1e[2]: 1,4
\x1e[2]: 2,3
\x1e[2]: 2,4
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 40 | 40 | 36 | -4 |
| `cl100k_base` | 40 | 40 | 36 | -4 |

### manual.composition.arity.combinations.1

```
# jq
[
  0,
  0
]
[
  0,
  1
]
[
  1,
  0
]
[
  1,
  1
]

# tq -o json
[
  0,
  0
]
[
  0,
  1
]
[
  1,
  0
]
[
  1,
  1
]

# tq --seq
\x1e[2]: 0,0
\x1e[2]: 0,1
\x1e[2]: 1,0
\x1e[2]: 1,1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 40 | 40 | 36 | -4 |
| `cl100k_base` | 40 | 40 | 36 | -4 |

### manual.composition.arity.contains.1

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.copysign.2

```
# jq
-2

# tq -o json
-2

# tq
-2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 |
| `cl100k_base` | 3 | 3 | 2 | -1 |

### manual.composition.arity.cos.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.cosh.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.debug.0

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.debug.1

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.del.1

```
# jq
{
  "bar": 9001,
  "baz": 42
}

# tq -o json
{
  "bar": 9001,
  "baz": 42
}

# tq
bar: 9001
baz: 42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 10 | -7 |
| `cl100k_base` | 17 | 17 | 10 | -7 |

### manual.composition.arity.delpaths.1

```
# jq
{
  "a": {},
  "x": {
    "y": 2
  }
}

# tq -o json
{
  "a": {},
  "x": {
    "y": 2
  }
}

# tq
a:
x:
  y: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 21 | 21 | 9 | -12 |
| `cl100k_base` | 21 | 21 | 9 | -12 |

### manual.composition.arity.drem.2

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.empty.0

```
# jq

# tq -o json

# tq
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.composition.arity.endswith.1

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 5 | -3 |
| `cl100k_base` | 8 | 8 | 5 | -3 |

### manual.composition.arity.env.0

```
# jq
"less"

# tq -o json
"less"

# tq
less
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual.composition.arity.erf.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.erfc.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.error.0

```
# jq
"error message"

# tq -o json
"error message"

# tq
error message
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### manual.composition.arity.error.1

```
# jq


[stderr]
jq: error (at <stdin>:0): boom

# tq -o json


[stderr]
tq: runtime error: boom

# tq


[stderr]
tq: runtime error: boom
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.composition.arity.exp.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.exp10.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.exp2.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.explode.0

```
# jq
[
  65,
  233
]

# tq -o json
[
  65,
  233
]

# tq
[2]: 65,233
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual.composition.arity.expm1.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.fabs.0

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.fdim.2

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.finites.0

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### manual.composition.arity.first.0

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.composition.arity.first.1

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.composition.arity.flatten.0

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.composition.arity.flatten.1

```
# jq
[
  1,
  2,
  [
    3
  ]
]

# tq -o json
[
  1,
  2,
  [
    3
  ]
]

# tq
[3]:
  - 1
  - 2
  - [1]: 3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 20 | +2 |
| `cl100k_base` | 18 | 18 | 20 | +2 |

### manual.composition.arity.floor.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.fma.3

```
# jq
10

# tq -o json
10

# tq
10
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.fmax.2

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.fmin.2

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.fmod.2

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.frexp.0

```
# jq
[
  0.5,
  4
]

# tq -o json
[
  0.5,
  4
]

# tq
[2]: 0.5,4
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 9 | -3 |
| `cl100k_base` | 12 | 12 | 9 | -3 |

### manual.composition.arity.from-entries.0

```
# jq
{
  "a": 1,
  "b": 2
}

# tq -o json
{
  "a": 1,
  "b": 2
}

# tq
a: 1
b: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### manual.composition.arity.fromdate.0

```
# jq
1425599507

# tq -o json
1425599507

# tq
1425599507
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 4 | -1 |
| `cl100k_base` | 5 | 5 | 4 | -1 |

### manual.composition.arity.fromdateiso8601.0

```
# jq
1425596400

# tq -o json
1425596400

# tq
1425596400
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 4 | -1 |
| `cl100k_base` | 5 | 5 | 4 | -1 |

### manual.composition.arity.fromjson.0

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### manual.composition.arity.fromstream.1

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 4 | -2 |
| `cl100k_base` | 6 | 6 | 4 | -2 |

### manual.composition.arity.gamma.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.getpath.1

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.gmtime.0

```
# jq
[
  "2015-03-05T23:51:47Z",
  [
    2015,
    2,
    5,
    23,
    51,
    47,
    4,
    63
  ],
  1425599507
]

# tq -o json
[
  "2015-03-05T23:51:47Z",
  [
    2015,
    2,
    5,
    23,
    51,
    47,
    4,
    63
  ],
  1425599507
]

# tq
[3]:
  - "2015-03-05T23:51:47Z"
  - [8]: 2015,2,5,23,51,47,4,63
  - 1425599507
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 62 | 62 | 50 | -12 |
| `cl100k_base` | 62 | 62 | 50 | -12 |

### manual.composition.arity.group-by.1

```
# jq
[
  [
    {
      "k": 1
    }
  ],
  [
    {
      "k": 2
    },
    {
      "k": 2
    }
  ]
]

# tq -o json
[
  [
    {
      "k": 1
    }
  ],
  [
    {
      "k": 2
    },
    {
      "k": 2
    }
  ]
]

# tq
[2]:
  - [1]{k}:
    1
  - [2]{k}:
    2
    2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 43 | 43 | 30 | -13 |
| `cl100k_base` | 43 | 43 | 30 | -13 |

### manual.composition.arity.gsub.2

```
# jq
"+A-+a-"

# tq -o json
"+A-+a-"

# tq
+A-+a-
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 7 | 7 | 5 | -2 |
| `cl100k_base` | 7 | 7 | 5 | -2 |

### manual.composition.arity.gsub.3

```
# jq
"x"

# tq -o json
"x"

# tq
x
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.halt-error.0

```
# jq

# tq -o json

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | n/a | n/a |
| `cl100k_base` | 0 | 0 | n/a | n/a |

### manual.composition.arity.halt-error.1

```
# jq


[stderr]
failure

# tq -o json


[stderr]
failure

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | n/a | n/a |
| `cl100k_base` | 0 | 0 | n/a | n/a |

### manual.composition.arity.halt.0

```
# jq

# tq -o json

# tq
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.composition.arity.has.1

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 5 | -3 |
| `cl100k_base` | 8 | 8 | 5 | -3 |

### manual.composition.arity.have-decnum.0

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.have-literal-numbers.0

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.hypot.2

```
# jq
5

# tq -o json
5

# tq
5
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.implode.0

```
# jq
"Aé"

# tq -o json
"Aé"

# tq
Aé
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 |
| `cl100k_base` | 3 | 3 | 2 | -1 |

### manual.composition.arity.in.1

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.index.1

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.indices.1

```
# jq
[
  3,
  7,
  12
]

# tq -o json
[
  3,
  7,
  12
]

# tq
[3]: 3,7,12
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.composition.arity.infinite.0

```
# jq
"number"
"number"

# tq -o json
"number"
"number"

# tq --seq
\x1enumber
\x1enumber
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 |
| `cl100k_base` | 6 | 6 | 6 | 0 |

### manual.composition.arity.input-filename.0

```
# jq
"<stdin>"

# tq -o json
"<stdin>"

# tq
<stdin>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 3 | 0 |
| `cl100k_base` | 3 | 3 | 3 | 0 |

### manual.composition.arity.input-line-number.0

```
# jq
1
2

# tq -o json
1
2

# tq --seq
\x1e1
\x1e2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.composition.arity.input.0

```
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

# tq --seq
\x1e[2]: 1,2
\x1e[2]: 3,4
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 18 | -2 |
| `cl100k_base` | 20 | 20 | 18 | -2 |

### manual.composition.arity.inputs.0

```
# jq
6

# tq -o json
6

# tq
6
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.inside.1

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.isempty.1

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.isfinite.0

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.isinfinite.0

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.isnan.0

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.isnormal.0

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.iterables.0

```
# jq
[]
{}

# tq -o json
[]
{}

# tq --seq
\x1e[0]:
\x1e
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 6 | +4 |
| `cl100k_base` | 2 | 2 | 6 | +4 |

### manual.composition.arity.j0.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.j1.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.jn.2

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.join.1

```
# jq
"a, b,c,d, e"

# tq -o json
"a, b,c,d, e"

# tq
"a, b,c,d, e"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 8 | 0 |
| `cl100k_base` | 8 | 8 | 8 | 0 |

### manual.composition.arity.keys-unsorted.0

```
# jq
[
  "z",
  "a"
]

# tq -o json
[
  "z",
  "a"
]

# tq
[2]: z,a
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 5 | -5 |
| `cl100k_base` | 10 | 10 | 5 | -5 |

### manual.composition.arity.keys.0

```
# jq
[
  "10",
  "2",
  "a",
  "z"
]

# tq -o json
[
  "10",
  "2",
  "a",
  "z"
]

# tq
[4]: "10","2",a,z
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 10 | -8 |
| `cl100k_base` | 18 | 18 | 10 | -8 |

### manual.composition.arity.last.0

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.composition.arity.last.1

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.composition.arity.ldexp.2

```
# jq
8

# tq -o json
8

# tq
8
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.length.0

```
# jq
[
  2,
  1,
  1,
  0
]

# tq -o json
[
  2,
  1,
  1,
  0
]

# tq
[4]: 2,1,1,0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 11 | -7 |
| `cl100k_base` | 18 | 18 | 11 | -7 |

### manual.composition.arity.lgamma.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.limit.2

```
# jq
0
1

# tq -o json
0
1

# tq --seq
\x1e0
\x1e1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.composition.arity.localtime.0

```
# jq
[
  1970,
  0,
  1,
  0,
  0,
  0,
  4,
  0
]

# tq -o json
[
  1970,
  0,
  1,
  0,
  0,
  0,
  4,
  0
]

# tq
[8]: 1970,0,1,0,0,0,4,0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 35 | 35 | 20 | -15 |
| `cl100k_base` | 35 | 35 | 20 | -15 |

### manual.composition.arity.log.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.log10.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.log1p.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.log2.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.logb.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.ltrim.0

```
# jq
"abc"
"abc "
" abc"

# tq -o json
"abc"
"abc "
" abc"

# tq --seq
\x1eabc
\x1e"abc "
\x1e" abc"
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 11 | +2 |
| `cl100k_base` | 9 | 9 | 11 | +2 |

### manual.composition.arity.ltrimstr.1

```
# jq
"fix"

# tq -o json
"fix"

# tq
fix
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual.composition.arity.map-values.1

```
# jq
{
  "a": 2,
  "b": 3
}

# tq -o json
{
  "a": 2,
  "b": 3
}

# tq
a: 2
b: 3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### manual.composition.arity.map.1

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.composition.arity.match.1

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 28 | 28 | 18 | -10 |
| `cl100k_base` | 28 | 28 | 18 | -10 |

### manual.composition.arity.match.2

```
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

# tq --seq
\x1eoffset: 0
length: 3
string: abc
captures[1]{offset,length,string,name}:
  0,3,abc,null
\x1eoffset: 4
length: 3
string: abc
captures[1]{offset,length,string,name}:
  4,3,abc,null
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 122 | 122 | 68 | -54 |
| `cl100k_base` | 122 | 122 | 68 | -54 |

### manual.composition.arity.max-by.1

```
# jq
{
  "k": 2
}

# tq -o json
{
  "k": 2
}

# tq
k: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### manual.composition.arity.max.0

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.min-by.1

```
# jq
{
  "k": 1
}

# tq -o json
{
  "k": 1
}

# tq
k: 1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### manual.composition.arity.min.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.mktime.0

```
# jq
[
  "2015-03-05T23:51:47Z",
  [
    2015,
    2,
    5,
    23,
    51,
    47,
    4,
    63
  ],
  1425599507
]

# tq -o json
[
  "2015-03-05T23:51:47Z",
  [
    2015,
    2,
    5,
    23,
    51,
    47,
    4,
    63
  ],
  1425599507
]

# tq
[3]:
  - "2015-03-05T23:51:47Z"
  - [8]: 2015,2,5,23,51,47,4,63
  - 1425599507
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 62 | 62 | 50 | -12 |
| `cl100k_base` | 62 | 62 | 50 | -12 |

### manual.composition.arity.modf.0

```
# jq
[
  0.5,
  3
]

# tq -o json
[
  0.5,
  3
]

# tq
[2]: 0.5,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 9 | -3 |
| `cl100k_base` | 12 | 12 | 9 | -3 |

### manual.composition.arity.modulemeta.0

```
# jq
{
  "homepage": "https://example.invalid/basic",
  "deps": [],
  "defs": [
    "value/0"
  ]
}

# tq -o json
{
  "homepage": "https://example.invalid/basic",
  "deps": [],
  "defs": [
    "value/0"
  ]
}

# tq
homepage: "https://example.invalid/basic"
deps[0]:
defs[1]: value/0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 31 | 31 | 20 | -11 |
| `cl100k_base` | 31 | 31 | 20 | -11 |

### manual.composition.arity.nan.0

```
# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.nearbyint.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.nextafter.2

```
# jq
1.0000000000000002

# tq -o json
1.0000000000000002

# tq
1.0000000000000002
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 8 | -1 |
| `cl100k_base` | 9 | 9 | 8 | -1 |

### manual.composition.arity.nexttoward.2

```
# jq
1.0000000000000002

# tq -o json
1.0000000000000002

# tq
1.0000000000000002
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 8 | -1 |
| `cl100k_base` | 9 | 9 | 8 | -1 |

### manual.composition.arity.normals.0

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### manual.composition.arity.not.0

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.now.0

```
# jq
"number"

# tq -o json
"number"

# tq
number
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual.composition.arity.nth.1

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.composition.arity.nth.2

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.composition.arity.nulls.0

```
# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.numbers.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.objects.0

```
# jq
{}

# tq -o json
{}

# tq
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 0 | -1 |
| `cl100k_base` | 1 | 1 | 0 | -1 |

### manual.composition.arity.path.1

```
# jq
[
  "a",
  0
]

# tq -o json
[
  "a",
  0
]

# tq
[2]: a,0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 6 | -4 |
| `cl100k_base` | 10 | 10 | 6 | -4 |

### manual.composition.arity.paths.0

```
# jq
[
  [
    0
  ],
  [
    1
  ],
  [
    1,
    0
  ],
  [
    1,
    1
  ],
  [
    1,
    1,
    "a"
  ]
]

# tq -o json
[
  [
    0
  ],
  [
    1
  ],
  [
    1,
    0
  ],
  [
    1,
    1
  ],
  [
    1,
    1,
    "a"
  ]
]

# tq
[5]:
  - [1]: 0
  - [1]: 1
  - [2]: 1,0
  - [2]: 1,1
  - [3]: 1,1,a
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 58 | 58 | 49 | -9 |
| `cl100k_base` | 58 | 58 | 49 | -9 |

### manual.composition.arity.paths.1

```
# jq
[
  [
    0
  ],
  [
    1,
    1,
    "a"
  ]
]

# tq -o json
[
  [
    0
  ],
  [
    1,
    1,
    "a"
  ]
]

# tq
[2]:
  - [1]: 0
  - [3]: 1,1,a
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 26 | 26 | 21 | -5 |
| `cl100k_base` | 26 | 26 | 21 | -5 |

### manual.composition.arity.pick.1

```
# jq
{
  "a": 1,
  "b": {
    "c": 2
  },
  "x": null
}

# tq -o json
{
  "a": 1,
  "b": {
    "c": 2
  },
  "x": null
}

# tq
a: 1
b:
  c: 2
x: null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 29 | 29 | 16 | -13 |
| `cl100k_base` | 29 | 29 | 16 | -13 |

### manual.composition.arity.pow.2

```
# jq
8

# tq -o json
8

# tq
8
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.range.1

```
# jq
[
  0,
  1,
  2,
  3
]

# tq -o json
[
  0,
  1,
  2,
  3
]

# tq
[4]: 0,1,2,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 11 | -7 |
| `cl100k_base` | 18 | 18 | 11 | -7 |

### manual.composition.arity.range.2

```
# jq
2
3

# tq -o json
2
3

# tq --seq
\x1e2
\x1e3
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.composition.arity.range.3

```
# jq
[
  0,
  3,
  6,
  9
]

# tq -o json
[
  0,
  3,
  6,
  9
]

# tq
[4]: 0,3,6,9
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 11 | -7 |
| `cl100k_base` | 18 | 18 | 11 | -7 |

### manual.composition.arity.recurse.0

```
# jq
{
  "a": 0,
  "b": [
    1
  ]
}
0
[
  1
]
1

# tq -o json
{
  "a": 0,
  "b": [
    1
  ]
}
0
[
  1
]
1

# tq --seq
\x1ea: 0
b[1]: 1
\x1e0
\x1e[1]: 1
\x1e1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 26 | -4 |
| `cl100k_base` | 30 | 30 | 26 | -4 |

### manual.composition.arity.recurse.1

```
# jq
{
  "foo": [
    {
      "foo": []
    },
    {
      "foo": [
        {
          "foo": []
        }
      ]
    }
  ]
}
{
  "foo": []
}
{
  "foo": [
    {
      "foo": []
    }
  ]
}
{
  "foo": []
}

# tq -o json
{
  "foo": [
    {
      "foo": []
    },
    {
      "foo": [
        {
          "foo": []
        }
      ]
    }
  ]
}
{
  "foo": []
}
{
  "foo": [
    {
      "foo": []
    }
  ]
}
{
  "foo": []
}

# tq --seq
\x1efoo[2]:
  - foo[0]:
  - foo[1]:
      - foo[0]:
\x1efoo[0]:
\x1efoo[1]:
  - foo[0]:
\x1efoo[0]:
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 70 | 70 | 44 | -26 |
| `cl100k_base` | 70 | 70 | 44 | -26 |

### manual.composition.arity.recurse.2

```
# jq
2
4
16

# tq -o json
2
4
16

# tq --seq
\x1e2
\x1e4
\x1e16
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 9 | +3 |
| `cl100k_base` | 6 | 6 | 9 | +3 |

### manual.composition.arity.remainder.2

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.repeat.1

```
# jq
[
  2
]

# tq -o json
[
  2
]

# tq
[1]: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 |
| `cl100k_base` | 6 | 6 | 5 | -1 |

### manual.composition.arity.reverse.0

```
# jq
[
  3,
  2,
  1
]

# tq -o json
[
  3,
  2,
  1
]

# tq
[3]: 3,2,1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.composition.arity.rindex.1

```
# jq
12

# tq -o json
12

# tq
12
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.rint.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.round.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.rtrim.0

```
# jq
"abc"
"abc "
" abc"

# tq -o json
"abc"
"abc "
" abc"

# tq --seq
\x1eabc
\x1e"abc "
\x1e" abc"
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 11 | +2 |
| `cl100k_base` | 9 | 9 | 11 | +2 |

### manual.composition.arity.rtrimstr.1

```
# jq
[
  "fo",
  "",
  "bar",
  "foobar",
  "foob"
]

# tq -o json
[
  "fo",
  "",
  "bar",
  "foobar",
  "foob"
]

# tq
[5]: fo,"",bar,foobar,foob
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 21 | 21 | 12 | -9 |
| `cl100k_base` | 21 | 21 | 12 | -9 |

### manual.composition.arity.scalars.0

```
# jq
null
false
0
"x"

# tq -o json
null
false
0
"x"

# tq --seq
\x1enull
\x1efalse
\x1e0
\x1ex
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 12 | +4 |
| `cl100k_base` | 8 | 8 | 12 | +4 |

### manual.composition.arity.scalb.2

```
# jq
[
  null,
  0,
  null,
  null
]

# tq -o json
[
  null,
  0,
  null,
  null
]

# tq
[4]: null,0,null,null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 15 | 15 | 8 | -7 |
| `cl100k_base` | 15 | 15 | 8 | -7 |

### manual.composition.arity.scalbln.2

```
# jq
[
  0,
  1.7976931348623157e+308,
  0,
  0,
  0
]

# tq -o json
[
  1.7976931348623157e+308,
  1.7976931348623157e+308,
  0,
  2,
  2
]

# tq
[5]: 1.7976931348623157e+308,1.7976931348623157e+308,0,2,2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 32 | 42 | 33 | +1 |
| `cl100k_base` | 32 | 42 | 33 | +1 |

### manual.composition.arity.scan.1

```
# jq
"c"
"c"

# tq -o json
"c"
"c"

# tq --seq
\x1ec
\x1ec
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.composition.arity.scan.2

```
# jq
"a"

# tq -o json
"a"

# tq
a
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.select.1

```
# jq
2
4

# tq -o json
2
4

# tq --seq
\x1e2
\x1e4
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.composition.arity.setpath.2

```
# jq
{
  "a": [
    null,
    7
  ]
}

# tq -o json
{
  "a": [
    null,
    7
  ]
}

# tq
a[2]: null,7
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 7 | -9 |
| `cl100k_base` | 16 | 16 | 7 | -9 |

### manual.composition.arity.significand.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.sin.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.sinh.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.skip.2

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 17 | -13 |
| `cl100k_base` | 30 | 30 | 17 | -13 |

### manual.composition.arity.sort-by.1

```
# jq
[
  {
    "n": 1
  },
  {
    "n": 2
  },
  {
    "n": 2
  }
]

# tq -o json
[
  {
    "n": 1
  },
  {
    "n": 2
  },
  {
    "n": 2
  }
]

# tq
[3]{n}:
  1
  2
  2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 35 | 35 | 17 | -18 |
| `cl100k_base` | 35 | 35 | 17 | -18 |

### manual.composition.arity.sort.0

```
# jq
[
  null,
  false,
  3,
  "x",
  []
]

# tq -o json
[
  null,
  false,
  3,
  "x",
  []
]

# tq
[5]:
  - null
  - false
  - 3
  - x
  - [0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 25 | +7 |
| `cl100k_base` | 18 | 18 | 25 | +7 |

### manual.composition.arity.split.1

```
# jq
[
  "a",
  "b,c,d",
  "e",
  ""
]

# tq -o json
[
  "a",
  "b,c,d",
  "e",
  ""
]

# tq
[4]: a,"b,c,d",e,""
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 12 | -6 |
| `cl100k_base` | 18 | 18 | 12 | -6 |

### manual.composition.arity.split.2

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 8 | -6 |
| `cl100k_base` | 14 | 14 | 8 | -6 |

### manual.composition.arity.splits.1

```
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

# tq --seq
\x1eab
\x1ecd
\x1eef
\x1egh
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 12 | 0 |
| `cl100k_base` | 12 | 12 | 12 | 0 |

### manual.composition.arity.splits.2

```
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

# tq --seq
\x1eab
\x1ecd
\x1eef
\x1egh
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 12 | 0 |
| `cl100k_base` | 12 | 12 | 12 | 0 |

### manual.composition.arity.sqrt.0

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.startswith.1

```
# jq
[
  false,
  true,
  false,
  true,
  false
]

# tq -o json
[
  false,
  true,
  false,
  true,
  false
]

# tq
[5]: false,true,false,true,false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 8 | -9 |
| `cl100k_base` | 17 | 17 | 8 | -9 |

### manual.composition.arity.stderr.0

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual.composition.arity.strflocaltime.1

```
# jq
"2015"

# tq -o json
"2015"

# tq
"2015"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 |
| `cl100k_base` | 4 | 4 | 4 | 0 |

### manual.composition.arity.strftime.1

```
# jq
"2015-03-05T23:51:47Z"

# tq -o json
"2015-03-05T23:51:47Z"

# tq
"2015-03-05T23:51:47Z"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 15 | 15 | 15 | 0 |
| `cl100k_base` | 15 | 15 | 15 | 0 |

### manual.composition.arity.strings.0

```
# jq
"x"

# tq -o json
"x"

# tq
x
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.strptime.1

```
# jq
[
  2015,
  2,
  5,
  23,
  51,
  47,
  4,
  63
]

# tq -o json
[
  2015,
  2,
  5,
  23,
  51,
  47,
  4,
  63
]

# tq
[8]: 2015,2,5,23,51,47,4,63
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 35 | 35 | 20 | -15 |
| `cl100k_base` | 35 | 35 | 20 | -15 |

### manual.composition.arity.sub.2

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 11 | 11 | 6 | -5 |
| `cl100k_base` | 11 | 11 | 6 | -5 |

### manual.composition.arity.sub.3

```
# jq
"ZabcZdef"

# tq -o json
"ZabcZdef"

# tq
ZabcZdef
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 4 | -2 |
| `cl100k_base` | 6 | 6 | 4 | -2 |

### manual.composition.arity.tan.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.tanh.0

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.test.1

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.test.2

```
# jq
true
true

# tq -o json
true
true

# tq --seq
\x1etrue
\x1etrue
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.composition.arity.tgamma.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.to-entries.0

```
# jq
[
  {
    "key": "z",
    "value": 1
  },
  {
    "key": "a",
    "value": 2
  }
]

# tq -o json
[
  {
    "key": "z",
    "value": 1
  },
  {
    "key": "a",
    "value": 2
  }
]

# tq
[2]{key,value}:
  z,1
  a,2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 38 | 38 | 16 | -22 |
| `cl100k_base` | 38 | 38 | 16 | -22 |

### manual.composition.arity.toboolean.0

```
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

# tq --seq
\x1etrue
\x1efalse
\x1etrue
\x1efalse
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 12 | +4 |
| `cl100k_base` | 8 | 8 | 12 | +4 |

### manual.composition.arity.todate.0

```
# jq
"1970-01-01T00:00:00Z"

# tq -o json
"1970-01-01T00:00:00Z"

# tq
"1970-01-01T00:00:00Z"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 15 | 15 | 15 | 0 |
| `cl100k_base` | 15 | 15 | 15 | 0 |

### manual.composition.arity.todateiso8601.0

```
# jq
[
  "2015-03-05T23:51:47Z",
  [
    2015,
    2,
    5,
    23,
    51,
    47,
    4,
    63
  ],
  1425599507
]

# tq -o json
[
  "2015-03-05T23:51:47Z",
  [
    2015,
    2,
    5,
    23,
    51,
    47,
    4,
    63
  ],
  1425599507
]

# tq
[3]:
  - "2015-03-05T23:51:47Z"
  - [8]: 2015,2,5,23,51,47,4,63
  - 1425599507
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 62 | 62 | 50 | -12 |
| `cl100k_base` | 62 | 62 | 50 | -12 |

### manual.composition.arity.tojson.0

```
# jq
"{\"a\":1}"

# tq -o json
"{\"a\":1}"

# tq
"{\"a\":1}"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 |
| `cl100k_base` | 6 | 6 | 6 | 0 |

### manual.composition.arity.tonumber.0

```
# jq
[
  -2,
  1.5,
  1E+3
]

# tq -o json
[
  -2,
  1.5,
  1E+3
]

# tq
[3]: -2,1.5,1000
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 19 | 19 | 12 | -7 |
| `cl100k_base` | 19 | 19 | 12 | -7 |

### manual.composition.arity.tostream.0

```
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

# tq --seq
\x1e[2]:
  - [2]: a,0
  - 1
\x1e[1]:
  - [2]: a,0
\x1e[1]:
  - [1]: a
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 42 | 42 | 42 | 0 |
| `cl100k_base` | 42 | 42 | 42 | 0 |

### manual.composition.arity.tostring.0

```
# jq
[
  "null",
  "true",
  "1",
  "x",
  "[2]"
]

# tq -o json
[
  "null",
  "true",
  "1",
  "x",
  "[2]"
]

# tq
[5]: "null","true","1",x,"[2]"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 15 | -7 |
| `cl100k_base` | 22 | 22 | 15 | -7 |

### manual.composition.arity.transpose.0

```
# jq
[
  [
    1,
    2
  ],
  [
    null,
    3
  ]
]

# tq -o json
[
  [
    1,
    2
  ],
  [
    null,
    3
  ]
]

# tq
[2]:
  - [2]: 1,2
  - [2]: null,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 25 | 25 | 21 | -4 |
| `cl100k_base` | 25 | 25 | 21 | -4 |

### manual.composition.arity.trim.0

```
# jq
"abc"
"abc "
" abc"

# tq -o json
"abc"
"abc "
" abc"

# tq --seq
\x1eabc
\x1e"abc "
\x1e" abc"
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 11 | +2 |
| `cl100k_base` | 9 | 9 | 11 | +2 |

### manual.composition.arity.trimstr.1

```
# jq
[
  "fo",
  "",
  "bar",
  "bar",
  "b"
]

# tq -o json
[
  "fo",
  "",
  "bar",
  "bar",
  "b"
]

# tq
[5]: fo,"",bar,bar,b
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 10 | -10 |
| `cl100k_base` | 20 | 20 | 10 | -10 |

### manual.composition.arity.trunc.0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.truncate-stream.1

```
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

# tq --seq
\x1e[2]:
  - [1]: 0
  - b
\x1e[1]:
  - [1]: 0
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 24 | 24 | 28 | +4 |
| `cl100k_base` | 24 | 24 | 28 | +4 |

### manual.composition.arity.type.0

```
# jq
[
  "null",
  "boolean",
  "number",
  "string",
  "array",
  "object"
]

# tq -o json
[
  "null",
  "boolean",
  "number",
  "string",
  "array",
  "object"
]

# tq
[6]: "null",boolean,number,string,array,object
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 26 | 26 | 13 | -13 |
| `cl100k_base` | 26 | 26 | 12 | -14 |

### manual.composition.arity.unique-by.1

```
# jq
[
  {
    "a": 1,
    "b": 1,
    "id": "first"
  },
  {
    "a": 1,
    "b": 2,
    "id": "distinct"
  }
]

# tq -o json
[
  {
    "a": 1,
    "b": 1,
    "id": "first"
  },
  {
    "a": 1,
    "b": 2,
    "id": "distinct"
  }
]

# tq
[2]{a,b,id}:
  1,1,first
  1,2,distinct
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 52 | 52 | 23 | -29 |
| `cl100k_base` | 52 | 52 | 24 | -28 |

### manual.composition.arity.unique.0

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.composition.arity.until.2

```
# jq
24

# tq -o json
24

# tq
24
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.upper-in.1

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.upper-in.2

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.upper-index.2

```
# jq
{
  "a": {
    "id": "a",
    "v": 1
  },
  "b": {
    "id": "b",
    "v": 2
  }
}

# tq -o json
{
  "a": {
    "id": "a",
    "v": 1
  },
  "b": {
    "id": "b",
    "v": 2
  }
}

# tq
a:
  id: a
  v: 1
b:
  id: b
  v: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 44 | 44 | 25 | -19 |
| `cl100k_base` | 44 | 44 | 25 | -19 |

### manual.composition.arity.upper-join.2

```
# jq


[stderr]
jq: error (at <stdin>:0): Cannot index string with string "id"

# tq -o json


[stderr]
tq: runtime error: field access cannot be applied to string

# tq


[stderr]
tq: runtime error: field access cannot be applied to string
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.composition.arity.upper-join.3

```
# jq
[
  {
    "id": "a",
    "v": 1
  },
  {
    "id": "a",
    "v": 1
  }
]
[
  {
    "id": "b",
    "v": 2
  },
  {
    "id": "b",
    "v": 2
  }
]

# tq -o json
[
  {
    "id": "a",
    "v": 1
  },
  {
    "id": "a",
    "v": 1
  }
]
[
  {
    "id": "b",
    "v": 2
  },
  {
    "id": "b",
    "v": 2
  }
]

# tq --seq
\x1e[2]{id,v}:
  a,1
  a,1
\x1e[2]{id,v}:
  b,2
  b,2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 76 | 76 | 36 | -40 |
| `cl100k_base` | 76 | 76 | 36 | -40 |

### manual.composition.arity.upper-join.4

```
# jq
[
  {
    "id": "a",
    "v": 1
  },
  {
    "id": "a",
    "v": 1
  }
]
[
  {
    "id": "b",
    "v": 2
  },
  {
    "id": "b",
    "v": 2
  }
]

# tq -o json
[
  {
    "id": "a",
    "v": 1
  },
  {
    "id": "a",
    "v": 1
  }
]
[
  {
    "id": "b",
    "v": 2
  },
  {
    "id": "b",
    "v": 2
  }
]

# tq --seq
\x1e[2]{id,v}:
  a,1
  a,1
\x1e[2]{id,v}:
  b,2
  b,2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 76 | 76 | 36 | -40 |
| `cl100k_base` | 76 | 76 | 36 | -40 |

### manual.composition.arity.utf8bytelength.0

```
# jq
6

# tq -o json
6

# tq
6
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.arity.values.0

```
# jq
false
0
"x"

# tq -o json
false
0
"x"

# tq --seq
\x1efalse
\x1e0
\x1ex
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 9 | +3 |
| `cl100k_base` | 6 | 6 | 9 | +3 |

### manual.composition.arity.walk.1

```
# jq
[
  [
    1,
    4,
    7
  ],
  [
    2,
    5,
    8
  ],
  [
    3,
    6,
    9
  ]
]

# tq -o json
[
  [
    1,
    4,
    7
  ],
  [
    2,
    5,
    8
  ],
  [
    3,
    6,
    9
  ]
]

# tq
[3]:
  - [3]: 1,4,7
  - [3]: 2,5,8
  - [3]: 3,6,9
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 50 | 50 | 38 | -12 |
| `cl100k_base` | 50 | 50 | 38 | -12 |

### manual.composition.arity.while.2

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 17 | -13 |
| `cl100k_base` | 30 | 30 | 17 | -13 |

### manual.composition.arity.with-entries.1

```
# jq
{
  "a": 2,
  "b": 3
}

# tq -o json
{
  "a": 2,
  "b": 3
}

# tq
a: 2
b: 3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### manual.composition.arity.y0.0

```
# jq
0.08825696421567698

# tq -o json
0.08825696421567697

# tq
0.08825696421567697
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 8 | -1 |
| `cl100k_base` | 9 | 9 | 8 | -1 |

### manual.composition.arity.y1.0

```
# jq
-0.7812128213002887

# tq -o json
-0.7812128213002887

# tq
-0.7812128213002887
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 9 | -1 |
| `cl100k_base` | 10 | 10 | 9 | -1 |

### manual.composition.arity.yn.2

```
# jq
0.08825696421567698

# tq -o json
0.08825696421567697

# tq
0.08825696421567697
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 8 | -1 |
| `cl100k_base` | 9 | 9 | 8 | -1 |

### manual.composition.inventory.bindings.around

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.bindings.def

```
# jq
[
  1,
  null
]

# tq -o json
[
  1,
  null
]

# tq
[2]: 1,null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 6 | -3 |
| `cl100k_base` | 9 | 9 | 6 | -3 |

### manual.composition.inventory.bindings.filter

```
# jq
[
  1
]

# tq -o json
[
  1
]

# tq
[1]: 1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 |
| `cl100k_base` | 6 | 6 | 5 | -1 |

### manual.composition.inventory.bindings.value

```
# jq
[
  1,
  1
]

# tq -o json
[
  1,
  1
]

# tq
[2]: 1,1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual.composition.inventory.callbacks.around

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.callbacks.def

```
# jq
[
  2,
  3
]

# tq -o json
[
  2,
  3
]

# tq
[2]: 2,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual.composition.inventory.callbacks.filter

```
# jq
[
  2,
  3
]

# tq -o json
[
  2,
  3
]

# tq
[2]: 2,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual.composition.inventory.callbacks.value

```
# jq
[
  11,
  12
]

# tq -o json
[
  11,
  12
]

# tq
[2]: 11,12
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual.composition.inventory.constructors.around

```
# jq
{
  "head": 1
}

# tq -o json
{
  "head": 1
}

# tq
head: 1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### manual.composition.inventory.constructors.def

```
# jq
{
  "head": 1,
  "whole": [
    1,
    2
  ]
}

# tq -o json
{
  "head": 1,
  "whole": [
    1,
    2
  ]
}

# tq
head: 1
whole[2]: 1,2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 24 | 24 | 13 | -11 |
| `cl100k_base` | 24 | 24 | 13 | -11 |

### manual.composition.inventory.constructors.filter

```
# jq
[
  {
    "value": 1
  }
]

# tq -o json
[
  {
    "value": 1
  }
]

# tq
[1]{value}:
  1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 13 | 13 | 9 | -4 |
| `cl100k_base` | 13 | 13 | 9 | -4 |

### manual.composition.inventory.constructors.value

```
# jq
{
  "value": [
    1,
    2
  ],
  "first": 1
}

# tq -o json
{
  "value": [
    1,
    2
  ],
  "first": 1
}

# tq
value[2]: 1,2
first: 1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 24 | 24 | 13 | -11 |
| `cl100k_base` | 24 | 24 | 13 | -11 |

### manual.composition.inventory.effects.around

```
# jq
"1"


[stderr]
["DEBUG:",1]

# tq -o json
"1"


[stderr]
["DEBUG:",1]

# tq
"1"

[stderr]
["DEBUG:",1]
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 3 | 0 |
| `cl100k_base` | 3 | 3 | 3 | 0 |

### manual.composition.inventory.effects.def

```
# jq
1


[stderr]
["DEBUG:",1]

# tq -o json
1


[stderr]
["DEBUG:",1]

# tq
1

[stderr]
["DEBUG:",1]
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.effects.filter

```
# jq
1


[stderr]
["DEBUG:",1]

# tq -o json
1


[stderr]
["DEBUG:",1]

# tq
1

[stderr]
["DEBUG:",1]
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.effects.value

```
# jq
"x"


[stderr]
["DEBUG:","x"]

# tq -o json
"x"


[stderr]
["DEBUG:","x"]

# tq
x

[stderr]
["DEBUG:","x"]
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.errors.around

```
# jq
"x"

# tq -o json
"x"

# tq
x
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.errors.def

```
# jq
"x"

# tq -o json
"x"

# tq
x
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.errors.filter

```
# jq
"x"

# tq -o json
"x"

# tq
x
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.errors.value

```
# jq
"x"

# tq -o json
"x"

# tq
x
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.folds.around

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.folds.def

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.folds.filter

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.folds.value

```
# jq
[
  1,
  3
]

# tq -o json
[
  1,
  3
]

# tq
[2]: 1,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual.composition.inventory.generators.around

```
# jq
10
11
12

# tq -o json
10
11
12

# tq --seq
\x1e10
\x1e11
\x1e12
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 9 | +3 |
| `cl100k_base` | 6 | 6 | 9 | +3 |

### manual.composition.inventory.generators.def

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.composition.inventory.generators.filter

```
# jq
[
  0,
  1
]

# tq -o json
[
  0,
  1
]

# tq
[2]: 0,1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual.composition.inventory.generators.value

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.composition.inventory.math.around

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.math.def

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.math.filter

```
# jq
[
  0,
  0
]

# tq -o json
[
  0,
  0
]

# tq
[2]: 0,0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual.composition.inventory.math.value

```
# jq
4

# tq -o json
4

# tq
4
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.navigation.around

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.navigation.def

```
# jq
"a"
"b"

# tq -o json
"a"
"b"

# tq --seq
\x1ea
\x1eb
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 6 | +1 |
| `cl100k_base` | 5 | 5 | 6 | +1 |

### manual.composition.inventory.navigation.filter

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 5 | -5 |
| `cl100k_base` | 10 | 10 | 5 | -5 |

### manual.composition.inventory.navigation.value

```
# jq
"a"

# tq -o json
"a"

# tq
a
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.paths.around

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.paths.def

```
# jq
{
  "a": 2
}

# tq -o json
{
  "a": 2
}

# tq
a: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### manual.composition.inventory.paths.filter

```
# jq
{
  "a": 2
}

# tq -o json
{
  "a": 2
}

# tq
a: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### manual.composition.inventory.paths.value

```
# jq
{
  "a": 2,
  "b": 2
}

# tq -o json
{
  "a": 2,
  "b": 2
}

# tq
a: 2
b: 2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### manual.composition.inventory.regex.around

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.regex.def

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.regex.filter

```
# jq
"xbx"

# tq -o json
"xbx"

# tq
xbx
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### manual.composition.inventory.regex.value

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.scalar-control.around

```
# jq
4

# tq -o json
4

# tq
4
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.scalar-control.def

```
# jq
4

# tq -o json
4

# tq
4
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.scalar-control.filter

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.inventory.scalar-control.value

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.accessfield

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.accessindex

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.array

```
# jq
[
  1
]

# tq -o json
[
  1
]

# tq
[1]: 1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 |
| `cl100k_base` | 6 | 6 | 5 | -1 |

### manual.composition.operation.assignment

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### manual.composition.operation.binary

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.bind

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.bindalternatives

```
# jq
[
  1,
  null
]

# tq -o json
[
  1,
  null
]

# tq
[2]: 1,null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 6 | -3 |
| `cl100k_base` | 9 | 9 | 6 | -3 |

### manual.composition.operation.break

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.call

```
# jq
[
  1,
  2
]

# tq -o json
[
  1,
  2
]

# tq
[2]: 1,2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual.composition.operation.comma

```
# jq
1
1

# tq -o json
1
1

# tq --seq
\x1e1
\x1e1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.composition.operation.conditional

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.empty

```
# jq

# tq -o json

# tq
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.composition.operation.foreach

```
# jq
[
  1,
  3
]

# tq -o json
[
  1,
  3
]

# tq
[2]: 1,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual.composition.operation.identity

```
# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.interpolation

```
# jq
"1"

# tq -o json
"1"

# tq
"1"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 3 | 0 |
| `cl100k_base` | 3 | 3 | 3 | 0 |

### manual.composition.operation.iterate

```
# jq
1
2

# tq -o json
1
2

# tq --seq
\x1e1
\x1e2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.composition.operation.label

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.literal

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.object

```
# jq
{
  "value": 1
}

# tq -o json
{
  "value": 1
}

# tq
value: 1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### manual.composition.operation.optional

```
# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.parametercall

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.pipe

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.recursivedescent

```
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

# tq --seq
\x1ea[1]: 1
\x1e[1]: 1
\x1e1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 21 | 21 | 18 | -3 |
| `cl100k_base` | 21 | 21 | 18 | -3 |

### manual.composition.operation.reduce

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.slice

```
# jq
[
  1
]

# tq -o json
[
  1
]

# tq
[1]: 1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 |
| `cl100k_base` | 6 | 6 | 5 | -1 |

### manual.composition.operation.trycatch

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.unary

```
# jq
-1

# tq -o json
-1

# tq
-1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 |
| `cl100k_base` | 3 | 3 | 2 | -1 |

### manual.composition.operation.usercall

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.composition.operation.variable

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.condcomp.alt-empty

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.condcomp.alt-generator

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.condcomp.alt-missing

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.condcomp.alt-pipe

```
# jq
42
42
1

# tq -o json
42
42
1

# tq --seq
\x1e42
\x1e42
\x1e1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 9 | +3 |
| `cl100k_base` | 6 | 6 | 9 | +3 |

### manual.condcomp.alt-present

```
# jq
19

# tq -o json
19

# tq
19
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.condcomp.and-stream

```
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

# tq --seq
\x1etrue
\x1efalse
\x1etrue
\x1efalse
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 12 | +4 |
| `cl100k_base` | 8 | 8 | 12 | +4 |

### manual.condcomp.and-string

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.condcomp.compare-lt

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.condcomp.eq-false

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.condcomp.eq-object

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.condcomp.eq-stream

```
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

# tq --seq
\x1etrue
\x1etrue
\x1efalse
\x1efalse
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 12 | +4 |
| `cl100k_base` | 8 | 8 | 12 | +4 |

### manual.condcomp.fence-break-error

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.condcomp.fence-generic

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.condcomp.fence-not

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.condcomp.fence-precedence

```
# jq
false
1

# tq -o json
false
1

# tq --seq
\x1efalse
\x1e1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.condcomp.if-elif

```
# jq
"many"

# tq -o json
"many"

# tq
many
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual.condcomp.not-array

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 5 | -3 |
| `cl100k_base` | 8 | 8 | 5 | -3 |

### manual.condcomp.optional-field

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 6 | -3 |
| `cl100k_base` | 9 | 9 | 6 | -3 |

### manual.condcomp.optional-tonumber

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.condcomp.or-stream

```
# jq
true
false

# tq -o json
true
false

# tq --seq
\x1etrue
\x1efalse
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.condcomp.prose-default

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.condcomp.prose-precedence

```
# jq
false
1

# tq -o json
false
1

# tq --seq
\x1efalse
\x1e1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.condcomp.try-catch-array

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 6 | -3 |
| `cl100k_base` | 9 | 9 | 6 | -3 |

### manual.condcomp.try-catch-error

```
# jq
"some exception"

# tq -o json
"some exception"

# tq
some exception
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### manual.condcomp.try-catch-type

```
# jq
". is not an object"

# tq -o json
". is not an object"

# tq
. is not an object
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 |
| `cl100k_base` | 6 | 6 | 5 | -1 |

### manual.introduction.literal-string

```
# jq
"hello"

# tq -o json
"hello"

# tq
hello
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual.invoking.arg-string

```
# jq
"bar"

# tq -o json
"bar"

# tq
bar
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual.invoking.arg-string-number

```
# jq
"123"

# tq -o json
"123"

# tq
"123"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 3 | 0 |
| `cl100k_base` | 3 | 3 | 3 | 0 |

### manual.invoking.argjson

```
# jq
123

# tq -o json
123

# tq
123
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.invoking.args

```
# jq
"foo"
"bar"

# tq -o json
"foo"
"bar"

# tq --seq
\x1efoo
\x1ebar
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 |
| `cl100k_base` | 6 | 6 | 6 | 0 |

### manual.invoking.args-named

```
# jq
"value"

# tq -o json
"value"

# tq
value
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.invoking.argument-terminator

```
# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.invoking.ascii-escape

```
# jq
"\u03bc"

# tq -o json
"\u03bc"

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | n/a | n/a |
| `cl100k_base` | 5 | 5 | n/a | n/a |

### manual.invoking.binary-portable

```
# jq
"line\nnext"

# tq -o json
"line\nnext"

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | n/a | n/a |
| `cl100k_base` | 5 | 5 | n/a | n/a |

### manual.invoking.build-configuration

```
# jq
--build=x86_64-redhat-linux-gnu --host=x86_64-redhat-linux-gnu --program-prefix= --disable-dependency-tracking --prefix=/usr --exec-prefix=/usr --bindir=/usr/bin --sbindir=/usr/bin --sysconfdir=/etc --datadir=/usr/share --includedir=/usr/include --libdir=/usr/lib64 --libexecdir=/usr/libexec --localstatedir=/var --runstatedir=/run --sharedstatedir=/var/lib --mandir=/usr/share/man --infodir=/usr/share/info --disable-static build_alias=x86_64-redhat-linux-gnu host_alias=x86_64-redhat-linux-gnu CC=gcc 'CFLAGS=-O2 -flto=auto -ffat-lto-objects -fexceptions -g -grecord-gcc-switches -pipe -Wall -Werror=format-security -Wp,-U_FORTIFY_SOURCE,-D_FORTIFY_SOURCE=3 -Wp,-D_GLIBCXX_ASSERTIONS -specs=/usr/lib/rpm/redhat/redhat-hardened-cc1 -fstack-protector-strong -specs=/usr/lib/rpm/redhat/redhat-annobin-cc1  -m64 -march=x86-64 -mtune=generic -fasynchronous-unwind-tables -fstack-clash-protection -fcf-protection -mtls-dialect=gnu2 -fno-omit-frame-pointer -mno-omit-leaf-frame-pointer  ' 'LDFLAGS=-Wl,-z,relro -Wl,--as-needed  -Wl,-z,pack-relative-relocs -Wl,-z,now -specs=/usr/lib/rpm/redhat/redhat-hardened-ld -specs=/usr/lib/rpm/redhat/redhat-hardened-ld-errors -specs=/usr/lib/rpm/redhat/redhat-annobin-cc1  -Wl,--build-id=sha1 -specs=/usr/lib/rpm/redhat/redhat-package-notes  ' LT_SYS_LIBRARY_PATH=/usr/lib64:

# tq -o json
target=linux binary-stdio=native formats=toon,yaml,json,jsonl jq-target=1.8.x

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 445 | 24 | n/a | n/a |
| `cl100k_base` | 438 | 25 | n/a | n/a |

### manual.invoking.color-output

```
# jq
\x1b[1;39m{\x1b[0m
  \x1b[1;34m"z"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;39m1\x1b[0m\x1b[1;39m,\x1b[0m
  \x1b[1;34m"a"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;39m2\x1b[0m
\x1b[1;39m}\x1b[0m

# tq -o json
\x1b[1;39m{\x1b[0m
  \x1b[1;34m"z"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;39m1\x1b[0m\x1b[1;39m,\x1b[0m
  \x1b[1;34m"a"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;39m2\x1b[0m
\x1b[1;39m}\x1b[0m

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 112 | 112 | n/a | n/a |
| `cl100k_base` | 94 | 94 | n/a | n/a |

### manual.invoking.compact-output

```
# jq
{"a":1,"b":[2]}

# tq -o json
{"a":1,"b":[2]}

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | n/a | n/a |
| `cl100k_base` | 9 | 9 | n/a | n/a |

### manual.invoking.exit-empty

```
# jq

# tq -o json

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | n/a | n/a |
| `cl100k_base` | 0 | 0 | n/a | n/a |

### manual.invoking.exit-false

```
# jq
false

# tq -o json
false

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | n/a | n/a |
| `cl100k_base` | 2 | 2 | n/a | n/a |

### manual.invoking.exit-null

```
# jq
null

# tq -o json
null

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | n/a | n/a |
| `cl100k_base` | 2 | 2 | n/a | n/a |

### manual.invoking.exit-true

```
# jq
true

# tq -o json
true

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | n/a | n/a |
| `cl100k_base` | 2 | 2 | n/a | n/a |

### manual.invoking.from-file

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.invoking.halt-error

```
# jq


[stderr]
failure

# tq -o json


[stderr]
failure

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | n/a | n/a |
| `cl100k_base` | 0 | 0 | n/a | n/a |

### manual.invoking.help

```
# jq
jq - commandline JSON processor [version 1.8.1]

Usage:	jq [options] <jq filter> [file...]
	jq [options] --args <jq filter> [strings...]
	jq [options] --jsonargs <jq filter> [JSON_TEXTS...]

jq is a tool for processing JSON inputs, applying the given filter to
its JSON text inputs and producing the filter's results as JSON on
standard output.

The simplest filter is ., which copies jq's input to its output
unmodified except for formatting. For more advanced filters see
the jq(1) manpage ("man jq") and/or https://jqlang.org/.

Example:

	$ echo '{"foo": 0}' | jq .
	{
	  "foo": 0
	}

Command options:
  -n, --null-input          use `null` as the single input value;
  -R, --raw-input           read each line as string instead of JSON;
  -s, --slurp               read all inputs into an array and use it as
                            the single input value;
  -c, --compact-output      compact instead of pretty-printed output;
  -r, --raw-output          output strings without escapes and quotes;
      --raw-output0         implies -r and output NUL after each output;
  -j, --join-output         implies -r and output without newline after
                            each output;
  -a, --ascii-output        output strings by only ASCII characters
                            using escape sequences;
  -S, --sort-keys           sort keys of each object on output;
  -C, --color-output        colorize JSON output;
  -M, --monochrome-output   disable colored output;
      --tab                 use tabs for indentation;
      --indent n            use n spaces for indentation (max 7 spaces);
      --unbuffered          flush output stream after each output;
      --stream              parse the input value in streaming fashion;
      --stream-errors       implies --stream and report parse error as
                            an array;
      --seq                 parse input/output as application/json-seq;
  -f, --from-file           load the filter from a file;
  -L, --library-path dir    search modules from the directory;
      --arg name value      set $name to the string value;
      --argjson name value  set $name to the JSON value;
      --slurpfile name file set $name to an array of JSON values read
                            from the file;
      --rawfile name file   set $name to string contents of file;
      --args                consume remaining arguments as positional
                            string values;
      --jsonargs            consume remaining arguments as positional
                            JSON values;
  -e, --exit-status         set exit status code based on the output;
  -V, --version             show the version;
  --build-configuration     show jq's build configuration;
  -h, --help                show the help;
  --                        terminates argument processing;

Named arguments are also available as $ARGS.named[], while
positional arguments are available as $ARGS.positional[].

# tq -o json
tq - jq-compatible queries over TOON, YAML, JSON, JSON5, and JSON Lines

Usage: tq [OPTIONS] [FILTER [FILE...]]
       tq [OPTIONS] -f FILE [INPUT...]
       tq compatibility

Options:
  -i, --input-format FORMAT       select auto, TOON, YAML, JSON, JSON5, JSON Lines, or TOON sequence input
  -o, --output-format FORMAT      select TOON, YAML, JSON, or JSON Lines output
  -n, --null-input                run once with null input
  -R, --raw-input                 read physical lines as strings
  -s, --slurp                     collect ordered inputs and run once
  -c, --compact-output            emit compact JSON
  -r, --raw-output                emit strings without JSON quotes
  --raw-output0                   emit NUL-separated raw outputs
  -j, --join-output               emit raw output without separators
  -a, --ascii-output              escape non-ASCII JSON output
  -S, --sort-keys                 sort object keys recursively
  -C, --color-output              force stable ANSI JSON color
  -M, --monochrome-output         disable ANSI color
  --tab                           indent JSON with tabs
  --indent N                      indent structured output
  --unbuffered                    flush after every output
  --allow-environment             permit env to inspect a redaction-safe process snapshot
  --allow-platform                permit clock, timezone, and input metadata built-ins
  --stream                        read path/value events
  --stream-errors                 report stream parse errors as values
  -x, --proxy-on-error            pass through sources rejected by structured parsing
  --seq                           use sequence framing for the selected structured output
  -f, --from-file FILE            load filter from a file
  -L, --library-path DIR          add an explicit jq module search path
  --arg NAME VALUE                bind a string variable
  --argjson NAME JSON             bind a JSON variable
  --argtoon NAME TOON             bind a TOON variable
  --slurpfile NAME FILE           bind JSON texts as an array
  --rawfile NAME FILE             bind complete UTF-8 file text
  --args                          bind remaining argv strings
  --jsonargs                      bind remaining argv JSON texts
  -e, --exit-status               derive status from the last result
  -b, --binary                    request binary-safe platform output
  -V, --version                   print version targets
  --build-configuration           print stable build capabilities
  --run-tests [FILE]              run a jq-compatible test file
  -h, --help                      print this generated help

Formats: -i, --input-format auto|toon|yaml|json|json5|jsonl|toon-seq|json-seq
-o, --output-format toon|yaml|json|jsonl, --toon-sequence-input, --unframed
TOON:    --delimiter comma|tab|pipe, --fold-keys, --flatten-depth N, --non-strict
Reports: --explain, --explain-json, --trace, --trace-limit N, --report-file FILE
Limits:  --max-input-bytes N, --max-depth N, --max-token-bytes N,
--max-line-bytes N, --max-lookahead-bytes N, --max-vm-steps N,
--max-results N, --max-output-bytes N, --prepare-memory-bytes N,
--hybrid-batch-values N, --hybrid-in-flight-batches N,
--hybrid-in-flight-bytes N, --decode-batch-values N,
--decode-batch-bytes N, --decode-in-flight-batches N,
--decode-in-flight-bytes N, --max-spool-bytes N

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 652 | 801 | n/a | n/a |
| `cl100k_base` | 652 | 804 | n/a | n/a |

### manual.invoking.identity

```
# jq
{
  "foo": "bar"
}

# tq -o json
{
  "foo": "bar"
}

# tq
foo: bar
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 3 | -6 |
| `cl100k_base` | 9 | 9 | 3 | -6 |

### manual.invoking.indent-output

```
# jq
{
    "a": {
        "b": 1
    }
}

# tq -o json
{
    "a": {
        "b": 1
    }
}

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | n/a | n/a |
| `cl100k_base` | 16 | 16 | n/a | n/a |

### manual.invoking.join-output

```
# jq
ab

# tq -o json
ab

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | n/a | n/a |
| `cl100k_base` | 1 | 1 | n/a | n/a |

### manual.invoking.jq-colors

```
# jq
\x1b[1;37m{\x1b[0m
  \x1b[1;34m"z"\x1b[0m\x1b[1;37m:\x1b[0m \x1b[1;34m1\x1b[0m\x1b[1;37m,\x1b[0m
  \x1b[1;34m"a"\x1b[0m\x1b[1;37m:\x1b[0m \x1b[1;34m2\x1b[0m
\x1b[1;37m}\x1b[0m

# tq -o json
\x1b[1;37m{\x1b[0m
  \x1b[1;34m"z"\x1b[0m\x1b[1;37m:\x1b[0m \x1b[1;34m1\x1b[0m\x1b[1;37m,\x1b[0m
  \x1b[1;34m"a"\x1b[0m\x1b[1;37m:\x1b[0m \x1b[1;34m2\x1b[0m
\x1b[1;37m}\x1b[0m

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 112 | 112 | n/a | n/a |
| `cl100k_base` | 94 | 94 | n/a | n/a |

### manual.invoking.jsonargs

```
# jq
1
{
  "a": 2
}

# tq -o json
1
{
  "a": 2
}

# tq --seq
\x1e1
\x1ea: 2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 11 | 11 | 9 | -2 |
| `cl100k_base` | 11 | 11 | 9 | -2 |

### manual.invoking.library-path

```
# jq
"included"

# tq -o json
"included"

# tq
included
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual.invoking.monochrome-output

```
# jq
{
  "z": 1,
  "a": 2
}

# tq -o json
{
  "z": 1,
  "a": 2
}

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | n/a | n/a |
| `cl100k_base` | 16 | 16 | n/a | n/a |

### manual.invoking.no-color-forced

```
# jq
\x1b[1;39m{\x1b[0m
  \x1b[1;34m"foo"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;39m1\x1b[0m
\x1b[1;39m}\x1b[0m

# tq -o json
\x1b[1;39m{\x1b[0m
  \x1b[1;34m"foo"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;39m1\x1b[0m
\x1b[1;39m}\x1b[0m

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 63 | 63 | n/a | n/a |
| `cl100k_base` | 53 | 53 | n/a | n/a |

### manual.invoking.null-input

```
# jq
[
  1,
  2
]

# tq -o json
[
  1,
  2
]

# tq
[2]: 1,2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 |
| `cl100k_base` | 10 | 10 | 7 | -3 |

### manual.invoking.raw-input

```
# jq
"one"
"two"

# tq -o json
"one"
"two"

# tq --seq
\x1eone
\x1etwo
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 |
| `cl100k_base` | 6 | 6 | 6 | 0 |

### manual.invoking.raw-output

```
# jq
hello

# tq -o json
hello

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | n/a | n/a |
| `cl100k_base` | 2 | 2 | n/a | n/a |

### manual.invoking.raw-output0

```
# jq
a\x00b\x00

# tq -o json
a\x00b\x00

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | n/a | n/a |
| `cl100k_base` | 4 | 4 | n/a | n/a |

### manual.invoking.rawfile

```
# jq
"{\"a\":1}\n\"two\"\n"

# tq -o json
"{\"a\":1}\n\"two\"\n"

# tq
"{\"a\":1}\n\"two\"\n"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 12 | 0 |
| `cl100k_base` | 12 | 12 | 12 | 0 |

### manual.invoking.run-tests

```
# jq
Test #1: '.' at line number 2
1 of 1 tests passed (0 malformed, 0 skipped)
Test jq_state: .[]
Test jq_state: .[] | if .%2 == 0 then halt_error else . end

# tq -o json
Test #1: '.' at line number 2
1 of 1 tests passed (0 malformed, 0 skipped)
Test jq_state: .[]
Test jq_state: .[] | if .%2 == 0 then halt_error else . end

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 52 | 52 | n/a | n/a |
| `cl100k_base` | 52 | 52 | n/a | n/a |

### manual.invoking.seq

```
# jq --seq
\x1e1
\x1e2

# tq -o json --seq
\x1e1
\x1e2

# tq
<not run>
```

| Tokenizer | jq --seq | tq -o json --seq | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | n/a | n/a |
| `cl100k_base` | 6 | 6 | n/a | n/a |

### manual.invoking.shell-unix

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.invoking.slurp

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.invoking.slurpfile

```
# jq
[
  {
    "a": 1
  },
  "two"
]

# tq -o json
[
  {
    "a": 1
  },
  "two"
]

# tq
[2]:
  - a: 1
  - two
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 13 | -4 |
| `cl100k_base` | 17 | 17 | 13 | -4 |

### manual.invoking.sort-keys

```
# jq
{
  "a": 0,
  "z": {
    "a": 1,
    "b": 2
  }
}

# tq -o json
{
  "a": 0,
  "z": {
    "a": 1,
    "b": 2
  }
}

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | n/a | n/a |
| `cl100k_base` | 30 | 30 | n/a | n/a |

### manual.invoking.stdin-default-filter

```
# jq
"foo"

# tq -o json
"foo"

# tq
foo
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual.invoking.stream-errors

```
# jq
[
  [
    0
  ],
  "a"
]
[
  "Invalid numeric literal at line 1, column 7",
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
  "Invalid numeric literal at line 1, column 7",
  [
    1
  ]
]

# tq --seq
\x1e[2]:
  - [1]: 0
  - a
\x1e[2]:
  - "Invalid numeric literal at line 1, column 7"
  - [1]: 1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 38 | 38 | 43 | +5 |
| `cl100k_base` | 38 | 38 | 43 | +5 |

### manual.invoking.stream-nested

```
# jq
[
  [
    0
  ],
  []
]
[
  [
    1
  ],
  "a"
]
[
  [
    2,
    0
  ],
  "b"
]
[
  [
    2,
    0
  ]
]
[
  [
    2
  ]
]

# tq -o json
[
  [
    0
  ],
  []
]
[
  [
    1
  ],
  "a"
]
[
  [
    2,
    0
  ],
  "b"
]
[
  [
    2,
    0
  ]
]
[
  [
    2
  ]
]

# tq --seq
\x1e[2]:
  - [1]: 0
  - [0]:
\x1e[2]:
  - [1]: 1
  - a
\x1e[2]:
  - [2]: 2,0
  - b
\x1e[1]:
  - [2]: 2,0
\x1e[1]:
  - [1]: 2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 68 | 68 | 77 | +9 |
| `cl100k_base` | 68 | 68 | 77 | +9 |

### manual.invoking.stream-reduce

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.invoking.stream-scalar

```
# jq
[
  [],
  "a"
]

# tq -o json
[
  [],
  "a"
]

# tq
[2]:
  - [0]:
  - a
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 11 | +3 |
| `cl100k_base` | 8 | 8 | 11 | +3 |

### manual.invoking.tab-output

```
# jq
{
	"a": {
		"b": 1
	}
}

# tq -o json
{
	"a": {
		"b": 1
	}
}

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | n/a | n/a |
| `cl100k_base` | 16 | 16 | n/a | n/a |

### manual.invoking.unbuffered

```
# jq
1
2

# tq -o json
1
2

# tq --seq
\x1e1
\x1e2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.invoking.unquoted-filter

```
# jq


[stderr]
jq: error: foo/0 is not defined at <top-level>, line 1, column 1:
    foo
    ^^^
jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-RESOLVE-BUILTIN-001: unknown filter foo/0

# tq


[stderr]
tq: query compilation failed: TQ-RESOLVE-BUILTIN-001: unknown filter foo/0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.invoking.user-function

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.invoking.version

```
# jq
jq-1.8.1

# tq -o json
tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 25 | n/a | n/a |
| `cl100k_base` | 8 | 25 | n/a | n/a |

### manual.invoking.whitespace-stream

```
# jq
1
true
[
  2
]

# tq -o json
1
true
[
  2
]

# tq --seq
\x1e1
\x1etrue
\x1e[1]: 2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 13 | +3 |
| `cl100k_base` | 10 | 10 | 13 | +3 |

### manual.io.debug

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.io.debug-example

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.io.debug-msgs

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.io.input

```
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

# tq --seq
\x1e[2]: 1,2
\x1e[2]: 3,4
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 18 | -2 |
| `cl100k_base` | 20 | 20 | 18 | -2 |

### manual.io.input-filename

```
# jq
"<stdin>"

# tq -o json
"<stdin>"

# tq
<stdin>
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 3 | 0 |
| `cl100k_base` | 3 | 3 | 3 | 0 |

### manual.io.input-line-number

```
# jq
1
2

# tq -o json
1
2

# tq --seq
\x1e1
\x1e2
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.io.input-null-input

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 6 | -3 |
| `cl100k_base` | 9 | 9 | 6 | -3 |

### manual.io.inputs

```
# jq
6

# tq -o json
6

# tq
6
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.io.stderr

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual.math.acos

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.acosh

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.asin

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.asinh

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.atan

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.atan2

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.atanh

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.cbrt

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.ceil

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.copysign

```
# jq
-2

# tq -o json
-2

# tq
-2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 |
| `cl100k_base` | 3 | 3 | 2 | -1 |

### manual.math.cos

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.cosh

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.drem

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.erf

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.erfc

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.exp

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.exp10

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.exp2

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.expm1

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.fabs

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.fdim

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.floor

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.fma

```
# jq
10

# tq -o json
10

# tq
10
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.fmax

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.fmin

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.fmod

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.frexp-actual-arity

```
# jq
[
  0.5,
  4
]

# tq -o json
[
  0.5,
  4
]

# tq
[2]: 0.5,4
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 9 | -3 |
| `cl100k_base` | 12 | 12 | 9 | -3 |

### manual.math.frexp-reference-unavailable

```
# jq


[stderr]
jq: error: frexp/2 is not defined at <top-level>, line 1, column 1:
    frexp(8;0)
    ^^^^^
jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-RESOLVE-ARITY-001: invalid arity 2 for built-in frexp

# tq


[stderr]
tq: query compilation failed: TQ-RESOLVE-ARITY-001: invalid arity 2 for built-in frexp
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.math.gamma

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.hypot

```
# jq
5

# tq -o json
5

# tq
5
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.j0

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.j1

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.jn

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.ldexp

```
# jq
8

# tq -o json
8

# tq
8
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.lgamma

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.log

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.log10

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.log1p

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.log2

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.logb

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.modf-actual-arity

```
# jq
[
  0.5,
  3
]

# tq -o json
[
  0.5,
  3
]

# tq
[2]: 0.5,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 9 | -3 |
| `cl100k_base` | 12 | 12 | 9 | -3 |

### manual.math.modf-reference-unavailable

```
# jq


[stderr]
jq: error: modf/2 is not defined at <top-level>, line 1, column 1:
    modf(3;0)
    ^^^^
jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-RESOLVE-ARITY-001: invalid arity 2 for built-in modf

# tq


[stderr]
tq: query compilation failed: TQ-RESOLVE-ARITY-001: invalid arity 2 for built-in modf
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.math.nearbyint

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.nextafter

```
# jq
1.0000000000000002

# tq -o json
1.0000000000000002

# tq
1.0000000000000002
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 8 | -1 |
| `cl100k_base` | 9 | 9 | 8 | -1 |

### manual.math.nexttoward

```
# jq
1.0000000000000002

# tq -o json
1.0000000000000002

# tq
1.0000000000000002
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 8 | -1 |
| `cl100k_base` | 9 | 9 | 8 | -1 |

### manual.math.pow

```
# jq
8

# tq -o json
8

# tq
8
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.remainder

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.rint

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.round

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.scalb

```
# jq
16

# tq -o json
16

# tq
16
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.scalbln

```
# jq
16

# tq -o json
16

# tq
16
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.significand

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.sin

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.sinh

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.sqrt

```
# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.tan

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.tanh

```
# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.tgamma

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.trunc

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.math.y0

```
# jq
0.08825696421567698

# tq -o json
0.08825696421567697

# tq
0.08825696421567697
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 8 | -1 |
| `cl100k_base` | 9 | 9 | 8 | -1 |

### manual.math.y1

```
# jq
-0.7812128213002887

# tq -o json
-0.7812128213002887

# tq
-0.7812128213002887
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 9 | -1 |
| `cl100k_base` | 10 | 10 | 9 | -1 |

### manual.math.yn

```
# jq
0.08825696421567698

# tq -o json
0.08825696421567697

# tq
0.08825696421567697
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 8 | -1 |
| `cl100k_base` | 9 | 9 | 8 | -1 |

### manual.modules.default-search-path

```
# jq
"tilde-home"

# tq -o json
"tilde-home"

# tq
tilde-home
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 3 | -2 |
| `cl100k_base` | 5 | 5 | 3 | -2 |

### manual.modules.foo-directory

```
# jq
"directory-module"

# tq -o json
"directory-module"

# tq
directory-module
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### manual.modules.foo-single

```
# jq
"single-file"

# tq -o json
"single-file"

# tq
single-file
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### manual.modules.home-auto-source

```
# jq
"auto-home"

# tq -o json
"auto-home"

# tq
auto-home
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### manual.modules.import

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.modules.import-json

```
# jq
[
  {
    "name": "manual",
    "count": 3
  }
]

# tq -o json
[
  {
    "name": "manual",
    "count": 3
  }
]

# tq
[1]{name,count}:
  manual,3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 11 | -9 |
| `cl100k_base` | 20 | 20 | 11 | -9 |

### manual.modules.import-metadata

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.modules.import-search

```
# jq
"prefixed"

# tq -o json
"prefixed"

# tq
prefixed
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### manual.modules.include

```
# jq
"included"

# tq -o json
"included"

# tq
included
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |

### manual.modules.include-metadata

```
# jq
"prefixed"

# tq -o json
"prefixed"

# tq
prefixed
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### manual.modules.metadata-constant

```
# jq


[stderr]
jq: error: Module metadata must be constant at <top-level>, line 1, column 21:
    import "basic" as b {"homepage": .}; b::value
                        ^^^^^^^^^^^^^^^
jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-MODULE-METADATA-001: module metadata must be a constant expression

# tq


[stderr]
tq: query compilation failed: TQ-MODULE-METADATA-001: module metadata must be a constant expression
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.modules.modulemeta

```
# jq
{
  "homepage": "https://example.invalid/basic",
  "deps": [],
  "defs": [
    "value/0"
  ]
}

# tq -o json
{
  "homepage": "https://example.invalid/basic",
  "deps": [],
  "defs": [
    "value/0"
  ]
}

# tq
homepage: "https://example.invalid/basic"
deps[0]:
defs[1]: value/0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 31 | 31 | 20 | -11 |
| `cl100k_base` | 31 | 31 | 20 | -11 |

### manual.modules.modulemeta-deps

```
# jq
{
  "deps": [
    {
      "search": "..",
      "as": "helper",
      "is_data": false,
      "relpath": "helper"
    }
  ],
  "defs": [
    "value/0"
  ]
}

# tq -o json
{
  "deps": [
    {
      "search": "..",
      "as": "helper",
      "is_data": false,
      "relpath": "helper"
    }
  ],
  "defs": [
    "value/0"
  ]
}

# tq
deps[1]{search,as,is_data,relpath}:
  ..,helper,false,helper
defs[1]: value/0
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 54 | 54 | 29 | -25 |
| `cl100k_base` | 54 | 54 | 29 | -25 |

### manual.modules.path-dot

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.modules.path-dot-including

```
# jq
"including-file"

# tq -o json
"including-file"

# tq
including-file
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### manual.modules.path-origin

```
# jq
"origin-path"

# tq -o json
"origin-path"

# tq
origin-path
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### manual.modules.path-tilde

```
# jq
"tilde-home"

# tq -o json
"tilde-home"

# tq
tilde-home
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 3 | -2 |
| `cl100k_base` | 5 | 5 | 3 | -2 |

### manual.modules.relative-directory

```
# jq
"relative-directory"

# tq -o json
"relative-directory"

# tq
relative-directory
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### manual.modules.relative-single

```
# jq
"relative-file"

# tq -o json
"relative-file"

# tq
relative-file
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 2 | -2 |
| `cl100k_base` | 4 | 4 | 2 | -2 |

### manual.modules.repeated-component

```
# jq


[stderr]
jq: error: module names must not have equal consecutive components: foo/foo

jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-MODULE-PATH-001: module path "foo/foo" repeats a component

# tq


[stderr]
tq: query compilation failed: TQ-MODULE-PATH-001: module path "foo/foo" repeats a component
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.modules.search-terminator

```
# jq


[stderr]
jq: error: module not found: foo

jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-MODULE-NOT-FOUND-001: module "foo" was not found in configured roots

# tq


[stderr]
tq: query compilation failed: TQ-MODULE-NOT-FOUND-001: module "foo" was not found in configured roots
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.regex.array-capture

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 3 | -6 |
| `cl100k_base` | 9 | 9 | 3 | -6 |

### manual.regex.array-capture-flags

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 4 | -6 |
| `cl100k_base` | 10 | 10 | 4 | -6 |

### manual.regex.array-match

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 28 | 28 | 18 | -10 |
| `cl100k_base` | 28 | 28 | 18 | -10 |

### manual.regex.array-test

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.regex.fence-inline-flags

```
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

# tq --seq
\x1etrue
\x1etrue
\x1efalse
\x1efalse
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 12 | +4 |
| `cl100k_base` | 8 | 8 | 12 | +4 |

### manual.regex.fence-whitespace-extended

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.regex.flag-l

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 28 | 0 | 0 | -28 |
| `cl100k_base` | 28 | 0 | 0 | -28 |

### manual.regex.flag-m

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.regex.flag-p

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 5 | -3 |
| `cl100k_base` | 8 | 8 | 5 | -3 |

### manual.regex.flag-s

```
# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.regex.table-001

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.regex.table-002

```
# jq
true
true

# tq -o json
true
true

# tq --seq
\x1etrue
\x1etrue
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 6 | +2 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.regex.table-003

```
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

# tq --seq
\x1eoffset: 0
length: 3
string: abc
captures[1]{offset,length,string,name}:
  0,3,abc,null
\x1eoffset: 4
length: 3
string: abc
captures[1]{offset,length,string,name}:
  4,3,abc,null
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 122 | 122 | 68 | -54 |
| `cl100k_base` | 122 | 122 | 68 | -54 |

### manual.regex.table-004

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 28 | 28 | 18 | -10 |
| `cl100k_base` | 28 | 28 | 18 | -10 |

### manual.regex.table-005

```
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

# tq --seq
\x1eoffset: 0
length: 3
string: foo
captures[0]:
\x1eoffset: 8
length: 3
string: FOO
captures[0]:
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 57 | 57 | 39 | -18 |
| `cl100k_base` | 57 | 57 | 39 | -18 |

### manual.regex.table-006

```
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

# tq --seq
\x1eoffset: 0
length: 11
string: foo bar foo
captures[1]{offset,length,string,name}:
  4,3,bar,bar123
\x1eoffset: 12
length: 8
string: foo  foo
captures[1]{offset,string,length,name}:
  -1,null,0,bar123
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 129 | 129 | 75 | -54 |
| `cl100k_base` | 129 | 129 | 75 | -54 |

### manual.regex.table-007

```
# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.regex.table-008

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 10 | -7 |
| `cl100k_base` | 17 | 17 | 10 | -7 |

### manual.regex.table-009

```
# jq
"c"
"c"

# tq -o json
"c"
"c"

# tq --seq
\x1ec
\x1ec
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 |
| `cl100k_base` | 4 | 4 | 6 | +2 |

### manual.regex.table-010

```
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

# tq --seq
\x1e[2]: a,b
\x1e[2]: aa,bb
\x1e[2]: aaa,bbb
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 23 | -7 |
| `cl100k_base` | 30 | 30 | 23 | -7 |

### manual.regex.table-011

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 8 | -6 |
| `cl100k_base` | 14 | 14 | 8 | -6 |

### manual.regex.table-012

```
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

# tq --seq
\x1eab
\x1ecd
\x1eef
\x1egh
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 12 | 0 |
| `cl100k_base` | 12 | 12 | 12 | 0 |

### manual.regex.table-013

```
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

# tq --seq
\x1eab
\x1ecd
\x1eef
\x1egh
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 12 | 0 |
| `cl100k_base` | 12 | 12 | 12 | 0 |

### manual.regex.table-014

```
# jq
"ZabcZdef"

# tq -o json
"ZabcZdef"

# tq
ZabcZdef
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 4 | -2 |
| `cl100k_base` | 6 | 6 | 4 | -2 |

### manual.regex.table-015

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 11 | 11 | 6 | -5 |
| `cl100k_base` | 11 | 11 | 6 | -5 |

### manual.regex.table-016

```
# jq
"+A-+a-"

# tq -o json
"+A-+a-"

# tq
+A-+a-
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 7 | 7 | 5 | -2 |
| `cl100k_base` | 7 | 7 | 5 | -2 |

### manual.regex.table-017

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 5 | -5 |
| `cl100k_base` | 10 | 10 | 5 | -5 |

### manual.streaming.fromstream-truncate

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 4 | -2 |
| `cl100k_base` | 6 | 6 | 4 | -2 |

### manual.streaming.stream-error-literal

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 7 | 7 | 5 | -2 |
| `cl100k_base` | 7 | 7 | 5 | -2 |

### manual.streaming.stream-errors-form

```
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

# tq --seq
\x1e[2]:
  - [1]: 0
  - 1
\x1e[2]:
  - "Invalid numeric literal at line 1, column 8"
  - [1]: 1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 38 | 38 | 44 | +6 |
| `cl100k_base` | 38 | 38 | 44 | +6 |

### manual.streaming.stream-option

```
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

# tq --seq
\x1e[2]:
  - [1]: 0
  - a
\x1e[2]:
  - [2]: 1,0
  - b
\x1e[1]:
  - [2]: 1,0
\x1e[1]:
  - [1]: 1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 56 | 56 | 60 | +4 |
| `cl100k_base` | 56 | 56 | 60 | +4 |

### manual.streaming.tostream-roundtrip

```
# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.streaming.truncate-stream

```
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

# tq --seq
\x1e[2]:
  - [1]: 0
  - b
\x1e[1]:
  - [1]: 0
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 24 | 24 | 28 | +4 |
| `cl100k_base` | 24 | 24 | 28 | +4 |

### manual.types.fence-object-field

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### manual.types.fence-object-selection

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 19 | 19 | 10 | -9 |
| `cl100k_base` | 19 | 19 | 11 | -8 |

### manual.types.fence-variable-key

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 14 | -8 |
| `cl100k_base` | 22 | 22 | 14 | -8 |

### manual.types.inline-array-fields

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.types.inline-array-literal

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.types.inline-array-names

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 6 | -4 |
| `cl100k_base` | 10 | 10 | 6 | -4 |

### manual.types.inline-current-value

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### manual.types.inline-empty-array

```
# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 |
| `cl100k_base` | 1 | 1 | 3 | +2 |

### manual.types.inline-empty-object

```
# jq
{}

# tq -o json
{}

# tq
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 0 | -1 |
| `cl100k_base` | 1 | 1 | 0 | -1 |

### manual.types.inline-invalid-recursive

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 |
| `cl100k_base` | 0 | 0 | 0 | 0 |

### manual.types.inline-literal-number

```
# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.types.inline-object-expression-key

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 |
| `cl100k_base` | 9 | 9 | 4 | -5 |

### manual.types.inline-object-identifier-keys

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### manual.types.inline-object-literal

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 |
| `cl100k_base` | 16 | 16 | 9 | -7 |

### manual.types.inline-object-shorthand

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 19 | 19 | 10 | -9 |
| `cl100k_base` | 19 | 19 | 11 | -8 |

### manual.types.inline-recurse-builtin

```
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

# tq --seq
\x1ea: 1
\x1e1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 11 | 11 | 9 | -2 |
| `cl100k_base` | 11 | 11 | 9 | -2 |

### manual.types.inline-recursive-descent

```
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

# tq --seq
\x1ea[1]: 1
\x1e[1]: 1
\x1e1
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 21 | 21 | 18 | -3 |
| `cl100k_base` | 21 | 21 | 18 | -3 |

### manual.types.inline-recursive-pipe

```
# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### manual.types.table-001

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 11 | -6 |
| `cl100k_base` | 17 | 17 | 12 | -5 |

### manual.types.table-002

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 |
| `cl100k_base` | 14 | 14 | 9 | -5 |

### manual.types.table-003

```
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

# tq --seq
\x1euser: stedolan
title: JQ Primer
\x1euser: stedolan
title: More JQ
```

| Tokenizer | jq | tq -o json | tq --seq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 38 | 38 | 24 | -14 |
| `cl100k_base` | 38 | 38 | 26 | -12 |

### manual.types.table-004

```
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

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 12 | -10 |
| `cl100k_base` | 22 | 22 | 12 | -10 |

### manual.types.table-005

```
# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 1 | -1 |
| `cl100k_base` | 2 | 2 | 1 | -1 |

### platform.localtime-year

```
# jq
"2015"

# tq -o json
"2015"

# tq
"2015"
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 |
| `cl100k_base` | 4 | 4 | 4 | 0 |

### platform.now-type

```
# jq
"number"

# tq -o json
"number"

# tq
number
```

| Tokenizer | jq | tq -o json | tq | diff |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 1 | -2 |
| `cl100k_base` | 3 | 3 | 1 | -2 |
