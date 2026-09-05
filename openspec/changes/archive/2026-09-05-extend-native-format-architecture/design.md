## Context

`tq-formats` already owns the shared `InputFormat`, `OutputFormat`, `DocumentSource`, and `write_results` interfaces. Format selection still leaks into CLI parsing, extension lookup, planning, event eligibility, document-source construction, output validation, and reporting. Several non-JSON formats also pass through `VecDocumentSource`, which forces the CLI to know whether an adapter materializes its complete source.

The terms in [`CONTEXT.md`](../../../CONTEXT.md) and the module ownership decision in [ADR 0001](../../../docs/adr/0001-compose-native-formats.md) govern this design. In particular, jq event input mode remains separate from streaming document decoding and encoding.

## Goals / Non-Goals

**Goals:**

- Give the CLI one source for format names, aliases, extensions, capabilities, and compatible controls.
- Give committed native input and native output sequence ownership to deep modules in `tq-formats`.
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

### Use prevalidated format selections

The closed catalog produces opaque committed-input and selected-output values containing compatible framing, profiles, and controls. Committed input excludes automatic detection. Capability and option checks precede semantic input consumption or output commitment. These are format selections; execution plans remain query-planning concepts.

### Own committed native input across both representations

One module owns framing, sequence context, decoder selection, source identity, frame indices, resource enforcement, and recovery. Complete Document observations use a pull interface for on-demand `input` and `inputs`. Structural events use a push consumer interface, matching existing streaming decoders without requiring an event queue or worker merely to simulate pulls. Representation is selected before semantic input consumption.

Both delivery modes report recoverable document failures as ordered native input observations. The module never writes diagnostics. The runner renders warnings during top-level input advancement, maps observations into jq-compatible error values for `--stream-errors`, or exposes errors consumed by `input` and `inputs` to the query. Query semantics remain in the runner. Fatal input failures and consumer failures remain separately classified. Early termination stops further decoding, and recovery resets decoder state without inventing successful document completion.

YAML, JSON5, and existing TOON adapters may still materialize internally during migration. All ordinary, remaining-input, automatic-event, and jq event-input paths use this module rather than selecting concrete adapters in the CLI.

A push-only interface complicates on-demand `inputs`; a pull-only event interface requires resumable decoder changes or buffering. The hybrid preserves both existing execution patterns while concentrating native-format behavior. Remove `DocumentSource` and its obsolete concrete public adapters as callers migrate. This repository is their only consumer, so no deprecation period or compatibility shim is required.

Automatic selected decoding also enters through committed native input. Its
`consume_selected` operation fuses field selection with decoding, preserving
the existing skip and parallel JSON paths. It publishes selected records and
Document completion separately and returns decoder retention observations.
The CLI supplies the query selection and consumes observations; it does not
choose concrete native decoders.

Structural transcode likewise uses committed input through `consume_codec_events`.
This entry retains the codec consumer's allocation-avoiding text and numeric
hooks and explicit source spans. Native failures remain distinct from typed
event-consumer errors and the existing consumer trait's text-hook errors.
The transcode proof and output consumer remain unchanged.

### Keep jq stream projection separate from query execution plans

The jq `--stream` and `--stream-errors` switches project structural observations into ordinary query input values. They do not request a core event execution plan. The resulting path/value records share one cursor between top-level evaluation and query `input` or `inputs`. `-n` evaluates null once without consuming that cursor; slurp collects projected records. Ordinary query operations, including updates and reductions, remain available. Automatic decoder-event plans retain their existing capability checks and are separate from this input projection.

### Separate framing from document codecs

Introduce internal bounded record-separator readers with segment indices and sequence context. Newline framing remains separate from record-separator scanning. Record-separator scanning supports policy parameters for preamble handling, redundant separators, recovery, and LF expectations so TOON and RFC 7464 can share boundary mechanics without sharing payload rules. TOON assigns one document frame to each segment; jq-compatible JSON input may decode multiple document frames within one recovery segment. Keep document indices separate from recovery-segment indices.

