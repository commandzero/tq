---
type: Report
title: Test fixture storage savings
description: Exact character and tokenizer measurements for the root tests directory migration to TOON.
generated: { by: codex/gpt-5, at: 2026-09-08T04:25:45Z }
---

# Test fixture storage savings

The migration replaces 80 JSON files under the repository-root `tests/`
directory with TOON. Only those 80 converted files enter this report's totals.

The retained conversions and ledger model corrections save **163,266 o200k_base tokens, or 18.41%**,
against the original files as stored, not a regenerated compact-JSON baseline.

## Totals

| Storage change | Files | Original o200k_base tokens | Current o200k_base tokens | Tokens saved | Saved % |
| --- | ---: | ---: | ---: | ---: | ---: |
| JSON to TOON, including ledger model corrections | 80 | 886,759 | 723,493 | 163,266 | 18.41% |

| Tokenizer, converted JSON documents only | Original tokens | TOON tokens | Tokens saved | Saved % |
| --- | ---: | ---: | ---: | ---: |
| o200k_base | 886,759 | 723,493 | 163,266 | 18.41% |
| cl100k_base | 884,231 | 721,824 | 162,407 | 18.37% |

Character counts for the 80 converted files fall from 3,009,601 to 2,310,612.
UTF-8 bytes fall from 3,009,669 to 2,310,680. These are storage measurements,
not the basis of Saved %.

Three retained JSON-to-TOON conversions also increase token counts. Their
negative savings remain visible in the per-file table.

The corrected [math ledger](../tests/compatibility/reviews/jq-manual/math.toon)
remains a useful tabular example. Its 62 executable examples and 6 coverage notes
use scalar columns. It shrinks from 5,518 to 2,409 o200k_base tokens, saving
56.34%. Its character count falls from 21,091 to 10,486.

## Scope and method

The baseline is the exact pre-migration file text, including existing whitespace,
compared with newline-terminated, unframed TOON documents. It comprises 78
formatted JSON files, one compact JSON file, and one JSON5 source fixture.
The per-file table labels each baseline format. The current totals include the subsequent
manual-ledger model correction. Its effect and a same-model encoding comparison
are reported separately below, so model savings are not called encoding savings.

Character counts are Unicode scalar counts. Token counts use `tiktoken 0.14.0`
with `o200k_base` and `cl100k_base`, tokenizing each file independently with
`disallowed_special=()`, then summing. Counts exclude chat framing.

Saved % means `100 * (original_tokens - current_tokens) / original_tokens`,
using o200k_base unless another tokenizer is named. Totals sum tokens before
calculating percentages; they do not average per-file percentages.
The tables retain negative savings. They describe this fixture collection,
not a general claim about TOON or model-specific billing.

The existing math TOON copy is counted once as the destination of its JSON
counterpart. Removing that duplicate saves another 10,486 characters on disk
beyond the encoding-only comparison above.

The scope excludes `schemas/`, `benchmarks/`, downloaded data, and crate-local
`crates/*/tests/` assets. In particular, the CLI's separately packaged JSON
compatibility report keeps its existing public output contract.

## Manual ledger model

All 14 section ledgers, including introduction, use schema version 2. No example
has a `case_ids` array. The model separates runnable scenarios from the source
claims that cite them.

| Collection | Row meaning | Case reference |
| --- | --- | --- |
| `examples` | One executable scenario, unique by case within its section | Required scalar `case_id` |
| `coverage_notes` | One original source claim, shared input, output passage, exclusion, or additional citation | Nullable scalar `evidence_case_id` for a single witness |
| `coverage_evidence` | One relationship for a note with multiple witnesses | Scalar `note_id` and `case_id` |
| `source_values` | Original structured expected or observed output, kept outside scalar audit rows | Scalar `entry_id` and `field`, with the original typed `value` |

Examples and notes have uniform scalar columns and encode as TOON tables.
Missing optional fields are explicit nulls. Structured output samples retain
their arrays and objects in `source_values`; they are not stringified to force
a table. `entry_id` points to the original example or note.

Repeated citations become coverage notes rather than duplicate executable
examples. Cases previously cited only by prose receive one concrete witness
example using the catalog's query and input. One control-byte input is preserved
as scalar `input_hex` with null `input`; the tests compare its exact bytes.

