## ADDED Requirements

### Requirement: Refreshable natural-source corpus
The benchmark system SHALL refresh benchmark inputs from documented public source URLs at the beginning of a benchmark campaign. It MUST preserve each source payload at its natural size and structure.

#### Scenario: Refresh USGS feeds
- **WHEN** a benchmark campaign refreshes the earthquake corpus
- **THEN** it fetches the natural `all_hour`, `all_day`, `all_week`, and `all_month` GeoJSON feeds without selecting, padding, repeating, or truncating features

#### Scenario: Refresh natural large input
- **WHEN** a benchmark campaign includes the large tier
- **THEN** it downloads the complete configured large source artifact and does not resize it to meet a nominal byte target

### Requirement: Initial corpus definitions
The initial corpus SHALL include the natural USGS earthquake feed family and a naturally large public GeoJSON file. The initial large-source manifest MUST identify the Microsoft Georgia building-footprint GeoJSON archive unless a reviewed manifest revision selects a replacement.

#### Scenario: List corpus
- **WHEN** the corpus manager lists configured datasets
- **THEN** it reports the source identifier, source URL, expected media/archive type, refresh policy, license/provenance reference, and intended benchmark campaign for every dataset

#### Scenario: Source naturally changes size
- **WHEN** a refreshed USGS file differs in byte size or feature count from a previous campaign
- **THEN** the new natural payload is accepted and recorded as a new snapshot rather than resized to match the previous payload

### Requirement: Snapshot provenance manifest
Every fetched source artifact SHALL have a machine-readable snapshot manifest containing its source identifier, resolved URL, retrieval timestamp in UTC, HTTP validators when available, compressed and uncompressed byte sizes, detected document shape, logical record count when computable, SHA-256 digest, media type, archive member name, and provenance/license reference.

#### Scenario: Successful fetch
- **WHEN** a source artifact is fetched and validated
- **THEN** the corpus manager writes a complete snapshot manifest before the artifact is admitted to benchmarks

#### Scenario: Source cannot be identified
- **WHEN** a fetched artifact lacks required provenance or its digest cannot be computed
- **THEN** the artifact is rejected from benchmark execution with a diagnostic naming the missing manifest fields

### Requirement: Cross-format materialization
The corpus manager SHALL generate JSON, YAML, and TOON representations of the same logical dataset outside the timed benchmark interval. YAML generation and validation SHALL prefer the actively maintained `yaml_serde` crate rather than deprecated `serde_yaml`. Generated representations MUST be validated against the source JSON data model before use.

#### Scenario: Generate native inputs
- **WHEN** a source snapshot is prepared
- **THEN** the corpus manager creates the configured native JSON, YAML, and TOON artifacts and records their byte sizes and SHA-256 digests

#### Scenario: Semantic validation
- **WHEN** a generated YAML or TOON artifact is decoded
- **THEN** its ordered JSON data model is semantically equivalent to the source snapshot, including primitive types, array order, object member order, and configured numeric fidelity

#### Scenario: Validate every tq input path
- **WHEN** a JSON, YAML, and TOON representation is admitted to a tq benchmark
- **THEN** tq's corresponding input adapter produces the same ordered runtime value and result digest for the benchmark query

#### Scenario: Conversion is lossy
- **WHEN** a generated representation changes a value, order, or supported numeric representation
- **THEN** the representation is rejected and no benchmark using it is run

### Requirement: Corpus integrity verification
The corpus manager SHALL verify archive integrity, decompression success, digest identity, parseability, and configured structural invariants before a benchmark campaign.

#### Scenario: Earthquake shape validation
- **WHEN** a USGS GeoJSON snapshot is verified
- **THEN** it contains a root GeoJSON feature collection with a feature array and the manifest records the feature count

#### Scenario: Corrupt cache
- **WHEN** a cached artifact digest differs from its snapshot manifest
- **THEN** the artifact is quarantined or replaced through a fresh fetch and is not used for a benchmark

### Requirement: External storage policy
Large source artifacts, generated YAML/TOON artifacts, temporary conversions, and benchmark caches SHALL remain outside Git tracking. The repository MAY retain small development snapshots in `examples/`, but benchmark results MUST distinguish those snapshots from freshly collected campaign data.

#### Scenario: Prepare large corpus
- **WHEN** the large corpus is downloaded and converted
- **THEN** no payload or generated large representation appears as a Git-tracked file

#### Scenario: Run smoke tests
- **WHEN** network access or large storage is unavailable
- **THEN** smoke tests may use checked-in `examples/` snapshots and label the campaign as `smoke`, not `refreshed` or `release`

### Requirement: Recollectable and attributable campaigns
The benchmark system SHALL support refreshed campaigns as the default and explicit frozen replay of an already recorded snapshot for investigation. Every result MUST identify the exact snapshot manifests it consumed.

#### Scenario: Default campaign
- **WHEN** a user starts a standard benchmark campaign without a frozen snapshot option
- **THEN** the corpus manager attempts to recollect the configured natural datasets before benchmarking

#### Scenario: Frozen diagnostic replay
- **WHEN** a user explicitly selects an existing snapshot manifest
- **THEN** the campaign reuses only artifacts matching that manifest and marks results as a frozen replay
