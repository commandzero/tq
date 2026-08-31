## ADDED Requirements

### Requirement: Issue 5 comparative performance objective
The benchmark suite SHALL include representative correctness-gated JSON workloads for the issue #5 builtin families. On a manifest-recorded host, the report SHALL evaluate a soft objective that tq's median wall-clock duration is no more than 2.0 times jq's and tq's median peak resident memory is no more than 1.5 times jq's. Each comparison MUST use the same input, equivalent query, output sink, warmup and sampling policy, and complete process invocation. A miss SHALL remain visible for investigation but MUST NOT fail implementation acceptance or override tq's own regression policy.

#### Scenario: Both objectives are met
- **WHEN** a representative tq workload has a median wall-time ratio at or below 2.0 and a median peak-RSS ratio at or below 1.5 relative to jq
- **THEN** the report marks both issue #5 objectives as met and retains the measured values and ratios

#### Scenario: Time objective is missed
- **WHEN** tq's median wall time exceeds 2.0 times jq's for a comparable workload
- **THEN** the report marks the soft time objective as missed without failing the correctness gate or the benchmark campaign

#### Scenario: Memory objective is missed
- **WHEN** tq's median peak RSS exceeds 1.5 times jq's for a comparable workload
- **THEN** the report marks the soft memory objective as missed without failing the correctness gate or the benchmark campaign

#### Scenario: Comparison is not valid
- **WHEN** correctness, host identity, corpus identity, output behavior, or a required metric differs or is unavailable
- **THEN** the report marks the objective not comparable rather than claiming it passed or failed

