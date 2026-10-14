# Streaming Transcode Specification

## Purpose

Define bounded-memory identity conversion from structural input events to
canonical TOON without constructing a complete runtime document.

## Requirements

### Requirement: Proven transcode eligibility
The system SHALL select the transcode plan only when analysis proves that the resolved query is semantic identity, the selected decoder exposes ordered structural events with the requested duplicate-key semantics, and the requested output is canonical TOON 4.1 with supported options. The proof MUST be complete before semantic input consumption. If any condition fails, the system SHALL select another sound plan without speculative transcode output. Removed folding options SHALL NOT remain proof conditions.

#### Scenario: JSON identity conversion
- **WHEN** a JSON input uses the identity query and default TOON writer options
- **THEN** the system selects the transcode plan before decoding the root value

#### Scenario: Implicit identity conversion
- **WHEN** the CLI supplies its implicit identity query for eligible JSON-to-TOON conversion
- **THEN** the system applies the same transcode proof as it does for an explicit `.` query

#### Scenario: Unsupported writer option
- **WHEN** a supported CLI invocation requests a writer option not supported by structural transcode
- **THEN** the system selects the document plan before consuming semantic input and explains the rejected transcode condition

#### Scenario: Non-identity query
- **WHEN** the query performs selection, projection, mutation, or any other non-identity operation
- **THEN** the system does not select the transcode plan

#### Scenario: Non-strict duplicate resolution
- **WHEN** a decoder's direct events cannot preserve required non-strict last-write-wins semantics
- **THEN** analysis chooses a path that can preserve those values before consuming input and does not claim direct event equivalence

### Requirement: Document-equivalent output
For every accepted input without duplicate object names, the transcode plan SHALL emit the same canonical TOON 4.1 bytes and result framing as the document plan under the same input and output options. It MUST preserve outer object encounter order, array order, supported exact numbers, string contents, booleans, nulls, and empty containers. Table-value object key order SHALL follow the recursive header order required by 4.1, identically in both paths.

#### Scenario: Differential identity conversion
- **WHEN** a correctness fixture is encoded through both transcode and forced-document plans
- **THEN** their output bytes and exit classifications are identical

#### Scenario: Exact number
- **WHEN** JSON contains an accepted number outside the lossless IEEE-754 integer range
- **THEN** transcode output preserves the same exact numeric value as document-mode output

#### Scenario: Ordered object
- **WHEN** an ordinary object uses non-lexicographic key order
- **THEN** transcode output preserves that encounter order

#### Scenario: Recursive keyed table
- **WHEN** a root object has two or more uniform object values with nested-uniform columns
- **THEN** both paths emit identical keyed headers, recursive field groups and entry rows

#### Scenario: Anonymous nested array
- **WHEN** an array of uniform objects is itself an anonymous array list item
- **THEN** both paths emit nested list form rather than a keyless fields-bearing header

### Requirement: Bounded transcode retention
The transcode plan SHALL NOT construct the complete root document or a result-sized output string. Retained memory, including active object/array shape preparations and recursive schema state, MUST remain bounded by configured aggregate preparation, nesting, token, decoder, writer and output-buffer limits rather than total input bytes. Canonical shape decisions SHALL permit delayed object-member publication; retained completed members MUST be releasable or securely spooled within those bounds.

#### Scenario: Wide root object
- **WHEN** identity conversion processes a wide object whose canonical shape remains undecided
- **THEN** completed members share bounded replay storage and spill or fail under configured limits instead of accumulating a root-sized DOM

#### Scenario: Deep input within limits
- **WHEN** input depth stays within the configured maximum
- **THEN** retained structural state grows with active depth rather than completed sibling count outside bounded preparation

### Requirement: Streaming duplicate-key limitation
The transcode plan SHALL preserve object encounter order and reject a repeated object name when encountered, without copying or prevalidating the complete source. JSON transcode does not provide the document plan's last-value-first-position normalization. Shape preparation MAY delay member publication, but MUST NOT silently normalize duplicates or promise already-published member bytes before shape determination. Explanations and compatibility documentation MUST identify this limitation before input consumption.

#### Scenario: Late JSON duplicate
- **WHEN** JSON sequence transcode encounters a repeated object name
- **THEN** transcode reports an input error and identifies the current output record as incomplete, whether only its RS or additional body bytes were already written

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
When TOON 4.1 array or keyed-object syntax requires a final count or recursive layout decision, the transcode plan SHALL retain one replayable representation of each pending element or entry. All active replay preparations, schema state and transient composites MUST share the configured in-memory budget. Once exhausted, eligible root/object-field preparations SHALL move to secure bounded spooling; unspillable state SHALL fail with a resource diagnostic rather than allocate past the aggregate budget. After the candidate closes, the writer MUST select the canonical position-appropriate form and replay stored values once in order.

#### Scenario: Unknown root array
- **WHEN** an unknown-length root array exceeds the shared in-memory preparation budget
- **THEN** preparation moves to disk, emits the final count before the body, replays every element once, and cleans up the spool

#### Scenario: Nested active arrays
- **WHEN** several object and array preparations are open at once
- **THEN** replay storage, schemas and transient composite state are charged to one shared budget, and exhaustion either spools an eligible preparation or returns a resource error

