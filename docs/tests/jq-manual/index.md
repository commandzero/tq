# jq manual test reviews

1. [Coverage metadata](../../../tests/compatibility/reviews/jq-manual): machine-readable case mappings, source inventory, and reference pins. Full execution reports are generated under ignored `target/` storage.
2. [advanced features](advanced-features.md)
3. [arity inventory](arity-inventory.md)
4. [assignment](assignment.md)
5. [basic filters](basic-filters.md)
6. [builtin operators and functions](builtin-operators-and-functions.md)
7. [colors](colors.md)
8. [composition inventory](composition-inventory.md)
9. [conditionals and comparisons](conditionals-and-comparisons.md)
10. [coverage](coverage.md)
11. [introduction](introduction.md)
12. [invoking jq](invoking-jq.md)
13. [io](io.md)
14. [math model](math-model.md)
15. [math](math.md)
16. [math boundaries](math-boundaries.md)
17. [modules](modules.md)
18. [path boundaries](path-boundaries.md)
19. [prose boundaries](prose-boundaries.md)
20. [regex boundaries](regex-boundaries.md)
21. [regular expressions](regular-expressions.md)
22. [result stream boundaries](result-stream-boundaries.md)
23. [streaming](streaming.md)
24. [types and values](types-and-values.md)

The Results below use the 2026-10-06 newhelp-final macOS ARM64 final-candidate observations:
tq 0.4.1, release/default, compared with pinned jq 1.8.2. The source is base
`3e0dedb` plus uncommitted implementation, reconciled catalog, runner P2, and
CLI help fixes, not a clean-commit build or the historical bench-profile capture. The run covers
952 cases; 9 exact-contract differences remain and the strict command exits 1.

See the [numbered difference list](coverage.md#differences-by-test) for each
test's practical impact. [Earlier late-P2 closeout metadata](../../../tests/compatibility/reviews/parity-closeout.toon)
records that checkpoint's source manifests, profiles, executable/report hashes,
and retained raw evidence. The newhelp-final snapshot adds corrected CLI help
and its regression test; fresh product/helper binaries were frozen after the
final release/default build. Current hashes and validation provenance are retained
in `target/closeout/macos/newhelp-final/summary.json`. Full observations are
retained locally in ignored
`target/closeout/macos/newhelp-final/manual-full-final.json`.

The native Windows host was offline on 2026-10-06 (SSH exit 255), so its
candidate refresh remains unresolved. [Historical platform metadata](../../../tests/compatibility/reviews/native-platform-acceptance.toon)
retains the Windows v6 checkpoint; it does not prove current-candidate execution.
These macOS results do not establish all-platform or performance acceptance.
See the [jq/tq option inventory](../../jq-1.8-cli-options.md) for the CLI contract.

<!-- tq-manual-compare:begin section=index -->
## Results

| Verdict | Cases |
| --- | ---: |
| Differences | 9 |
| Exact match | 943 |

Independent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.

| Output campaign | Exact matches | Cases |
| --- | ---: | ---: |
| compact_json | 919 | 921 |
| toon | 921 | 921 |

An exact match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from exact matches. Historical expected-difference labels do not pass either gate.

Differences include missing features and unaccepted mismatches; these still fail the strict and completion gates. Reference discrepancies describe errors in the imported manual, not successful compatibility.

JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.

### Output size

877 eligible examples. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with default LF-terminated `-o toon` results; TOON sequence captures, when available, are shown in the cases but excluded from size totals. Diff is TOON tokens minus JSON tokens. % is the signed percent difference `(TOON - JSON) / JSON`, so savings are negative and growth is positive.

| Tokenizer | JSON tokens | TOON tokens | Diff | % |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 11391 | 8204 | -3187 | -27.98% |
| `cl100k_base` | 11365 | 8214 | -3151 | -27.73% |

Only successful jq/JSON/TOON-equivalent results enter the totals. A negative `Diff` means TOON uses fewer tokens; `%` is negative for savings and positive for growth. The manual is a correctness corpus, not a representative workload benchmark.

### Differences

1. `manual.audit.math.erfc-ulp`: Expected rounding difference: tq is 2 ULP higher for erfc(2).
2. `manual.audit.math.tgamma-ulp`: Expected rounding difference: tq is 1 ULP lower for tgamma(0.5).
3. `manual.colors.ansi-values`: Expected presentation difference: quote styling and ANSI escapes differ; JSON data is unchanged.
4. `manual.colors.custom-1-31`: Expected presentation difference: quote styling and ANSI escapes differ; JSON data is unchanged.
5. `manual.colors.default-palette`: Expected presentation difference: quote styling and ANSI escapes differ; JSON data is unchanged.
6. `manual.invoking.color-output`: Expected presentation difference: quote styling and ANSI escapes differ; JSON data is unchanged.
7. `manual.invoking.jq-colors`: Expected presentation difference: quote styling and ANSI escapes differ; JSON data is unchanged.
8. `manual.invoking.no-color-forced`: Expected presentation difference: quote styling and ANSI escapes differ; JSON data is unchanged.
9. `manual.invoking.run-tests`: jq prints extra internal self-test messages; both tools pass the supplied test and exit 0.

### Sections

Each page uses the matching case collection in tests/compatibility/reviews/jq-manual, including witnesses assigned to that collection by completeness.toon. Cases linked by multiple collections appear in each page; the totals above count each case once. Inventory and execution metadata do not create case pages.

1. [advanced-features](advanced-features.md): 45 cases
2. [arity-inventory](arity-inventory.md): 1 case
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

    cargo run -p tq-test-support --bin tq-manual-compare -- target/manual-comparison.toon

To render the saved observations without running the tools again:

    cargo run -p tq-test-support --bin tq-manual-compare -- --render-only target/manual-comparison.toon
<!-- tq-manual-compare:end -->
