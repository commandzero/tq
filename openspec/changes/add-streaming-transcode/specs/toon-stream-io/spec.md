## MODIFIED Requirements

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
array preparations SHALL use one configurable aggregate in-memory threshold and
then spool securely to temporary storage. The writer MUST write the final header
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
- **THEN** their combined retained bytes do not exceed the configured aggregate threshold apart from bounded bookkeeping and current tokens

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