The correction preserves original passage IDs, source locations, kinds, query
and input text, rationale, output samples, and every evidence relationship.
Source fences can be accounted for by either examples or notes. Every evidence
case must also have an executable example in its section.

The 13 newly corrected ledgers use 49,907 o200k_base tokens, down from 57,110
in their previous TOON model. That is a **12.61% model-change saving**. Math
was already corrected and is excluded from this comparison.

| Same corrected model, 13 ledgers | o200k_base tokens | TOON saved % |
| --- | ---: | ---: |
| Pretty JSON, two-space indentation | 92,129 | 45.83% |
| Compact JSON | 65,733 | 24.08% |
| TOON | 49,907 | 0.00% |

These JSON measurements serialize the current decoded models with
`ensure_ascii=False` and one final newline, using `indent=2` or
`separators=(",", ":")`. They are generated comparisons, not stored fixtures.
Some individual ledgers grow because they now record concrete witnesses and
explicit evidence relationships. The totals and per-file table retain those costs.

## Preservation and test behavior

The initial encoding migration checked equality before deletion. The later
ledger model correction separately verified every original field and every
source-to-case relationship against backups before replacing the ledgers.

Every source was parsed and compared with its decoded TOON replacement before
any source was deleted. The check used arbitrary-precision numbers, not binary64
or a JavaScript JSON parse. Record order, string contents, and numeric lexemes
were preserved.

The [fixture reader](../crates/tq-test-support/src/fixture_data.rs) uses two
reserved single-field objects for values outside ordinary TOON storage.

| Stored record | Meaning | Occurrences |
| --- | --- | ---: |
| `$tq.fixture.number: "<literal>"` | Restore the exact numeric literal, including values outside tq's numeric envelope | 103 |
| `$tq.fixture.utf8hex: "<hex>"` | Restore UTF-8 string bytes containing unsupported control characters | 0 |

These are fixture-schema records, not extensions to TOON syntax. A generic TOON
reader sees the objects; the fixture reader restores their original types.
Their storage overhead is included in every measurement.

The JSON5 regression fixture stores its original source text in a TOON
`source_text` field. Its comments, unquoted keys, and multiline literal still
reach the JSON5 parser unchanged. The jq JSON-module example materializes
`data.json` from `data.toon` in a temporary directory for each execution.

Corpus registries, baseline writers, Stack Overflow scenario generation, manual
reports, and documentation links now use the TOON files. Runtime inputs still
use each case's declared format. Historical
campaign hashes remain historical evidence, not hashes of the migrated catalog.

One policy-document path in the stored manual comparison was updated from
`.json` to `.toon` after the equivalence check. Its final text is included in
the measurements.

## Verification and reproduction

The migration retains exact originals locally under
`target/fixture-json-backup/tests/`. That ignored backup is recoverable until
the build directory is cleaned. JSON originals no longer exist under `tests/`.
A regression test checks fixture storage conventions.

The post-migration manual run retained all 518 per-case verdicts, TOON
equivalence results, and output character counts. The jq reference check also
passed against all 251 published input/output examples.

Run the storage and compatibility checks from the repository root:

```sh
rtk cargo test --workspace --locked
rtk cargo clippy --workspace --all-targets --locked -- -D warnings
```

Clippy exits successfully, with a pre-existing unknown-lint warning for
`clippy::assert_is_empty`. Full `okf validate docs` still reports missing
metadata on eight existing documents; this report and the new index have no
reported validation errors.

To reproduce a file pair's counts, pass its backup and TOON paths:

```sh
uv run --no-project --with tiktoken==0.14.0 python -c '
import sys, pathlib, tiktoken
for filename in sys.argv[1:]:
    raw = pathlib.Path(filename).read_bytes()
    text = raw.decode("utf8")
    counts = [
        len(tiktoken.get_encoding(name).encode(text, disallowed_special=()))
        for name in ["o200k_base", "cl100k_base"]
    ]
    print(filename, "bytes", len(raw), "characters", len(text), "tokens", counts)
' target/fixture-json-backup/tests/compatibility/reviews/manual-math.json \
  tests/compatibility/reviews/jq-manual/math.toon
```

## Per-file measurements

These measurements compare **original JSON as stored**, not uniformly compact
JSON, with the current TOON files. The **Baseline format** column identifies
formatted JSON, compact JSON, or the JSON5 source exception. Formatted JSON keeps
its original indentation and spacing; it has not been reserialized or minified.

