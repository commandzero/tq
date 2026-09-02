## Context

`tq-formats` already owns the shared `InputFormat`, `OutputFormat`, `DocumentSource`, and `write_results` interfaces. Format selection still leaks into CLI parsing, extension lookup, planning, event eligibility, document-source construction, output validation, and reporting. Several non-JSON formats also pass through `VecDocumentSource`, which forces the CLI to know whether an adapter materializes its complete source.

The terms in [`CONTEXT.md`](../../../CONTEXT.md) and the module ownership decision in [ADR 0001](../../../docs/adr/0001-compose-native-formats.md) govern this design. In particular, jq event input mode remains separate from streaming document decoding and encoding.

## Goals / Non-Goals

**Goals:**

- Give the CLI one source for format names, aliases, extensions, capabilities, and compatible controls.
- Keep `DocumentSource` and `write_results` as the deep public interfaces while moving adapter construction and output dispatch into `tq-formats`.
- Reuse record-separator framing for TOON and RFC 7464 without coupling either payload codec to the other.
- Decode and encode JSON sequences and delimited rows incrementally under existing resource limits.
- Make profile normalization and strict conversion explicit before each document commits.

**Non-Goals:**

- A runtime plugin registry or third-party codec API.
- XML, TOML, Properties, or INI adapters in this change.
- HCL or Lua parsing.
- Source-syntax preservation, comment preservation, or dialect auto-detection for CSV and TSV.
- Renaming jq event input mode or changing its query semantics.

## Decisions

### Keep a closed format catalog in `tq-formats`

Add a catalog module beside the existing format enums. It owns canonical names, aliases, recognized extensions, probing eligibility, read and write support, document-event support, framing kind, profile identity, and option families. CLI parsing and reporting ask this module for facts instead of matching every enum independently.

The catalog remains closed and exhaustively matched. A dynamic registry was considered, but tq has no third-party extension requirement and format profiles still require deliberate CLI, diagnostics, tests, and documentation. A registry would move those decisions behind a shallow interface without removing them.

### Move adapter construction behind `DocumentSource`

Add a `tq-formats` factory that accepts the committed input format, reader, source identity, decode limits, and selected profile. It returns a pull-based `DocumentSource`. The CLI remains responsible for source order and bounded auto-detection, but it no longer selects concrete decoder types or decides which formats materialize.

Existing JSON and JSON Lines sources migrate through the factory first. YAML, JSON5, and current TOON adapters may still materialize internally during the migration, but that fact is no longer visible to the runner. New JSON sequence and delimited sources implement incremental pulls directly.

An exported trait per parser was considered and rejected. Callers need ordered documents and classified failures, not codec-specific methods.

### Separate framing from document codecs

Introduce an internal framing module that yields bounded frame readers plus frame indices and sequence context. Newline framing remains separate from record-separator framing. Record-separator framing supports policy parameters for preamble handling, redundant separators, recovery, and LF expectations so TOON and RFC 7464 can share boundary mechanics without sharing payload rules.

The JSON document decoder receives one frame and knows nothing about neighboring frames. The delimited framer identifies logical rows while honoring quoted newlines, and its sequence header establishes row shape before any row document is emitted.

The alternative was to add JSON-sequence and CSV splitting directly inside their decoders. That repeats boundary, indexing, limit, and recovery behavior and makes later framing formats harder to add.

### Use internal output adapters behind `write_results`

Keep `write_results` as the public serialization entry point. Internally, dispatch to TOON, YAML, JSON, JSON Lines, JSON sequence, CSV, or TSV adapters. Split the shared option bag into nested format-specific options and a small common envelope so each adapter validates only controls it understands.

Each adapter exposes internal validation for one result before its document or row commits. Strict conversion calls the selected profile's semantic-preservation check during that validation. Earlier frames may remain published, but the rejected document writes no bytes.

### Implement RFC 7464 as RS framing plus the existing JSON codec

