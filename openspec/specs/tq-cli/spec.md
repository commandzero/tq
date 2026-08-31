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
When `--input-format` is absent, tq SHALL select `.jsonl` and `.ndjson` file paths as JSON Lines, `.yaml` and `.yml` paths as YAML, `.json` paths as JSON, and `.toon` paths as TOON before applying bounded syntax probing to sources without a recognized extension. Canonical TOON syntax SHALL remain preferred during probing, JSON object and non-TOON array openers SHALL commit to JSON before YAML, and YAML document, directive, or root-sequence markers SHALL commit to YAML. Once a parser commits, later syntax errors SHALL be reported for that format without restarting detection. If every parser rejects, tq SHALL emit a combined input diagnostic containing bounded, useful failure context from each candidate.

`--input-format toon|yaml|json|jsonl` SHALL select exactly one parser and disable detection or faildown. `ndjson` SHALL be accepted as an alias for `jsonl`. An explicit override SHALL take precedence over a recognized file extension. TOON SHALL remain the default structured output format, while JSON and JSON Lines output are available through `--output-format json|jsonl`, with `ndjson` accepted as an alias for `jsonl`.

#### Scenario: Default format
- **WHEN** no format option is provided
- **THEN** bounded syntax probing selects TOON, JSON, or YAML and structured output uses TOON Text Sequence framing

#### Scenario: JSON interoperability
- **WHEN** both input and output formats are explicitly set to JSON
- **THEN** the CLI evaluates the same core program over the JSON data model and emits jq-compatible JSON result texts

#### Scenario: YAML interoperability
- **WHEN** YAML input and JSON output are selected
- **THEN** each accepted YAML document is converted to the shared ordered JSON-shaped value model and evaluated by the same core program

#### Scenario: YAML output remains deferred
- **WHEN** YAML is requested as an output format in the MVP
- **THEN** the CLI reports an unsupported-output-format usage error rather than implying source-preserving YAML support

#### Scenario: Ambiguous content
- **WHEN** input bytes would be valid as both a JSON container and YAML
- **THEN** automatic mode commits an object or non-TOON array opener to JSON, while an explicit override selects only the requested parser

#### Scenario: JSON container enables automatic events
- **WHEN** an eligible query receives a JSON object or array without an input-format override
- **THEN** bounded detection commits to JSON before planning and execution uses JSON decoder events

#### Scenario: JSON Lines extension
- **WHEN** automatic input selection receives a file ending in `.jsonl` or `.ndjson`
- **THEN** tq selects JSON Lines without content probing

#### Scenario: Override beats extension
- **WHEN** `--input-format json` is supplied for a file ending in `.jsonl`
- **THEN** tq invokes the JSON document parser and does not select JSON Lines from the extension

#### Scenario: Strict input override
- **WHEN** `--input-format json` is supplied for bytes that YAML could also parse
- **THEN** tq invokes only the JSON parser and never probes TOON or YAML

#### Scenario: Mixed-format files
- **WHEN** multiple files are supplied without an input-format override
- **THEN** tq selects each recognized extension or probes each remaining file independently and evaluates the resulting documents or records in file order

#### Scenario: Override applies to every source
- **WHEN** multiple files are supplied with an input-format override
- **THEN** tq applies only that parser to every structured source and fails on the first source invalid for it

#### Scenario: All probes reject
- **WHEN** TOON, YAML, and JSON all reject before commitment
- **THEN** tq exits with an input error that identifies bounded failure information for all three formats

#### Scenario: Late selected-format failure
- **WHEN** automatic detection commits to a parser and that parser encounters a later syntax error
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
The MVP SHALL support `-n/--null-input`, `-R/--raw-input`, `-s/--slurp`, `--stream`, `--stream-errors`, and explicit sequence input. Their combinations MUST follow accepted jq behavior where the data model is shared and documented tq behavior for TOON framing.

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
