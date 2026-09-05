## MODIFIED Requirements

### Requirement: Best-effort input detection with strict override
When `--input-format` is absent, tq SHALL select `.jsonl` and `.ndjson` file paths as JSON Lines, `.json-seq` and `.jsonseq` paths as RFC 7464 JSON Text Sequences, `.json5` paths as JSON5, `.yaml` and `.yml` paths as YAML, `.json` paths as JSON, `.toon` paths as TOON, `.csv` paths as CSV, and `.tsv` paths as TSV before applying bounded syntax probing to sources without a recognized extension. Canonical TOON syntax SHALL remain preferred during probing, an RS first non-whitespace byte SHALL commit to JSON Text Sequence input, JSON object and non-TOON array openers SHALL commit to strict JSON before YAML, and YAML document, directive, or root-sequence markers SHALL commit to YAML. Content probing MUST NOT select JSON5, CSV, TSV, TOML, INI, or Properties. Once a parser commits, later syntax errors SHALL be reported for that format without restarting detection. If every probed parser rejects, tq SHALL emit a combined input diagnostic containing bounded, useful failure context from each candidate.

`--input-format toon|toon-seq|yaml|json|json5|jsonl|json-seq|csv|tsv` SHALL select exactly one parser and disable detection or faildown. `ndjson` SHALL be accepted as an alias for `jsonl`, `toon-sequence` as an alias for `toon-seq`, and `jsonseq` as an alias for `json-seq`. An explicit override SHALL take precedence over a recognized file extension. TOON SHALL remain the default structured output format, while TOON sequence, YAML, JSON, JSON Lines, JSON sequence, CSV, and TSV output SHALL be available through `--output-format toon|toon-seq|yaml|json|jsonl|json-seq|csv|tsv` with the same aliases. JSON5 output SHALL remain unsupported. `--seq` SHALL select jq-compatible JSON sequence input and output; it MUST conflict with explicit non-JSON-sequence native formats rather than silently changing one direction.

#### Scenario: Default format
- **WHEN** no format option is provided for a source without a recognized extension
- **THEN** bounded syntax probing selects TOON, strict JSON, JSON Text Sequence, or YAML and structured output uses TOON Text Sequence framing

#### Scenario: JSON interoperability
- **WHEN** both input and output formats are explicitly set to JSON
- **THEN** the CLI evaluates the same core program over the JSON data model and emits jq-compatible JSON result texts

#### Scenario: JSON5 interoperability
- **WHEN** JSON5 input and JSON output are selected
- **THEN** the CLI evaluates the same core program over the decoded JSON-shaped value and emits JSON result text

#### Scenario: YAML interoperability
- **WHEN** YAML input and JSON output are selected
- **THEN** each accepted YAML document is converted to the shared ordered JSON-shaped value model and evaluated by the same core program

#### Scenario: YAML output remains deferred
- **WHEN** YAML output is selected
- **THEN** each result is encoded through the documented YAML output profile without implying source-syntax preservation

#### Scenario: JSON5 output remains unsupported
- **WHEN** `--output-format json5` is requested
- **THEN** the CLI reports an unsupported-output-format usage error

#### Scenario: Ambiguous content
- **WHEN** input bytes would be valid as strict JSON, JSON5, and YAML
- **THEN** automatic mode commits an object or non-TOON array opener to strict JSON, while an explicit override selects only the requested parser

#### Scenario: JSON container enables automatic events
- **WHEN** an eligible query receives a JSON object or array without an input-format override
- **THEN** bounded detection commits to JSON before planning and execution uses JSON decoder events

#### Scenario: JSON Lines extension
- **WHEN** automatic input selection receives a file ending in `.jsonl` or `.ndjson`
- **THEN** tq selects JSON Lines without content probing

#### Scenario: JSON sequence extension
- **WHEN** automatic input selection receives a file ending in `.json-seq` or `.jsonseq`
- **THEN** tq selects RFC 7464 JSON Text Sequence input without content probing

#### Scenario: Delimited extension
- **WHEN** automatic input selection receives a file ending in `.csv` or `.tsv`
- **THEN** tq selects the corresponding delimited format without content probing

#### Scenario: Delimited stdin requires selection
- **WHEN** extensionless stdin contains syntactically valid CSV or TSV without an explicit input format
- **THEN** content probing does not guess CSV or TSV

#### Scenario: JSON5 extension
- **WHEN** automatic input selection receives a file ending in `.json5`
- **THEN** tq selects document-at-a-time JSON5 without content probing

#### Scenario: JSON extension remains strict
- **WHEN** automatic input selection receives a `.json` file containing JSON5-only syntax
- **THEN** tq reports a strict JSON parse error and does not retry the source as JSON5

#### Scenario: Override beats extension
- **WHEN** `--input-format json` is supplied for a file ending in `.jsonl`
- **THEN** tq invokes the strict JSON document parser and does not select JSON Lines from the extension

#### Scenario: Override beats JSON5 extension
- **WHEN** `--input-format json` is supplied for a file ending in `.json5`
- **THEN** tq invokes the strict JSON parser and does not select JSON5 from the extension

#### Scenario: Strict input override
- **WHEN** `--input-format json` is supplied for bytes that YAML could also parse
- **THEN** tq invokes only the strict JSON parser and never probes TOON or YAML

#### Scenario: Strict JSON5 input override
- **WHEN** `--input-format json5` is supplied for bytes that strict JSON or YAML could also parse
- **THEN** tq invokes only the JSON5 parser and never probes TOON, strict JSON, or YAML