#### Scenario: Tabular candidate becomes ineligible
- **WHEN** a later element invalidates the recursive schema inferred from earlier elements
- **THEN** the writer selects the canonical non-tabular layout without losing, duplicating, or reordering earlier elements

#### Scenario: Keyed candidate becomes ineligible
- **WHEN** a late object entry contains an array, an empty object, or an incompatible nested field
- **THEN** the entire object uses canonical expanded form and no speculative keyed header escapes

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
Human-readable and machine-readable explanations and run reports SHALL identify the transcode plan, its eligibility proof, duplicate-key limitation, retained-state description, active limits, in-memory preparation high-water mark, spool bytes and output commitment mode. They MUST distinguish direct sequence publication from atomic unframed replay and describe shape-dependent object and array preparation without claiming that every object member is immediately writable or that upstream full-document streaming is bounded.

#### Scenario: Explain eligible transcode
- **WHEN** explanation is requested for an eligible JSON identity conversion
- **THEN** it reports `transcode`, no root materialization, object/array preparation policy, and output commitment mode

#### Scenario: Report array spool
- **WHEN** a transcode run moves array or unframed preparation to disk
- **THEN** its report records that spooling occurred and reports written and replayed byte counts

#### Scenario: Report object preparation
- **WHEN** a wide keyed-object candidate needs spooling
- **THEN** reports disclose its actual bounded retention and spool activity rather than describing an array-only or immediate-member path

### Requirement: Focused TOON migration performance guard
The TOON 4.1 migration SHALL include a focused correctness-gated performance comparison of frozen PR68 baseline and final candidate release/default binaries on the same native host with comparable build features, settings and timing boundaries. Selected workloads SHALL cover TOON decoding, document encoding, identity transcode, nested/keyed shapes and spool-triggering preparation. Each workload SHALL pass only when comparable repeated measurements show no confirmed median wall-time increase greater than 20%; exactly +20% SHALL NOT constitute a breach. An observed median increase greater than 10% SHALL produce an advisory review note without failing solely for crossing that advisory boundary. Other workloads' improvements SHALL NOT offset a hard-limit regression. RSS SHALL remain diagnostic for this guard.

The guard SHALL use one warmup per binary and three alternating measured pairs initially, with seven additional paired measurements to confirm initial hard-limit breaches or noisy hard-threshold results. It SHALL retain samples, medians and dispersion separately for initial and confirmation rounds. A median-plus/minus-MAD ratio band straddling 1.20, a nonpositive baseline lower bound, or disagreement between rounds' hard-threshold classifications SHALL be inconclusive rather than passing; the band SHALL NOT be described as a confidence interval. Stable confirmed results whose dispersion band is wholly above 1.20 SHALL fail. Crossing the 10% advisory boundary alone SHALL NOT trigger confirmation or block acceptance. Missing, incorrect, incompatible, timed-out, crashed or inconclusive rows SHALL block acceptance under an explicit bounded campaign deadline. Earlier reports under the original 10% hard-limit policy SHALL retain their original outcomes and policy.

Both versions SHALL satisfy the same semantic/process expectations; expected canonical format changes SHALL NOT require identical TOON output bytes. Version-specific input representations of one fixed semantic source SHALL be identified explicitly with separate byte hashes/sizes rather than called identical-byte comparisons. Reports SHALL retain binary/input identities, native target, profile/features, measurement settings, samples, per-workload ratios and actual outcomes. The one-sample quick profile SHALL NOT qualify, and this local guard SHALL NOT waive calibrated performance/publication requirements or deferred platform obligations.

#### Scenario: Confirmed significant slowdown
- **WHEN** a workload's initial and confirmation medians exceed the baseline by more than 20% and confirmation dispersion is wholly above the hard threshold
- **THEN** the guard fails even if all other workloads improve

#### Scenario: Exact threshold
- **WHEN** comparable candidate median wall time is exactly 1.20 times baseline time without hard-threshold-straddling dispersion
- **THEN** that workload passes the threshold check

#### Scenario: Advisory slowdown pending later review
- **WHEN** a comparable workload passes the 20% hard-limit gate but its observed median ratio is greater than 1.10
- **THEN** the report and CLI include a review note while preserving the passing outcome
- **AND** an exactly 1.10 median ratio does not receive that advisory note

#### Scenario: Noisy threshold result
- **WHEN** the dispersion ratio band straddles 1.20 or initial and confirmation rounds disagree on a hard-threshold breach
- **THEN** the result remains inconclusive and cannot establish acceptance

#### Scenario: Canonical output changes
- **WHEN** baseline and candidate produce different conforming TOON bytes for the same workload values
- **THEN** timing remains eligible only after each independently satisfies the shared semantic/process contract

#### Scenario: Missing or interrupted workload
- **WHEN** the deadline expires or a selected workload lacks valid comparable measurements
- **THEN** completed evidence is retained and the guard cannot return a passing result

#### Scenario: One-sample shortcut
- **WHEN** a one-sample quick report or an incompatible historical baseline is supplied
- **THEN** it cannot satisfy the migration guard or calibrated regression acceptance
