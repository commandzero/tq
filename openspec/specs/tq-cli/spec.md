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

`--input-format toon|toon-seq|yaml|json|json5|jsonl|json-seq|csv|tsv` SHALL select exactly one parser and disable detection or faildown. `ndjson` SHALL be accepted as an alias for `jsonl`, `toon-sequence` as an alias for `toon-seq`, and `jsonseq` as an alias for `json-seq`. An explicit override SHALL take precedence over a recognized file extension. TOON SHALL remain the default structured output format, while TOON sequence, YAML, JSON, JSON Lines, JSON sequence, CSV, and TSV output SHALL be available through `--output-format toon|toon-seq|yaml|json|jsonl|json-seq|csv|tsv` with the same aliases. JSON5 output SHALL remain unsupported.

`--seq` SHALL select jq-compatible JSON sequence input and TOON sequence output by default. `--seq --output-format json` SHALL select JSON sequence output, and `--seq -c` SHALL select compact JSON sequence output. Explicit JSON input SHALL be accepted with `--seq` and normalized to JSON sequence input. Explicit incompatible input formats and `--unframed` MUST fail before input is consumed, regardless of option order.

#### Scenario: Default format
- **WHEN** no format option is provided for a source without a recognized extension
- **THEN** bounded syntax probing selects TOON, strict JSON, JSON Text Sequence, or YAML and structured output emits LF-terminated canonical TOON values without RS

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
- **WHEN** `--seq --output-format json` is supplied
- **THEN** tq reads and writes RFC 7464 JSON Text Sequences with jq-compatible recovery and framing

#### Scenario: Default sequence output
- **WHEN** `--seq` is supplied without an output-format option
- **THEN** tq reads JSON Text Sequences and emits TOON Text Sequence frames

#### Scenario: Explicit TOON sequence
- **WHEN** `--input-format toon-seq --output-format toon-seq` is supplied
- **THEN** tq reads and writes TOON Text Sequence frames without selecting JSON sequence input

#### Scenario: Sequence switch conflict
- **WHEN** `--seq` is combined with `--input-format csv` or `--unframed`
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
Structured TOON output SHALL emit zero or more canonical values, each followed
by LF, without RS by default. Explicit `--output-format toon-seq` SHALL select
TOON Text Sequence framing without changing input selection. `--seq` SHALL
additionally select JSON sequence input and, when structured output is default
or explicit TOON (`--output-format toon`), TOON Text Sequence output; when
structured output is JSON (`--output-format json` or `-c`), JSON Text Sequence
output. Other explicit native structured output formats SHALL retain their
native framing. `--unframed` SHALL require exactly one result. `-r/--raw-output`
SHALL write strings without structured quoting, and `-j/--join-output` SHALL
suppress raw-output separators as defined by jq-compatible cases.

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
For implementation/evidence closeout only, the 32 native Windows newline target/case contracts and the Linux/Windows presentation and test-runner contracts in the `cross-tool-compatibility` follow-up map remain unresolved under #69/#70. This qualification applies only to those enumerated observations, including compact/raw/sequence/stream bytes where mapped; no other option, process-effect, framing, input, or default-output behavior is deferred. Captured bytes and both manual gate policies SHALL remain unchanged, without blanket CRLF/ANSI normalization or deferred-as-pass counting.

TOON output SHALL support indentation, comma/tab/pipe delimiter selection, and safe key folding options. When no output-format selector is supplied and none of `-c`/`--compact-output`, `--seq`, or `--unframed` is supplied, structured output SHALL emit zero or more canonical TOON values, each followed by LF without RS. `--seq` SHALL select JSON Text Sequence input. With default TOON output or explicit TOON output (`-o toon`/`--output-format toon`), it SHALL select RS-framed TOON Text Sequence output; with JSON output (`-o json`/`--output-format json` or `-c`), it SHALL select RS-framed JSON Text Sequence output. Other explicit native structured outputs SHALL retain their native framing. `-o toon-seq`/`--output-format toon-seq` SHALL select TOON sequence output without changing input selection. `-o json` SHALL select JSON, pretty by default. `-c` and `--compact-output`, including bundled short options, SHALL select compact JSON without requiring `-o json`. An explicit TOON output selection combined with compact JSON SHALL fail before input consumption regardless of argument order. Explicit JSON selection with `-c` SHALL be valid in either order. JSON Lines output SHALL remain compact and SHALL reject `--pretty-output`, `--indent`, `--tab`, and raw or joined output modes. JSON Lines and its NDJSON alias SHALL accept color under the archived `output-colors` main spec, including automatic terminal color and forced `-C`; removing tq-generated SGR SHALL preserve the plain compact LF-terminated bytes. Consumers requiring directly parseable JSON Lines SHALL use redirected automatic output or `-M`. Compact output, ASCII escaping, and recursive key sorting MAY be combined with JSON Lines output. Incompatible options MUST fail before input is consumed. An output-format selector MUST NOT silently select an input parser; `--seq` explicitly selects JSON sequence input and framing only for TOON or JSON output.

