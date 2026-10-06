## ADDED Requirements

### Requirement: Reviewed safe-library disparities
The implementation SHALL prioritize safe Rust and existing Rust libraries over exact parity requiring unsafe code or native FFI dependencies. Every manual behavior SHALL remain in the coverage inventory. A measured library or platform limitation MAY be accepted as a documented disparity only with a reproducible witness, pinned reference/target observations, a cause, practical impact, regression test, and future reconsideration condition. Missing implementation, uninvestigated failures, skipped cases, crashes, and timeouts SHALL NOT qualify. Previously matching cases SHALL remain regression requirements. These scoped disparities qualify the matching requirements in this change; they do not count as exact matches. The separate implementation/evidence closeout requirement below transfers only enumerated unresolved target/case contracts; transfer is not disparity approval and does not satisfy either acceptance gate.

The suite SHALL publish exact matches, reviewed disparities, and unresolved failures separately. Exact-parity mode SHALL continue to fail on any disparity. Reviewed-completion mode MAY accept only explicitly reviewed disparities whose recorded observations still match the witness contract. Unknown, stale, or overly broad approvals SHALL fail validation. Numeric bounds SHALL be specific to a function and supported by boundary evidence, never a blanket tolerance. `docs/tests/jq-manual/coverage.md` SHALL summarize the differences; supporting metadata in `tests/compatibility/reviews/` SHALL retain evidence and post-implementation reconsideration criteria.

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
A strict manual campaign SHALL execute every admitted case and exit unsuccessfully for any behavioral mismatch, unexpected error, timeout, crash, missing reference, skipped execution, normalization failure, or missing coverage. No case SHALL pass merely because its mismatch is named an expected difference. Reviewed-completion acceptance SHALL require all initial 198 failures and 15 policy-difference cases to satisfy their specified contracts, continued success of the initial 303 matches, and executed reference contracts for both initial arity-discrepancy probes. The campaign SHALL publish the denominator and passing count by section and platform, and SHALL distinguish source audit coverage from execution compatibility.

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

### Requirement: Implementation and evidence closeout with enumerated follow-ups
Change closeout SHALL be distinct from exact manual acceptance and reviewed-completion acceptance. The user confirmed transfer of the target/case contracts below to Linux issue #70 and Windows issue #69. Every case SHALL remain executable, source-mapped, and present in reports with its actual observations and verdict. A transferred difference SHALL remain unresolved/unapproved unless independently fixed or approved through the existing reviewed-disparity contract; issue ownership SHALL NOT change passing counts, renew approvals, normalize bytes or values, or make either gate pass. No other target/case contract is transferred by this requirement.

The authoritative witness record is `tests/compatibility/reviews/native-platform-acceptance.toon`, snapshot `implementation-v6`, bound to base `3605d2de42a9099d67b5f6dc758bf5e030ad7c75` plus its recorded 59-file overlay, not automatically to the current worktree. Its jq 1.8.2 executable identities, raw evidence hashes, and `difference_groups` identify these contracts. Primary/compact campaigns overlap: the unique difference counts are 9 macOS, 15 Linux, and 60 Windows; each strict command exits 1. The map below transfers all 15 Linux and 60 Windows IDs, not macOS compatibility differences or every future difference on those targets.

