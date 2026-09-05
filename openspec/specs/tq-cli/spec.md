# tq CLI Specification

## Purpose

Define tq's jq-shaped command interface, format selection, input/output modes,
variables, diagnostics, exit statuses, and explicitly deferred CLI behavior.

## Requirements

### Requirement: jq-like command shape
The CLI SHALL support `tq [OPTIONS] [FILTER [FILE...]]` and a filter file option. When neither a positional filter nor a filter file is supplied, the CLI MUST use the identity filter `.`. With no input files it MUST read stdin; a file argument of `-` MUST refer to stdin in the ordered input list.

#### Scenario: Implicit identity filter
- **WHEN** structured input is piped to `tq` without arguments
- **THEN** the CLI evaluates the identity filter against stdin

#### Scenario: Pipe input
- **WHEN** TOON is piped to `tq '.name'`
- **THEN** the CLI evaluates the filter against the stdin document

#### Scenario: Multiple files
- **WHEN** two file paths are provided
- **THEN** the CLI evaluates them as two ordered input documents without slurping unless requested

#### Scenario: Filter file
- **WHEN** `-f query.tq` is provided
- **THEN** the query is loaded from that file and a positional filter is not required

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

### Requirement: Structured and raw output modes
Structured TOON output SHALL use TOON Text Sequence framing by default. `--unframed` SHALL require exactly one result. `-r/--raw-output` SHALL write strings without structured quoting, and `-j/--join-output` SHALL suppress raw-output separators as defined by jq-compatible cases.

#### Scenario: Raw string
- **WHEN** `-r '.name'` emits a string
- **THEN** its contents are written as raw text followed by the configured jq-compatible separator

#### Scenario: Raw non-string
- **WHEN** raw output receives a non-string value
- **THEN** the value is rendered according to the accepted jq raw-output behavior

#### Scenario: Unframed cardinality error
- **WHEN** `--unframed` receives zero or multiple results
- **THEN** the CLI exits nonzero with a cardinality diagnostic

### Requirement: Core input modes
The MVP SHALL support `-n/--null-input`, `-R/--raw-input`, `-s/--slurp`, `--stream`, `--stream-errors`, and explicit sequence input. Their combinations MUST follow accepted jq behavior where the data model is shared and documented tq behavior for TOON framing. JSON5 and YAML SHALL remain document-at-a-time formats without decoder-event stream support.

#### Scenario: Null input
- **WHEN** `-n` is provided without files
- **THEN** the filter runs once with null and stdin is not read as structured input

#### Scenario: Raw slurp
- **WHEN** `-R -s` is used
- **THEN** all raw input text is supplied as one string according to compatibility cases

#### Scenario: Explicit stream
- **WHEN** `--stream` is used with TOON input
- **THEN** the filter receives jq-compatible path/value events from the incremental decoder

#### Scenario: YAML stream is not implied
- **WHEN** `--stream --input-format yaml` is requested in the MVP
- **THEN** planning rejects the combination before consuming input and explains that YAML input is document-at-a-time

#### Scenario: JSON5 stream is not implied
- **WHEN** `--stream --input-format json5` is requested
- **THEN** planning rejects the combination before consuming input and explains that JSON5 input is document-at-a-time

#### Scenario: Automatic extension selects JSON5 in stream mode
- **WHEN** `--stream` receives a `.json5` file without an explicit input format
- **THEN** planning rejects the combination before consuming the file and explains that JSON5 input is document-at-a-time

#### Scenario: Automatic detection selects YAML in stream mode
- **WHEN** `--stream` uses automatic detection and the bounded probe selects YAML
- **THEN** tq rejects the mode after detection but before emitting query results

#### Scenario: Sequence input
- **WHEN** TOON sequence input is enabled
- **THEN** every RS-framed record becomes one ordered input value

### Requirement: External variables
The CLI SHALL support repeated `--arg name value`, `--argjson name json`, and `--argtoon name toon` options. Duplicate variable names, invalid names, and parse failures MUST follow documented deterministic behavior.

