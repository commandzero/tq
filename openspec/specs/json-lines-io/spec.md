# JSON Lines I/O specification

## Purpose

Define strict, ordered JSON Lines input and output so each physical record has
an unambiguous parse boundary and each emitted result remains independently
consumable.

## Requirements

### Requirement: JSON Lines input records
JSON Lines input SHALL be UTF-8 text in which each non-empty physical line contains exactly one JSON value. The decoder MUST preserve record order, MUST accept a final record without LF, and MUST reject a non-empty line containing an invalid value or trailing non-whitespace content. Empty lines SHALL be ignored.

#### Scenario: Ordered records
- **WHEN** JSON Lines input contains an object, a scalar, and an array on separate non-empty lines
- **THEN** tq evaluates the three values as ordered input records

#### Scenario: Final line without LF
- **WHEN** the final valid JSON Lines record ends at EOF without LF
- **THEN** tq evaluates that record normally

#### Scenario: Invalid record
- **WHEN** a non-empty line contains invalid JSON or more than one JSON value
- **THEN** tq reports a JSON Lines input error containing the source identity and physical line number

#### Scenario: Empty lines
- **WHEN** JSON Lines input contains empty physical lines between valid records
- **THEN** tq ignores the empty lines without creating null or empty-string records

### Requirement: Record input modes
Each JSON Lines record SHALL act as one input value in normal mode. Slurp mode SHALL collect all records from the ordered sources into one array. Explicit stream mode SHALL produce jq-compatible path/value events for each record independently and MUST reset the root path between records.

#### Scenario: Normal record processing
- **WHEN** a filter receives three JSON Lines records without slurp or stream mode
- **THEN** the filter runs once for each record in source order

#### Scenario: Slurp records
- **WHEN** `--slurp --input-format jsonl` receives three records
- **THEN** the filter runs once with an array containing those three values in order

#### Scenario: Stream records
- **WHEN** `--stream --input-format jsonl` receives two composite records
- **THEN** the filter receives a complete jq-compatible event sequence for the first root followed by a separate sequence for the second root

### Requirement: JSON Lines output framing
JSON Lines output SHALL encode every result as one compact JSON value followed by one LF byte. It MUST preserve exact numeric tokens supported by the shared value model and MUST emit no record for an empty result sequence.

#### Scenario: Multiple results
- **WHEN** a filter emits an object, a scalar, and an array with JSON Lines output selected
- **THEN** stdout contains three compact JSON texts separated and terminated by LF

#### Scenario: No results
- **WHEN** a filter emits no results with JSON Lines output selected
- **THEN** stdout is empty

#### Scenario: Late failure
- **WHEN** one JSON Lines result is complete before a later runtime or input error
- **THEN** the complete record remains on stdout and tq reports the error on stderr with a nonzero status

### Requirement: JSON Lines resource limits
JSON Lines decoding SHALL apply the configured source-byte, physical-line-byte, token-byte, nesting-depth, VM-step, result, and output-byte limits. The decoder MUST process records incrementally and MUST NOT retain earlier records unless the selected query plan or slurp mode requires them.

#### Scenario: Oversized physical line
- **WHEN** one JSON Lines record exceeds the configured physical-line-byte limit
- **THEN** tq stops with a resource diagnostic that identifies the source and line without echoing the full record

#### Scenario: Incremental document plan
- **WHEN** a document plan processes many bounded JSON Lines records without slurp or blocking behavior
- **THEN** tq may release each completed record before reading the next one
