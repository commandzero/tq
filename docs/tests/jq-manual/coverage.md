---
type: Report
title: "jq manual coverage"
description: "Current jq manual test coverage, comparison results, and known differences."
---

# jq manual coverage

The suite covers all **251 published input/output examples** in the jq manual,
accounts for **68 fenced snippets**, and adds tests for documented behavior,
composition, and edge cases. It runs **952 unique cases** against pinned jq 1.8.1.
A mapped example is covered, but coverage alone does not mean it passes.

## Results

The latest recorded macOS comparison reports:

| Check | Matches | Cases |
| --- | ---: | ---: |
| Values and process behavior, including CLI contracts | 944 | 952 |
| Exact compact JSON output | 919 | 921 |
| TOON values and process behavior | 921 | 921 |

JSON value comparison ignores whitespace and object key order, but preserves
array and result order. Compact JSON and CLI contracts compare exact bytes.
Raw CLI cases are not included in the JSON/TOON output checks.

**Eight cases remain strict failures:** two accepted math rounding differences
and six color-output differences. The all-cases exact gate therefore does not
pass. These results describe the measured build, not every platform or release.

## Known differences

### Math rounding

Two cases are accepted as **low-significance rounding differences**:

| Query and input | jq result | tq result | Difference |
| --- | --- | --- | --- |
| `erfc` on `2` | `0.0046777349810472645` | `0.004677734981047266` | tq is 2 ULP higher |
| `tgamma` on `0.5` | `1.772453850905516` | `1.7724538509055159` | tq is 1 ULP lower |

ULP measures spacing between adjacent floating-point values. tq's safe-Rust
`libm` implementation rounds differently from the reference's system math
functions. Both tools exit successfully with empty stderr.

Acceptance applies only to these measured inputs. The tests retain the exact
mismatches; there is no blanket tolerance or guarantee for other inputs.

### Color output

Six cases in [colors](colors.md) and [invoking jq](invoking-jq.md) differ in ANSI
styling. tq uses its own palette and enclosing-quote styling rather than jq's
exact presentation. These byte-contract failures are separate from the accepted
math differences.

## Longest-match regex support

The manual's `match("a|ab"; "l")` example passes. The implementation uses the
existing safe-Rust `fancy-regex` library, with no added dependency or FFI.
Conservative match-length bounds avoid unnecessary endpoint searches.

Local release-build measurements include startup, execution, and output:

| Workload | jq median | tq median | tq / jq |
| --- | ---: | ---: | ---: |
| One manual example | 4.69 ms | 5.31 ms | 1.13× |
| 1,000 manual examples in one process | 9.37 ms | 54.93 ms | 5.87× |
| `match("a"; "l")` on `"a"` followed by 500 `"b"` characters | 4.67 ms | 6.66 ms | 1.43× |

This performance is accepted for the fix. Repeated compilation remains a cost
for batched inputs. Complex searches can reach resource limits; whole-pattern
recursion with `l` is unsupported, and some engine-specific edge cases remain.
These measurements are not general performance guarantees.

## Details and verification

- [Comparison details](../../jq-compatibility-disparities.md): exact math observations, regex limits, benchmark method, and executable identities.
- [Source inventory](../../../tests/compatibility/reviews/jq-manual/source-examples.toon): manual examples and source-to-test mappings.
- [Test setup](../../../tests/compatibility/readme.md#jq-manual-coverage): reference setup and coverage checks.

The core suite passed, including focused runs of 36 core regex tests and 3 CLI
regex tests. Core Clippy and formatting checks passed. Full-workspace testing
has not been verified to completion.

Run a new comparison from the repository root after configuring the reference
and building tq:

```sh
cargo run -p tq-test-support --bin tq-manual-compare -- \
  --markdown-dir target/jq-manual-review target/manual-comparison.toon
```

The latest local evidence is in `target/manual-comparison-optimized.toon`, with
rendered pages in `target/jq-manual-review-optimized`. These generated files are
ignored by Git.
