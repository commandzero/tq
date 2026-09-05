## Why

Adding each native format currently spreads aliases, extensions, capabilities, adapter selection, validation, and output policy across the CLI. Establishing explicit framing, document-codec, and profile boundaries now lets tq add formats without multiplying format-specific branches, while the first adapters close important jq and yq compatibility gaps.

## What Changes

- Centralize native-format metadata and capability decisions in a closed format catalog while preserving exhaustive Rust enums.
- Produce validated format selections before semantic I/O; unify committed native input across pull Document observations and push structural events.
- Give one native output sequence ownership of framing, profiles, and sequence context across Results, input sources, and proxy interruptions.
- **BREAKING** Remove obsolete `DocumentSource` interfaces and concrete public adapters immediately; the repository is their only consumer. Retain `write_results` as an iterator convenience over the new stateful output module.
- Preserve jq-compatible observations published before recoverable JSON sequence failures and render failures according to the selected input mode.
- Match jq 1.8.1's same-segment parser reset and its distinction between top-level recovery warnings and errors consumed by `input` or `inputs`.
- Evaluate jq `--stream` records as ordinary query inputs sharing the `input`/`inputs` cursor, independently of automatic decoder-event plans.
- Distinguish RS recovery segments from document frames, accepting multiple JSON Documents between separators as jq does while emitting one RS-prefixed frame per Result.
- Separate sequence framing from per-document decoding and encoding, including shared record-separator framing for TOON and RFC 7464 payloads.
- **BREAKING** Reassign `--seq` from TOON sequence output to jq-compatible RFC 7464 JSON sequence input and output; keep TOON Text Sequences available through the explicit `toon-seq` native format.
- Add native CSV and TSV input and output where the header establishes row shape and each data row is one document.
- Add strict conversion as an opt-in check while keeping declared scalar normalizations permissive by default.
- Extend compatibility cases, format documentation, resource-limit coverage, and benchmarks for the new formats.
- Validate existing workloads against a pre-refactor tq baseline, targeting less than 10% growth in both time and peak RSS and investigating every increase above 25%.
- Leave XML, TOML, Properties, and INI as follow-on adapters. HCL and Lua remain deferred.

## Capabilities

### New Capabilities

- `json-sequence-io`: RFC 7464 framing, jq-compatible recovery, ordered document input, and framed JSON output.
- `delimited-text-io`: CSV and TSV row documents, sequence headers, scalar profiles, type-preserving quoting, and row-shape validation.

### Modified Capabilities

- `tq-cli`: Add format names, aliases, extensions, probing rules, option compatibility, strict conversion, diagnostics, and help for the new native formats.
- `cross-tool-compatibility`: Add jq-target JSON sequence cases and yq-peer CSV and TSV cases with normalized values, diagnostics, and exit status.
- `performance-benchmarks`: Add pre/post-refactor time and RSS regression checks alongside correctness-gated native-format comparisons with the existing soft reference-relative objectives.

## Impact

The main changes affect `tq-formats`, CLI argument validation and runner planning, structured output, compatibility manifests, format documentation, and benchmark coverage. New parser or encoder dependencies must support incremental byte I/O, configured resource limits, ordered values, and explicit duplicate or malformed-input behavior. Existing JSON, JSON Lines, JSON5, YAML, TOON, raw, and jq event-input behavior must remain unchanged.