The input framer discards the preamble only when JSON sequence input was explicitly selected. Auto-detection recognizes the format only when RS is the first non-whitespace byte. A frame reader stops at the next RS without collecting later frames.

Strict JSON decoding failures become warnings when the next RS leaves recovery unambiguous. The source then advances without emitting a document. Fatal I/O and resource errors retain their existing classifications. Output writes RS, delegates one document to the JSON encoder with compatible formatting controls, and writes LF.

`--seq` changes to select JSON sequence input and output. Explicit `toon-seq` names preserve TOON Text Sequence access. Treating `--seq` as a framing modifier independent of payload was considered, but that would continue to differ from jq and create invalid combinations with CSV or YAML.

### Implement one quote-aware delimited scanner for CSV and TSV

Use one incremental state machine parameterized by comma or tab. It tracks whether each field was quoted, handles doubled quotes and quoted newlines, and applies logical-row and field limits while reading. Retaining quote provenance is required because the agreed profile distinguishes quoted numeric-looking strings from unquoted numbers and booleans.

The first logical row produces ordered unique header keys as sequence context. Later rows become object-root documents. The writer pulls the first result to establish the header, then validates and writes rows one at a time.

Using a parser that returns only decoded field bytes was considered and rejected because quote provenance cannot be reconstructed after parsing. Guessing types from every decoded field would break the documented string escape mechanism.

### Keep profile normalization permissive by default

Default conversion accepts declared normalizations. Delimited output uses quoting to preserve scalar types where the format can carry the distinction. Future string-map adapters may stringify scalar values by default. Arrays and objects in scalar field positions remain profile rejections.

`--strict-conversion` asks the output profile whether re-decoding would return an equal tq value. This check is distinct from input parser strictness and TOON's `--non-strict` syntax option.

### Test at the catalog, framing, codec, and CLI seams

Catalog tests assert that aliases, extensions, capabilities, and option families are complete for every enum variant. Framing tests use tiny reader chunks and malformed boundaries. Codec tests cover event-versus-document agreement where events are supported, scalar profiles, row shape, resource limits, and round trips. CLI tests cover selection precedence, the `--seq` migration, diagnostics, stdout discipline, and partial-output commitment.

Compatibility manifests compare JSON sequence behavior with jq and delimited behavior with yq. Benchmarks remain correctness-gated. Authoritative macOS RSS runs use `/usr/bin/time -l` outside the sandbox with elevated process-inspection permission, as required by the main performance specification.

## Risks / Trade-offs

- [Existing scripts use `--seq` for TOON output] -> Document the breaking change, keep explicit `toon-seq`, and add CLI migration tests.
- [The format catalog becomes another broad switch] -> Keep exhaustive decisions inside one module and add completeness tests that fail when a variant lacks metadata.
- [CSV and TSV dialect expectations vary] -> Publish one deterministic dialect, require explicit format selection or extension, and reject unsupported structure rather than guessing.
- [Recoverable JSON frames hide bad input] -> Always warn with source and frame context, match jq's successful exit behavior, and preserve fatal classifications for resource and framing failures.
- [Strict validation delays a document's first byte] -> Limit atomicity to one document and leave default conversion permissive; earlier complete frames remain published.
- [Incremental adapters regress existing formats] -> Migrate existing construction through the factory without changing their codecs first, then add differential tests before removing runner dispatch.

## Migration Plan

1. Add catalog and factory APIs while existing enum variants still use their current adapters.
2. Route CLI parsing, extension lookup, capability checks, and reporting through the catalog with no user-visible format changes.
3. Move output dispatch behind internal adapters and keep existing output bytes covered by differential tests.
4. Add shared framing, RFC 7464, and CSV/TSV adapters behind the new seams.
5. Change `--seq`, update help and compatibility documentation, and retain explicit `toon-seq` migration guidance.
6. Run unit, integration, compatibility, conformance, and elevated benchmark campaigns before enabling the formats in the support matrix.

Rollback can disable the new catalog entries and selectors while retaining the internal refactor. Reverting the `--seq` behavior requires reverting its CLI and documentation changes together.