#### Scenario: jq-compatible sequence switch
- **WHEN** `--seq` is supplied without conflicting format options
- **THEN** tq reads and writes RFC 7464 JSON Text Sequences with jq-compatible recovery and framing

#### Scenario: Explicit TOON sequence
- **WHEN** `--input-format toon-seq --output-format toon-seq` is supplied
- **THEN** tq reads and writes TOON Text Sequence frames without interpreting `--seq` as TOON framing

#### Scenario: Sequence switch conflict
- **WHEN** `--seq` is combined with `--input-format csv` or `--output-format toon-seq`
- **THEN** tq reports an incompatible-option usage error before consuming input

#### Scenario: Mixed-format files
- **WHEN** multiple files are supplied without an input-format override
- **THEN** tq selects each recognized extension or probes each remaining file independently and evaluates the resulting documents or frames in file order

#### Scenario: Override applies to every source
- **WHEN** multiple files are supplied with an input-format override
- **THEN** tq applies only that parser to every structured source and fails on the first non-recoverable source error

#### Scenario: All probes reject
- **WHEN** TOON, YAML, strict JSON, and JSON sequence probing all reject before commitment
- **THEN** tq exits with an input error that identifies bounded failure information for all probed formats

#### Scenario: Late selected-format failure
- **WHEN** automatic detection commits to a parser and that parser encounters a later non-recoverable syntax error
- **THEN** tq reports that format's parse error and does not reinterpret the source with another parser

## ADDED Requirements

### Requirement: Native output lifecycle and command budget
The CLI SHALL maintain one native output sequence across all structured Results for a command. Raw and proxy bytes SHALL bypass native encoding without resetting sequence context. Command output SHALL apply one shared output-byte budget and flushing policy to native, raw, and proxy bytes. Native output SHALL validate each complete Result against its output profile and sequence context before committing that Document's bytes. Any native output write failure SHALL terminate the sequence; later writes MUST be rejected. I/O or resource failures after commitment MAY leave partial bytes.

#### Scenario: Shared byte limit across proxy and native output
- **WHEN** native output and proxy bytes together exceed the command output-byte limit
- **THEN** command output reports a resource failure even if each path individually remains below that limit

#### Scenario: Profile rejection is terminal
- **WHEN** a Result fails profile validation after earlier Results were written
- **THEN** its Document commits no bytes, earlier bytes remain, and no later Result is encoded

#### Scenario: Writer failure after commitment
- **WHEN** the writer fails after committing part of a validated Document
- **THEN** tq stops with the classified failure without promising to retract those bytes or successfully finish the sequence

### Requirement: Committed native input delivery
Committed native input SHALL supply on-demand complete Document observations and incremental structural-event consumption through a shared native-format module. Format selections and representation capabilities SHALL be validated before semantic input consumption. Recoverable failures SHALL preserve observation order and source/frame context; rendering and jq input-mode semantics SHALL remain caller responsibilities.

#### Scenario: Remaining input preserves observation order
- **WHEN** `input` or `inputs` requests later Documents across a recoverable JSON sequence failure
- **THEN** the same ordered native observations are used without retaining all remaining Documents, but a query-consumed failure becomes a catchable input error rather than a top-level recovery warning, matching jq

#### Scenario: Unsupported representation fails before decoding
- **WHEN** a selected native format cannot supply the requested structural events
- **THEN** tq rejects that selection before consuming semantic input

#### Scenario: jq stream records use ordinary query evaluation
- **WHEN** `--stream` or `--stream-errors` projects structural observations into path/value records
- **THEN** each record is an ordinary query input, with no core event-plan restriction on updates, reductions, `input`, or `inputs`

#### Scenario: Projected records share the remaining-input cursor
- **WHEN** a query uses `input` or `inputs` with `--stream`
- **THEN** top-level advancement and the query consume the same ordered projected records, and `-n` evaluates null once without consuming those records

### Requirement: Strict conversion option
Default native-format conversion SHALL permit every normalization declared by the selected output profile. `--strict-conversion` SHALL reject a result if encoding and decoding it through the selected profiles would not return an equal shared value. The check MUST complete before committing the affected document or row, but earlier complete frames MAY remain on stdout.

#### Scenario: Default scalar stringification
- **WHEN** a number is written to a string-map output profile without `--strict-conversion`
- **THEN** tq writes the declared string representation rather than rejecting the conversion

#### Scenario: Strict type loss
- **WHEN** a selected output profile would turn number `42` into string `"42"` on re-read under `--strict-conversion`
- **THEN** tq rejects that result before committing its document

#### Scenario: Earlier frames survive strict rejection
- **WHEN** a later result fails strict conversion after earlier framed results completed
- **THEN** the earlier frames remain valid on stdout and tq exits with a profile-rejection diagnostic

### Requirement: Native format option compatibility
The CLI SHALL validate format-specific controls before consuming semantic input. JSON sequence output SHALL accept the JSON formatting controls that preserve valid RFC 7464 framing. CSV and TSV output MUST reject JSON-only, TOON-only, raw-output, joined-output, and color controls unless a control has an explicitly documented delimited-text meaning.

#### Scenario: Pretty JSON sequence
- **WHEN** JSON sequence output uses a compatible JSON indentation control
- **THEN** each frame contains the configured valid JSON document between RS and LF

#### Scenario: Raw CSV conflict
- **WHEN** CSV output is combined with raw or joined output
- **THEN** tq reports an incompatible-option usage error before reading input

#### Scenario: TOON option on TSV
- **WHEN** TSV output is combined with a TOON folding or delimiter option
- **THEN** tq reports an incompatible-option usage error before reading input