The JSON document decoder identifies complete root values within the bounded recovery segment without knowing how neighboring segments are found. A complete root ends one document frame; another root may follow before the next RS. On syntax failure, preserve the consumed-token position and apply the pinned jq parser's reset behavior, which can resume within the same segment. Do not assume that jq always skips to the next RS merely because its diagnostic says "need RS to resync". The delimited framer identifies logical rows while honoring quoted newlines, and its sequence header establishes row shape before any row document is emitted.

The alternative was to add JSON-sequence and CSV splitting directly inside their decoders. That repeats boundary, indexing, limit, and recovery behavior and makes later framing formats harder to add.

### Own the native output sequence across calls

`NativeOutputSequence` is constructed from a validated selection, accepts one Result per `write_result` call, and finishes explicitly. It owns sequence context, output profiles, document encoders, framing, cardinality, and completion. Each operation receives the outer command-output module's limited writer. Callers must use the same logical command output throughout the sequence.

One native output sequence spans Results from all input sources. Proxy bytes bypass it without resetting sequence context. Command output owns raw/proxy ordering, the shared output-byte budget, flushing, result limits, key sorting, and exit policy. Existing proxy-only and unframed-cardinality behavior remains covered by CLI regression tests.

Validate the complete current Result against its output profile, strict conversion, and sequence context before committing document bytes or a first-row header. After successful validation, streaming encoding may leave partial bytes on I/O or output-limit failure. Any write failure, including profile rejection, makes the sequence terminal. Finish applies completion rules and prevents further writes; failure does not trigger successful completion.

Keep format-specific options and adapters internal. Retain `write_results` only as an iterator convenience that creates one sequence, writes all Results, and finishes it. The runner no longer calls it with singleton iterators. Remove obsolete dispatch and interfaces immediately rather than retaining parallel implementations.

### Implement RFC 7464 as RS framing plus the existing JSON codec

The input framer discards the preamble only when JSON sequence input was explicitly selected. Auto-detection recognizes the format only when RS is the first non-whitespace byte. A recovery-segment reader stops at the next RS without collecting later segments. Accept multiple roots per segment in ordinary, slurp, remaining-input, and jq event-input modes, matching jq 1.8.1. Output remains one RS, one JSON Document, and LF per Result.

Strict JSON decoding failures become ordered recoverable-failure observations under the pinned jq sequence profile. Publication follows jq: a complete Document may precede a later trailing-syntax failure, and Event input mode may receive partial structural events before a failure. Already published observations are never retracted. Top-level normal and `--stream` advancement warns; `--stream-errors` emits an error value and suppresses the warning. A failure consumed by `input` or `inputs` is instead a query-visible error, fatal unless handled by the query. Top-level numbers cut off directly by RS or EOF are potentially truncated and are not published. Fatal I/O and resource errors remain fatal regardless of available separators. Output writes RS, delegates one document to the JSON encoder, and writes LF.

Reference checks use the official jq 1.8.1 binary. `RS {"a":1} broken {"b":2} RS {"c":3} LF` publishes all three Documents in normal mode but terminates uncaught `-n inputs` after the first Document with status 5. The jq 1.8.1 `parser_reset` implementation explains why normal-mode parsing can resume before another RS. Preserve these cases as versioned compatibility observations; the earlier unconditional skip-to-RS assumption is superseded.

`--seq` changes to select JSON sequence input and output. Explicit `toon-seq` names preserve TOON Text Sequence access. Treating `--seq` as a framing modifier independent of payload was considered, but that would continue to differ from jq and create invalid combinations with CSV or YAML.

### Implement one quote-aware delimited scanner for CSV and TSV

Use one incremental state machine parameterized by comma or tab. It tracks whether each field was quoted, handles doubled quotes and quoted newlines, and applies logical-row and field limits while reading. Retaining quote provenance is required because the agreed profile distinguishes quoted numeric-looking strings from unquoted numbers and booleans.

The first logical row produces ordered unique header keys as sequence context. Later rows become object-root documents. Native output accepts its first Result to establish the header, then validates and writes rows one at a time across source boundaries and proxy interruptions.