#### Scenario: Default TOON
- **WHEN** `tq '.'` runs without output options
- **THEN** it emits each canonical TOON value followed by LF without RS; zero results produce empty stdout and multiple results succeed

#### Scenario: Explicit TOON sequence output
- **WHEN** `tq --seq '.'` runs with default TOON output
- **THEN** it emits one RS-framed canonical TOON record for each result and preserves completed records before a later error

#### Scenario: Compact JSON selector
- **WHEN** `tq -c '.'` or `tq --compact-output '.'` processes structured input
- **THEN** it emits jq-compatible compact JSON with LF-separated records and no TOON framing

#### Scenario: Compact option combinations
- **WHEN** compact output is combined with `-o json`, `-r`, `-j`, `-S`, or a bundled option such as `-cr`
- **THEN** JSON and raw output behavior matches the corresponding jq flags

#### Scenario: JSON-only compact option
- **WHEN** `-c -o toon` or `-o toon -c` is supplied
- **THEN** the CLI reports an incompatible-option usage error before reading input

#### Scenario: Pipe delimiter
- **WHEN** TOON output selects the pipe delimiter
- **THEN** eligible arrays use valid TOON pipe-delimited syntax with correct quoting

#### Scenario: JSON Lines aliases
- **WHEN** `--output-format jsonl` or `--output-format ndjson` is selected
- **THEN** tq selects the same compact LF-terminated JSON Lines writer

#### Scenario: Pretty JSON Lines conflict
- **WHEN** JSON Lines output is combined with `--pretty-output`, `--indent`, or `--tab`
- **THEN** the CLI reports an incompatible-option usage error before reading input

#### Scenario: Raw JSON Lines conflict
- **WHEN** JSON Lines output is combined with raw or joined output
- **THEN** the CLI reports an incompatible-option usage error before reading input

#### Scenario: Colored JSON Lines
- **WHEN** JSON Lines output is combined with forced color
- **THEN** tq accepts the option and styles compact JSON tokens while preserving the underlying LF-terminated serialization
- **AND** with terminal capability permitted, JSON Lines and its NDJSON alias use the shared tq palette, and removing generated SGR yields exactly the corresponding plain compact records and LF framing

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
Every option documented in the pinned jq manual SHALL implement its documented contract, subject to the explicit TOON default-output, product-identity, and archived `output-colors` presentation contracts. Such options MUST NOT be reported as deferred. Unknown options and historical options absent from the reference SHALL fail with the reference usage contract and MUST NOT be silently ignored.

#### Scenario: Deferred module path
- **WHEN** a user supplies the documented jq library-path syntax
- **THEN** the CLI applies jq-compatible module lookup rather than reporting a deferred capability

#### Scenario: Unknown option
- **WHEN** an unrecognized option is supplied
- **THEN** the CLI emits a usage diagnostic and the reference-compatible exit status before consuming input

### Requirement: Remaining input consumption
`input` and `inputs` SHALL share the top-level evaluator's ordered input cursor, including stdin and file arguments. `input` SHALL pull one value and report jq's end-of-input error when exhausted; `inputs` SHALL pull all remaining values and emit nothing at exhaustion. Consumed documents MUST NOT later become top-level evaluation inputs. `--null-input` SHALL suppress only the implicit initial read, not reads requested by these built-ins. Decoding, proxy, byte, depth, cancellation, and source-order behavior MUST remain the same as top-level CLI processing. Source filename and line metadata SHALL reflect the active consumed input.

#### Scenario: Consume remaining stdin values
- **WHEN** three JSON values are supplied and the first evaluation runs `[., inputs]`
- **THEN** one result contains all three values and the remaining values are not evaluated again

#### Scenario: Consume remaining files
- **WHEN** a filter calls `inputs` while processing the first of several files
- **THEN** it emits remaining documents in command-line order

#### Scenario: No remaining input
- **WHEN** the source set is exhausted
- **THEN** `inputs` emits nothing and `input` reports the reference end-of-input error

