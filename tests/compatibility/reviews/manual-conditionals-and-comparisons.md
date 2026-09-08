# Conditionals and comparisons manual audit

Source: [`conditionals-and-comparisons.md`](/Users/reno/Development/commandzero/tq-benchmarks/jq-manual/conditionals-and-comparisons.md), jq 1.8 manual. The machine-readable ledger is [manual-conditionals-and-comparisons.toon](manual-conditionals-and-comparisons.toon). `new` means an executable case was added to [manual-conditionals-and-comparisons.jsonl](../cases/manual-conditionals-and-comparisons.jsonl); `covered` reuses an exact case; `non-executable` marks symbolic or incomplete source text.

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
