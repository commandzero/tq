## ADDED Requirements

### Requirement: Release-bound TOON 4.1 manual regeneration
The TOON 4.1 release candidate SHALL regenerate the complete jq-manual comparison from fresh execution of release/default `tq` 0.5.0 product and helper binaries against the unchanged pinned jq 1.8.2 reference. Evidence SHALL bind source revision/dirty snapshot, executable versions/digests, native target, build profile/features, reference identity and TOON implementation/specification identity. Every mapped unique case, original 518 IDs and protected 303 baseline contracts SHALL remain accounted for. All primary JSON, compact JSON, default TOON and applicable sequence observations SHALL be captured; no changed-format candidate result SHALL reuse historical stdout or token counts. Promotion SHALL refresh all applicable generated manual sections, their unique-case index and current provenance/coverage summaries.

#### Scenario: Complete candidate execution
- **WHEN** the new format's manual comparison runs
- **THEN** every case in the current complete mapping is executed with missing mappings, skips, timeouts, crashes and malformed output retained as failures, not omitted

#### Scenario: Stale observations
- **WHEN** a saved 0.4.1 report is rendered with the new renderer
- **THEN** it remains historical evidence and MUST NOT be presented as regenerated 0.5.0 observations

#### Scenario: Build-profile identity
- **WHEN** candidate evidence is promoted to release documentation
- **THEN** it names the actual native release/default product and helper identities and does not substitute a debug or bench-profile executable

#### Scenario: Native target unavailable
- **WHEN** candidate comparison execution is unavailable on a supported native target
- **THEN** documentation marks that target unrenewed, preserves its historical record and does not claim release-wide or all-platform acceptance

### Requirement: Complete-stdout token accounting for format migration
Token savings SHALL be calculated independently for `o200k_base` and `cl100k_base` using ordinary-text tokenization over exact complete captured stdout, including terminating LF. Eligible samples SHALL require successful equivalent jq/JSON/TOON values and process contracts under existing gates. Raw, error-only, invalid-UTF-8, failed or non-equivalent observations SHALL NOT enter size totals. Default TOON totals SHALL exclude separately displayed sequence captures. Reports SHALL show JSON and TOON tokens, signed `TOON - JSON` differences and signed `(TOON - JSON) / JSON * 100` percentages, negative for savings and positive for growth; undefined zero-denominator percentages SHALL remain explicitly unavailable. Index totals SHALL count each unique case once despite section overlap. A reviewed baseline diff SHALL account for changed output, token counts and eligibility additions/removals; no minimum savings target SHALL conceal growth.

#### Scenario: Structural compression change
- **WHEN** a nested or keyed-table candidate emits different equivalent stdout under 4.1
- **THEN** both tokenizers count the new exact bytes and the review records their actual differences rather than copying old savings

#### Scenario: Terminal newline and special-marker text
- **WHEN** successful stdout ends in LF or contains a literal tokenizer-special-looking marker
- **THEN** LF participates in counting and the marker is counted as ordinary text rather than control syntax

#### Scenario: Eligibility regression
- **WHEN** a formerly eligible case fails TOON decoding or value/process equivalence
- **THEN** its token row is excluded with an explicit reason and its failed contract remains visible rather than disappearing from coverage

#### Scenario: Shared section case
- **WHEN** one case is linked by multiple section ledgers
- **THEN** every owning page may show it but aggregate token totals include it only once

### Requirement: Format migration preserves independent exact gates
Regeneration SHALL preserve independent primary JSON/process, compact-JSON byte/process and TOON ordered-value/process gates. Recursive table-header key reordering SHALL be accepted only where required by TOON 4.1; arrays, result cardinality/order, primitive types, required stderr, framing and complete stdout consumption SHALL remain protected. Reviewed disparities SHALL remain distinct from exact matches. A strict run with retained differences SHALL preserve its nonzero exit status even when complete reports are produced. TOON output updates SHALL NOT authorize blanket numeric tolerances, new disparity approvals, reference-pin changes, case removals or claims of calibrated performance.

#### Scenario: Existing exact difference
- **WHEN** a candidate retains a PR68 primary or compact exact mismatch
- **THEN** its actual observations and nonzero strict status remain explicit and correct TOON encoding does not turn it into an exact pass

#### Scenario: Nested table interpretation
- **WHEN** a decoder accepts a recursive header but materializes literal flat keys instead of nested objects
- **THEN** the TOON contract fails despite a successful parse and matching row cell count

#### Scenario: Result-boundary recovery
- **WHEN** default TOON emits several values including empty objects, empty arrays and keyed/nested tables
- **THEN** independently captured JSON results establish boundaries, each TOON value matches, and all stdout bytes are consumed in order

#### Scenario: Deferred ownership
- **WHEN** refreshed local evidence is published while platform or calibration follow-ups remain unresolved
- **THEN** those obligations and their actual native scopes remain explicit and are not counted as approvals or exact matches
