# Cross-Tool Compatibility Specification

## Purpose

Define data-driven jq, yq, and tq compatibility evidence, normalization,
baseline review, capability coverage, and report requirements.

## Requirements

### Requirement: Data-driven compatibility cases
The compatibility suite SHALL define cases in machine-readable manifests rather than embedding expected behavior only in test code. Every case MUST have a stable identifier, category, capability tags, input reference, query text or per-tool query adapter, invocation mode, expected result classification, and MVP/deferred status.

#### Scenario: Add a common filter case
- **WHEN** a contributor adds a field-selection case
- **THEN** the manifest identifies the jq-like query, fixture, expected ordered result sequence, participating tools, and `common` classification

#### Scenario: Add a known divergence
- **WHEN** yq intentionally differs from jq for a case
- **THEN** the case is classified as `jq-target` or `cli` and records the yq observation without weakening the jq requirement for tq

### Requirement: Reference tool discovery and identity
The suite SHALL support configurable jq, yq, and tq executable paths. It MUST prefer explicit environment/configuration values, support the local `../jq` and `../yq` development trees when built, and record each executable's resolved path, version output, file digest, and relevant build features.

#### Scenario: Run local references
- **WHEN** built reference binaries exist in the configured neighboring repositories
- **THEN** the suite can execute them and records their exact identities in the baseline report

#### Scenario: Missing tq during baseline phase
- **WHEN** jq and yq are available but tq has not yet been implemented
- **THEN** the suite runs the reference baseline successfully and marks tq as `not-yet-participating` rather than failing the campaign

### Requirement: Reference precedence
The suite SHALL treat jq 1.8.x as the semantic reference for the jq-compatible JSON data model. yq 4.x SHALL be a compatibility peer for the common subset, but a yq divergence MUST NOT redefine tq semantics.

#### Scenario: All tools agree
- **WHEN** jq and yq produce equivalent ordered result sequences for a `common` case
- **THEN** tq is required to produce that same semantic sequence when the case becomes enabled for tq

#### Scenario: yq diverges
- **WHEN** jq and yq differ in value type, cardinality, ordering, error behavior, or exit status
- **THEN** the report preserves both observations and the case explicitly states whether tq follows jq or the capability is deferred

### Requirement: Result-sequence normalization
The suite SHALL normalize structured output into an envelope that preserves the number and order of results, each result's JSON-model type and value, raw-versus-structured mode, stdout bytes when relevant, stderr classification, and process exit status. Normalization MUST NOT sort objects, sort results, coerce strings to numbers, or discard duplicate outputs.

#### Scenario: Multiple structured results
- **WHEN** a filter emits three structured values
- **THEN** the normalized envelope contains exactly three values in emission order

#### Scenario: No result
- **WHEN** a filter evaluates to `empty`
- **THEN** the normalized envelope distinguishes zero results from one `null` result

#### Scenario: Raw output
- **WHEN** a case uses raw output
- **THEN** the suite preserves the emitted bytes and line/join behavior without parsing them as structured values

### Requirement: Error and exit compatibility
Compatibility cases SHALL exercise parse errors, compile errors, runtime type errors, optional suppression, missing paths, false/null exit-status behavior, no-output behavior, invalid CLI usage, and input parse failures. The suite MUST compare stable error classes and required source locations without requiring byte-identical prose unless a case explicitly says so.

#### Scenario: Compile error
- **WHEN** a query is syntactically invalid
- **THEN** the suite records a compile-error class, nonzero exit status, stderr, and available source span for each tool

#### Scenario: Exit-status mode with false
- **WHEN** a tool runs a filter whose last result is `false` under exit-status mode
- **THEN** the normalized observation records the jq-compatible false/null exit code separately from the value itself

### Requirement: Baseline-first execution
The initial compatibility milestone SHALL execute the complete jq/yq-applicable MVP case manifest before tq language implementation begins. Its report and observations MUST be reviewable and versioned as inputs to tq development.

#### Scenario: Establish initial baseline
- **WHEN** the compatibility harness and initial MVP cases are complete
- **THEN** jq and yq are run across the suite and their observations are stored before a tq parser or evaluator task is marked complete

#### Scenario: Baseline contains unexplained failure
- **WHEN** a reference case times out, crashes, or cannot be normalized
- **THEN** the case is investigated or explicitly classified before tq implementation uses it as a target

### Requirement: Controlled baseline updates
Changing a reference version or expected observation SHALL create a baseline diff. The suite MUST require explicit review to accept the new observation and MUST NOT provide an unreviewed bulk “bless all” path.

#### Scenario: Upgrade jq
- **WHEN** the configured jq version changes
- **THEN** the suite displays all changed outputs, errors, and exit statuses by case before a new baseline is accepted

#### Scenario: No semantic change
- **WHEN** a tool binary changes but all normalized observations remain equal
- **THEN** only the tool identity metadata changes and the report states that no case semantics changed

