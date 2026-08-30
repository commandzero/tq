## ADDED Requirements

### Requirement: Parallel selected-decoding campaign
The benchmark suite SHALL compare one-worker and multi-worker tq execution for a correctness-approved blocking projection over the largest catalogued JSON input. Each row MUST record wall time, user CPU, system CPU, total CPU, peak RSS, effective worker count, and output digest.

#### Scenario: Multi-worker comparison
- **WHEN** the parallel selected decoder is benchmarked with one and fourteen effective workers on the recorded host
- **THEN** the report presents both measurements and their wall-time, CPU-time, and memory ratios

#### Scenario: Parallel candidate regresses
- **WHEN** multi-worker execution does not materially improve wall time or violates correctness or memory bounds
- **THEN** the report preserves the result and the optimization does not silently replace the serial path for that workload
