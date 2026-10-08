---
type: Report
title: "jq manual: colors coverage review"
description: "Recorded review of jq manual: colors coverage review."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-12T16:56:48Z }
benchmark_runs: [{"platform":"macos","target":"aarch64-macos","campaign_id":null,"captured_at":null,"binaries":{"jq":{"version":"jq-1.8.2","sha256":"2d75340ba57a4b4b4c8708a21c2dc8e958a48aaa8bba13b27f77f6e4c0eca07e","identity_status":"measured"},"tq":{"version":"tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)","sha256":"8a0ba42cf81881829b93ee950db0d4d3f808a22af574b210484be7833b238d00","identity_status":"measured"}}},{"platform":"linux","target":"x86_64-linux","campaign_id":null,"captured_at":null,"binaries":{"jq":{"version":"jq-1.8.2","sha256":"b1c22172dd303f3be49e935aa56aa48a8b7a46e0bc838b4997d3bb451495870f","identity_status":"measured"},"tq":{"version":"tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)","sha256":"474625b11f0ec3a1fa125bbeb4e55008efa50ec713b0c398822ada48f0289b91","identity_status":"measured"}}},{"platform":"windows","target":"x86_64-windows","campaign_id":null,"captured_at":null,"binaries":{"jq":{"version":"jq-1.8.2","sha256":"a6fc67fedaf9128a3309a1e2ebb8b986aeccf70122ee46d2cb4849e423f0c627","identity_status":"measured"},"tq":{"version":"tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)","sha256":"a1cec37e4c6e70bcd06647a5b5b5617ece0b88e41fe8a1ad9590bafc00992428","identity_status":"measured"}}}]
---

# jq manual: colors coverage review

Source: [pinned jq manual source inventory](../../../tests/compatibility/reviews/jq-manual/source-examples.toon), jq 1.8. The machine-readable ledger is [colors.toon](../../../tests/compatibility/reviews/jq-manual/colors.toon). This section has no fenced or tabular examples.

Inventory: 26 records and 3 raw-byte MVP cases. The default-palette case copies the exact eight-entry palette from line 27; the custom case repeats the documented `1;31` component across all eight slots; and the ANSI matrix assigns every listed style (`1`, `2`, `4`, `5`, `7`, `8`) and color (`30` through `37`) value to a slot. Each case keeps jq and tq enabled, with `-C`, explicit `JQ_COLORS`, and empty `NO_COLOR`; the cases exercise the implemented palette-override behavior and capture forced-color output bytes for comparison.

The eight slot records (lines 18–25) all reuse the aggregate object fixture, which emits null, false, true, a number, a string, an array, an object, and object keys. There are no non-executable records and no fenced blocks. The harness cannot make claims about tty auto-detection; it only compares captured bytes from forced-color invocations.

<!-- tq-manual-compare:begin section=colors -->
## Results

[Case collection](../../../tests/compatibility/reviews/jq-manual/colors.toon)


| Verdict | Cases |
| --- | ---: |
| Differences | 3 |

Independent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.

| Output campaign | Exact matches | Cases |
| --- | ---: | ---: |
| compact_json | 0 | 0 |
| toon | 0 | 0 |

An exact match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from exact matches. Historical expected-difference labels do not pass either gate.

Differences include missing features and unaccepted mismatches; these still fail the strict and completion gates. Reference discrepancies describe errors in the imported manual, not successful compatibility.

JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.

### Output size

0 eligible examples. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with default LF-terminated `-o toon` results; TOON sequence captures, when available, are shown in the cases but excluded from size totals. Diff is TOON tokens minus JSON tokens. % is the signed percent difference `(TOON - JSON) / JSON`, so savings are negative and growth is positive.

| Tokenizer | JSON tokens | TOON tokens | Diff | % |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | n/a |
| `cl100k_base` | 0 | 0 | 0 | n/a |

Only successful jq/JSON/TOON-equivalent results enter the totals. A negative `Diff` means TOON uses fewer tokens; `%` is negative for savings and positive for growth. The manual is a correctness corpus, not a representative workload benchmark.

### Differences

1. `manual.colors.ansi-values`: Expected presentation difference: quote styling and ANSI escapes differ; JSON data is unchanged.
2. `manual.colors.custom-1-31`: Expected presentation difference: quote styling and ANSI escapes differ; JSON data is unchanged.
3. `manual.colors.default-palette`: Expected presentation difference: quote styling and ANSI escapes differ; JSON data is unchanged.

### Cases

Each case shows the original jq invocation and the available jq, tq JSON, and tq TOON output captures. Missing captures are marked `<not run>`; these placeholders are not execution evidence. Control bytes use `\xNN` escapes so record separators remain visible. This section is generated by the separate `tq-manual-compare` command.

