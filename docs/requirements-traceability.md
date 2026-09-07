---
type: Reference
title: "Requirements traceability"
description: "Routes from specification scenarios to implementation and test evidence."
generated: { by: codex/gpt-6, at: 2026-09-07T06:22:04Z }
---

# Requirements traceability

This index covers the eight `build-tq-mvp` capability specifications. The
[scenario manifest](requirements-traceability.tsv) is the source of truth. It
has one row for every `#### Scenario:` heading, its requirement, and a typed
evidence locator. Locators use `test:path#symbol`, `report:path#symbol`, or
`manual:path#heading`.

The traceability test compares the manifest and OpenSpec files in both
directions. It rejects duplicate or stale IDs and titles, checks every evidence
path, and verifies that each named symbol or review heading exists.

| Capability spec | Scenarios | Primary automated evidence | Release evidence or manual check |
| --- | ---: | --- | --- |
| `benchmark-corpus` | 16 | `crates/tq-test-support/tests/corpus_*.rs` | Corpus source descriptors under `tests/corpus/`; generated corpus data stays ignored |
| `cross-tool-compatibility` | 20 | `crates/tq-test-support/tests/compatibility_*.rs` | `tests/compatibility/reviews/coverage-v1.json`; exact jq/tq divergence allowlist test |
| `jq-core-language` | 37 | `crates/tq-core/src/` unit/property tests and compatibility cases | Full compatibility report; unsupported/deferred capability matrix entries |
| `performance-benchmarks` | 26 | `crates/tq-test-support/tests/benchmark_*.rs` | Reviewed date-named report under `benchmarks/`; local collection data stays ignored |
| `query-runtime` | 21 | `crates/tq-core/src/` bytecode, compiler, evaluator, plan, and VM tests | Parser/bytecode/VM fuzz targets; `--explain-json` CLI tests |
| `resource-governance` | 17 | `crates/tq-core/src/`, `crates/tq-cli/src/`, and `crates/tq-toon/src/` limit/cancellation tests | Reviewed benchmark report and bounded fuzz targets |
| `toon-stream-io` | 21 | `crates/tq-toon/tests/` plus decoder/writer unit and property tests | TOON rows in standard/large reports and framing compatibility cases |
| `tq-cli` | 38 | `crates/tq-cli/src/` argument, source, execution, and output tests | Full compatibility report and README command examples |

All 196 scenarios have their own TSV row. Adding, renaming, removing, or moving
a scenario requires a reviewed evidence update. A source file alone does not
count as evidence. Manual locators expose requirement gaps but do not count as
automated coverage. Release reports record facts that hermetic tests cannot:
live source data, local tool identity, natural-large timing and RSS, signals,
and release fuzz time. Reviewers check the final date-named report by hand.

Release review checks the following without suppressing failures:

1. The full compatibility campaign contains no jq/tq mismatch outside the
   four reviewed framing/numeric-envelope cases.
2. Every capability has one of the six published dispositions and the
   `untested` count is zero.
3. Correctness gates run before benchmark timing, and JSON, YAML, and TOON
   corpus identities remain in each report. Structured correctness output is
   digested incrementally rather than accumulated as a complete result vector.
4. The natural-large explicit stream stays within its 128 MiB RSS envelope;
   blocking/document cases retain their observed outcome even when unfavorable.
5. Stable workspace tests and a Rust 1.88 compilation check, strict OpenSpec validation, Clippy,
   rustdoc, and all six bounded fuzz targets pass.

## Native format architecture

The active `extend-native-format-architecture` change extends the archived MVP
contract. Its evidence is listed below without rewriting historical MVP rows.
The change's task list records which live campaigns have completed.

| Requirement | Evidence |
| --- | --- |
| Closed catalog, format selection, and option compatibility | `crates/tq-formats/tests/catalog.rs`; `delimited_options_reject_incompatible_controls_before_input` and `seq_selects_json_sequences_and_rejects_conflicts_in_either_order` in `crates/tq-cli/tests/native_formats.rs` |
| Committed Documents, structural events, selected decoding, and typed limits | `crates/tq-formats/tests/committed_input.rs`, including `committed_input_selected_events_keep_json_lines_document_boundaries` and `committed_input_bounds_json_token_storage_before_reading_the_whole_token` |
| Direct transcode callbacks and source identity | `committed_input_codec_consumer_preserves_source_and_typed_failure` in `crates/tq-formats/tests/committed_input.rs`; existing CLI transcode commitment, limits, and multi-source tests |
| Shared native output lifetime, validation, and terminal failures | `crates/tq-formats/tests/native_output.rs`; `delimited_output_preserves_header_across_sources_and_proxy_bytes_with_one_budget` in `crates/tq-cli/tests/native_formats.rs` |
| RS detection, recovery, UTF-8, indices, publication, and resource bounds | `rs_framing_limits_are_terminal_at_every_chunk_boundary` in `crates/tq-formats/src/rs_framing.rs`; `json_sequence_input_recovery_and_indices_are_chunk_invariant` in `crates/tq-formats/tests/committed_input.rs` |
| jq stream projection, shared query cursor, slurp, and errors | `json_sequence_input_stream_records_share_the_query_cursor`, `json_sequence_input_stream_cursor_exposes_catchable_failures`, and `json_sequence_input_stream_slurp_is_one_input_in_the_shared_cursor` in `crates/tq-cli/tests/native_formats.rs` |
| JSON sequence output and migration from TOON `--seq` | `json_sequence_output_uses_json_formatting_controls` and `explicit_toon_sequence_output_uses_sequence_framing` in `crates/tq-cli/tests/native_formats.rs`; exact-byte cases in `tests/compatibility/cases/native-formats.jsonl` |
| Delimited header, row width, scalar types, quoting, and strict conversion | `crates/tq-formats/tests/delimited_input.rs`, `delimited_output.rs`, and `delimited_property.rs`; `strict_conversion_rejects_missing_key_normalization_after_prior_rows` in `crates/tq-cli/tests/native_formats.rs` |
| jq agreement and deliberate yq profile differences | [Reviewed native campaign](../tests/compatibility/reviews/native-formats-v1.md) and `tests/compatibility/baselines/native-formats-v1.json` |
| Hostile native input | `tests/fuzz/fuzz_targets/native_input.rs`, with bounded source, frame, row, field, token, and depth controls; tiny-chunk framing and committed-input tests above |
| Separate native-format reference objectives | `native_format_workloads_have_separate_correctness_gated_reference_pairs` in `crates/tq-test-support/tests/benchmark_schema.rs`; `native_format_objectives_use_the_matching_reference_and_require_rss` in `benchmark_reporting.rs`; `benchmarks/cases/native-formats.jsonl` |
| Refactor time/RSS targets and investigations | Frozen `benchmarks/cases/native-format-regression.py` and focused `native-format-probe.py`; release samples and reviewed reports belong in `tq-benchmarks`, not this repository |

The [format guide](formats.md) documents the new native formats and strict
conversion. XML, TOML, Properties, and INI are follow-on formats. HCL and Lua
are deferred entirely. Future string-map normalization scenarios describe an
output-profile contract; they do not claim an implemented string-map adapter.
