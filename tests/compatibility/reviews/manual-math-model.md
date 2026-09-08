# Math audit model and TOON savings

The corrected math ledger is stored as [canonical TOON](manual-math.toon).
The measurements below retain the comparison with equivalent JSON. Generate
JSON when needed rather than maintaining a duplicate fixture.

## Model

| Collection | Rows | Meaning | Case reference |
| --- | ---: | --- | --- |
| `examples` | 62 | One executable query/input scenario per row | Required scalar `case_id` |
| `coverage_notes` | 6 | One audited prose claim or scope statement per row | Nullable scalar `evidence_case_id` |

The examples cover 60 named functions and 2 arity corrections. The 3 arity
notes cite the existing `sin`, `pow`, and `fma` cases rather than duplicating
them as examples; the other 3 notes have no executable evidence.

Every row has scalar fields, so both collections use TOON tables. No arrays
were stringified, no case coverage was dropped, and no fixtures are needed in
this section beyond the inline inputs already recorded.

`query` and `input` retain their exact source-text meaning. For example, the
input text `"null"` is distinct from a null `evidence_case_id`, which means that
the note has no cited case.

The [glossary](../../../CONTEXT.md) separates source passages, executable
examples, cases, coverage notes, and fixtures. Every section now uses this model;
sections with several witnesses per claim also have a separate evidence table.

## Measurements

All three representations contain the same data. Character counts include
one final LF and count Unicode scalar values, not bytes; TOON is an unframed
document rather than a record-separated output sequence.

| Representation | Characters | `o200k_base` tokens | `cl100k_base` tokens |
| --- | ---: | ---: | ---: |
| Pretty JSON, two-space indentation | 21,091 | 5,518 | 5,571 |
| Compact JSON | 16,176 | 3,744 | 3,775 |
| Tabular TOON | 10,486 | 2,409 | 2,463 |

| TOON savings against | Characters | `o200k_base` tokens | `cl100k_base` tokens |
| --- | ---: | ---: | ---: |
| Pretty JSON | 50.28% | 56.34% | 55.79% |
| Compact JSON | 35.18% | 35.66% | 34.75% |

Token counts were measured with `tiktoken` 0.14.0 and the named encodings.
They count document text only, without chat-message framing or prompt instructions.

The old JSON ledger was 22,638 characters, but its model was different. The
tables above compare only the corrected model, so schema cleanup is not credited
as an encoding saving. This is one audit document, not a general workload benchmark.

## Verification and reproduction

The tests check TOON/JSON equivalence, scalar table rows, 62 unique case
references, matching queries and inputs, and the 6 notes. Audit readers prefer
TOON and fail on invalid canonical TOON.

```sh
cargo test -p tq-test-support --test compatibility_manual
tq --input-format toon -o json . tests/compatibility/reviews/manual-math.toon | tq -x
```

Measure either document by passing it on stdin. This installs the tokenizer
in an isolated uv environment, without adding a repository dependency.

```sh
uv run --no-project --with tiktoken==0.14.0 python -c '
import sys, tiktoken
text = sys.stdin.read()
print("characters", len(text))
for name in ["o200k_base", "cl100k_base"]:
    print(name, len(tiktoken.get_encoding(name).encode(text)))
' < tests/compatibility/reviews/manual-math.toon
```

For compact JSON, apply `json.dumps(json.loads(text), ensure_ascii=False,
separators=(",", ":")) + "\n"` before counting the JSON document.
