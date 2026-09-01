## MODIFIED Requirements

### Requirement: Best-effort input detection with strict override
When `--input-format` is absent, tq SHALL select `.jsonl` and `.ndjson` file paths as JSON Lines, `.json5` paths as JSON5, `.yaml` and `.yml` paths as YAML, `.json` paths as JSON, and `.toon` paths as TOON before applying bounded syntax probing to sources without a recognized extension. Canonical TOON syntax SHALL remain preferred during probing, JSON object and non-TOON array openers SHALL commit to strict JSON before YAML, and YAML document, directive, or root-sequence markers SHALL commit to YAML. Content probing MUST NOT select JSON5. Once a parser commits, later syntax errors SHALL be reported for that format without restarting detection. If every probed parser rejects, tq SHALL emit a combined input diagnostic containing bounded, useful failure context from each candidate.

`--input-format toon|yaml|json|json5|jsonl` SHALL select exactly one parser and disable detection or faildown. `ndjson` SHALL be accepted as an alias for `jsonl`. An explicit override SHALL take precedence over a recognized file extension. TOON SHALL remain the default structured output format, while JSON and JSON Lines output are available through `--output-format json|jsonl`, with `ndjson` accepted as an alias for `jsonl`. JSON5 output SHALL remain unsupported.

#### Scenario: Default format
- **WHEN** no format option is provided for a source without a recognized extension
- **THEN** bounded syntax probing selects TOON, strict JSON, or YAML and structured output uses TOON Text Sequence framing

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
- **WHEN** YAML is requested as an output format in the MVP
- **THEN** the CLI reports an unsupported-output-format usage error rather than implying source-preserving YAML support

#### Scenario: JSON5 output remains unsupported
- **WHEN** `--output-format json5` is requested
- **THEN** the CLI reports an unsupported-output-format usage error

#### Scenario: Ambiguous content
- **WHEN** input bytes would be valid as strict JSON, JSON5, and YAML
- **THEN** automatic mode commits an object or non-TOON array opener to strict JSON, while an explicit override selects only the requested parser

#### Scenario: JSON container enables automatic events
- **WHEN** an eligible query receives a JSON object or array without an input-format override
- **THEN** bounded detection commits to strict JSON before planning and execution uses JSON decoder events

#### Scenario: JSON Lines extension
- **WHEN** automatic input selection receives a file ending in `.jsonl` or `.ndjson`
- **THEN** tq selects JSON Lines without content probing

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

#### Scenario: Mixed-format files
- **WHEN** multiple files are supplied without an input-format override
- **THEN** tq selects each recognized extension or probes each remaining file independently and evaluates the resulting documents or records in file order

#### Scenario: Override applies to every source
- **WHEN** multiple files are supplied with an input-format override
- **THEN** tq applies only that parser to every structured source and fails on the first source invalid for it

#### Scenario: All probes reject
- **WHEN** TOON, YAML, and strict JSON all reject before commitment
- **THEN** tq exits with an input error that identifies bounded failure information for all three probed formats

#### Scenario: Late selected-format failure
- **WHEN** automatic detection commits to a parser and that parser encounters a later syntax error
- **THEN** tq reports that format's parse error and does not reinterpret the source with another parser

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