### Requirement: MVP compatibility coverage
The MVP suite SHALL cover identity, scalar and composite literals, field/index/slice access, missing and optional access, array/object iteration, pipes, comma generators, parentheses, array/object construction, conditionals, comparisons, boolean operators, alternative, arithmetic, variables, selected core built-ins, errors, raw output, and path updates. It SHALL exercise the same applicable logical cases through tq's TOON, JSON, and YAML input adapters, yq's JSON and YAML adapters, and jq's JSON input. Deferred jq features MUST have manifest entries or capability markers showing that they are unsupported rather than silently untested.

#### Scenario: Publish coverage
- **WHEN** a compatibility report is produced
- **THEN** it reports passing, divergent, unsupported, deferred, and untested counts grouped by capability

#### Scenario: Unsupported syntax reaches tq
- **WHEN** tq is given syntax classified as deferred
- **THEN** tq emits a compile-time unsupported-capability diagnostic and the compatibility case records that expected status

#### Scenario: Common JSON input
- **WHEN** a case belongs to the all-tools JSON subset
- **THEN** jq, yq, and tq consume the same JSON fixture and produce the required ordered result sequence

#### Scenario: Common YAML input
- **WHEN** a case belongs to the yq/tq YAML subset
- **THEN** yq and tq consume the same YAML fixture and any YAML-model divergence is preserved rather than normalized away

### Requirement: Compatibility report artifacts
Every suite run SHALL produce a human-readable summary and machine-readable report containing corpus identity, tool identities, case manifest revision, per-case observations, normalized diffs, duration, and final status.

#### Scenario: Compatibility failure status
- **WHEN** an enabled tq case differs from its jq target
- **THEN** the suite exits unsuccessfully and the report identifies the first semantic difference without discarding the remaining case results, whether invoked locally or by future automation

### Requirement: Issue 5 built-in compatibility coverage
The compatibility manifest SHALL contain jq-target cases for every built-in added by issue #5. The cases MUST cover successful values, output order and cardinality, supported arities, type or path errors, empty inputs, short-circuit behavior, and applicable resource limits. tq support MUST be enabled only after its normalized observations match the reviewed jq 1.8.x baseline, except for differences already permitted by the project compatibility policy.

#### Scenario: Every added built-in has a case
- **WHEN** the compatibility manifest coverage test runs
- **THEN** it finds at least one enabled tq case for each issue #5 built-in and its supported arity

#### Scenario: Generator cardinality differs
- **WHEN** an added generator emits too many, too few, or reordered results compared with jq
- **THEN** the compatibility campaign fails and identifies the first differing result

#### Scenario: Error classification differs
- **WHEN** tq accepts an invalid value that jq rejects or reports a different stable error class
- **THEN** the compatibility campaign records the mismatch rather than treating the filter as compatible

### Requirement: Native format compatibility coverage
The compatibility manifest SHALL include jq-target RFC 7464 cases and yq-peer CSV and TSV cases. Coverage MUST compare ordered document values, output bytes where framing or quoting matters, warnings, stable error classes, and exit status without coercing string and numeric values during normalization.

#### Scenario: jq JSON sequence recovery
- **WHEN** a JSON sequence fixture contains valid, malformed, and valid frames
- **THEN** the jq and tq observations compare emitted values, warning classification, continuation, and successful exit status

#### Scenario: JSON sequence output bytes
- **WHEN** jq and tq emit multiple results under sequence output
- **THEN** the compatibility report compares RS prefixes, JSON payloads, LF terminators, and result order

#### Scenario: JSON sequence publication before failure
- **WHEN** jq and tq process a complete value followed by invalid trailing bytes or partial structural events followed by malformed syntax
- **THEN** the suite compares observations published before failure, recovery, root reset, and both warning and `--stream-errors` behavior

#### Scenario: Multiple roots share a recovery segment
- **WHEN** jq and tq receive several valid roots between RS bytes, with a separate case containing malformed syntax before a later root in the same segment
- **THEN** the suite compares all input modes, per-Document root reset, and the pinned jq parser's same-segment reset behavior rather than assuming every failure skips to RS

#### Scenario: Input advancement policy differs by caller
- **WHEN** malformed sequence input is read by top-level advancement, `input`, `inputs`, or a query with `try/catch`
- **THEN** the suite compares warnings, query-visible errors, prior results, continuation, and exit status separately for each caller

#### Scenario: Stream projection shares the query cursor
- **WHEN** `--stream` or `--stream-errors` is combined with `-n`, slurp, `input`, `inputs`, or `try/catch`
- **THEN** jq and tq observations compare ordinary query evaluation over the projected records, shared cursor advancement, prior results, and failure handling

#### Scenario: CSV and TSV peer cases
- **WHEN** a delimited fixture exercises quoted strings, typed unquoted scalars, empty fields, and quoted newlines
- **THEN** the report preserves tq and yq observations and records any deliberate profile difference explicitly

