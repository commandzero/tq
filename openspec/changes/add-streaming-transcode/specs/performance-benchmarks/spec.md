## ADDED Requirements

### Requirement: Identity-transcode performance campaign
The benchmark suite SHALL include correctness-gated JSON-to-TOON identity cases
for wide root objects, nested objects, root arrays, nested arrays, scalar arrays,
and tabular candidates. Every case MUST compare transcode with a forced-document
baseline and record wall time, CPU time, peak RSS, output bytes, output commitment
mode, preparation high-water bytes, spool bytes, and time to first output byte
when output can commit incrementally.

#### Scenario: Cross-plan correctness gate
- **WHEN** an identity-transcode case is admitted for timing
- **THEN** transcode and forced-document output bytes and exit classifications match for the exact corpus snapshot

#### Scenario: Shape-sensitive report
- **WHEN** a campaign contains object-heavy and array-heavy inputs
- **THEN** the report presents each shape separately and includes preparation and spool observations beside time and RSS

#### Scenario: Unframed disk cost
- **WHEN** an unframed transcode result exceeds the in-memory publication threshold
- **THEN** the report includes temporary bytes written and replayed rather than presenting low RSS without its disk cost

### Requirement: Identity-transcode memory objectives
On the manifest-recorded Apple Silicon benchmark host, the accepted natural
`recovery` and `segments` identity cases SHALL each complete below 64 MiB peak
process RSS in direct TOON sequence mode. Array-heavy cases SHALL demonstrate
that peak RSS remains within the case's configured bounded-retention objective as
input size grows, while reports record proportional spool growth separately.
These campaigns SHALL remain reproducible and opt-in rather than required in CI.

#### Scenario: Wide-object memory gate
- **WHEN** the accepted natural `recovery` or `segments` identity case runs through direct sequence transcode on the recorded host
- **THEN** semantic correctness passes and peak process RSS is below 64 MiB

#### Scenario: Array scaling gate
- **WHEN** correctness-equivalent root-array inputs increase in element count beyond the in-memory preparation threshold
- **THEN** spool bytes may grow with input size but peak RSS stays within the configured case objective

#### Scenario: Document baseline remains visible
- **WHEN** transcode satisfies its memory objective but is slower than forced-document execution
- **THEN** the report retains both timings and does not hide the CPU or disk trade-off
