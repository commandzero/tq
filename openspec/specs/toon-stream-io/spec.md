# TOON Stream I/O Specification

## Purpose

Define incremental TOON decoding, canonical output, event contracts, framing,
bounded preparation, and conformance requirements for structured stream I/O.

## Requirements

### Requirement: Incremental byte-oriented TOON decoding
The TOON decoder SHALL consume `Read`/`BufRead` input incrementally without first loading the full document or collecting all Unicode scalar values. It MUST validate UTF-8 and maintain byte offset, line, and column positions.

#### Scenario: Decode large input
- **WHEN** a valid large TOON document is read from a non-seekable stream
- **THEN** decoding begins before EOF and memory does not scale with total document bytes unless a downstream consumer requests materialization

#### Scenario: Invalid UTF-8
- **WHEN** an invalid UTF-8 sequence is encountered
- **THEN** decoding stops with a source-positioned input error and does not replace the bytes silently

### Requirement: Unified structural event contract
The decoder SHALL emit a stable ordered event stream representing document boundaries, object boundaries, keys, array boundaries with declared length/delimiter/tabular fields, and typed scalar values. Both document construction and explicit stream execution MUST consume this same event contract.

#### Scenario: Tabular array
- **WHEN** a tabular TOON array is decoded
- **THEN** its start event carries the declared length, active delimiter, and ordered field names before row scalar events

#### Scenario: Build document
- **WHEN** the DOM builder consumes a valid event stream
- **THEN** it produces the same ordered JSON-model value as direct conformance decoding

### Requirement: Strict validation during consumption
Strict decoding SHALL validate root shape, indentation, whitespace rules, string escapes, delimiter scope, array count, tabular row width, field-list uniqueness, and configured nesting/token limits while events are consumed.

#### Scenario: Declared count mismatch
- **WHEN** an array ends after a different number of elements than its declared length
- **THEN** strict decoding returns a count-mismatch error at the array's source location

#### Scenario: Excessive nesting
- **WHEN** input exceeds the configured maximum nesting depth
- **THEN** decoding fails before growing parser state beyond the configured bound

### Requirement: Ordered loss-aware document model
The DOM builder SHALL preserve array order, object insertion order, string contents, boolean/null identity, and supported numeric representations. Duplicate object-key behavior and out-of-envelope number behavior MUST be documented and compatibility-tested.

#### Scenario: Ordered object
- **WHEN** an object contains keys in non-lexicographic order
- **THEN** document construction and canonical re-encoding preserve encounter order

### Requirement: Canonical structured output
The TOON writer SHALL emit valid canonical TOON v3 to a `Write` sink with LF line
endings, no trailing spaces, no document-internal trailing newline, correct
quoting, canonical number formatting, declared array lengths, deterministic
object order, and configured delimiter, indent, and folding options. It SHALL
write completed output incrementally where framing permits and MUST NOT require a
result-sized string as an intermediate representation.

#### Scenario: Canonical round trip
- **WHEN** a supported runtime value is encoded and decoded in strict mode
- **THEN** the decoded ordered JSON-model value equals the original

#### Scenario: Identity formatting
- **WHEN** tq applies identity to a noncanonical but valid TOON input
- **THEN** structured output is canonical and is not required to preserve original whitespace or delimiter choices

#### Scenario: Wide object sink output
- **WHEN** the writer receives a wide object from a consumer that releases completed members
- **THEN** it writes completed lines to the sink without collecting the complete output text

### Requirement: Bounded array preparation and spooling
When an output array's final length or tabular schema is unknown, the writer
SHALL retain only one replayable representation of pending values. All active
replay preparations SHALL use one configurable aggregate in-memory threshold and
then spool securely to temporary storage. Transient composite state that cannot
yet transfer into replay MUST use that same threshold and fail with a resource
diagnostic rather than allocate past it. The writer MUST write the final header
before replaying the prepared body, replay values once in order, and expose
whether spooling occurred.

#### Scenario: Unknown large array
- **WHEN** a generated array exceeds the aggregate in-memory preparation threshold
- **THEN** the writer spools excess content, emits the correct final array header, replays the body once, and cleans up temporary storage

#### Scenario: Spooling forbidden
- **WHEN** spooling is required but disabled
- **THEN** the writer fails with a resource-class diagnostic before claiming successful output

#### Scenario: Tabular eligibility changes
- **WHEN** later array elements invalidate the schema inferred from earlier elements
- **THEN** the prepared output uses a valid non-tabular representation without losing or reordering prior elements

#### Scenario: Nested preparation budget
- **WHEN** nested arrays are prepared concurrently
- **THEN** their replay and transient retained bytes do not exceed the configured aggregate threshold apart from bounded bookkeeping and current tokens

### Requirement: TOON Text Sequence framing
The structured multi-result transport SHALL encode every result as ASCII RS (`0x1e`), followed by one canonical TOON document, followed by LF. Sequence framing SHALL be distinct from the bytes of a standalone TOON document.

#### Scenario: Emit multiple results
- **WHEN** a filter emits two multiline objects
- **THEN** the output contains two independently parseable RS-framed records in emission order

#### Scenario: Emit zero results
- **WHEN** a filter emits no values
- **THEN** the structured sequence output is empty

#### Scenario: Emit one result
- **WHEN** a filter emits one structured value in sequence mode
- **THEN** the output still contains exactly one RS-framed record

### Requirement: Explicit unframed single output
The writer SHALL support an unframed mode that produces exactly one standalone
canonical TOON document. It MUST report an error if evaluation yields zero or
multiple values. Output in this mode MUST remain unpublished until successful
exactly-one-result cardinality is known, using bounded memory and secure spooling
when the result exceeds the preparation threshold.

#### Scenario: Exactly one result
- **WHEN** unframed mode receives one valid value
- **THEN** stdout contains only that canonical TOON document

#### Scenario: Multiple results
- **WHEN** unframed mode receives a second value
- **THEN** execution fails with a cardinality diagnostic and writes no result bytes

#### Scenario: Late preparation failure
- **WHEN** encoding or input validation fails before an unframed result is committed
- **THEN** the writer publishes no partial document and cleans up temporary storage

### Requirement: Sequence input
The decoder SHALL support explicit TOON Text Sequence input and MUST treat each framed record as a separate input document. Ordinary mode SHALL treat each file as one TOON document.

#### Scenario: Read framed stream
- **WHEN** sequence input contains three valid records
- **THEN** the evaluator receives three input documents in order

#### Scenario: Unframed multiple roots
- **WHEN** ordinary strict input contains multiple root documents
- **THEN** decoding fails rather than guessing record boundaries

### Requirement: Event-stream memory bound
With a consumer that does not materialize subtrees, event decoding SHALL retain memory proportional to configured nesting depth, the current source line/token, active delimiter/schema stacks, and current consumer result—not total input size.

#### Scenario: Stream a large feature collection
- **WHEN** a consumer discards completed path/value events from a large document
- **THEN** completed feature values are released and peak parser memory does not grow with the feature count

### Requirement: I/O conformance and differential tests
The I/O implementation SHALL pass the official TOON conformance fixtures, round-trip property tests, malformed-input tests, and event-versus-DOM equivalence tests before it is used by tq compatibility benchmarks.

#### Scenario: Event and DOM agreement
- **WHEN** a valid fixture is decoded through events and built into a document
- **THEN** its value equals the established conforming decoder result
