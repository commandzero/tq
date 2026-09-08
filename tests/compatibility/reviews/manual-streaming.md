# jq manual: streaming coverage review

Source: [`jq-manual/streaming.md`](/Users/reno/Development/commandzero/tq-benchmarks/jq-manual/streaming.md), jq 1.8. The machine-readable ledger is [manual-streaming.toon](manual-streaming.toon). The three table queries and inputs are copied exactly. The `--stream` and `--stream-errors` prose capabilities use executable adaptations; grammar placeholders are recorded as non-executable.

Inventory: 13 records (3 tables, 7 prose/examples, 3 non-executable grammar placeholders): 4 new, 3 adapted, 3 covered by exact cases, and 3 non-executable. The stream-option case also covers the concrete streamed representation and its array end markers. The error-form literal and malformed-input cases cover both the concrete placeholder and its executable witness. The final table covers the tostream/fromstream roundtrip.

| Line | Kind | Query / input | Case ID(s) | Disposition / reason |
| ---: | --- | --- | --- | --- |
| 16 | prose option | `.` / `["a",["b"]]` | `manual.streaming.stream-option` | adapted — representative small fixture for the documented `--stream` behavior |
| 18 | prose syntax | `[<path>, <leaf-value>]` | — | non-executable — placeholders |
| 22 | prose example | `.` / `["a",["b"]]` | `manual.streaming.stream-option` | new — emits the exact illustrated stream records |
| 24 | prose syntax | `[<path>, <leaf-value>]` | — | non-executable — placeholders |
| 24 | prose syntax | `[<path>]` | — | non-executable — placeholder |
| 24 | prose example | malformed JSON / `[1, bad, 2]` | `manual.streaming.stream-errors-form` | adapted — executable witness for generic `["error message"]` form |
| 24 | inline | `["error message"]` / `null` | `manual.streaming.stream-error-literal` | new — exact concrete error-form representation |
| 28 | prose definition | truncate stream / `1` | `manual.streaming.truncate-stream` | covered — exact table behavior |
| 30 | table | `truncate_stream([[0],"a"],[[1,0],"b"],[[1,0]],[[1]])` / `1` | `manual.streaming.truncate-stream` | new — exact query/input |
| 39 | prose definition | fromstream / `null` | `manual.streaming.fromstream-truncate` | covered — exact table behavior |
| 41 | table | `fromstream(1\|truncate_stream(...))` / `null` | `manual.streaming.fromstream-truncate` | new — exact query/input |
| 49 | prose definition | tostream roundtrip / `[0,[1,{"a":1},{"b":2}]]` | `manual.streaming.tostream-roundtrip` | covered — exact roundtrip behavior |
| 51 | table | `. as $dot\|fromstream($dot\|tostream)\|.==$dot` / roundtrip fixture | `manual.streaming.tostream-roundtrip` | new — exact query/input |
