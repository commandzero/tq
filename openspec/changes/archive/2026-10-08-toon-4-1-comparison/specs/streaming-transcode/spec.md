## ADDED Requirements

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
