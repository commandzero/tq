# Automatic Stream Planning Specification

## Purpose

Define sound pre-decoder plan selection, bounded event/subtree execution, and
observable retention for eligible jq-shaped queries.

## Requirements

### Requirement: Sound automatic plan selection
The system SHALL select transcode, event, subtree, document, whole-input, or
blocking plans before semantic decoder consumption and SHALL preserve jq result,
ordering, framing, and error semantics. Query analysis SHALL precede an
output-aware planning step so semantic identity can select transcode only for a
compatible input decoder and output writer. A bounded format probe MAY precede
selection when every probed byte is replayed unchanged to the selected decoder.

#### Scenario: Eligible identity transcode
- **WHEN** analysis proves semantic identity and the selected input and output formats support direct structural conversion
- **THEN** the planner produces a transcode plan without retaining the complete document

#### Scenario: Eligible projection
- **WHEN** analysis proves a query depends only on bounded event-local paths
- **THEN** the compiler produces an event plan without retaining the complete document

#### Scenario: Auto-detected decoder input
- **WHEN** an eligible query receives JSON or TOON through the default automatic input format
- **THEN** bounded probing and replay select the corresponding decoder-backed plan without requiring an input-format override

#### Scenario: Ineligible query
- **WHEN** a query requires whole-document, whole-input, mutation, or blocking state
- **THEN** the planner selects the corresponding retaining plan without speculative partial output

#### Scenario: Ineligible output
- **WHEN** an identity query requests an output mode or writer option unsupported by transcode
- **THEN** the planner selects the corresponding sound plan without speculative partial output from a rejected plan

### Requirement: Observable bounded retention
Automatic plans MUST expose their proof causes, retained working set, limits,
and high-water observations. Transcode plans MUST also expose their output
commitment mode, aggregate container preparation, and spool observations.

#### Scenario: Natural large regression
- **WHEN** an eligible query processes the versioned natural large corpus
- **THEN** compatibility passes and peak RSS stays within its manifest-aware regression gate

#### Scenario: Explain transcode fallback
- **WHEN** semantic identity cannot use transcode because of an output option
- **THEN** explanation names that option and reports the selected fallback plan before input consumption
