## Purpose

Define jq-compatible RFC 7464 JSON Text Sequence framing, recovery, ordered input semantics, output bytes, and bounded resource behavior.

## ADDED Requirements

### Requirement: RFC 7464 frame detection
JSON Text Sequence input SHALL identify each ASCII RS byte as the start of one candidate JSON document frame and SHALL end that frame at the next RS or EOF. Consecutive RS bytes MUST NOT create empty documents. Explicit JSON sequence input SHALL discard bytes before the first RS, while content probing MUST select JSON sequence input only when RS is the first non-whitespace byte.

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
Each non-empty frame SHALL be decoded as one UTF-8 strict JSON document with no trailing non-whitespace content. A document decoding failure SHALL write a source-positioned warning to stderr, emit no input document for that frame, resume at the next RS, and remain non-fatal when no other error occurs. A framing, I/O, or resource failure that does not leave the next boundary unambiguous MUST remain fatal.

#### Scenario: Malformed frame followed by valid frame
- **WHEN** a malformed JSON frame is followed by an RS-prefixed valid frame
- **THEN** tq warns, skips the malformed frame, evaluates the valid document, and exits successfully unless another failure occurs

#### Scenario: Invalid UTF-8 frame
- **WHEN** one frame contains invalid UTF-8 and a later RS identifies another frame
- **THEN** tq warns for the invalid frame and resumes at the later frame

#### Scenario: Warning stays off stdout
- **WHEN** tq recovers from a malformed JSON sequence frame
- **THEN** stdout contains only query results and stderr contains the warning

### Requirement: JSON sequence input modes
Normal input mode SHALL run the query once per successfully decoded document. Slurp mode SHALL collect only successfully decoded documents into one ordered array, and jq event input mode SHALL reset the root path for each successfully decoded document.

#### Scenario: Normal sequence processing
- **WHEN** JSON sequence input contains two valid frames
- **THEN** the query receives two separate input values in order

#### Scenario: Slurp skips malformed frame
- **WHEN** slurp mode receives a valid frame, a malformed frame, and another valid frame
- **THEN** the query receives one array containing the two valid values in order and the malformed frame produces a warning

#### Scenario: Event roots remain separate
- **WHEN** `--stream` is combined with JSON sequence input containing two composite documents
- **THEN** each document produces its own jq-compatible root event sequence

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
