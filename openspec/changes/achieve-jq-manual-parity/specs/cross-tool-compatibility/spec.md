## ADDED Requirements

### Requirement: Reviewed safe-library disparities
The implementation SHALL prioritize safe Rust and existing Rust libraries over exact parity requiring unsafe code or native FFI dependencies. Every manual behavior SHALL remain in the coverage inventory. A measured library or platform limitation MAY be accepted as a documented disparity only with a reproducible witness, pinned reference/target observations, a cause, practical impact, regression test, and future reconsideration condition. Missing implementation, uninvestigated failures, skipped cases, crashes, and timeouts SHALL NOT qualify. Previously matching cases SHALL remain regression requirements. These scoped disparities qualify the matching requirements in this change; they do not count as exact matches.

The suite SHALL publish exact matches, reviewed disparities, and unresolved failures separately. Exact-parity mode SHALL continue to fail on any disparity. Completion mode MAY accept only explicitly reviewed disparities whose recorded observations still match the witness contract. Unknown, stale, or overly broad approvals SHALL fail validation. Numeric bounds SHALL be specific to a function and supported by boundary evidence, never a blanket tolerance. `docs/tests/jq-manual/coverage.md` SHALL summarize the differences; supporting metadata in `tests/compatibility/reviews/` SHALL retain evidence and post-implementation reconsideration criteria.

#### Scenario: A safe implementation differs from platform math
- **WHEN** a tested function produces a measured rounding difference within its reviewed bound
- **THEN** the report retains both values and records a disparity, not an exact match

#### Scenario: A dependency does not expose a jq feature
- **WHEN** existing safe Rust libraries demonstrably cannot reproduce a behavior within the chosen resource contract
- **THEN** the limitation has an executable witness and documented impact for reconsideration after full implementation, without introducing unsafe or FFI code

### Requirement: Complete pinned manual parity inventory
The suite SHALL pin jq 1.8.2 as the execution reference on each required native target, preserving earlier jq 1.8.1 observations as historical evidence rather than renewed acceptance or approvals. It SHALL version the jq 1.8 manual source fingerprint, section inventory, source-to-case relationships, reference executable identity and build configuration, and a closure inventory for every baseline failure and policy difference. The initial inventory SHALL include all 518 referenced cases at commit `dcabecc`, all 251 published input/output examples, all 13 manual sections plus introduction, and all 68 fenced snippets. These numbers SHALL be lower bounds for coverage, not caps. Every documented runnable behavior SHALL have an executable case; prose-only explanations and incomplete syntax SHALL retain reviewed source notes with reasons rather than fabricated successful executions.

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
Structured manual programs SHALL compare jq with `tq -o json` for ordered JSON results and process behavior, and SHALL additionally compare compact JSON bytes using `jq -c` and `tq -c` where the contract is deterministic. The suite MUST NOT coerce values, drop results, apply blanket numeric tolerances, sort arrays, ignore required stderr, or change filters to conceal a mismatch. Whitespace and object member order SHALL be ignored only for semantic JSON comparisons. Query-level `tojson`, raw output, debug/stderr text, compact output, color, and framing contracts SHALL retain their observable bytes. Color presentation SHALL follow the archived `output-colors` main spec, not jq's default palette or quote styling. The suite SHALL assert tq's palette/override/fallback and all-format contracts, including JSON Lines, and verify that removing only tq-generated SGR yields exactly the corresponding plain tq bytes. Explicit tq presentation differences SHALL remain separately identified rather than counted as exact jq ANSI matches or newly approved safe-library disparities; program-generated escapes and data/process observations SHALL NOT be normalized away. Native TOON output SHALL be verified separately for representable results and MUST NOT contribute to a jq pass when JSON parity fails.

Only documented output-format selection, explicit strict JSON selection for input-parser conformance, controlled fixture locations, and matched environment/platform setup SHALL adapt an invocation. Normal valid JSON input cases SHALL also test automatic detection, so the strict override cannot conceal a detection regression. tq-specific allow flags or rewritten queries SHALL NOT be required to pass ordinary process-CLI cases. Program identity and help/build content SHALL use explicit tq contracts for actual product identity, documented option semantics, success status, and stream placement, not forged jq version strings or unconditional identity exemptions.

#### Scenario: Equivalent JSON with different layout
- **WHEN** jq and `tq -o json` emit equivalent JSON values in the same result order with matching process behavior
- **THEN** semantic comparison passes regardless of pretty-print indentation

#### Scenario: Compact output differs
- **WHEN** `jq -c` and `tq -c` produce different deterministic JSON bytes or separators
- **THEN** the compact-output contract fails even if parsed JSON values compare equal

