# JSON Sequence I/O Specification

## Purpose

Define jq-compatible RFC 7464 JSON Text Sequence framing, recovery, ordered input semantics, output bytes, and bounded resource behavior.

## Requirements

### Requirement: RFC 7464 frame detection
JSON Text Sequence input SHALL identify each ASCII RS byte as the start of a recovery segment ending at the next RS or EOF. For jq compatibility, each segment MAY contain multiple JSON document frames; decoding SHALL publish each complete root in order without requiring another RS. Recovery-segment indices MUST remain distinct from document indices. Consecutive RS bytes MUST NOT create empty documents. Explicit JSON sequence input SHALL discard bytes before the first RS, while content probing MUST select JSON sequence input only when RS is the first non-whitespace byte.

#### Scenario: Multiple Documents between separators
- **WHEN** input is `RS {"a":1} {"b":2} LF RS {"c":3} LF`
- **THEN** normal mode publishes all three Documents in order, slurp collects all three, remaining-input queries can consume each on demand, and jq event-input mode resets the root path for each Document

#### Scenario: Recovery follows jq within a segment
- **WHEN** input is `RS {"a":1} broken {"b":2} RS {"c":3} LF` in normal input mode
- **THEN** jq-compatible parser reset preserves all three complete Documents and reports the malformed token without requiring a new RS before the second Document

#### Scenario: Potentially truncated top-level number
- **WHEN** a top-level number ends immediately at RS or EOF without a terminating separator accepted by jq
- **THEN** tq reports a recoverable potentially-truncated-number failure instead of publishing that number; whitespace-terminated numbers remain accepted

#### Scenario: Ordered framed documents
- **WHEN** an RFC 7464 source contains three RS-prefixed valid JSON texts
- **THEN** tq exposes three documents in frame order

#### Scenario: Consecutive separators
- **WHEN** an RFC 7464 source contains consecutive RS bytes before a valid JSON text
- **THEN** tq ignores the redundant separators and emits no empty or null document

#### Scenario: Explicit preamble compatibility
- **WHEN** explicitly selected JSON sequence input contains unframed bytes before its first RS
- **THEN** tq discards that preamble and begins frame detection at the first RS

#### Scenario: Probe requires leading separator
- **WHEN** automatic input detection encounters an RS only after non-whitespace bytes
- **THEN** it does not select JSON sequence input from content probing

### Requirement: JSON sequence document decoding and recovery
JSON sequence decoding SHALL use strict UTF-8 JSON syntax and publish native input observations as decoding produces them. A recoverable document failure SHALL be an ordered observation with source, document, and recovery-segment context. Decoder reset and subsequent publication MUST match the pinned jq 1.8.x reference, including cases that resume within the same RS segment. Complete Documents and structural events published before the failure MUST NOT be retracted, including a complete Document followed by invalid trailing bytes. Top-level normal and `--stream` input advancement SHALL render a warning on stderr and continue; `--stream-errors` SHALL emit a jq-compatible error value and suppress that warning. A failure consumed through `input` or `inputs` SHALL instead become a query-visible input error and terminate an uncaught query with jq-compatible status. I/O and resource failures MUST remain fatal regardless of available separators; ambiguous framing outside the selected profile's recovery rules MUST remain fatal.

#### Scenario: Malformed frame followed by valid frame
- **WHEN** a JSON frame fails before producing a complete Document and is followed by an RS-prefixed valid frame in normal mode
- **THEN** tq warns, skips the malformed frame, evaluates the valid document, and exits successfully unless another failure occurs

#### Scenario: Invalid UTF-8 frame
- **WHEN** one frame contains invalid UTF-8 and a later RS identifies another frame
- **THEN** tq warns for the invalid frame and resumes at the later frame

#### Scenario: Warning stays off stdout
- **WHEN** tq recovers from a malformed JSON sequence frame without `--stream-errors`
- **THEN** stdout contains only query results and stderr contains the warning

#### Scenario: Input-sequence access exposes parse failure
- **WHEN** `-n 'inputs'` consumes a valid Document followed by a malformed sequence frame
- **THEN** the prior result remains published, the parse failure terminates the uncaught query with status 5, and tq does not also render a top-level recovery warning

#### Scenario: Query catches input failure
- **WHEN** `try inputs catch .` encounters a malformed sequence frame
- **THEN** the query receives the jq-compatible error message, no recovery warning is printed for that query-consumed failure, and normal query error-handling semantics apply

### Requirement: JSON sequence input modes
Normal input mode SHALL run the query once per published complete Document. Slurp mode SHALL collect those Documents into one ordered array, including Documents published before later trailing-syntax failure. Event input mode SHALL preserve already published events, and decoder state and root paths SHALL reset for subsequent documents after recovery. Recoverable failures MUST NOT invent successful document completion.

#### Scenario: Normal sequence processing
- **WHEN** JSON sequence input contains two valid frames
- **THEN** the query receives two separate input values in order

#### Scenario: Slurp skips malformed frame
- **WHEN** slurp mode receives a valid frame, a frame that fails before producing a complete Document, and another valid frame
- **THEN** the query receives one array containing the two valid values in order and the malformed frame produces a warning

#### Scenario: Event roots remain separate
- **WHEN** `--stream` is combined with JSON sequence input containing two composite documents
- **THEN** each document produces its own jq-compatible root event sequence

#### Scenario: Event records remain ordinary query inputs
- **WHEN** `--stream -n inputs` consumes JSON sequence input
- **THEN** the query receives projected records from the shared input cursor, including partial records before a catchable parse failure, without requiring a core event execution plan

#### Scenario: Complete Document precedes trailing failure
- **WHEN** a recovery segment contains a complete JSON object followed by invalid trailing text
- **THEN** normal mode preserves the published object, reports the later failure, and resumes at the next RS

#### Scenario: Partial events precede recovery
- **WHEN** `--stream` decodes nested values before encountering malformed syntax in the same frame
- **THEN** those events remain published, a warning follows, and the next valid document begins with reset decoder state

#### Scenario: Stream error observation mapping
- **WHEN** `--stream-errors` encounters a recoverable failure after partial events
- **THEN** the partial events precede the jq-compatible error value, no duplicate warning is written, and decoding resumes at the next RS

### Requirement: RFC 7464 output framing
JSON sequence output SHALL encode every result as ASCII RS, one JSON document using the selected compatible JSON formatting controls, and one trailing LF. It MUST preserve exact numeric tokens supported by the shared value model and MUST emit no bytes for an empty result sequence.

#### Scenario: Multiple JSON sequence results
- **WHEN** a query emits two results under JSON sequence output
- **THEN** stdout contains two RS-prefixed and LF-terminated JSON documents in result order

#### Scenario: No JSON sequence results
- **WHEN** a query emits no results under JSON sequence output
- **THEN** stdout is empty

#### Scenario: Exact number output
- **WHEN** a result contains an accepted exact number outside the lossless IEEE-754 integer range
- **THEN** JSON sequence output preserves the same mathematical value and token policy as ordinary JSON output

### Requirement: JSON sequence resource limits
JSON sequence processing SHALL enforce configured source-byte, frame-byte, token-byte, nesting-depth, result, output-byte, and query execution limits without reading the complete source first. A recoverable document failure MUST NOT bypass limits or allow retained memory to grow with completed frames.

#### Scenario: Oversized frame
- **WHEN** a frame exceeds the configured frame-byte limit before the next RS
- **THEN** tq reports a resource-class failure without retaining the rest of the source

#### Scenario: Many bounded frames
- **WHEN** a long sequence contains many valid frames within individual limits
- **THEN** completed frames are releasable and retained decoder state does not grow with their count
