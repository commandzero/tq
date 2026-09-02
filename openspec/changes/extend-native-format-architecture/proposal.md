## Why

Adding each native format currently spreads aliases, extensions, capabilities, adapter selection, validation, and output policy across the CLI. Establishing explicit framing, document-codec, and profile boundaries now lets tq add formats without multiplying format-specific branches, while the first adapters close important jq and yq compatibility gaps.

## What Changes

- Centralize native-format metadata and capability decisions in a closed format catalog while preserving exhaustive Rust enums.
- Separate sequence framing from per-document decoding and encoding, including shared record-separator framing for TOON and RFC 7464 payloads.
- **BREAKING** Reassign `--seq` from TOON sequence output to jq-compatible RFC 7464 JSON sequence input and output; keep TOON Text Sequences available through the explicit `toon-seq` native format.
- Add native CSV and TSV input and output where the header establishes row shape and each data row is one document.
- Add strict conversion as an opt-in check while keeping declared scalar normalizations permissive by default.
- Extend compatibility cases, format documentation, resource-limit coverage, and benchmarks for the new formats.
- Leave XML, TOML, Properties, and INI as follow-on adapters. HCL and Lua remain deferred.

## Capabilities

### New Capabilities

- `json-sequence-io`: RFC 7464 framing, jq-compatible recovery, ordered document input, and framed JSON output.
- `delimited-text-io`: CSV and TSV row documents, sequence headers, scalar profiles, type-preserving quoting, and row-shape validation.

### Modified Capabilities

- `tq-cli`: Add format names, aliases, extensions, probing rules, option compatibility, strict conversion, diagnostics, and help for the new native formats.
- `cross-tool-compatibility`: Add jq-target JSON sequence cases and yq-peer CSV and TSV cases with normalized values, diagnostics, and exit status.
- `performance-benchmarks`: Add correctness-gated native-format comparisons with the existing soft jq-relative time and RSS objectives.

## Impact

The main changes affect `tq-formats`, CLI argument validation and runner planning, structured output, compatibility manifests, format documentation, and benchmark coverage. New parser or encoder dependencies must support incremental byte I/O, configured resource limits, ordered values, and explicit duplicate or malformed-input behavior. Existing JSON, JSON Lines, JSON5, YAML, TOON, raw, and jq event-input behavior must remain unchanged.
