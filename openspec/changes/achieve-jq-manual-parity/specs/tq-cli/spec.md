## MODIFIED Requirements

### Requirement: Output formatting controls
TOON output SHALL support indentation, comma/tab/pipe delimiter selection, and safe key folding options. Without an output selector, structured output SHALL emit zero or more canonical TOON values, each followed by LF without RS. `--seq` SHALL select RS-framed TOON Text Sequence output for zero or more results. `-o json` SHALL select JSON, pretty by default. `-c` and `--compact-output`, including bundled short options, SHALL select compact JSON without requiring `-o json`. An explicit TOON output selection combined with compact JSON SHALL fail before input consumption regardless of argument order. Explicit JSON selection with `-c` SHALL be valid in either order. JSON Lines output SHALL remain compact and SHALL reject `--pretty-output`, `--indent`, `--tab`, forced color, and raw or joined output modes. Compact output, ASCII escaping, and recursive key sorting MAY be combined with JSON Lines output. Incompatible options MUST fail before input is consumed. Output selection MUST NOT silently select an input parser.

#### Scenario: Default TOON
- **WHEN** `tq '.'` runs without output options
- **THEN** it emits each canonical TOON value followed by LF without RS; zero results produce empty stdout and multiple results succeed

#### Scenario: Explicit TOON sequence output
- **WHEN** `tq --seq '.'` runs without a JSON output selector
- **THEN** it emits one RS-framed canonical TOON record for each result and preserves completed records before a later error

#### Scenario: Compact JSON selector
- **WHEN** `tq -c '.'` or `tq --compact-output '.'` processes structured input
- **THEN** it emits jq-compatible compact JSON with jq record separators and no TOON framing

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
- **WHEN** JSON Lines output is combined with raw, joined, or forced-color output
- **THEN** the CLI reports an incompatible-option usage error before reading input

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

### Requirement: Deferred jq CLI options are rejected clearly
Every option documented in the pinned jq manual SHALL implement its documented contract, subject to the explicit TOON default-output and product-identity contracts. Such options MUST NOT be reported as deferred. Unknown options and historical options absent from the reference SHALL fail with the reference usage contract and MUST NOT be silently ignored.

#### Scenario: Deferred module path
- **WHEN** a user supplies the documented jq library-path syntax
- **THEN** the CLI applies jq-compatible module lookup rather than reporting a deferred capability

#### Scenario: Unknown option
- **WHEN** an unrecognized option is supplied
- **THEN** the CLI emits a usage diagnostic and the reference-compatible exit status before consuming input

## ADDED Requirements

### Requirement: JSON sequence and stream parity
JSON input SHALL accept jq's whitespace-separated value stream. Explicit strict JSON input SHALL disable native-format probing, including for malformed-input conformance cases. JSON `--seq` input and JSON output SHALL implement jq record framing, malformed-record diagnostics, and recovery. `--seq` SHALL select sequence framing for the selected structured output format, while leaving the output syntax unchanged. JSON `--stream` and `--stream-errors` SHALL preserve reference event order, container-end events, parse-error events, source positions, and partial output.

#### Scenario: JSON sequence output
- **WHEN** `--seq -o json` or `--seq -c` processes JSON sequence input
- **THEN** record framing and recovery match the equivalent jq invocation

#### Scenario: Native sequence output
- **WHEN** TOON sequence input is processed with `--seq` and without a JSON output selector
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
