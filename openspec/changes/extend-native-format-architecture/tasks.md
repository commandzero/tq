## 1. Format architecture

- [ ] 1.1 Add failing catalog completeness tests, then implement the closed `tq-formats` catalog for names, aliases, extensions, read/write support, probing, framing, profiles, event support, and option families; verify with `cargo test -p tq-formats catalog`.
- [ ] 1.2 Route CLI format parsing, extension selection, help, compatibility reporting, and pre-input option validation through the catalog without changing existing formats; verify focused argument and runner tests with `cargo test -p tq-cli args` and `cargo test -p tq-cli format`.
- [ ] 1.3 Add failing factory tests, then move concrete input-adapter construction behind a `tq-formats` `DocumentSource` factory while preserving existing JSON, JSON Lines, JSON5, YAML, TOON, and TOON sequence behavior; verify with `cargo test -p tq-formats document_source` and existing CLI input tests.
- [ ] 1.4 Split format-specific output policy into internal adapters behind `write_results`, preserving existing output bytes and errors; verify differential output tests with `cargo test -p tq-formats output`.

## 2. Shared framing

- [ ] 2.1 Add chunk-boundary and malformed-boundary tests for a bounded record-separator framer, then implement preamble, redundant-separator, frame-index, LF, and recovery policies shared by TOON and JSON sequence payloads; verify with `cargo test -p tq-formats rs_framing`.
- [ ] 2.2 Migrate TOON sequence input and output to the shared RS framing mechanics without changing canonical bytes or strict errors; verify existing TOON sequence tests and `cargo test -p tq-formats toon_sequence`.
- [ ] 2.3 Add failing tiny-chunk tests for comma and tab logical-row framing, then implement the quote-aware incremental scanner with doubled quotes, quoted newlines, quote provenance, and row or field limits; verify with `cargo test -p tq-formats delimited_framing`.

## 3. RFC 7464 JSON Text Sequences

- [ ] 3.1 Implement the pull-based JSON sequence document source over shared RS framing, including explicit preamble discard, redundant RS handling, strict UTF-8 JSON decoding, recoverable warnings, and fatal resource errors; verify with `cargo test -p tq-formats json_sequence_input`.
- [ ] 3.2 Connect normal, slurp, remaining-input, and jq event-input execution to JSON sequence documents with root reset and skipped malformed frames; verify focused runner tests with `cargo test -p tq-cli json_sequence_input`.
- [ ] 3.3 Implement JSON sequence output as RS plus the existing configurable JSON document encoder plus LF, including exact numbers, zero results, and per-document commitment; verify with `cargo test -p tq-formats json_sequence_output`.
- [ ] 3.4 Reassign `--seq` to jq-compatible JSON sequence input and output, add `json-seq` and `jsonseq` selectors and extensions, retain explicit `toon-seq`, and reject conflicts before input; verify CLI parsing and end-to-end migration tests with `cargo test -p tq-cli seq`.
- [ ] 3.5 Add bounded-probe tests and implement JSON sequence commitment only when RS is the first non-whitespace byte; verify with `cargo test -p tq-formats probe`.

## 4. CSV and TSV profiles

- [ ] 4.1 Add profile tests for unique headers, ordered row objects, quoted scalar provenance, missing trailing fields, excess fields, and quoted newlines, then implement CSV and TSV document sources; verify with `cargo test -p tq-formats delimited_input`.
- [ ] 4.2 Add output tests for first-result headers, stable row shape, type-preserving quoting, escaping, empty strings, nulls, container rejection, and partial row commitment, then implement CSV and TSV output adapters; verify with `cargo test -p tq-formats delimited_output`.
- [ ] 4.3 Implement output-profile semantic-preservation checks and `--strict-conversion` with per-document atomic validation and permissive defaults; verify strict and normalized cases with `cargo test -p tq-cli strict_conversion`.
- [ ] 4.4 Add `csv` and `tsv` selectors, extension mapping, metadata-only selection, help text, and incompatible-option validation; verify with `cargo test -p tq-cli delimited`.
- [ ] 4.5 Add property and chunk-invariance tests for delimited decode and encode, including arbitrary Unicode, delimiter characters, quotes, newlines, and numeric-looking strings; verify with `cargo test -p tq-formats delimited_property`.

## 5. Compatibility and documentation

- [ ] 5.1 Add jq baseline and tq compatibility cases for JSON sequence framing, recovery, warnings, event input, output bytes, and exit status; verify the focused compatibility campaign passes against jq 1.8.x.
- [ ] 5.2 Add yq observations and tq cases for CSV and TSV headers, scalar typing, quoting, row width, errors, and output bytes, recording deliberate profile differences; verify the focused delimited compatibility campaign.
- [ ] 5.3 Update `docs/formats.md`, compatibility guidance, CLI help, requirements traceability, and the changelog with the new formats, the `--seq` migration, strict conversion, follow-on formats, and HCL or Lua deferral; verify documentation links and traceability checks.
- [ ] 5.4 Extend hostile-input and resource-limit coverage for RS framing and delimited scanning, then run the relevant fuzz regression and resource-governance tests.

## 6. Performance and final verification

- [ ] 6.1 Add correctness-gated RFC 7464 jq comparisons and CSV or TSV yq comparisons to the benchmark manifest, keeping comparison families separate; verify benchmark schema and correctness-gate tests.
- [ ] 6.2 Build release binaries and run authoritative benchmarks outside the sandbox with elevated process-inspection permission and `/usr/bin/time -l`; verify reports contain median time, maximum peak RSS, 2.0 time and 1.5 RSS objective evaluations, host identity, and valid comparison metadata.
- [ ] 6.3 Run `cargo fmt --check`, `cargo check --workspace --all-targets`, focused crate tests during repair, and `cargo test --workspace` once at the end; resolve every failure without weakening the specifications.
- [ ] 6.4 Review the completed change against repository standards and every OpenSpec scenario using the code-review workflow, fix all findings, and verify `openspec validate extend-native-format-architecture --strict` passes.
