# jq manual: advanced-features coverage review

Source: [`jq-manual/advanced-features.md`](/Users/reno/Development/commandzero/tq-benchmarks/jq-manual/advanced-features.md), jq 1.8. The machine-readable ledger is [manual-advanced-features.toon](manual-advanced-features.toon). Table queries and inputs are copied exactly; all MVP cases keep jq and tq enabled so incompatibilities remain observable. yq is marked unsupported because these are jq-target manual examples.

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
