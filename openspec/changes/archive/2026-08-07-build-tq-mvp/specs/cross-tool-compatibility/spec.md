## ADDED Requirements

### Requirement: Data-driven compatibility cases
The compatibility suite SHALL define cases in machine-readable manifests rather than embedding expected behavior only in test code. Every case MUST have a stable identifier, category, capability tags, input reference, query text or per-tool query adapter, invocation mode, expected result classification, and MVP/deferred status.

#### Scenario: Add a common filter case
- **WHEN** a contributor adds a field-selection case
- **THEN** the manifest identifies the jq-like query, fixture, expected ordered result sequence, participating tools, and `common` classification

#### Scenario: Add a known divergence
- **WHEN** yq intentionally differs from jq for a case
- **THEN** the case is classified as `jq-target` or `cli` and records the yq observation without weakening the jq requirement for tq

### Requirement: Reference tool discovery and identity
The suite SHALL support configurable jq, yq, and tq executable paths. It MUST prefer explicit environment/configuration values, support the local `../jq` and `../yq` development trees when built, and record each executable's resolved path, version output, file digest, and relevant build features.

#### Scenario: Run local references
- **WHEN** built reference binaries exist in the configured neighboring repositories
- **THEN** the suite can execute them and records their exact identities in the baseline report

#### Scenario: Missing tq during baseline phase
- **WHEN** jq and yq are available but tq has not yet been implemented
- **THEN** the suite runs the reference baseline successfully and marks tq as `not-yet-participating` rather than failing the campaign

### Requirement: Reference precedence
The suite SHALL treat jq 1.8.x as the semantic reference for the jq-compatible JSON data model. yq 4.x SHALL be a compatibility peer for the common subset, but a yq divergence MUST NOT redefine tq semantics.

#### Scenario: All tools agree
- **WHEN** jq and yq produce equivalent ordered result sequences for a `common` case
- **THEN** tq is required to produce that same semantic sequence when the case becomes enabled for tq

#### Scenario: yq diverges
- **WHEN** jq and yq differ in value type, cardinality, ordering, error behavior, or exit status
- **THEN** the report preserves both observations and the case explicitly states whether tq follows jq or the capability is deferred

### Requirement: Result-sequence normalization
The suite SHALL normalize structured output into an envelope that preserves the number and order of results, each result's JSON-model type and value, raw-versus-structured mode, stdout bytes when relevant, stderr classification, and process exit status. Normalization MUST NOT sort objects, sort results, coerce strings to numbers, or discard duplicate outputs.

#### Scenario: Multiple structured results
- **WHEN** a filter emits three structured values
- **THEN** the normalized envelope contains exactly three values in emission order

#### Scenario: No result
- **WHEN** a filter evaluates to `empty`
- **THEN** the normalized envelope distinguishes zero results from one `null` result

#### Scenario: Raw output
- **WHEN** a case uses raw output
- **THEN** the suite preserves the emitted bytes and line/join behavior without parsing them as structured values

### Requirement: Error and exit compatibility
Compatibility cases SHALL exercise parse errors, compile errors, runtime type errors, optional suppression, missing paths, false/null exit-status behavior, no-output behavior, invalid CLI usage, and input parse failures. The suite MUST compare stable error classes and required source locations without requiring byte-identical prose unless a case explicitly says so.

#### Scenario: Compile error
- **WHEN** a query is syntactically invalid
- **THEN** the suite records a compile-error class, nonzero exit status, stderr, and available source span for each tool

#### Scenario: Exit-status mode with false
- **WHEN** a tool runs a filter whose last result is `false` under exit-status mode
- **THEN** the normalized observation records the jq-compatible false/null exit code separately from the value itself

### Requirement: Baseline-first execution
The initial compatibility milestone SHALL execute the complete jq/yq-applicable MVP case manifest before tq language implementation begins. Its report and observations MUST be reviewable and versioned as inputs to tq development.

#### Scenario: Establish initial baseline
- **WHEN** the compatibility harness and initial MVP cases are complete
- **THEN** jq and yq are run across the suite and their observations are stored before a tq parser or evaluator task is marked complete

#### Scenario: Baseline contains unexplained failure
- **WHEN** a reference case times out, crashes, or cannot be normalized
- **THEN** the case is investigated or explicitly classified before tq implementation uses it as a target

### Requirement: Controlled baseline updates
Changing a reference version or expected observation SHALL create a baseline diff. The suite MUST require explicit review to accept the new observation and MUST NOT provide an unreviewed bulk “bless all” path.

#### Scenario: Upgrade jq
- **WHEN** the configured jq version changes
- **THEN** the suite displays all changed outputs, errors, and exit statuses by case before a new baseline is accepted

#### Scenario: No semantic change
- **WHEN** a tool binary changes but all normalized observations remain equal
- **THEN** only the tool identity metadata changes and the report states that no case semantics changed

### Requirement: MVP compatibility coverage
The MVP suite SHALL cover identity, scalar and composite literals, field/index/slice access, missing and optional access, array/object iteration, pipes, comma generators, parentheses, array/object construction, conditionals, comparisons, boolean operators, alternative, arithmetic, variables, selected core built-ins, errors, raw output, and path updates. It SHALL exercise the same applicable logical cases through tq's TOON, JSON, and YAML input adapters, yq's JSON and YAML adapters, and jq's JSON input. Deferred jq features MUST have manifest entries or capability markers showing that they are unsupported rather than silently untested.

#### Scenario: Publish coverage
- **WHEN** a compatibility report is produced
- **THEN** it reports passing, divergent, unsupported, deferred, and untested counts grouped by capability

#### Scenario: Unsupported syntax reaches tq
- **WHEN** tq is given syntax classified as deferred
- **THEN** tq emits a compile-time unsupported-capability diagnostic and the compatibility case records that expected status

#### Scenario: Common JSON input
- **WHEN** a case belongs to the all-tools JSON subset
- **THEN** jq, yq, and tq consume the same JSON fixture and produce the required ordered result sequence

#### Scenario: Common YAML input
- **WHEN** a case belongs to the yq/tq YAML subset
- **THEN** yq and tq consume the same YAML fixture and any YAML-model divergence is preserved rather than normalized away

### Requirement: Compatibility report artifacts
Every suite run SHALL produce a human-readable summary and machine-readable report containing corpus identity, tool identities, case manifest revision, per-case observations, normalized diffs, duration, and final status.

#### Scenario: Compatibility failure status
- **WHEN** an enabled tq case differs from its jq target
- **THEN** the suite exits unsuccessfully and the report identifies the first semantic difference without discarding the remaining case results, whether invoked locally or by future automation