#### Scenario: TOON remains the default
- **WHEN** a native-output test omits output selection
- **THEN** it verifies LF-terminated canonical TOON values without an RS prefix and round-trip values separately and does not claim that plain tq has jq's default output representation

#### Scenario: Version reports the wrong product
- **WHEN** tq prints a jq version string instead of its own identity
- **THEN** its identity-contract test fails

### Requirement: Reference corrections do not suppress execution
Reference-text errors SHALL be recorded as provenance separately from a case's execution verdict. The imported manual text SHALL remain intact. Corrected reference expectations SHALL be reviewed against the pinned executable. `frexp` and `modf` SHALL retain both the documented invalid-arity probes and the valid zero-arity witnesses; the two existing whitespace corrections SHALL retain original and corrected source text. Such corrections SHALL NOT remove cases or count unexecuted cases as passing.

#### Scenario: Incorrect arity in the manual
- **WHEN** the pinned jq rejects the manual's `frexp/2` or `modf/2` signature
- **THEN** tq must match that invalid-arity contract and separately execute the valid `/0` witness, with the source correction recorded independently

### Requirement: Reproducible platform and resource scope
The parity campaign SHALL run on every advertised supported operating-system/architecture combination. Completion SHALL require fresh native jq 1.8.2 evidence on local macOS, ironhide Linux x86_64, and smokescreen Windows 11 Pro `x86_64-pc-windows-msvc`, with pinned reference executable/build and runtime identities, matched C math environment, locale, timezone, environment variables, filesystem fixtures, and recorded resource limits. Windows acceptance SHALL execute native Windows jq/tq binaries in native PowerShell; SSH into WSL, cross-compilation, and non-Windows PowerShell SHALL NOT qualify. The suite SHALL cover POSIX shell, applicable cmd forms, Windows binary/newline behavior, terminal color detection, and observable unbuffered writes. Missing required native execution or reference evidence SHALL remain unverified and SHALL block completion as well as a compatibility claim. Defined platform-conditional errors SHALL be tested against the reference rather than skipped. This requirement supersedes the earlier Windows deferral, including stale task and historical checkpoint wording.

Native Windows compatibility capture SHALL preserve exact stdout/stderr bytes, status, bounded timeout cleanup/reaping, and wall time with executable/target identity. Capture is implemented with overlapped named pipes and JobObjects and SHALL NOT itself count as acceptance evidence. Native Windows CPU/RSS accounting is implemented through a retained exact-child process handle using `GetProcessTimes` and `PeakWorkingSetSize`, with a surviving isolated worker. Validation remains incomplete: separate control executions vary in 15.625 ms CPU quanta, with observed differences up to 62.5 ms exceeding the unchanged 20 ms check. The suite SHALL NOT loosen that check or infer performance acceptance or issue #31 closure from backend implementation. Strict-report differences SHALL remain differences, not exact matches. The user disabled Smart App Control on development-only smokescreen and reported native launch verified with state `0`; no further security changes are needed, and launch success SHALL NOT establish acceptance. Compatibility captures, wall-time-only measurements, WSL results, or placeholder zeros SHALL NOT satisfy native CPU/RSS or final resource/performance evidence. Target-scoped disparities SHALL require fresh identity-bound review against jq 1.8.2; this revision SHALL NOT renew historical approvals or approve new math differences.

#### Scenario: Required native Windows evidence is missing
- **WHEN** native smokescreen PowerShell execution or its pinned jq 1.8.2 reference evidence is missing
- **THEN** Windows remains unverified and the change remains incomplete, with no passing execution or accepted disparity manufactured from the missing evidence

#### Scenario: SSH reaches WSL on the Windows host
- **WHEN** a campaign executes Linux binaries through SSH into WSL on smokescreen
- **THEN** it cannot satisfy native Windows acceptance regardless of the host's Windows installation

#### Scenario: Native CPU/RSS backend validation is incomplete
- **WHEN** the implemented native Windows backend records exact-child CPU/RSS but separate control executions differ by up to 62.5 ms in 15.625 ms CPU quanta, exceeding the unchanged 20 ms check
- **THEN** observations retain their own verdicts, validation and performance acceptance remain incomplete, and issue #31 remains open without a relaxed check

#### Scenario: Math symbol is unavailable on a platform
- **WHEN** jq defines a documented function but raises a runtime availability error on the current platform
- **THEN** tq matches that defined availability behavior rather than returning a compile-time unknown-function error

#### Scenario: Tighter resource policy is tested
- **WHEN** a separate safety test intentionally lowers a resource limit below a case's requirement
- **THEN** it verifies bounded failure separately and does not count that failure as a jq compatibility pass

#### Scenario: Unbuffered process is still running
- **WHEN** a producer sends one input and waits before sending the next under `--unbuffered`
- **THEN** a process-level test observes the complete first output before producer completion, not merely the eventual combined stdout
