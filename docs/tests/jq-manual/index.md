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

The Results below are fresh **2026-10-07 tq 0.5.0 / native TOON 4.1**
observations on macOS ARM64, release/default, compared with unchanged pinned
jq 1.8.2. All 952 unique cases ran; 9 exact differences remain and strict exit
status is **1**. The original 518 IDs and all 303 protected exact contracts
remain present. No historical stdout or token rows were reused.

The reviewed C9 source snapshot contains 903 files on integrated revision
`407b35d81681b8ca4ab85968f7186eea645f250f`, SHA-256
`b0a3e076abc748319859ea9810fd75bc93132b05c5b07cc116f97ffe27ecbc62`.
C9 rebuilds the native manual comparator to enforce required process contracts;
all product sources and the frozen C8 native CLI remain unchanged. The CLI
SHA-256 is `8a0ba42cf81881829b93ee950db0d4d3f808a22af574b210484be7833b238d00`;
the rebuilt comparator is `a581e66aa551e2c3397ae4507c6daf8b7a6def331e2ce5697664e3ed134e4cf4`.
Later documentation/checklist/spec synchronization is outside that snapshot.
Full fresh observations, source/build manifests and executable identities are
retained locally under ignored `target/toon-4-1/candidate-c9/`; C8 evidence is
retained separately, not relabeled.

All 21 section Results blocks were regenerated from this candidate report.
Authored source-inventory/audit text outside those blocks is historical.
The unique-case index excludes section overlap. Ordinary stdout counts include
LF; both tokenizers retain signed growth and savings. Sequence/raw/error or
failed observations do not acquire ordinary-output size eligibility.

See [coverage](coverage.md) for current native-target scope and the
[numbered differences](coverage.md#differences-by-test) for practical impact.
The [PR68 closeout ledger](../../../tests/compatibility/reviews/parity-closeout.toon)
and [earlier platform metadata](../../../tests/compatibility/reviews/native-platform-acceptance.toon)
remain historical, unchanged evidence—not this migration's executable identity.
Deferred #69/#70/#31 obligations are not waived. No exact all-platform or
calibrated performance acceptance is claimed.
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
| `o200k_base` | 11391 | 8152 | -3239 | -28.43% |
| `cl100k_base` | 11365 | 8165 | -3200 | -28.16% |

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
