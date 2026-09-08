## ADDED Requirements

### Requirement: Complete pinned manual parity inventory
The suite SHALL version the jq manual source fingerprint, section inventory, source-to-case relationships, reference executable identity and build configuration, and a closure inventory for every baseline failure and policy difference. The initial inventory SHALL include all 518 referenced cases at commit `dcabecc`, all 251 published input/output examples, all 13 manual sections plus introduction, and all 68 fenced snippets. These numbers SHALL be lower bounds for coverage, not caps. Every documented runnable behavior SHALL have an executable case; prose-only explanations and incomplete syntax SHALL retain reviewed source notes with reasons rather than fabricated successful executions.

#### Scenario: A source example has no test
- **WHEN** a runnable source passage lacks an executable case or its case is disabled, deferred, or removed
- **THEN** the manual parity gate fails and identifies the source passage

#### Scenario: A manual function has several forms
- **WHEN** the manual documents several arities, argument forms, flags, or empty/error outcomes
- **THEN** the inventory accounts for those contracts individually, adding cases beyond the existing example count where needed

#### Scenario: A source snapshot changes
- **WHEN** a source fingerprint or reference build differs from the pinned inventory
- **THEN** the suite fails reference verification until the source and observation changes are explicitly reviewed

### Requirement: Manual parity is an execution gate
A strict manual campaign SHALL execute every admitted case and exit unsuccessfully for any behavioral mismatch, unexpected error, timeout, crash, missing reference, skipped execution, normalization failure, or missing coverage. No case SHALL pass merely because its mismatch is named an expected difference. Completion SHALL require all initial 198 failures and 15 policy-difference cases to satisfy their specified contracts, continued success of the initial 303 matches, and executed reference contracts for both initial arity-discrepancy probes. The campaign SHALL publish the denominator and passing count by section and platform, and SHALL distinguish source audit coverage from execution compatibility.

#### Scenario: An old policy rejects a supported filter
- **WHEN** the reference accepts a manual program but tq rejects it under a former numeric, regex, module, or I/O restriction
- **THEN** the strict campaign fails instead of accepting the former policy difference

#### Scenario: A report still contains one failure
- **WHEN** a manual run has any failing required contract
- **THEN** its command exits nonzero, preserves observations for the remaining cases, and cannot report 100% compatibility

#### Scenario: A historical regression appears
- **WHEN** a change fixes a listed gap but breaks a previously passing case
- **THEN** the release gate fails even if the net number of passing cases increases

### Requirement: Explicit output-mode and identity contracts
Structured manual programs SHALL compare jq with `tq -o json` for ordered JSON results and process behavior, and SHALL additionally compare compact JSON bytes using `jq -c` and `tq -c` where the contract is deterministic. The suite MUST NOT coerce values, drop results, apply blanket numeric tolerances, sort arrays, ignore required stderr, or change filters to conceal a mismatch. Whitespace and object member order SHALL be ignored only for semantic JSON comparisons. Query-level `tojson`, raw output, debug/stderr text, compact output, color, and framing contracts SHALL retain their observable bytes. Native TOON output SHALL be verified separately for representable results and MUST NOT contribute to a jq pass when JSON parity fails.

Only documented output-format selection, explicit strict JSON selection for input-parser conformance, controlled fixture locations, and matched environment/platform setup SHALL adapt an invocation. Normal valid JSON input cases SHALL also test automatic detection, so the strict override cannot conceal a detection regression. tq-specific allow flags or rewritten queries SHALL NOT be required to pass ordinary process-CLI cases. Program identity and help/build content SHALL use explicit tq contracts for actual product identity, documented option semantics, success status, and stream placement, not forged jq version strings or unconditional identity exemptions.

#### Scenario: Equivalent JSON with different layout
- **WHEN** jq and `tq -o json` emit equivalent JSON values in the same result order with matching process behavior
- **THEN** semantic comparison passes regardless of pretty-print indentation

#### Scenario: Compact output differs
- **WHEN** `jq -c` and `tq -c` produce different deterministic JSON bytes or separators
- **THEN** the compact-output contract fails even if parsed JSON values compare equal

#### Scenario: TOON remains the default
- **WHEN** a native-output test omits output selection
- **THEN** it verifies TOON framing and round-trip values separately and does not claim that plain tq has jq's default output representation

#### Scenario: Version reports the wrong product
- **WHEN** tq prints a jq version string instead of its own identity
- **THEN** its identity-contract test fails

### Requirement: Reference corrections do not suppress execution
Reference-text errors SHALL be recorded as provenance separately from a case's execution verdict. The imported manual text SHALL remain intact. Corrected reference expectations SHALL be reviewed against the pinned executable. `frexp` and `modf` SHALL retain both the documented invalid-arity probes and the valid zero-arity witnesses; the two existing whitespace corrections SHALL retain original and corrected source text. Such corrections SHALL NOT remove cases or count unexecuted cases as passing.

#### Scenario: Incorrect arity in the manual
- **WHEN** the pinned jq rejects the manual's `frexp/2` or `modf/2` signature
- **THEN** tq must match that invalid-arity contract and separately execute the valid `/0` witness, with the source correction recorded independently

### Requirement: Reproducible platform and resource scope
The parity campaign SHALL run on every advertised supported operating-system/architecture combination with a pinned jq build, matched C math environment, locale, timezone, environment variables, filesystem fixtures, and recorded resource limits. It SHALL cover POSIX shell, PowerShell and cmd invocation forms where applicable, Windows binary/newline behavior, terminal color detection, and observable unbuffered writes. An unavailable required platform check SHALL be unverified and SHALL block an unqualified cross-platform compatibility claim. Defined platform-conditional errors SHALL be tested against the reference rather than skipped.

#### Scenario: Math symbol is unavailable on a platform
- **WHEN** jq defines a documented function but raises a runtime availability error on the current platform
- **THEN** tq matches that defined availability behavior rather than returning a compile-time unknown-function error

#### Scenario: Tighter resource policy is tested
- **WHEN** a separate safety test intentionally lowers a resource limit below a case's requirement
- **THEN** it verifies bounded failure separately and does not count that failure as a jq compatibility pass

#### Scenario: Unbuffered process is still running
- **WHEN** a producer sends one input and waits before sending the next under `--unbuffered`
- **THEN** a process-level test observes the complete first output before producer completion, not merely the eventual combined stdout
