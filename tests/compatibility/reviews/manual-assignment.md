# Assignment manual audit

Source: [`jq-manual/assignment.md`](/Users/reno/Development/commandzero/tq-benchmarks/jq-manual/assignment.md), jq 1.8 manual. The machine-readable ledger is [manual-assignment.toon](manual-assignment.toon). `new` means an executable jq/tq case was added to [manual-assignment.jsonl](../cases/manual-assignment.jsonl); `covered` reuses an exact case; `non-executable` marks a symbolic or incomplete fragment.

Counts: 24 new, 3 covered, 5 non-executable, 32 examples inventoried. This covers all 6 source tables, all 8 fenced snippets, and the concrete inline/prose filters. Every fenced opening line is represented in the ledger. The unbound `$var` examples are retained as compile-error cases with both jq and tq enabled; `path()`, `del(path)`, `a = b`, and the operator fragments remain non-executable because the source leaves their operands symbolic or incomplete.

| Source lines | Kind | Coverage |
| ---: | --- | --- |
| 18–74 | Inline/prose | Field access/copy, immutable update, comma-assignment precedence, update assignment, empty, and range examples; five symbolic/incomplete fragments are documented as non-executable. |
| 54, 64, 92, 98, 104, 112 | Tables | Six exact query/input cases. |
| 29, 44, 80, 86, 122, 128, 138, 144 | Fences | Eight fenced snippets; the two unbound-variable snippets are compile-error cases and the two comparison snippets reuse exact table cases. |

The cases intentionally keep tq enabled for every executable example so assignment semantics and diagnostics remain observable in the compatibility campaign.

Narrow probes against `target/reference-build/jq/jq` pass for all six table queries. The current `target/debug/tq` reports runtime assignment-path errors for the recursive boolean update (table line 54) and the selected-post comment update (fence line 144); both cases remain enabled so the divergences are visible to the parent campaign.