| Target / issue | Contract group | Exact case IDs |
| --- | --- | --- |
| Linux x86_64 / #70 | Input-specific rounding (six witnesses) | `manual.audit.math.erfc-ulp`, `manual.audit.math.tgamma-ulp`, `manual.composition.arity.y0.0`, `manual.composition.arity.yn.2`, `manual.math.y0`, `manual.math.yn` |
| Linux x86_64 / #70 | Non-rounding scale conversion | `manual.audit.math.integer-scale-boundary`, `manual.composition.arity.scalbln.2` |
| Linux x86_64 / #70 | Color presentation | `manual.colors.ansi-values`, `manual.colors.custom-1-31`, `manual.colors.default-palette`, `manual.invoking.color-output`, `manual.invoking.jq-colors`, `manual.invoking.no-color-forced` |
| Linux x86_64 / #70 | Internal test-runner messages | `manual.invoking.run-tests` |
| Native Windows x86_64 MSVC / #69 | Input-specific rounding (six witnesses) | `manual.audit.math.erfc-ulp`, `manual.audit.math.tgamma-ulp`, `manual.composition.arity.y0.0`, `manual.composition.arity.yn.2`, `manual.math.y0`, `manual.math.yn` |
| Native Windows x86_64 MSVC / #69 | Non-rounding scale conversion | `manual.audit.math.integer-scale-boundary`, `manual.composition.arity.scalbln.2` |
| Native Windows x86_64 MSVC / #69 | Pinned reference function availability | `manual.audit.math.scalb-infinite-boundary`, `manual.composition.arity.drem.2`, `manual.composition.arity.exp10.0`, `manual.composition.arity.gamma.0`, `manual.composition.arity.scalb.2`, `manual.composition.arity.significand.0`, `manual.math.drem`, `manual.math.exp10`, `manual.math.gamma`, `manual.math.scalb`, `manual.math.significand` |
| Native Windows x86_64 MSVC / #69 | Raw-input / CLI / streaming newline bytes (32 witnesses) | `manual.audit.streaming.json-root-boundary-auto`, `manual.audit.streaming.json-root-boundary-explicit`, `manual.audit.streaming.json-root-boundary-hybrid`, `manual.audit.streaming.json-root-boundary-nested-empty`, `manual.audit.streaming.json-root-boundary-object-order`, `manual.audit.streaming.json-root-boundary-prefix-reset`, `manual.audit.streaming.json-root-boundary-prior-valid`, `manual.audit.streaming.malformed-eof-array-stream-error`, `manual.audit.streaming.malformed-eof-object-stream-error`, `manual.audit.streaming.malformed-eof-string-stream-error`, `manual.audit.streaming.malformed-missing-separator-stream-error`, `manual.audit.streaming.malformed-plain-partial-output`, `manual.audit.streaming.malformed-trailing-comma-stream-error`, `manual.composition.arity.input.0`, `manual.composition.arity.inputs.0`, `manual.invoking.ascii-escape`, `manual.invoking.compact-output`, `manual.invoking.exit-false`, `manual.invoking.exit-null`, `manual.invoking.exit-true`, `manual.invoking.indent-output`, `manual.invoking.monochrome-output`, `manual.invoking.raw-input`, `manual.invoking.raw-output`, `manual.invoking.seq`, `manual.invoking.sort-keys`, `manual.invoking.tab-output`, `manual.io.input`, `manual.io.input-null-input`, `manual.io.inputs`, `manual.runtime.continuation.raw-lines`, `manual.streaming.stream-errors-form` |
| Native Windows x86_64 MSVC / #69 | Color presentation plus native newline effects | `manual.colors.ansi-values`, `manual.colors.custom-1-31`, `manual.colors.default-palette`, `manual.invoking.color-output`, `manual.invoking.jq-colors`, `manual.invoking.no-color-forced` |
| Native Windows x86_64 MSVC / #69 | Internal test-runner messages plus native newline effects | `manual.invoking.run-tests` |
| Native Windows x86_64 MSVC / #69 | Native module lookup | `manual.modules.path-origin`, `manual.modules.path-tilde` |

Scale-conversion witnesses compare jq `[0,M,0,0,0]` with tq `[M,M,0,2,2]`, where `M` is the maximum finite binary64 value; these SHALL NOT be classified as ULP rounding. Pinned Windows jq function absence SHALL remain reference-build availability evidence, not a missing tq implementation or a successful match. Color presentation SHALL retain the archived `output-colors` contract without claiming exact jq ANSI bytes or new safe-library approvals. Target-specific practical impact, regression coverage, and reconsideration conditions SHALL accompany the follow-up reconciliation.

Calibrated performance acceptance SHALL be deferred under #31 with #69/#70 platform follow-ups. The v6 record establishes no fresh calibration on any platform. Native Windows backend implementation and same-child counter checks SHALL NOT close #31 or establish calibration. Unfinished macOS calibration and affected benchmark comparisons SHALL remain explicitly unpublished as accepted performance evidence; #31's Windows scope SHALL NOT be treated as proof of macOS acceptance. No tolerance relaxation, placeholder counter, sampled-memory substitution, or reuse of changed-source calibration is authorized. Correctness and bounded-resource regression evidence remain closeout obligations.

Shared full-campaign harness debt is transferred to #69/#70: `environment.denied` and `platform.denied` are embedded-denial fixtures routed through the permissive CLI, with three recorded `platform.denied` malformed-output errors for JSON/YAML/TOON companion executions. This is harness debt, not a product disparity or a full-campaign pass. Complete Linux validator provisioning/full preflight and automated native Windows release-host wiring remain platform follow-ups; their absent checks SHALL NOT be represented as executed passes.

