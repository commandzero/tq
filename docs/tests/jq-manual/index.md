# jq manual test reviews

1. [Review data](../../../tests/compatibility/reviews/jq-manual): machine-readable case mappings, source inventory, and execution evidence.
2. [advanced features](advanced-features.md)
3. [assignment](assignment.md)
4. [basic filters](basic-filters.md)
5. [builtin operators and functions](builtin-operators-and-functions.md)
6. [colors](colors.md)
7. [conditionals and comparisons](conditionals-and-comparisons.md)
8. [coverage](coverage.md)
9. [introduction](introduction.md)
10. [invoking jq](invoking-jq.md)
11. [io](io.md)
12. [math model](math-model.md)
13. [math](math.md)
14. [modules](modules.md)
15. [regular expressions](regular-expressions.md)
16. [streaming](streaming.md)
17. [types and values](types-and-values.md)

<!-- tq-manual-compare:begin section=index -->
## Results

| Verdict | Cases |
| --- | ---: |
| failure | 3 |
| match | 949 |

Independent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.

| Output campaign | Matches | Cases |
| --- | ---: | ---: |
| compact_json | 918 | 921 |
| toon | 921 | 921 |

A match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from matches. Historical expected-difference labels do not pass either gate.

Missing features and unaccepted mismatches remain failures. Reference discrepancies describe errors in the imported manual, not successful compatibility.

JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.

### Output size

876 eligible examples. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with default LF-terminated `-o toon` results; explicitly requested `--seq -o toon` output is shown in the cases but excluded from size totals. Diff is TOON tokens minus JSON tokens. % is the signed percent difference `(TOON - JSON) / JSON`, so savings are negative and growth is positive.

| Tokenizer | JSON tokens | TOON tokens | Diff | % |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 11363 | 8187 | -3176 | -27.95% |
| `cl100k_base` | 11337 | 8197 | -3140 | -27.7% |

Only successful jq/JSON/TOON-equivalent results enter the totals. A negative `Diff` means TOON uses fewer tokens; `%` is negative for savings and positive for growth. The manual is a correctness corpus, not a representative workload benchmark.

### Reviewed disparities and historical differences

| Case | Reason |
| --- | --- |

### Sections

Each page uses the matching case collection in tests/compatibility/reviews/jq-manual, including witnesses assigned to that collection by completeness.toon. Cases linked by multiple collections appear in each page; the totals above count each case once. Inventory and execution metadata do not create case pages.

1. [advanced-features](advanced-features.md): 45 cases
2. [arity-inventory](arity-inventory.md): 1 cases
3. [assignment](assignment.md): 24 cases
4. [basic-filters](basic-filters.md): 51 cases
5. [builtin-operators-and-functions](builtin-operators-and-functions.md): 229 cases
6. [colors](colors.md): 3 cases
7. [composition-inventory](composition-inventory.md): 297 cases
8. [conditionals-and-comparisons](conditionals-and-comparisons.md): 25 cases
9. [introduction](introduction.md): 3 cases
10. [invoking-jq](invoking-jq.md): 48 cases
11. [io](io.md): 9 cases
12. [math](math.md): 64 cases
13. [math-boundaries](math-boundaries.md): 10 cases
14. [modules](modules.md): 21 cases
15. [path-boundaries](path-boundaries.md): 8 cases
16. [prose-boundaries](prose-boundaries.md): 41 cases
17. [regex-boundaries](regex-boundaries.md): 3 cases
18. [regular-expressions](regular-expressions.md): 28 cases
19. [result-stream-boundaries](result-stream-boundaries.md): 23 cases
20. [streaming](streaming.md): 7 cases
21. [types-and-values](types-and-values.md): 23 cases

### Regenerate

Run from the repository root. The comparison binary runs separately from the default test suite.

    cargo run -p tq-test-support --bin tq-manual-compare -- tests/compatibility/reviews/jq-manual/comparison.toon

To render the saved observations without running the tools again:

    cargo run -p tq-test-support --bin tq-manual-compare -- --render-only tests/compatibility/reviews/jq-manual/comparison.toon
<!-- tq-manual-compare:end -->
