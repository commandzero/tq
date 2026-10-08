## ADDED Requirements

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
