## 0. Pre-refactor performance baseline

- [x] 0.1 Before implementation, record the baseline revision and correctness-gated existing-workload matrix covering JSON, JSON Lines, JSON5, YAML, TOON, TOON sequences, complete-Document and supported event paths, remaining input, and multi-Result and multi-source output. Include small workloads, large Documents, and long sequences.
- [x] 0.2 Build the baseline in release mode and measure repeated samples using `/usr/bin/time -l` outside the sandbox with elevated process-inspection permissions. Retain commands, fixture hashes, host and toolchain details, build settings, warm-up policy, run counts, wall time, and valid peak RSS for the paired post-refactor comparison.

Baseline evidence: `tq-benchmarks/.work/native-format-refactor-20260904-v8/`, revision `ea81b21e1ea8d8e711dd53cb4ce79aff1ea037f2`, 30 correctness-approved workloads with seven measured samples each and valid peak RSS. This refresh uses the frozen pre-refactor binary, validates complete generated fixtures before timing, and records the finalized measurement-script hash for exact replay. Replay with `benchmarks/cases/native-format-regression.py`; earlier preparation attempts, the v6 output-only checkpoint, and v7 validation refresh remain separate and are not the final comparison.

## 1. Format architecture

- [x] 1.1 Add failing catalog completeness tests, then implement the closed `tq-formats` catalog for names, aliases, extensions, read/write support, probing, framing, profiles, event support, and option families; verify with `cargo test -p tq-formats catalog`.
- [x] 1.2 Route CLI format parsing, extension selection, help, compatibility reporting, and pre-input option validation through the catalog without changing existing formats; verify focused argument and runner tests with `cargo test -p tq-cli args` and `cargo test -p tq-cli format`.
- [x] 1.3 Test and implement opaque validated format selections and committed native input with pull Document observations and push structural-event consumption; cover ordered recoverable failures, early termination, root reset, and resource errors with `cargo test -p tq-formats committed_input`.
- [x] 1.4 Test and implement `NativeOutputSequence` construction, incremental Result writes, and explicit completion; retain `write_results` as an iterator convenience over it. Cover profile validation before header commitment, sequence context, terminal failures, and injected writer errors with `cargo test -p tq-formats output`.
- [x] 1.5 Migrate ordinary, remaining-input, automatic-event, and jq event-input runner paths to committed native input; remove obsolete `DocumentSource` interfaces and concrete public adapters immediately with no deprecation shim. Keep jq stream projection separate from core event plans and verify on-demand `input`/`inputs`, including `-n` and slurp, and existing format behavior through focused CLI tests and `cargo check --workspace --all-targets`.
- [x] 1.6 Route native Results through one command-scoped native output sequence. Keep raw/proxy ordering, key sorting, result limits, flushing, and the shared writer byte budget in command output. Test cross-source row shape, proxy interruption, proxy-only output, unframed cardinality, and mixed-path byte limits with focused CLI output tests; replace redundant tests of removed dispatch.

## 2. Shared framing

- [x] 2.1 Add chunk-boundary and malformed-boundary tests for bounded record-separator scanning, then implement preamble, redundant-separator, recovery-segment indices distinct from document indices, LF, and recovery policies shared by TOON and JSON sequence payloads; verify with `cargo test -p tq-formats rs_framing`.
- [x] 2.2 Migrate TOON sequence input and output to the shared RS framing mechanics without changing canonical bytes or strict errors; verify existing TOON sequence tests and `cargo test -p tq-formats toon_sequence`.
- [x] 2.3 Add failing tiny-chunk tests for comma and tab logical-row framing, then implement the quote-aware incremental scanner with doubled quotes, quoted newlines, quote provenance, and row or field limits; verify with `cargo test -p tq-formats delimited_framing`.

## 3. RFC 7464 JSON Text Sequences