#### Scenario: String argument
- **WHEN** `--arg name Alice '$name'` is executed
- **THEN** the filter receives the string `"Alice"`

#### Scenario: Structured argument parse error
- **WHEN** `--argjson` or `--argtoon` receives invalid structured text
- **THEN** the CLI exits with a usage/input diagnostic before processing documents

### Requirement: Capability-governed jq environment variable
The CLI SHALL apply the existing environment capability contract to `$ENV` as well as `env`. `--allow-environment` SHALL admit one startup snapshot for the query, while the default-denied mode and an explicitly denying `CapabilityPolicy` SHALL prevent ambient values from becoming query-visible.

#### Scenario: Explicitly allowed environment
- **WHEN** `--allow-environment` is supplied and environment access is permitted by `CapabilityPolicy`
- **THEN** `$ENV` and `env` expose the same startup snapshot during that invocation

#### Scenario: Default environment denial
- **WHEN** a query references `$ENV` without `--allow-environment`
- **THEN** the query is not rejected as an unknown variable, but evaluation returns an environment-policy error before any environment value is exposed

#### Scenario: Library policy denial wins
- **WHEN** `--allow-environment` is supplied but the library's `CapabilityPolicy.environment` is false
- **THEN** the CLI rejects the incompatible request or otherwise returns the existing environment-policy classification before input processing, consistent with `env`

#### Scenario: Environment values stay out of diagnostics
- **WHEN** an environment-dependent query fails because ambient access is denied
- **THEN** stderr and machine-readable diagnostics identify the denied operation and policy class without serializing environment contents

#### Scenario: CLI special-variable names are reserved
- **WHEN** an external argument is supplied with the name `ENV` or `__loc__`
- **THEN** it does not replace the corresponding jq special variable reference

### Requirement: Output formatting controls
TOON output SHALL support indentation, comma/tab/pipe delimiter selection, and safe key folding options. JSON output SHALL support pretty and compact formatting options. JSON Lines output SHALL always be compact and SHALL reject `--pretty-output`, `--indent`, `--tab`, forced color, and raw or joined output modes. `--compact-output`, ASCII escaping, and recursive key sorting MAY be combined with JSON Lines output. Incompatible option and format combinations MUST fail before input is consumed.

#### Scenario: Pipe delimiter
- **WHEN** TOON output selects the pipe delimiter
- **THEN** eligible arrays use valid TOON pipe-delimited syntax with correct quoting

#### Scenario: JSON-only compact option
- **WHEN** a JSON-only compact option is applied to TOON output
- **THEN** the CLI reports an incompatible-option usage error

#### Scenario: JSON Lines aliases
- **WHEN** `--output-format jsonl` or `--output-format ndjson` is selected
- **THEN** tq selects the same compact LF-terminated JSON Lines writer

#### Scenario: Pretty JSON Lines conflict
- **WHEN** JSON Lines output is combined with `--pretty-output`, `--indent`, or `--tab`
- **THEN** the CLI reports an incompatible-option usage error before reading input

#### Scenario: Raw JSON Lines conflict
- **WHEN** JSON Lines output is combined with raw, joined, or forced-color output
- **THEN** the CLI reports an incompatible-option usage error before reading input

### Requirement: Strictness
TOON input SHALL use strict validation by default. A documented non-strict option MAY relax only the TOON rules permitted by the underlying spec and MUST NOT disable resource limits or UTF-8 validation.

#### Scenario: Invalid strict count
- **WHEN** a TOON array count is wrong under default settings
- **THEN** the CLI exits with an input-parse error

### Requirement: YAML input profile
YAML input SHALL be parsed with the actively maintained `yaml_serde` crate and converted one document at a time into the shared JSON-shaped value model. Accepted mappings MUST have string keys. Duplicate keys, unsupported custom tags, non-string keys, non-finite numbers, and numeric values that cannot enter the documented hybrid envelope without loss MUST fail explicitly. YAML comments, styles, anchors, aliases, directives, and tags SHALL NOT be retained in runtime values.

