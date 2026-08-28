## ADDED Requirements

### Requirement: Hybrid blocking benchmark coverage
The benchmark catalogue SHALL include correctness-gated workloads in which a small projection from a large structured document feeds an order-sensitive blocking operator. Reports MUST distinguish document, single-thread hybrid, and configured multi-thread hybrid execution and capture wall time, user CPU, system CPU, total CPU, peak resident memory, and effective worker count.

#### Scenario: Large GeoJSON projected sort
- **WHEN** the large GeoJSON campaign evaluates a sort over projected feature metadata
- **THEN** jq, single-thread tq, and multi-thread tq run against the same source and result contract, and the report records each required timing and memory metric

#### Scenario: Compare hybrid with document baseline
- **WHEN** both hybrid and document execution are available for the same tq query
- **THEN** the report labels their retained-state guarantees and presents wall-time, CPU-time, and peak-memory differences without treating the hybrid plan as bounded-memory event execution

### Requirement: Optimizer-resistant performance cases
A benchmark intended to measure a blocking operator MUST make that operator's result observable under its correctness contract. The harness MUST record optimizer rewrites and MUST reject or relabel a sample when the measured operator was removed before execution.

#### Scenario: Cardinality ignores sorted order
- **WHEN** a candidate benchmark ends in `sort | length` and array length is its only observable result
- **THEN** the harness does not label it a blocking-sort measurement if explain output reports dead-sort elimination

#### Scenario: Sorted content is observed
- **WHEN** a blocking-sort benchmark passes its correctness gate
- **THEN** the expected result depends on sorted content and explain output confirms that the sort remained in the executed plan

