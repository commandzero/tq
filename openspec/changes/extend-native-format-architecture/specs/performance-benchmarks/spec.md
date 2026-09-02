## ADDED Requirements

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