Using a parser that returns only decoded field bytes was considered and rejected because quote provenance cannot be reconstructed after parsing. Guessing types from every decoded field would break the documented string escape mechanism.

### Keep profile normalization permissive by default

Default conversion accepts declared normalizations. Delimited output uses quoting to preserve scalar types where the format can carry the distinction. Future string-map adapters may stringify scalar values by default. Arrays and objects in scalar field positions remain profile rejections.

`--strict-conversion` asks the output profile whether re-decoding would return an equal tq value. This check is distinct from input parser strictness and TOON's `--non-strict` syntax option.

### Test at the catalog, framing, codec, and CLI seams

Catalog tests assert completeness and rejection of invalid selections before I/O. Committed-input tests exercise both delivery modes, ordered failures, early termination, resource enforcement, and root reset. Native-output tests exercise cross-call row shape, profile rejection before header commitment, terminal errors, completion, and injected writer failures. Tiny-chunk framing tests and profile round trips remain internal tests. CLI tests cover on-demand `inputs`, cross-source output, proxy interruption, one shared byte budget, selection precedence, the `--seq` migration, and diagnostic rendering. Replace redundant tests of removed dispatch with tests through the new interfaces.

Compatibility manifests compare JSON sequence behavior with jq and delimited behavior with yq. Benchmarks remain correctness-gated. Authoritative macOS RSS runs use `/usr/bin/time -l` outside the sandbox with elevated process-inspection permission, as required by the main performance specification.

Before changing implementation code, record the pre-refactor revision and release-build timing and peak-RSS baseline. Compare the candidate on the same host, toolchain, build settings, fixtures, equivalent queries and output, warm-ups, and repeated-run counts. Cover existing formats, complete-Document and supported event paths, remaining input, and multi-Result and multi-source output with small workloads, large Documents, and long sequences. Keep individual samples and report per-workload changes in median wall time and maximum observed peak RSS. The target is less than 10% growth in each metric. Mark every target miss and investigate any increase above 25% with repeated paired measurements and cause analysis. Verification cannot complete with an unexplained increase above 25%; any remaining regression above that threshold requires explicit user acceptance. New-format jq/yq objectives remain separate and do not replace this baseline comparison.

## Risks / Trade-offs

- [Existing scripts use `--seq` for TOON output] -> Document the breaking change, keep explicit `toon-seq`, and add CLI migration tests.
- [The format catalog becomes another broad switch] -> Keep exhaustive decisions inside one module and add completeness tests that fail when a variant lacks metadata.
- [CSV and TSV dialect expectations vary] -> Publish one deterministic dialect, require explicit format selection or extension, and reject unsupported structure rather than guessing.
- [Recoverable JSON frames hide bad input] -> Report ordered failures as warnings or explicit stream-error values, preserve published observations, and retain fatal resource classifications.
- [Strict validation delays a document's first byte] -> Limit atomicity to one document and leave default conversion permissive; earlier complete frames remain published.
- [Incremental adapters regress existing formats] -> Migrate both representations through committed native input and verify existing behavior before removing runner dispatch.

## Migration Plan

Before step 1, capture the existing-workload release baseline using elevated `/usr/bin/time -l` runs and record reproducible comparison metadata.

1. Add validated catalog selections and committed native input with pull Document and push structural-event delivery.
2. Route CLI parsing, extension lookup, capability checks, and reporting through the catalog with no user-visible format changes.
3. Migrate every runner input path and native output to the new modules, remove obsolete input interfaces immediately, and implement the retained `write_results` convenience through `NativeOutputSequence`.
4. Add shared framing, RFC 7464, and CSV/TSV adapters behind the new seams.
5. Change `--seq`, update help and compatibility documentation, and retain explicit `toon-seq` migration guidance.
6. Run unit, integration, compatibility, conformance, and elevated benchmark campaigns before enabling the formats in the support matrix.

Rollback can disable the new catalog entries and selectors while retaining the internal refactor. Reverting the `--seq` behavior requires reverting its CLI and documentation changes together.
