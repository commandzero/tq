# jq manual: types-and-values coverage review

Source: [`jq-manual/types-and-values.md`](/Users/reno/Development/commandzero/tq-benchmarks/jq-manual/types-and-values.md), jq 1.8. The machine-readable ledger is [manual-types-and-values.toon](manual-types-and-values.toon). Table queries and inputs are copied exactly; runnable fenced and inline examples retain their exact query text. Synthesized fixtures are called out where the prose gives only a shape or no concrete input. All executable MVP cases keep jq and tq enabled; yq is marked unsupported because these are jq-target manual examples.

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
