## MODIFIED Requirements

### Requirement: Best-effort input detection with strict override
When `--input-format` is absent, tq SHALL select `.jsonl` and `.ndjson` file paths as JSON Lines, `.yaml` and `.yml` paths as YAML, `.json` paths as JSON, and `.toon` paths as TOON before applying bounded syntax probing to sources without a recognized extension. Canonical TOON syntax SHALL remain preferred during probing, JSON object and non-TOON array openers SHALL commit to JSON before YAML, and YAML document, directive, or root-sequence markers SHALL commit to YAML. Once a parser commits, later syntax errors SHALL be reported for that format without restarting detection. If every parser rejects, tq SHALL emit a combined input diagnostic containing bounded, useful failure context from each candidate.

`--input-format toon|yaml|json|jsonl` SHALL select exactly one parser and disable detection or faildown. `ndjson` SHALL be accepted as an alias for `jsonl`. An explicit override SHALL take precedence over a recognized file extension. TOON SHALL remain the default structured output format, while JSON and JSON Lines output are available through `--output-format json|jsonl`, with `ndjson` accepted as an alias for `jsonl`.

#### Scenario: Default format
- **WHEN** no format option is provided for a source without a recognized extension
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

#### Scenario: Strict input override
- **WHEN** `--input-format json` is supplied for bytes that YAML could also parse
- **THEN** tq invokes only the JSON parser and never probes TOON or YAML

#### Scenario: JSON Lines extension
- **WHEN** automatic input selection receives a file ending in `.jsonl` or `.ndjson`
- **THEN** tq selects JSON Lines without content probing

#### Scenario: Override beats extension
- **WHEN** `--input-format json` is supplied for a file ending in `.jsonl`
- **THEN** tq invokes the JSON document parser and does not select JSON Lines from the extension

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
