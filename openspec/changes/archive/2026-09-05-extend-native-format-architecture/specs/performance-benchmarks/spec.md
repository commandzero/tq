## ADDED Requirements

### Requirement: Refactor performance and memory regression validation
The refactor SHALL be validated against a recorded pre-refactor tq release build on correctness-approved workloads supported by both builds. The soft target SHALL be less than a 10% increase in median wall-clock duration and less than a 10% increase in maximum observed peak RSS for each workload. Reports SHALL show both percentage changes independently; improvements in one metric or workload MUST NOT hide regressions in another. These targets SHALL remain separate from the jq and yq comparative objectives.

Baseline and candidate runs SHALL use the same host, toolchain, release-build settings, fixtures, equivalent query and output behavior, warm-up policy, and repeated-run counts. Reports SHALL identify both revisions, commands, fixture hashes, host details, individual samples, and summary measurements. Coverage SHALL include existing JSON, JSON Lines, JSON5, YAML, TOON, and TOON sequence paths, complete-Document and supported structural-event execution, remaining-input queries, and native output across multiple Results and sources. Small workloads SHALL expose per-call overhead; large Documents and long sequences SHALL expose buffering and retained-memory regressions. New formats without a baseline SHALL be reported separately, not counted as refactor passes.

Every authoritative benchmark run SHALL use `/usr/bin/time -l` outside the sandbox with elevated process-inspection permissions to collect wall-clock duration and `maximum resident set size`. Missing or invalid RSS SHALL invalidate the comparison and require a rerun; it MUST NOT count as passing memory validation.

Any increase above 25% in either metric for any workload MUST trigger investigation before verification is complete. Investigation SHALL repeat the paired measurement, assess noise, locate the cause when reproducible, and record supporting evidence, attempted mitigation, and remaining impact. A remaining regression above 25% MUST receive explicit user acceptance before completion. Increases at or above 10% SHALL remain visible as target misses even when they do not cross the investigation threshold.

#### Scenario: Both regression targets are met
- **WHEN** a valid paired workload shows time and peak-RSS increases each below 10%
- **THEN** the report marks both refactor targets met and retains the measurements

#### Scenario: Target miss below the investigation threshold
- **WHEN** either metric increases by at least 10% but neither increases by more than 25%
- **THEN** the report marks the affected target missed without hiding it in aggregate results

#### Scenario: Major regression requires investigation
- **WHEN** either metric increases by more than 25% for any workload
- **THEN** verification remains incomplete until the investigation is documented and the regression is mitigated to at most 25% or explicitly accepted by the user

#### Scenario: Baseline measurements are missing
- **WHEN** the candidate has no comparable pre-refactor timing and peak-RSS measurements for an existing workload
- **THEN** the report marks regression validation incomplete and requires measurements from the recorded baseline build

### Requirement: Native format comparative performance objective
The benchmark suite SHALL include correctness-gated RFC 7464 workloads comparable with jq and CSV or TSV workloads comparable with yq. On a manifest-recorded host, reports SHALL evaluate the soft objective that tq's median wall-clock duration is no more than 2.0 times the applicable reference and tq's maximum observed peak resident memory is no more than 1.5 times the reference. A miss MUST remain visible but MUST NOT fail implementation acceptance.

#### Scenario: JSON sequence objective is measured
- **WHEN** tq and jq process the same correctness-approved RFC 7464 workload with equivalent query and output behavior
- **THEN** the report records time and peak-RSS ratios and marks each soft objective met or missed

#### Scenario: Delimited objective is measured
- **WHEN** tq and yq process the same correctness-approved CSV or TSV workload with equivalent profile behavior
- **THEN** the report records comparable time and peak-RSS ratios without combining them with JSON sequence rankings

#### Scenario: RSS is unavailable
- **WHEN** a macOS run lacks the measured process's `maximum resident set size` from `/usr/bin/time -l` because it ran without required elevated process-inspection permission
- **THEN** the campaign is invalid and must be rerun with elevated permissions rather than marking the memory objective met or missed

#### Scenario: Soft objective is missed
- **WHEN** a valid comparable run exceeds either the 2.0 time ratio or the 1.5 peak-RSS ratio
- **THEN** the report retains the measurements and marks the objective missed without failing the correctness gate