#### manual.colors.ansi-values

```
# input
jq -C -c .

# jq
\x1b[1;36m{\x1b[0m\x1b[2;37m"null"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[1;30mnull\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"false"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[2;31mfalse\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"true"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[4;32mtrue\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"number"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[5;33m1\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"string"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[7;34m"x"\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"array"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[8;35m[]\x1b[0m\x1b[1;36m,\x1b[0m\x1b[2;37m"object"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[1;36m{}\x1b[0m\x1b[1;36m}\x1b[0m

# tq -o json
\x1b[1;36m{\x1b[0m\x1b[1;36m"\x1b[0m\x1b[2;37mnull\x1b[0m\x1b[1;36m"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[1;30mnull\x1b[0m\x1b[1;36m,\x1b[0m\x1b[1;36m"\x1b[0m\x1b[2;37mfalse\x1b[0m\x1b[1;36m"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[2;31mfalse\x1b[0m\x1b[1;36m,\x1b[0m\x1b[1;36m"\x1b[0m\x1b[2;37mtrue\x1b[0m\x1b[1;36m"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[4;32mtrue\x1b[0m\x1b[1;36m,\x1b[0m\x1b[1;36m"\x1b[0m\x1b[2;37mnumber\x1b[0m\x1b[1;36m"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[5;33m1\x1b[0m\x1b[1;36m,\x1b[0m\x1b[1;36m"\x1b[0m\x1b[2;37mstring\x1b[0m\x1b[1;36m"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[1;36m"\x1b[0m\x1b[7;34mx\x1b[0m\x1b[1;36m"\x1b[0m\x1b[1;36m,\x1b[0m\x1b[1;36m"\x1b[0m\x1b[2;37marray\x1b[0m\x1b[1;36m"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[8;35m[\x1b[0m\x1b[8;35m]\x1b[0m\x1b[1;36m,\x1b[0m\x1b[1;36m"\x1b[0m\x1b[2;37mobject\x1b[0m\x1b[1;36m"\x1b[0m\x1b[1;36m:\x1b[0m\x1b[1;36m{\x1b[0m\x1b[1;36m}\x1b[0m\x1b[1;36m}\x1b[0m

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | n/a | n/a | n/a | n/a | n/a |
| `cl100k_base` | n/a | n/a | n/a | n/a | n/a |

#### manual.colors.custom-1-31

```
# input
jq -C -c .

# jq
\x1b[1;31m{\x1b[0m\x1b[1;31m"null"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31mnull\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"false"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31mfalse\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"true"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31mtrue\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"number"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m1\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"string"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m"x"\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"array"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m[]\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"object"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m{}\x1b[0m\x1b[1;31m}\x1b[0m

# tq -o json
\x1b[1;31m{\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31mnull\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31mnull\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31mfalse\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31mfalse\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31mtrue\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31mtrue\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31mnumber\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m1\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31mstring\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31mx\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31marray\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m[\x1b[0m\x1b[1;31m]\x1b[0m\x1b[1;31m,\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31mobject\x1b[0m\x1b[1;31m"\x1b[0m\x1b[1;31m:\x1b[0m\x1b[1;31m{\x1b[0m\x1b[1;31m}\x1b[0m\x1b[1;31m}\x1b[0m

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | n/a | n/a | n/a | n/a | n/a |
| `cl100k_base` | n/a | n/a | n/a | n/a | n/a |

#### manual.colors.default-palette

```
# input
jq -C -c .

# jq
\x1b[1;39m{\x1b[0m\x1b[1;34m"null"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;90mnull\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"false"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;39mfalse\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"true"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;39mtrue\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"number"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;39m1\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"string"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;32m"x"\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"array"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[1;39m[]\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;34m"object"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[1;39m{}\x1b[0m\x1b[1;39m}\x1b[0m

# tq -o json
\x1b[1;39m{\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;34mnull\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;90mnull\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;34mfalse\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;39mfalse\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;34mtrue\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;39mtrue\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;34mnumber\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;39m1\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;34mstring\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[1;39m"\x1b[0m\x1b[0;32mx\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;34marray\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[1;39m[\x1b[0m\x1b[1;39m]\x1b[0m\x1b[1;39m,\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;34mobject\x1b[0m\x1b[1;39m"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[1;39m{\x1b[0m\x1b[1;39m}\x1b[0m\x1b[1;39m}\x1b[0m

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | n/a | n/a | n/a | n/a | n/a |
| `cl100k_base` | n/a | n/a | n/a | n/a | n/a |

<!-- tq-manual-compare:end -->
