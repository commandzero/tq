# Regular expressions coverage audit

Source: [jq manual, Regular expressions](https://jqlang.org/manual/#regular-expressions). See the [machine-readable inventory](manual-regular-expressions.toon).

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
