# Basic filters manual audit

Source: [`jq-manual/basic-filters.md`](/Users/reno/Development/commandzero/tq-benchmarks/jq-manual/basic-filters.md), jq 1.8 manual. The machine-readable ledger is [manual-basic-filters.toon](manual-basic-filters.toon). `new` means an exact executable query/input case was added to [manual-basic-filters.jsonl](../cases/manual-basic-filters.jsonl); `non-executable` means the prose or syntax fragment supplies no concrete fixture/output; `gap` means the fragment is runnable but the source supplies no fixture.

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