#### Scenario: Remaining input fails to decode
- **WHEN** `inputs` reaches malformed structured input without proxy-on-error
- **THEN** evaluation stops with the same classified input failure used by top-level processing and preserves already committed output

#### Scenario: Null input mode
- **WHEN** `-n '[inputs]'` receives values on stdin
- **THEN** it reads those values into one array despite suppressing the implicit top-level read

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
The CLI SHALL validate format-specific controls before consuming semantic input. All supported native output formats SHALL accept color and monochrome controls as presentation options under the output-colors policy. JSON sequence color SHALL decorate only the document payload and preserve unstyled RS/LF boundaries. JSON sequence output SHALL accept the JSON formatting controls that preserve valid RFC 7464 framing. CSV and TSV output MUST reject JSON-only, TOON-only, raw-output, and joined-output controls unless a control has an explicitly documented delimited-text meaning.

#### Scenario: Pretty JSON sequence
- **WHEN** JSON sequence output uses a compatible JSON indentation control
- **THEN** each frame contains the configured valid JSON document between RS and LF

#### Scenario: Raw CSV conflict
- **WHEN** CSV output is combined with raw or joined output
- **THEN** tq reports an incompatible-option usage error before reading input

#### Scenario: TOON option on TSV
- **WHEN** TSV output is combined with a TOON folding or delimiter option
- **THEN** tq reports an incompatible-option usage error before reading input

#### Scenario: Colored delimited output
- **WHEN** CSV or TSV output is combined with `-C`
- **THEN** headers, scalar fields, delimiters, and quotes use the default theme without changing undecorated field or row bytes

#### Scenario: Colored JSON sequence
- **WHEN** JSON sequence output is combined with `-C`
- **THEN** tq accepts forced color and stripping tq-generated SGR yields valid RFC 7464 output identical to monochrome output

### Requirement: JSON sequence and stream parity
JSON input SHALL accept jq's whitespace-separated value stream. Explicit strict JSON input SHALL disable native-format probing, including for malformed-input conformance cases. JSON `--seq` input and JSON output SHALL implement jq record framing, malformed-record diagnostics, and recovery. `--seq` SHALL select JSON Text Sequence input. If the selected structured output is default or explicit TOON (`-o toon`/`--output-format toon`), output SHALL use TOON Text Sequence framing. If output is JSON (`-o json`/`--output-format json` or `-c`), output SHALL use JSON Text Sequence framing. Other explicit native output formats SHALL retain their native framing. Explicit `-i json` SHALL be accepted with `--seq` and normalized to JSON sequence input. Explicit non-JSON input and `--unframed` SHALL conflict with `--seq` in either argument order. JSON `--stream` and `--stream-errors` SHALL preserve reference event order, container-end events, parse-error events, source positions, and partial output.

#### Scenario: JSON sequence output
- **WHEN** `--seq -o json` or `--seq -c` processes JSON sequence input
- **THEN** record framing and recovery match the equivalent jq invocation

#### Scenario: Native sequence output
- **WHEN** JSON sequence input is processed with `--seq` and default or explicit TOON output
- **THEN** output remains RS-framed TOON Text Sequence rather than implicitly becoming JSON

#### Scenario: Recover after malformed record
- **WHEN** a malformed JSON sequence record precedes a valid record
- **THEN** diagnostics, recovery at the next record separator, results, and exit status match jq

#### Scenario: Stream reconstruction
- **WHEN** manual `tostream`, `fromstream`, or `truncate_stream` programs process nested and empty containers
- **THEN** their ordered results and error behavior match jq

### Requirement: Process effects and termination
`debug`, `debug(msgs)`, and `stderr` SHALL emit jq-compatible stderr payloads while preserving their documented filter result streams. `halt` and `halt_error` SHALL terminate the process with jq-compatible status and stderr bytes. Termination MUST abandon pending evaluations, MUST NOT become an ordinary catchable runtime error, and MUST preserve output committed before termination. Unbuffered output SHALL become observable before the next input is requested.

#### Scenario: Debug generator
- **WHEN** `debug(msgs)` evaluates a message generator
- **THEN** stderr message order and stdout values match the reference independently

#### Scenario: Explicit halt error
- **WHEN** a program emits a result then calls `halt_error(7)`
- **THEN** it retains the emitted result, exits with status 7, and writes the reference stderr payload without an added diagnostic wrapper

#### Scenario: Incremental pipe output
- **WHEN** an unbuffered filter emits a result while its input pipe remains open
- **THEN** a consumer observes that result without waiting for input EOF