The separate [same-model comparison](#manual-ledger-model) explicitly measures
both pretty JSON and compact JSON. Its compact-JSON figures are not used below.

Labels retain the original extension to identify the baseline. Links open the
replacement TOON file.

| Fixture | Baseline format | Original JSON o200k tokens | TOON o200k tokens | Saved % | Original JSON characters | TOON characters |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| [tests/compatibility/baselines/jq-yq-mvp-v1.json](../tests/compatibility/baselines/jq-yq-mvp-v1.toon) | Formatted JSON | 65,421 | 56,436 | 13.73 | 219,238 | 180,664 |
| [tests/compatibility/reviews/coverage-v1.json](../tests/compatibility/reviews/coverage-v1.toon) | Formatted JSON | 216,909 | 178,167 | 17.86 | 770,553 | 584,084 |
| [tests/compatibility/reviews/jq-yq-mvp-v1.json](../tests/compatibility/reviews/jq-yq-mvp-v1.toon) | Formatted JSON | 14,194 | 6,399 | 54.92 | 57,195 | 27,743 |
| [tests/compatibility/reviews/manual-advanced-features.json](../tests/compatibility/reviews/jq-manual/advanced-features.toon) | Formatted JSON | 5,618 | 4,668 | 16.91 | 19,308 | 15,358 |
| [tests/compatibility/reviews/manual-assignment.json](../tests/compatibility/reviews/jq-manual/assignment.toon) | Formatted JSON | 2,349 | 2,234 | 4.90 | 9,222 | 8,832 |
| [tests/compatibility/reviews/manual-basic-filters.json](../tests/compatibility/reviews/jq-manual/basic-filters.toon) | Formatted JSON | 6,380 | 5,287 | 17.13 | 23,550 | 19,340 |
| [tests/compatibility/reviews/manual-builtin-operators-and-functions.json](../tests/compatibility/reviews/jq-manual/builtin-operators-and-functions.toon) | Formatted JSON | 27,476 | 18,875 | 31.30 | 95,275 | 57,387 |
| [tests/compatibility/reviews/manual-colors.json](../tests/compatibility/reviews/jq-manual/colors.toon) | Formatted JSON | 3,262 | 2,491 | 23.64 | 12,289 | 9,561 |
| [tests/compatibility/reviews/manual-comparison.json](../tests/compatibility/reviews/jq-manual/comparison.toon) | Formatted JSON | 320,112 | 262,515 | 17.99 | 1,055,087 | 812,434 |
| [tests/compatibility/reviews/manual-conditionals-and-comparisons.json](../tests/compatibility/reviews/jq-manual/conditionals-and-comparisons.toon) | Formatted JSON | 2,655 | 1,909 | 28.10 | 9,815 | 6,710 |
| [tests/compatibility/reviews/manual-execution.json](../tests/compatibility/reviews/jq-manual/execution.toon) | Formatted JSON | 97,165 | 75,602 | 22.19 | 358,100 | 270,958 |
| [tests/compatibility/reviews/manual-introduction.json](../tests/compatibility/reviews/jq-manual/introduction.toon) | Formatted JSON | 291 | 301 | -3.44 | 1,126 | 1,337 |
| [tests/compatibility/reviews/manual-invoking-jq.json](../tests/compatibility/reviews/jq-manual/invoking-jq.toon) | Formatted JSON | 6,136 | 5,248 | 14.47 | 24,421 | 22,041 |
| [tests/compatibility/reviews/manual-io.json](../tests/compatibility/reviews/jq-manual/io.toon) | Formatted JSON | 1,533 | 1,140 | 25.64 | 5,779 | 4,582 |
| [tests/compatibility/reviews/manual-math.json](../tests/compatibility/reviews/jq-manual/math.toon) | Formatted JSON | 5,518 | 2,409 | 56.34 | 21,091 | 10,486 |
| [tests/compatibility/reviews/manual-modules.json](../tests/compatibility/reviews/jq-manual/modules.toon) | Formatted JSON | 2,812 | 3,072 | -9.25 | 12,129 | 13,530 |
| [tests/compatibility/reviews/manual-regular-expressions.json](../tests/compatibility/reviews/jq-manual/regular-expressions.toon) | Formatted JSON | 2,752 | 1,833 | 33.39 | 9,524 | 6,355 |
| [tests/compatibility/reviews/manual-source-examples.json](../tests/compatibility/reviews/jq-manual/source-examples.toon) | Formatted JSON | 52,091 | 47,634 | 8.56 | 129,300 | 107,099 |
| [tests/compatibility/reviews/manual-streaming.json](../tests/compatibility/reviews/jq-manual/streaming.toon) | Formatted JSON | 1,518 | 1,029 | 32.21 | 5,612 | 3,984 |
| [tests/compatibility/reviews/manual-types-and-values.json](../tests/compatibility/reviews/jq-manual/types-and-values.toon) | Formatted JSON | 2,600 | 1,820 | 30.00 | 10,382 | 7,171 |
| [tests/compatibility/reviews/numeric-policy-v1.json](../tests/compatibility/reviews/numeric-policy-v1.toon) | Formatted JSON | 622 | 522 | 16.08 | 2,685 | 2,385 |
| [tests/corpus/sources/microsoft-us-buildings-georgia.json](../tests/corpus/sources/microsoft-us-buildings-georgia.toon) | Formatted JSON | 272 | 203 | 25.37 | 948 | 743 |
| [tests/corpus/sources/usgs-all-day.json](../tests/corpus/sources/usgs-all-day.toon) | Formatted JSON | 254 | 197 | 22.44 | 854 | 682 |
| [tests/corpus/sources/usgs-all-hour.json](../tests/corpus/sources/usgs-all-hour.toon) | Formatted JSON | 254 | 197 | 22.44 | 857 | 685 |
| [tests/corpus/sources/usgs-all-month.json](../tests/corpus/sources/usgs-all-month.toon) | Formatted JSON | 260 | 201 | 22.69 | 875 | 696 |
| [tests/corpus/sources/usgs-all-week.json](../tests/corpus/sources/usgs-all-week.toon) | Formatted JSON | 256 | 199 | 22.27 | 859 | 687 |
| [tests/fixtures/esdiag-saved-object.json](../tests/fixtures/esdiag-saved-object.toon) | JSON5 source | 76 | 101 | -32.89 | 307 | 361 |
| [tests/fixtures/manual-modules/data.json](../tests/fixtures/manual-modules/data.toon) | Compact JSON | 9 | 9 | 0.00 | 28 | 22 |
| [tests/platform/regex-date-platform-v1.json](../tests/platform/regex-date-platform-v1.toon) | Formatted JSON | 399 | 334 | 16.29 | 1,436 | 1,237 |
| [tests/stack-overflow/01-parsing-json-with-unix-tools.json](../tests/stack-overflow/01-parsing-json-with-unix-tools.toon) | Formatted JSON | 1,676 | 1,617 | 3.52 | 6,406 | 6,219 |
| [tests/stack-overflow/02-how-to-remove-double-quotes-in-jq-output-for-parsing-json-fi.json](../tests/stack-overflow/02-how-to-remove-double-quotes-in-jq-output-for-parsing-json-fi.toon) | Formatted JSON | 476 | 423 | 11.13 | 1,566 | 1,400 |
| [tests/stack-overflow/03-select-objects-based-on-value-of-variable-in-object-using-jq.json](../tests/stack-overflow/03-select-objects-based-on-value-of-variable-in-object-using-jq.toon) | Formatted JSON | 786 | 709 | 9.80 | 2,501 | 2,265 |
| [tests/stack-overflow/04-using-jq-to-parse-and-display-multiple-fields-in-a-json-seri.json](../tests/stack-overflow/04-using-jq-to-parse-and-display-multiple-fields-in-a-json-seri.toon) | Formatted JSON | 804 | 729 | 9.33 | 2,649 | 2,384 |
| [tests/stack-overflow/05-jq-how-to-filter-an-array-of-objects-based-on-values-in-an-i.json](../tests/stack-overflow/05-jq-how-to-filter-an-array-of-objects-based-on-values-in-an-i.toon) | Formatted JSON | 1,553 | 1,466 | 5.60 | 4,631 | 4,275 |
| [tests/stack-overflow/06-how-to-count-items-in-json-object-using-command-line.json](../tests/stack-overflow/06-how-to-count-items-in-json-object-using-command-line.toon) | Formatted JSON | 630 | 559 | 11.27 | 1,893 | 1,658 |
| [tests/stack-overflow/07-how-to-install-jq-on-mac-on-the-command-line.json](../tests/stack-overflow/07-how-to-install-jq-on-mac-on-the-command-line.toon) | Formatted JSON | 433 | 378 | 12.70 | 1,496 | 1,328 |
| [tests/stack-overflow/08-how-do-i-select-multiple-fields-in-jq.json](../tests/stack-overflow/08-how-do-i-select-multiple-fields-in-jq.toon) | Formatted JSON | 482 | 427 | 11.41 | 1,439 | 1,268 |
| [tests/stack-overflow/09-how-to-get-key-names-from-json-using-jq.json](../tests/stack-overflow/09-how-to-get-key-names-from-json-using-jq.toon) | Formatted JSON | 892 | 834 | 6.50 | 2,779 | 2,596 |
| [tests/stack-overflow/10-jq-select-multiple-conditions.json](../tests/stack-overflow/10-jq-select-multiple-conditions.toon) | Formatted JSON | 469 | 400 | 14.71 | 1,491 | 1,239 |
| [tests/stack-overflow/11-passing-bash-variable-to-jq.json](../tests/stack-overflow/11-passing-bash-variable-to-jq.toon) | Formatted JSON | 799 | 725 | 9.26 | 2,761 | 2,499 |
| [tests/stack-overflow/12-how-to-merge-2-json-objects-from-2-files-using-jq.json](../tests/stack-overflow/12-how-to-merge-2-json-objects-from-2-files-using-jq.toon) | Formatted JSON | 1,881 | 1,795 | 4.57 | 5,291 | 4,968 |
| [tests/stack-overflow/13-how-to-convert-arbitrary-simple-json-to-csv-using-jq.json](../tests/stack-overflow/13-how-to-convert-arbitrary-simple-json-to-csv-using-jq.toon) | Formatted JSON | 1,412 | 1,276 | 9.63 | 4,577 | 4,085 |
| [tests/stack-overflow/14-how-to-use-jq-in-a-shell-pipeline.json](../tests/stack-overflow/14-how-to-use-jq-in-a-shell-pipeline.toon) | Formatted JSON | 644 | 569 | 11.65 | 2,162 | 1,899 |
| [tests/stack-overflow/15-how-do-i-update-a-single-value-in-a-json-document-using-jq.json](../tests/stack-overflow/15-how-do-i-update-a-single-value-in-a-json-document-using-jq.toon) | Formatted JSON | 824 | 759 | 7.89 | 2,844 | 2,631 |
| [tests/stack-overflow/16-how-to-parse-a-json-string-with-jq-or-other-alternatives.json](../tests/stack-overflow/16-how-to-parse-a-json-string-with-jq-or-other-alternatives.toon) | Formatted JSON | 697 | 643 | 7.75 | 2,103 | 1,934 |
| [tests/stack-overflow/17-using-jq-or-alternative-command-line-tools-to-compare-json-f.json](../tests/stack-overflow/17-using-jq-or-alternative-command-line-tools-to-compare-json-f.toon) | Formatted JSON | 1,124 | 1,049 | 6.67 | 3,591 | 3,304 |
| [tests/stack-overflow/18-get-outputs-from-jq-on-a-single-line.json](../tests/stack-overflow/18-get-outputs-from-jq-on-a-single-line.toon) | Formatted JSON | 957 | 865 | 9.61 | 3,128 | 2,769 |
| [tests/stack-overflow/19-jq-to-replace-text-directly-on-file-like-sed-i.json](../tests/stack-overflow/19-jq-to-replace-text-directly-on-file-like-sed-i.toon) | Formatted JSON | 1,187 | 1,109 | 6.57 | 3,992 | 3,705 |
| [tests/stack-overflow/20-concat-2-fields-in-json-using-jq.json](../tests/stack-overflow/20-concat-2-fields-in-json-using-jq.toon) | Formatted JSON | 632 | 577 | 8.70 | 2,025 | 1,852 |
| [tests/stack-overflow/21-how-to-sort-a-json-file-by-keys-and-values-of-those-keys-in-.json](../tests/stack-overflow/21-how-to-sort-a-json-file-by-keys-and-values-of-those-keys-in-.toon) | Formatted JSON | 994 | 916 | 7.85 | 3,488 | 3,196 |
| [tests/stack-overflow/22-modify-a-key-value-in-a-json-using-jq-in-place.json](../tests/stack-overflow/22-modify-a-key-value-in-a-json-using-jq-in-place.toon) | Formatted JSON | 576 | 522 | 9.38 | 1,960 | 1,794 |
| [tests/stack-overflow/23-how-to-format-a-json-string-as-a-table-using-jq.json](../tests/stack-overflow/23-how-to-format-a-json-string-as-a-table-using-jq.toon) | Formatted JSON | 1,659 | 1,562 | 5.85 | 4,951 | 4,619 |
| [tests/stack-overflow/24-jq-print-key-and-value-for-each-entry-in-an-object.json](../tests/stack-overflow/24-jq-print-key-and-value-for-each-entry-in-an-object.toon) | Formatted JSON | 1,063 | 999 | 6.02 | 2,993 | 2,780 |
| [tests/stack-overflow/25-extract-a-specific-field-from-json-output-using-jq.json](../tests/stack-overflow/25-extract-a-specific-field-from-json-output-using-jq.toon) | Formatted JSON | 604 | 518 | 14.24 | 2,020 | 1,676 |
| [tests/stack-overflow/26-how-to-run-jq-from-gitbash-in-windows.json](../tests/stack-overflow/26-how-to-run-jq-from-gitbash-in-windows.toon) | Formatted JSON | 715 | 660 | 7.69 | 2,058 | 1,890 |
| [tests/stack-overflow/27-jq-output-array-of-json-objects.json](../tests/stack-overflow/27-jq-output-array-of-json-objects.toon) | Formatted JSON | 767 | 690 | 10.04 | 2,329 | 2,062 |
| [tests/stack-overflow/28-add-new-element-to-existing-json-array-with-jq.json](../tests/stack-overflow/28-add-new-element-to-existing-json-array-with-jq.toon) | Formatted JSON | 2,355 | 2,259 | 4.08 | 7,122 | 6,736 |
| [tests/stack-overflow/29-how-do-i-use-jq-to-convert-number-to-string.json](../tests/stack-overflow/29-how-do-i-use-jq-to-convert-number-to-string.toon) | Formatted JSON | 636 | 568 | 10.69 | 1,908 | 1,670 |
| [tests/stack-overflow/30-get-the-first-or-n-39-th-element-in-a-jq-json-parsing.json](../tests/stack-overflow/30-get-the-first-or-n-39-th-element-in-a-jq-json-parsing.toon) | Formatted JSON | 909 | 840 | 7.59 | 2,672 | 2,442 |
| [tests/stack-overflow/31-can-i-pass-a-string-variable-to-jq-rather-than-passing-a-fil.json](../tests/stack-overflow/31-can-i-pass-a-string-variable-to-jq-rather-than-passing-a-fil.toon) | Formatted JSON | 440 | 388 | 11.82 | 1,423 | 1,262 |
| [tests/stack-overflow/32-iterating-through-json-array-in-shell-script.json](../tests/stack-overflow/32-iterating-through-json-array-in-shell-script.toon) | Formatted JSON | 557 | 471 | 15.44 | 1,942 | 1,620 |
| [tests/stack-overflow/33-how-to-check-for-presence-of-39-key-39-in-jq-before-iteratin.json](../tests/stack-overflow/33-how-to-check-for-presence-of-39-key-39-in-jq-before-iteratin.toon) | Formatted JSON | 946 | 867 | 8.35 | 3,243 | 2,945 |
| [tests/stack-overflow/34-how-to-filter-array-of-objects-by-element-property-values-us.json](../tests/stack-overflow/34-how-to-filter-array-of-objects-by-element-property-values-us.toon) | Formatted JSON | 886 | 789 | 10.95 | 2,627 | 2,265 |
| [tests/stack-overflow/35-jq-conditional-output.json](../tests/stack-overflow/35-jq-conditional-output.toon) | Formatted JSON | 530 | 472 | 10.94 | 1,786 | 1,601 |
| [tests/stack-overflow/36-jq-cannot-index-array-with-string.json](../tests/stack-overflow/36-jq-cannot-index-array-with-string.toon) | Formatted JSON | 718 | 653 | 9.05 | 2,206 | 1,962 |
| [tests/stack-overflow/37-install-jq-json-processor-on-ubuntu-10-04.json](../tests/stack-overflow/37-install-jq-json-processor-on-ubuntu-10-04.toon) | Formatted JSON | 797 | 741 | 7.03 | 2,418 | 2,245 |
| [tests/stack-overflow/38-how-to-combine-the-sequence-of-objects-in-jq-into-one-object.json](../tests/stack-overflow/38-how-to-combine-the-sequence-of-objects-in-jq-into-one-object.toon) | Formatted JSON | 882 | 819 | 7.14 | 2,811 | 2,600 |
| [tests/stack-overflow/39-jq-not-working-on-tag-name-with-dashes-and-numbers.json](../tests/stack-overflow/39-jq-not-working-on-tag-name-with-dashes-and-numbers.toon) | Formatted JSON | 1,067 | 985 | 7.69 | 3,398 | 3,075 |
| [tests/stack-overflow/40-convert-string-to-json-in-jq.json](../tests/stack-overflow/40-convert-string-to-json-in-jq.toon) | Formatted JSON | 810 | 752 | 7.16 | 2,682 | 2,497 |
| [tests/stack-overflow/41-output-specific-key-value-in-object-for-each-element-in-arra.json](../tests/stack-overflow/41-output-specific-key-value-in-object-for-each-element-in-arra.toon) | Formatted JSON | 650 | 568 | 12.62 | 2,105 | 1,817 |
| [tests/stack-overflow/42-jq-how-to-query-for-array-values-that-don-39-t-contain-text-.json](../tests/stack-overflow/42-jq-how-to-query-for-array-values-that-don-39-t-contain-text-.toon) | Formatted JSON | 587 | 513 | 12.61 | 1,891 | 1,592 |
| [tests/stack-overflow/43-how-do-i-keep-colors-when-piping-quot-jq-quot-output-to-quot.json](../tests/stack-overflow/43-how-do-i-keep-colors-when-piping-quot-jq-quot-output-to-quot.toon) | Formatted JSON | 621 | 566 | 8.86 | 2,013 | 1,845 |
| [tests/stack-overflow/44-exclude-column-from-jq-json-output.json](../tests/stack-overflow/44-exclude-column-from-jq-json-output.toon) | Formatted JSON | 767 | 692 | 9.78 | 2,443 | 2,171 |
| [tests/stack-overflow/45-how-to-use-jq-when-the-variable-has-reserved-characters.json](../tests/stack-overflow/45-how-to-use-jq-when-the-variable-has-reserved-characters.toon) | Formatted JSON | 671 | 609 | 9.24 | 2,116 | 1,914 |
| [tests/stack-overflow/46-how-to-extract-a-field-from-each-object-in-an-array-with-jq.json](../tests/stack-overflow/46-how-to-extract-a-field-from-each-object-in-an-array-with-jq.toon) | Formatted JSON | 575 | 506 | 12.00 | 1,738 | 1,496 |
| [tests/stack-overflow/47-how-to-check-if-element-exists-in-array-with-jq.json](../tests/stack-overflow/47-how-to-check-if-element-exists-in-array-with-jq.toon) | Formatted JSON | 862 | 798 | 7.42 | 2,616 | 2,398 |
| [tests/stack-overflow/48-jq-select-value-from-array.json](../tests/stack-overflow/48-jq-select-value-from-array.toon) | Formatted JSON | 645 | 562 | 12.87 | 1,977 | 1,699 |
| [tests/stack-overflow/49-getting-all-the-values-of-an-array-with-jq.json](../tests/stack-overflow/49-getting-all-the-values-of-an-array-with-jq.toon) | Formatted JSON | 1,260 | 1,194 | 5.24 | 3,972 | 3,730 |
| [tests/stack-overflow/50-using-jq-with-bash-to-run-command-for-each-object-in-array.json](../tests/stack-overflow/50-using-jq-with-bash-to-run-command-for-each-object-in-array.toon) | Formatted JSON | 688 | 608 | 11.63 | 2,151 | 1,879 |
| [tests/stack-overflow-benchmarks.json](../tests/stack-overflow-benchmarks.toon) | Formatted JSON | 3,970 | 2,465 | 37.91 | 13,372 | 7,702 |