#### Scenario: Multi-document YAML
- **WHEN** a YAML stream contains multiple documents
- **THEN** tq evaluates them as ordered input documents without slurping unless requested

#### Scenario: Non-string mapping key
- **WHEN** YAML contains a mapping keyed by an array, object, boolean, or number
- **THEN** tq reports an unsupported YAML-to-runtime mapping-key diagnostic instead of stringifying the key

#### Scenario: YAML numeric fidelity
- **WHEN** an accepted YAML numeric scalar passes through identity
- **THEN** conversion into the hybrid number model does not silently lose its mathematical value

### Requirement: jq-aligned exit statuses
The CLI SHALL distinguish success, usage/system error, query compile error, input/runtime error, and jq-compatible `--exit-status` outcomes. Under `-e`, false/null as the last result and no result MUST use distinct jq-aligned statuses.

#### Scenario: Normal success
- **WHEN** evaluation completes and `-e` is not requested
- **THEN** the process exits zero even when the last emitted value is false or null

#### Scenario: Exit status false
- **WHEN** `-e` is requested and the last result is false or null
- **THEN** the process exits with the accepted jq false/null status

#### Scenario: Exit status no output
- **WHEN** `-e` is requested and no valid result is emitted
- **THEN** the process exits with the accepted jq no-output status

#### Scenario: Compile failure
- **WHEN** the filter does not compile
- **THEN** the process uses the compile-error status and emits no structured result

### Requirement: stdout and stderr discipline
Data results SHALL be written only to stdout. Diagnostics, warnings, explanations, statistics, and trace output SHALL be written only to stderr unless an explicit report-file option is used.

#### Scenario: Pipeline
- **WHEN** stdout is piped into another structured-data command
- **THEN** no progress or diagnostic text contaminates the result stream

### Requirement: Help, version, and support matrix
The CLI SHALL provide stable help, version, and compatibility-report commands. Version output MUST include the tq version, TOON spec target, jq compatibility target, and optional build revision.

#### Scenario: Compatibility report
- **WHEN** `tq compatibility` is executed
- **THEN** it displays machine-readable or human-readable supported, partial, deferred, and unsupported capabilities derived from the test manifest

### Requirement: Deferred jq CLI options are rejected clearly
MVP-unimplemented jq options such as modules/library paths, slurp/raw files, positional argument modes, color configuration, and platform-specific options SHALL fail as unsupported if recognized. They MUST NOT be silently ignored.

#### Scenario: Deferred module path
- **WHEN** a user supplies jq-compatible module path syntax in the MVP
- **THEN** the CLI reports the deferred module capability and exits with a usage/unsupported status

### Requirement: Remaining input consumption
The `inputs` built-in SHALL pull and decode the remaining ordered documents from the active stdin or file source set. A document consumed by `inputs` MUST NOT later become a separate top-level evaluation input. Decoding, proxy, byte, depth, cancellation, and source-order behavior MUST remain the same as top-level CLI input processing.

#### Scenario: Consume remaining stdin values
- **WHEN** three JSON values are supplied on stdin and the first evaluation runs `[., inputs]`
- **THEN** one result containing all three values is emitted and tq does not run the filter again for the second or third value

#### Scenario: Consume remaining files
- **WHEN** the filter calls `inputs` while tq is processing the first of several input files
- **THEN** it emits documents from the remaining files in command-line order

#### Scenario: No remaining input
- **WHEN** `inputs` is evaluated after the active source set is exhausted
- **THEN** it emits zero results rather than `null`

#### Scenario: Remaining input fails to decode
- **WHEN** `inputs` reaches malformed structured input without proxy-on-error
- **THEN** evaluation stops with the same classified input failure used by top-level processing

#### Scenario: Null input mode
- **WHEN** `inputs` is evaluated under `--null-input` with no file sources
- **THEN** it emits zero results

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