#### Scenario: Deferred formats remain visible
- **WHEN** the format coverage report is generated
- **THEN** XML, TOML, Properties, and INI are marked as follow-on work while HCL and Lua remain deferred

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

### Requirement: Release-bound TOON 4.1 manual regeneration
The TOON 4.1 release candidate SHALL regenerate the complete jq-manual comparison from fresh execution of release/default `tq` 0.5.0 product and helper binaries against the unchanged pinned jq 1.8.2 reference. Evidence SHALL bind source revision/dirty snapshot, executable versions/digests, native target, build profile/features, reference identity and TOON implementation/specification identity. Every mapped unique case, original 518 IDs and protected 303 baseline contracts SHALL remain accounted for. All primary JSON, compact JSON, default TOON and applicable sequence observations SHALL be captured; no changed-format candidate result SHALL reuse historical stdout or token counts. Promotion SHALL refresh all applicable generated manual sections, their unique-case index and current provenance/coverage summaries.

#### Scenario: Complete candidate execution
- **WHEN** the new format's manual comparison runs
- **THEN** every case in the current complete mapping is executed with missing mappings, skips, timeouts, crashes and malformed output retained as failures, not omitted

#### Scenario: Stale observations
- **WHEN** a saved 0.4.1 report is rendered with the new renderer
- **THEN** it remains historical evidence and MUST NOT be presented as regenerated 0.5.0 observations

#### Scenario: Build-profile identity
- **WHEN** candidate evidence is promoted to release documentation
- **THEN** it names the actual native release/default product and helper identities and does not substitute a debug or bench-profile executable

#### Scenario: Native target unavailable
- **WHEN** candidate comparison execution is unavailable on a supported native target
- **THEN** documentation marks that target unrenewed, preserves its historical record and does not claim release-wide or all-platform acceptance

### Requirement: Complete-stdout token accounting for format migration
Token savings SHALL be calculated independently for `o200k_base` and `cl100k_base` using ordinary-text tokenization over exact complete captured stdout, including terminating LF. Eligible samples SHALL require successful equivalent jq/JSON/TOON values and process contracts under existing gates. Raw, error-only, invalid-UTF-8, failed or non-equivalent observations SHALL NOT enter size totals. Default TOON totals SHALL exclude separately displayed sequence captures. Reports SHALL show JSON and TOON tokens, signed `TOON - JSON` differences and signed `(TOON - JSON) / JSON * 100` percentages, negative for savings and positive for growth; undefined zero-denominator percentages SHALL remain explicitly unavailable. Index totals SHALL count each unique case once despite section overlap. A reviewed baseline diff SHALL account for changed output, token counts and eligibility additions/removals; no minimum savings target SHALL conceal growth.

#### Scenario: Structural compression change
- **WHEN** a nested or keyed-table candidate emits different equivalent stdout under 4.1
- **THEN** both tokenizers count the new exact bytes and the review records their actual differences rather than copying old savings

#### Scenario: Terminal newline and special-marker text
- **WHEN** successful stdout ends in LF or contains a literal tokenizer-special-looking marker
- **THEN** LF participates in counting and the marker is counted as ordinary text rather than control syntax

#### Scenario: Eligibility regression
- **WHEN** a formerly eligible case fails TOON decoding or value/process equivalence
- **THEN** its token row is excluded with an explicit reason and its failed contract remains visible rather than disappearing from coverage

#### Scenario: Shared section case
- **WHEN** one case is linked by multiple section ledgers
- **THEN** every owning page may show it but aggregate token totals include it only once

### Requirement: Format migration preserves independent exact gates
Regeneration SHALL preserve independent primary JSON/process, compact-JSON byte/process and TOON ordered-value/process gates. Recursive table-header key reordering SHALL be accepted only where required by TOON 4.1; arrays, result cardinality/order, primitive types, required stderr, framing and complete stdout consumption SHALL remain protected. Reviewed disparities SHALL remain distinct from exact matches. A strict run with retained differences SHALL preserve its nonzero exit status even when complete reports are produced. TOON output updates SHALL NOT authorize blanket numeric tolerances, new disparity approvals, reference-pin changes, case removals or claims of calibrated performance.

#### Scenario: Existing exact difference
- **WHEN** a candidate retains a PR68 primary or compact exact mismatch
- **THEN** its actual observations and nonzero strict status remain explicit and correct TOON encoding does not turn it into an exact pass

#### Scenario: Nested table interpretation
- **WHEN** a decoder accepts a recursive header but materializes literal flat keys instead of nested objects
- **THEN** the TOON contract fails despite a successful parse and matching row cell count

#### Scenario: Result-boundary recovery
- **WHEN** default TOON emits several values including empty objects, empty arrays and keyed/nested tables
- **THEN** independently captured JSON results establish boundaries, each TOON value matches, and all stdout bytes are consumed in order

#### Scenario: Deferred ownership
- **WHEN** refreshed local evidence is published while platform or calibration follow-ups remain unresolved
- **THEN** those obligations and their actual native scopes remain explicit and are not counted as approvals or exact matches
