---
type: Report
title: "jq manual coverage audit, original baseline"
description: "Recorded review of jq manual coverage audit, original baseline."
generated: { by: codex/gpt-6, at: 2026-09-10T02:12:04Z }
---

# jq manual coverage audit, original baseline

This audit records the initial 518-case baseline, before the manual-parity
implementation. Its execution counts and verification notes are historical.
See the current comparison results in the
[jq manual index](index.md) and
[compatibility README](../../../tests/compatibility/README.md#jq-manual-coverage) for later evidence and
the expanded inventory. A source mapping establishes coverage, not a passing
execution.

All 251 published input/output examples have exact, runnable jq/tq cases. All 13 sections and 68 fenced snippets are audited. The suite adds 517 cases.

Each section received a separate gpt-5.6-luna audit at xhigh reasoning. The parent reviewed the integration and the introduction's 3 filter examples.

The [original execution snapshot](../../../tests/compatibility/reviews/jq-manual/execution.toon) records 518 unique referenced cases: 303 matches, 15 historical expected differences, 198 compatibility failures, and 2 reference discrepancies. These are baseline classifications, not current approvals or passing totals.

The original snapshot measured characters, not tokens. Use the current
comparison's tokenizer measurements for token savings; the original character
counts do not establish LLM billing savings.

The 649 inventory entries include 601 mappings to executable cases and 48 descriptive statements or incomplete syntax fragments. Each unmapped entry has a reason in its section ledger. Output-only blocks and repeated examples can point to the same case.

## Section results

Structured cases compare jq with explicit `tq -o json`, ignoring JSON presentation differences. CLI contracts retain their original arguments. Cases reused across sections appear in more than one row. The full campaign also executes supported native input-format variants.

| Section | Tables | Fences | Cases | Match | Expected difference | Failure | Reference discrepancy |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [advanced features](advanced-features.md) | 25 | 25 | 44 | 20 | 0 | 24 | 0 |
| [assignment](assignment.md) | 6 | 8 | 24 | 17 | 0 | 7 | 0 |
| [basic filters](basic-filters.md) | 30 | 2 | 51 | 43 | 1 | 7 | 0 |
| [builtin operators and functions](builtin-operators-and-functions.md) | 146 | 5 | 175 | 103 | 0 | 72 | 0 |
| [colors](colors.md) | 0 | 0 | 3 | 0 | 3 | 0 | 0 |
| [conditionals and comparisons](conditionals-and-comparisons.md) | 19 | 7 | 25 | 24 | 0 | 1 | 0 |
| [invoking jq](invoking-jq.md) | 0 | 3 | 48 | 35 | 8 | 5 | 0 |
| [io](io.md) | 0 | 5 | 9 | 1 | 1 | 7 | 0 |
| [math](math.md) | 0 | 0 | 62 | 3 | 0 | 57 | 2 |
| [modules](modules.md) | 0 | 1 | 21 | 16 | 1 | 4 | 0 |
| [regular expressions](regular-expressions.md) | 17 | 3 | 27 | 15 | 1 | 11 | 0 |
| [streaming](streaming.md) | 3 | 0 | 6 | 3 | 0 | 3 | 0 |
| [types and values](types-and-values.md) | 5 | 9 | 23 | 23 | 0 | 0 | 0 |
| [introduction](introduction.md) | 0 | 0 | 3 | 3 | 0 | 0 | 0 |

## Source and scope

1. [Source inventory](../../../tests/compatibility/reviews/jq-manual/source-examples.toon) preserves the 251 table examples, 68 fenced snippets, exact queries/inputs, published outputs, source lines, and section fingerprints from the companion jq-manual bundle.
2. Two imported expected strings omit a space: the `join(" ")` example and an optional regex capture example. Published text stays intact; `reference_stdout` and `reference_note` record the jq 1.8.1 observations.
3. The math section lists `frexp` and `modf` as two-input functions. jq 1.8.1 provides `/0`. Tests retain both `/2` compile-error probes and valid `/0` witnesses.
4. Prose examples without inputs use documented representative fixtures. Incomplete metavariables and ellipses remain explicit exclusions.
5. Shell pipelines become stdin/argv cases. PowerShell/cmd tokenization, Windows CRLF translation, terminal auto-detection, and precise flush timing remain unverified. Adapted cases test the observable direct-process behavior.
6. Expected differences require a specific documented policy and observed behavior, including numeric limits, regex longest-match mode, module startup, line metadata, and CLI output conventions. Missing features and unaccepted mismatches remain compatibility failures. No new compatibility baseline was accepted.

## Verification

1. The test-support suite passed 110 tests serially. Its external-reference test is ignored by default and passed when run explicitly, for 111 passing tests across both runs.
2. The source-reference check verified all 251 table outputs against jq 1.8.1, with the 2 recorded whitespace discrepancies and controlled `PAGER=less`.
3. The full campaign ran 784 catalog cases with 0 harness errors. The portable report records executable hashes and the catalog hash; the complete local observations are in `target/manual-compatibility.json`.
4. Formatting, catalog schemas, source-to-case guards, and test-support Clippy checks passed.

`tq` was built from this worktree at base commit `6fc9c54`. The reference tools were jq 1.8.1 and yq 4.53.2. These results describe that build, not later changes on other branches.

See the [compatibility README](../../../tests/compatibility/README.md#jq-manual-coverage) for commands to rerun coverage checks and the full campaign.