- [x] 3.1 Implement JSON sequence input over shared RS framing and committed native input, including preamble discard, redundant RS handling, strict UTF-8 decoding, ordered failure observations, and fatal resource errors. Preserve complete Documents and partial events published before later failures; verify with `cargo test -p tq-formats json_sequence_input`.
- [x] 3.2 Connect normal, slurp, remaining-input, and jq event-input execution to JSON sequence observations. Test multiple Documents per recovery segment, jq-compatible same-segment reset, root reset after each Document and recovery, complete values followed by trailing errors, partial events, top-level warnings versus query-consumed input errors, `try/catch`, and `--stream-errors` mapping with `cargo test -p tq-cli json_sequence_input`.
- [x] 3.3 Implement JSON sequence output as RS plus the existing configurable JSON document encoder plus LF, including exact numbers, zero results, and per-document commitment; verify with `cargo test -p tq-formats json_sequence_output`.
- [x] 3.4 Reassign `--seq` to jq-compatible JSON sequence input and output, add `json-seq` and `jsonseq` selectors and extensions, retain explicit `toon-seq`, and reject conflicts before input; verify CLI parsing and end-to-end migration tests with `cargo test -p tq-cli seq`.
- [x] 3.5 Add bounded-probe tests and implement JSON sequence commitment only when RS is the first non-whitespace byte; verify with `cargo test -p tq-formats probe`.

## 4. CSV and TSV profiles

- [x] 4.1 Add profile tests for unique headers, ordered row objects, quoted scalar provenance, missing trailing fields, excess fields, and quoted newlines, then implement CSV and TSV document sources; verify with `cargo test -p tq-formats delimited_input`.
- [x] 4.2 Add output tests for first-result headers, stable row shape, type-preserving quoting, escaping, empty strings, nulls, container rejection, and partial row commitment, then implement CSV and TSV output adapters; verify with `cargo test -p tq-formats delimited_output`.
- [x] 4.3 Implement output-profile semantic-preservation checks and `--strict-conversion` with per-document atomic validation and permissive defaults; verify strict and normalized cases with `cargo test -p tq-cli strict_conversion`.
- [x] 4.4 Add `csv` and `tsv` selectors, extension mapping, metadata-only selection, help text, and incompatible-option validation; verify with `cargo test -p tq-cli delimited`.
- [x] 4.5 Add property and chunk-invariance tests for delimited decode and encode, including arbitrary Unicode, delimiter characters, quotes, newlines, and numeric-looking strings; verify with `cargo test -p tq-formats delimited_property`.

## 5. Compatibility and documentation

- [x] 5.1 Add jq baseline and tq compatibility cases for JSON sequence framing, recovery, warnings, event input, output bytes, and exit status; verify the focused compatibility campaign passes against jq 1.8.x.
- [x] 5.2 Add yq observations and tq cases for CSV and TSV headers, scalar typing, quoting, row width, errors, and output bytes, recording deliberate profile differences; verify the focused delimited compatibility campaign.
- [x] 5.3 Update `docs/formats.md`, compatibility guidance, CLI help, requirements traceability, and the changelog with the new formats, the `--seq` migration, strict conversion, follow-on formats, and HCL or Lua deferral; verify documentation links and traceability checks.
- [x] 5.4 Extend hostile-input and resource-limit coverage for RS framing and delimited scanning, then run the relevant fuzz regression and resource-governance tests.

## 6. Performance and final verification

- [x] 6.1 Add correctness-gated RFC 7464 jq comparisons and CSV or TSV yq comparisons to the benchmark manifest, keeping comparison families separate; verify benchmark schema and correctness-gate tests.
- [x] 6.2 Build release binaries and run authoritative benchmarks outside the sandbox with elevated process-inspection permission and `/usr/bin/time -l`; verify reports contain median time, maximum peak RSS, 2.0 time and 1.5 RSS objective evaluations, host identity, and valid comparison metadata.
- [x] 6.2a Repeat the baseline workload matrix under matching conditions for the candidate. Report per-workload percentage changes in median wall time and maximum observed peak RSS, targeting less than 10% growth in each. Mark all target misses and treat missing or invalid RSS as an incomplete comparison, not a pass.
- [x] 6.2b Investigate every time or RSS increase above 25% with repeated paired measurements, noise assessment, cause analysis, and mitigation. Record evidence and remaining impact; require explicit user acceptance for any remaining increase above 25% before completing verification.
- [x] 6.3 Run `cargo fmt --check`, `cargo check --workspace --all-targets`, focused crate tests during repair, and `cargo test --workspace` once at the end; resolve every failure without weakening the specifications.
- [x] 6.4 Review the completed change against repository standards and every OpenSpec scenario using the code-review workflow, fix all findings, and verify `openspec validate extend-native-format-architecture --strict` passes.