Closeout SHALL require final-candidate source/executable/profile reconciliation or refreshed native campaigns where identity cannot be proved, complete original 518-case and added-witness accounting, continued verification of the 303 protected baseline contracts without silently changing the pin or gate, complete CLI/embedded composition and bounded-resource regressions, truthful documentation, and ownership of every remaining obligation. MacOS's nine differences are not transferred here: the two math witnesses require current identity-bound review, the six color witnesses require existing tq presentation-contract verification, and `manual.invoking.run-tests` requires diagnostic-contract resolution or a separately reviewed scope decision before closeout. A successful build, task count, scope transfer, or OpenSpec syntax validation SHALL NOT establish manual acceptance. Final implementation verification, synchronization, and archival SHALL occur only after the retained closeout obligations have proof.

#### Scenario: A recorded Linux or Windows contract is transferred
- **WHEN** a target/case pair in the map remains different and is assigned to its named issue with source-bound evidence
- **THEN** the closeout records its unresolved verdict and owner without approving it or making exact or reviewed-completion mode pass

#### Scenario: An unmapped failure or missing execution appears
- **WHEN** a refreshed campaign introduces an unowned difference, regression, missing case, skip, timeout, crash, or missing reference
- **THEN** the transfer does not cover it and closeout remains incomplete until it is resolved or separately reviewed within scope

#### Scenario: Calibration remains unfinished
- **WHEN** v6 or later observations lack current calibrated performance acceptance, including macOS evidence
- **THEN** performance acceptance remains unpublished and #31 stays open; correctness and resource regressions may be verified independently

### Requirement: Reproducible platform and resource scope
The parity campaign SHALL run on every advertised supported operating-system/architecture combination. Native acceptance SHALL require fresh native jq 1.8.2 evidence on local macOS, ironhide Linux x86_64, and smokescreen Windows 11 Pro `x86_64-pc-windows-msvc`, with pinned reference executable/build and runtime identities, matched C math environment, locale, timezone, environment variables, filesystem fixtures, and recorded resource limits. Windows acceptance SHALL execute native Windows jq/tq binaries in native PowerShell; SSH into WSL, cross-compilation, and non-Windows PowerShell SHALL NOT qualify. The suite SHALL cover POSIX shell, applicable cmd forms, Windows binary/newline behavior, terminal color detection, and observable unbuffered writes. Missing required native execution or reference evidence SHALL remain unverified and SHALL block native acceptance as well as a compatibility claim. Implementation/evidence closeout SHALL still require source-bound native campaign execution and reference evidence on the three named hosts, with unresolved acceptance restricted to the explicit follow-up map below. Defined platform-conditional errors SHALL be tested against the reference rather than skipped. Native execution remains required; the confirmed implementation/evidence closeout scope below supersedes earlier wording that made all-target acceptance and calibrated performance acceptance prerequisites for archiving this change. It does not waive missing execution, reference identity, or unowned failures.

Native Windows compatibility capture SHALL preserve exact stdout/stderr bytes, status, bounded timeout cleanup/reaping, and wall time with executable/target identity. Capture is implemented with overlapped named pipes and JobObjects and SHALL NOT itself count as acceptance evidence. Native Windows CPU/RSS accounting is implemented through a retained exact-child process handle using `GetProcessTimes` and `PeakWorkingSetSize`, with a surviving isolated worker. Validation remains incomplete: separate control executions vary in 15.625 ms CPU quanta, with observed differences up to 62.5 ms exceeding the unchanged 20 ms check. The suite SHALL NOT loosen that check or infer performance acceptance or issue #31 closure from backend implementation. Strict-report differences SHALL remain differences, not exact matches. The user disabled Smart App Control on development-only smokescreen and reported native launch verified with state `0`; no further security changes are needed, and launch success SHALL NOT establish acceptance. Compatibility captures, wall-time-only measurements, WSL results, or placeholder zeros SHALL NOT satisfy native CPU/RSS or final resource/performance acceptance evidence. Calibration and calibrated performance acceptance are follow-up obligations under the closeout requirement below, not prerequisites for this change's revised closeout; configured resource-limit and bounded-execution regressions remain required. Target-scoped disparities SHALL require fresh identity-bound review against jq 1.8.2; this revision SHALL NOT renew historical approvals or approve new math differences.

#### Scenario: Required native Windows evidence is missing
- **WHEN** native smokescreen PowerShell execution or its pinned jq 1.8.2 reference evidence is missing
- **THEN** Windows remains unverified and implementation/evidence closeout remains incomplete, with no passing execution or accepted disparity manufactured from the missing evidence

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
