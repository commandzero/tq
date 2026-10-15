## ADDED Requirements

### Requirement: Version-bound native release completion
Native release acceptance MAY use reviewed-completion mode only with explicitly reviewed, tracked approvals for the candidate version and required native target. Each approval SHALL bind the exact executable case fingerprint, declared contract, difference summary, jq reference identity, tq identity, primary observations, applicable compact observations, and TOON contract. Exact-parity mode SHALL remain independent and fail on every retained difference. Reports SHALL retain separate exact-match, reviewed-disparity, and unapproved-failure counts; completion acceptance SHALL NOT be described as universal jq parity.

Approvals SHALL record a demonstrated cause, practical impact, regression coverage, and reconsideration condition. Safe-library rounding SHALL be input/target/identity-specific, never a global numeric tolerance. Intentional presentation and internal-diagnostic release exceptions MAY be reviewed separately against tq's established contracts; they SHALL NOT become exact jq matches or safe-library approvals, and raw captured bytes SHALL remain unchanged. Missing or incorrect implementations that can be corrected safely, changed supplied-program results, reordered or missing values, incorrect status/stderr, skipped cases, crashes, timeouts, missing coverage, and unapproved differences SHALL block completion. The original protected baseline contracts SHALL remain exact regression requirements.

The release campaign SHALL select approvals using the candidate package version and native target and fail closed when no reviewed file exists. Every required native manual target SHALL execute on the same pinned host class used for its packaging recipe. Native archive validation SHALL still run for the complete advertised archive matrix. An unavailable target or reference SHALL NOT inherit another target's approval or be reported as validated. Compiler, dependency, implementation, reference, runtime-library, executable-byte, case, or observation changes SHALL require renewed exact-evidence review before acceptance.

#### Scenario: Explicit completion with scoped exceptions
- **WHEN** every admitted case executes and only exact identity-bound reviewed exceptions remain different
- **THEN** completion succeeds while publishing separate exact-match and reviewed-disparity counts and preserving all raw evidence
- **AND** an exact-parity run of those observations still fails

#### Scenario: Unapproved or stale evidence
- **WHEN** a required observation differs without approval, an approval's executable digest or case fingerprint changes, or the version/target approval file is missing
- **THEN** release completion fails rather than regenerating approvals, broadening them, normalizing bytes, or silently substituting another target

#### Scenario: Presentation exception preserves data and effects
- **WHEN** an intentional quote-style/SGR boundary or internal-runner diagnostic difference is reviewed for release
- **THEN** its supplied-program data, result order/count, status, stderr, framing, and tq presentation contract remain protected and it is not counted as exact jq output or safe-library parity

#### Scenario: Correctable conversion defect
- **WHEN** safe target-specific argument conversion can reproduce the pinned native exponent-conversion contract
- **THEN** the implementation is corrected and freshly verified instead of approving the incorrect result as a rounding disparity

#### Scenario: Native source and package matrix
- **WHEN** a version is proposed for publication
- **THEN** required jq-manual targets execute with reviewed reference identities on the pinned native packaging host classes, every advertised archive passes its own native checks, and no unverified target is represented as accepted
