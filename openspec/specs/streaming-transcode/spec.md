# Streaming Transcode Specification

## Purpose

Define bounded-memory identity conversion from structural input events to
canonical TOON without constructing a complete runtime document.

## Requirements

### Requirement: Proven transcode eligibility
The system SHALL select the transcode plan only when analysis proves that the
resolved query is semantic identity, the selected decoder exposes ordered
structural events, and the requested output is canonical TOON with options the
transcode writer supports. The proof MUST be complete before semantic input
consumption. If any condition fails, the system SHALL select another sound plan
without speculative transcode output.

#### Scenario: JSON identity conversion
- **WHEN** a JSON input uses the identity query and default TOON writer options
- **THEN** the system selects the transcode plan before decoding the root value

#### Scenario: Implicit identity conversion
- **WHEN** the CLI supplies its implicit identity query for eligible JSON-to-TOON conversion
- **THEN** the system applies the same transcode proof as it does for an explicit `.` query

#### Scenario: Unsupported writer option
- **WHEN** safe key folding or another unsupported TOON writer option is requested
- **THEN** the system selects the document plan before consuming semantic input and explains the rejected transcode condition

#### Scenario: Non-identity query
- **WHEN** the query performs selection, projection, mutation, or any other non-identity operation
- **THEN** the system does not select the transcode plan

### Requirement: Document-equivalent output
For every accepted input without duplicate object names, the transcode plan SHALL
emit the same canonical TOON bytes and result framing as the document plan under
the same input and output options. It MUST preserve object encounter order, array
order, supported exact numbers, string contents, booleans, nulls, and empty
containers.

#### Scenario: Differential identity conversion
- **WHEN** a correctness fixture is encoded through both transcode and forced-document plans
- **THEN** their output bytes and exit classifications are identical

#### Scenario: Exact number
- **WHEN** JSON contains an accepted number outside the lossless IEEE-754 integer range
- **THEN** transcode output preserves the same exact numeric value as document-mode output

#### Scenario: Ordered object
- **WHEN** an object uses non-lexicographic key order
- **THEN** transcode output preserves that encounter order

### Requirement: Bounded transcode retention
The transcode plan SHALL NOT construct the complete root document or a
result-sized output string. Outside active container preparation, retained
memory MUST be bounded by configured nesting, token, decoder, writer, and
output-buffer limits rather than total input bytes.

#### Scenario: Wide root object
- **WHEN** an eligible identity conversion processes a wide object whose members can be completed one at a time
- **THEN** completed members are releasable and retained memory does not grow with the number of members

#### Scenario: Deep input within limits
- **WHEN** input depth stays within the configured maximum
- **THEN** retained structural state grows with active depth rather than completed sibling count

### Requirement: Streaming duplicate-key limitation
The transcode plan SHALL write object members in encounter order without staging
or prevalidating the complete input source. It MUST reject a repeated object name
when encountered. JSON transcode does not provide the document plan's
last-value-first-position normalization. Explanations and compatibility
documentation MUST identify this difference before input consumption.

#### Scenario: Late JSON duplicate
- **WHEN** JSON sequence transcode encounters a repeated object name after earlier members were written
- **THEN** transcode reports an input error and identifies the current output record as incomplete

#### Scenario: Atomic duplicate failure
- **WHEN** unframed JSON transcode encounters a repeated object name
- **THEN** it reports an input error and publishes no output bytes

### Requirement: Single-pass sequence publication
Direct sequence transcode SHALL consume each source once. It MUST NOT copy or
validate the complete source before structural decoding. It SHALL write and flush
the record separator when document decoding starts, before consuming the root
value.

#### Scenario: First sequence byte
- **WHEN** direct sequence transcode receives a document
- **THEN** the RS byte becomes observable before the complete document has been read

### Requirement: Single-source array preparation
When TOON array syntax requires a final count or layout decision, the transcode
plan SHALL retain one replayable representation of each pending array element.
All active replay preparations MUST share the configured in-memory preparation
budget. Once that budget is exhausted, root arrays and arrays that are direct
object members SHALL move to secure bounded spooling. A composite value nested
inside another prepared array MAY retain state only within that same aggregate
budget and SHALL fail with a resource diagnostic rather than allocate past it.
After an array closes, the writer MUST choose one valid layout and replay each
stored element exactly once in input order.

#### Scenario: Unknown root array
- **WHEN** an unknown-length root array exceeds the shared in-memory preparation budget
- **THEN** preparation moves to disk, emits the final count before the body, replays every element once, and cleans up the spool

#### Scenario: Nested active arrays
- **WHEN** several arrays are open at once
- **THEN** replay storage and transient composite state are charged to one shared budget, and exhaustion either spools an eligible preparation or returns a resource error

#### Scenario: Tabular candidate becomes ineligible
- **WHEN** a later array element invalidates the schema inferred from earlier elements
- **THEN** the writer selects expanded layout without losing, duplicating, or reordering earlier elements

### Requirement: Framing-aware output commitment
TOON Text Sequence transcode output SHALL write each record incrementally and
MAY leave an incomplete final record when a late input, resource, or output error
occurs. Earlier complete records MUST remain valid. Unframed output SHALL remain
unpublished until the input document succeeds and exactly-one-result cardinality
is established, using bounded memory and secure spooling when necessary.

#### Scenario: Late error in sequence mode
- **WHEN** malformed input is discovered after bytes of the current framed result were written
- **THEN** the process exits nonzero, reports the late failure, and does not describe the incomplete record as valid output

#### Scenario: Late error in unframed mode
- **WHEN** malformed input is discovered while preparing an unframed identity result
- **THEN** no bytes from that result are written to stdout and temporary storage is cleaned up

#### Scenario: Multiple unframed results
- **WHEN** identity conversion would produce more than one unframed result
- **THEN** the system reports a cardinality error without publishing either result

### Requirement: Transcode observability
Human-readable and machine-readable explanations and run reports SHALL identify
the transcode plan, its eligibility proof, duplicate-key limitation,
retained-state description, active limits, in-memory preparation high-water
mark, spool bytes, and output commitment mode. They MUST distinguish direct
sequence writes from atomic unframed replay.

#### Scenario: Explain eligible transcode
- **WHEN** explanation is requested for an eligible JSON identity conversion
- **THEN** it reports `transcode`, no root materialization, the array preparation policy, and the output commitment mode

#### Scenario: Report array spool
- **WHEN** a transcode run moves array or unframed preparation to disk
- **THEN** its report records that spooling occurred and reports the written and replayed byte counts
