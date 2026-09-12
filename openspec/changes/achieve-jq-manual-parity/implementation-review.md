# Implementation review and checkpoint history

Implementation resumed after the dependency-policy decision. The user selected
close compatibility through safe Rust and existing Rust libraries, with measured
disparities documented for later reconsideration. Completed tasks are recorded in
the live task checklist. The final verification section distinguishes accepted
evidence from historical checkpoints. No checkpoint establishes 100% exact parity.

## Current acceptance checkpoint, 2026-09-10

Acceptance is 63 of 69 tasks. Task 7.2 is complete after primary review of
root staging, typed replay, duplicate-key ordering, shared memory limits,
per-root VM limits, and recoverable runtime-error continuation.

The capture-path decoder optimization passes primary review and 11 integration
tests comparing automatic execution with forced document execution. Witnesses
assert admission to the new mode and cover empty retained shells, sibling and
terminal paths, wrong-kind values, duplicate replacement without cross-item
loss, and discarded malformed or resource-limited input. Decoder tests also
verify cancellation inside a newly discarded item field. TOON and existing
projection routing remain unchanged.

Fresh full-worktree v10 archive
`2ffabd9f97fe70e6c62acce13b74891416e0882d7a7913153b415e0479a1fde0`
passes 1,150 Linux workspace tests across 110 suites, with two reference tests
ignored in the ordinary run. Formatting and the separate Rust 1.88 all-target,
all-feature check pass. Before/after source manifests match. Strict Clippy finds
one report-test function three lines over its limit; a setup-helper extraction
fixed it. The next run exposed a second report-command length lint, fixed by
extracting validation without changing explicit-empty approval semantics.
Fresh v12 passes all 1,150 tests, formatting, and all-target/all-feature
keep-going Clippy. Its before/after manifests match SHA-256
`235f669b8272b1f6efc69f80ead5b9fed882708c2e115ecebdf0d0c877d7ec62`.
Evidence is in `target/linux-review/capture-integration-20260910/`.
This snapshot includes the concurrent report relocation. No fresh performance
or release campaign is inferred from its tests.

The v9 component release, SHA-256
`8753d36404758d9b30dfcb41da55f050026cf5fd6a87de906f9876aef8e8f4a6`,
preserves all nine benchmark outputs and passes every wall-time gate. Four RSS
gates remain outside the 1.20 limit: projection 1.259, select 1.286, sort 1.280,
and regex 1.217. The isolated report SHA-256 is
`555bb9cab56b1483a0ef0e8d0390427cb610bd1360fa86b16e825d2284a04736`.
The approved acceptance policy now requires disclosure above 20% for both
wall time and peak RSS, accepts documented increases through 50%, and blocks
anything above 50%. All v9 measurements meet that revised limit. The seven
observations above 20% are recorded in `docs/performance-baseline.md` and the
companion checkpoint summary. Historical raw gate results remain unchanged.
Fresh v12 native release campaigns and renderer-only metadata verification
are in progress; this policy decision alone does not complete their tasks.

The last fully campaigned release archive is
`e7e3e58d5a8a244837ebfbef8c75db2c35d4f6de7e94eef8ba2fba9cb6fcb78e`.
It passes 1,135 Linux workspace tests and 1,136 macOS tests, with two
reference-dependent tests ignored in each ordinary run. Linux Rust 1.88
all-target/all-feature checking passes. The native Linux release build and
source-manifest before/after comparison pass. Five Clippy findings were
confined to new test closures. The separately frozen v8 test-only patch passes
all-target/all-feature Clippy, formatting, MSRV checking, and 1,133 tests across
104 suites, with two ignored reference tests. Its all-targets command excludes
the two passing doctests included in v7. An initial missing PTY helper was
resolved by restoring the existing tool directory to PATH, without source edits.

The v7 macOS release executable
`284c6df9d9ad23a6cf1565f57224857d1dd78cb62a46876cdf6ece1ce219dd6b`
matches all nine benchmark outputs. Four unchanged gates still fail:
projection RSS 1.266, sort RSS 1.319, select wall/RSS 1.660/1.391, and regex
1.530/1.279. Report SHA-256 is
`998f86378e02696922acd818e5819b1c8c900d740996fa4fe9c783beabf60cea`.
Both fresh v7 native release campaigns are complete. Each executes all 952
cases and preserves all 518 original cases and 303 protected exact matches.
macOS has 949 exact matches and three known differences; Linux has 944 exact
matches and eight known differences. Neither campaign applies approvals.
Both pass all 921 TOON contracts, with no skipped or nonexecuted observations.
The normally ignored manual and portability reference tests pass explicitly
on both hosts. Evidence is retained under `evidence-macos-v7/manual/` and
`evidence-v7-final/manual/` in the root-integration evidence directory.

Primary approval review finds two former macOS numerical witnesses and one
Linux numerical witness now exact. The remaining changed stable observations
differ only in the approved trailing TOON LF, apart from executable identities.
Renewal remains deferred until the final production build. Parser capture-path
retention and a separate opt-level=2 diagnostic are in progress. No final
acceptance or performance exception follows from the campaign results.

Fresh-target Linux archive
`989f90ee4603b038febfedb740396d32e2a4d227eb2f4615f8d47758d3b5f8e5`
passes all 33 root-boundary, resource, disk-spool, and continuation tests.
Its executable SHA-256 is
`a8a03f462474eaaeb7996e9313b0bb7d3126da80dc65be7c892b7f794b01e34b`.
All 61 differential parser probes against pinned native jq and all six
depth/token-limit probes pass. Evidence is retained under
`target/linux-review/root-integration-20260910/evidence-v3/`.

The full workspace run reached every test target. It found one stale walk
stderr assertion and a test-only executable race. Luna reproduced the race
with parallel fixture-writing tests; isolated and serial runs passed.
The tests now serialize executable creation and use the approved runtime
diagnostic contract. Remaining test/report helper Clippy findings are fixed
and passed the v6 fresh-target run. These fixes do not change tq execution.
V6 passes 1,118 workspace tests, stable Clippy, formatting, and the Rust 1.88
all-target/all-feature locked offline check. Twelve repeated fixture-executable
suites pass all 72 tests. Both normally ignored pinned-reference tests pass
when explicitly enabled. New performance changes still require fresh checks.

The v6 native debug campaign executes 952 cases: 944 exact matches and eight
known math/regex difference IDs, with no reviewed approvals applied. All
4,667 observations execute without skips or timeouts. TOON passes 921/921
contracts and compact JSON passes 913/921. All 518 original cases and 303
protected exact matches remain. This diagnostic report does not refresh
release approvals. Evidence is in `evidence-v6/manual/` beside the build logs.

The fresh macOS release build of that source completed with Homebrew Rust
1.98 in 10m 55s. Binary SHA-256
`1c7a77c13b8658a86fd4b16afccfb53a8febf296a151a57acf07e5403bfc3b99`
matches all nine benchmark outputs but fails four unchanged regression
checks. Projection/sort RSS ratios are 1.262/1.213; select wall/RSS ratios
are 2.024/4.187 and regex ratios are 1.809/3.751. Raw evidence remains in
the companion archive's `.work/jq-parity-perf-20260910-roots-v3/` directory.
The diagnosis identified full-feature staging for select/regex and eight-byte
length/count fields in the private spool. No performance exception is accepted.

The bounded canonical-varint change passes 29 native spool tests and focused
Clippy. A red witness against v6 encodes 21 bytes where the new codec needs 14.
Primary review checked overflow, noncanonical and truncated varints, decoded
allocation limits, numeric provenance, cancellation, and cumulative write limits.
The codec keeps its fixed record header and indexed replay semantics.

The conservative core field-retention proof now prunes unobservable object
fields before root staging. It leaves the original bytecode and VM steps
unchanged, rejects uncertain aliases and shape observers, and keeps whole
terminal-path values. Shared objects are not copied for pruning. Primary review
required root-only application, separate sibling prefixes, and preservation of
standalone select results and already-projected values. The focused run passes
two core proof tests and 111 CLI unit/integration tests. The earlier red suite
fails only the two retention-reduction assertions and passes five behavior tests.
The exact final test revision also has a matching fresh-target macOS v6 red
run: five behavior tests pass and the two retention assertions fail with equal
encoded sizes, 947/947 and 396/396 bytes. A separate eight-case debug probe
matches pinned jq stdout and status for sibling/overlapping paths, scalar
errors, duplicate keys, optional/short-circuit predicates, multiple roots, and
malformed later roots. Error wording remains tq-specific. A one-step VM limit
fails boundedly with no stdout. These diagnostic probes are retained in
`evidence-footprint-final-red/` and `evidence-footprint-differential/` under the
root-integration evidence directory. Combined fresh-target Linux checks and
macOS release builds are running; component results do not close the performance
gate or renew manual approvals.

The v4 clean-target check passes all 33 root regressions and formatting,
but the report-helper refactor introduced a String/byte-vector type mismatch.
Clippy also rejected a redundant closure in the new test mutex helper.
Both corrections are scoped to test/report code and await v5 verification.

The v5 full workspace run passes 1,118 tests across 108 suites, with two
reference-dependent tests ignored. Formatting passes. Four unnecessary raw
string delimiters in boundary tests remain Clippy errors; their mechanical
correction is being checked in v6. OpenSpec strict validation and the complete
docs OKF bundle pass. The docs validator reports only an informational link
to the traceability TSV, not a missing document.

The earlier v2 non-CLI workspace run passed 831 tests across 85 suites, with
two reference-dependent tests ignored. It is retained in
`evidence-v2/workspace-non-cli.log`; it does not replace final reference runs.

## Historical parser and performance checkpoint

Acceptance is 62 of 69 tasks after reopening 7.2 for the optimized root-boundary
failures below. The parser-ready native Linux diagnostic
executes 929 cases: 920 exact matches and nine differences with unchanged
fingerprints and process observations from the previously reviewed Linux
disparities. All 518 original cases remain and all 303 protected original
matches still match. All 900 TOON encoding contracts pass. The two normally
ignored reference tests also pass when run explicitly. This debug checkpoint
does not renew final release approvals.

The source archive SHA-256 is
`3622621674b5d2fee9e8e0dffca7a87e251feb27820f2969c019930c377ca830`.
Local provenance and raw logs are under
`target/linux-review/parser-ready-20260909/`. The original diagnostic executable
was placed outside the ORIGIN fixture's required directory layout; that failed
report is retained beside the corrected-layout run using identical executable
bytes. The corrected report SHA-256 is
`017c76b866f41e6c70e9a0f40d18fd061f1b0868ca832e7c03dc503cd6afdd29`.

The first parser release benchmark failed the unchanged wall-time and RSS gates.
Projection and sort took 3.37 and 3.21 times the original baseline; regex RSS was
1.2018 times baseline against a 1.2 limit. Outputs matched on all nine workloads.
The remaining six workloads passed. These measurements are in the companion
repository's `.work/jq-parity-perf-20260909-parser-prelint/report.toon` and apply
only to executable SHA-256
`f8ab385a42221d1bf34a4942792bd3e6565cc3c64fd37493172cdb399bcb1164`.

Luna's diagnosis identified lost selection-aware skipping in the shared parser.
The optimization is in progress under primary Astra review. Review requires
shared grammar and resource validation for discarded data, no added path
allocation on ordinary decoding, correct skipped-array indexes, unchanged
unselected event timing, and source-depth observations that still include
discarded containers. Earlier passing performance reports do not establish
acceptance for this parser. Fresh source-bound release campaigns and benchmarks
are required after the optimization.

The subsequent numeric-validation/selection-skip release checkpoint is
`967d94f9c1f802278078993d7df44ffdd83f81a18590e615b0c601bded0bcd3d`,
from source archive
`5e7ffac52e3a26b2a6e6d0c2b4291ff4662d611485f009045439b6cea8ebb6ec`.
Its authoritative nine-workload report
`d93da995de2c67a3fbb4a53e15e9025ceab6c61e015ed6765a3759db9bec1054`
has matching outputs throughout, but projection/sort/select wall ratios
1.850/1.748/1.575 and regex RSS ratio 1.206 still fail the unchanged gates.
The prior wall-only diagnostic did not provide RSS evidence. Primary separately
reran the numeric-selection suite and verified four passing tests; an agent's
initially cited log was an earlier failed run and was rejected as evidence.

### Optimized ordinary JSON root boundaries

Primary review subsequently found failures outside the 929-case checkpoint.
These were reproduced with the same frozen parser executable `f8ab385a` and
pinned macOS jq 1.8.1, using compact JSON output and `.features[].id`:

1. Input `{"features":[{"id":1}]} {"features":[{"id":2}]}` produces `1` then `2`
   and exit 0 in jq. tq with `-i json` exits 5 with empty output and a trailing
   JSON input diagnostic at byte 24. The automatic selected decoder still
   requires exactly one root rather than ordinary JSON's whitespace stream.
2. Input `{"features":[{"id":1},{"id":2}],"discarded":[1,]}` makes jq exit 5
   with empty stdout. tq exits 5 after emitting `1`. The optimized executor
   publishes a selected result before its containing JSON document is valid.
3. Adding `| debug` to that malformed-root query also publishes debug messages
   before the input failure. jq produces neither query results nor debug effects.
4. Input `{"features":[{"id":1}],"features":[]}` produces no results in jq,
   honoring the last duplicate key. tq emits `1` from the overwritten value.
5. Inputs `{}`, `{"other":1}`, and `{"features":null}` produce iteration errors
   with exit 5 in jq. tq instead exits 0 with no results. An empty object at
   `features` correctly produces no results in both tools and must remain distinct
   from a missing or null value.

These are unresolved implementation failures, not safe-library disparities.
Task 7.2 is reopened. The next integration work must preserve ordinary JSON
document boundaries, per-document executor state, prior valid-root output,
last-key-wins object behavior, and bounded storage without changing explicit
`--stream` event semantics. Selected input must be validated before VM execution;
buffering only stdout would still leak process effects. The existing TOON output
replay codec projects NaN to null, so it cannot stage runtime input unchanged.
The passing parser probes and source inventory do not cover these interactions.

### Ordinary runtime-error continuation

For `error("runtime-before-parse")` with input `{"ok":1} {"bad":[1,]}`, jq
prints the first root's runtime diagnostic and then the second root's parse
diagnostic. Both the frozen parser executable and the current debug executable
print only the runtime diagnostic. The document route stops after the first VM
error instead of continuing to the next input root. This failure is separate
from automatic selection and remains open.

Primary review rejected an initially passing ordering assertion because its
search for `parse` matched the marker `runtime-before-parse` in the first
diagnostic. The regression must require a distinct second diagnostic, not a
substring of the first one.

### Integration review checkpoint, 22:37 UTC

The private runtime-value spool is implemented but not connected to production
execution. Primary reviewed source `bbe3f096465cf779393ab2eab6cf05fd4fb24f6a74eaf993478034bd958e86c3`.
The focused unit run reports 11 passing tests. Review corrected buffered file
reads, decoded-token budget checks before allocation, memory reservation based
on vector length, record-header accounting, and single-consumption replay.
Forced-file tests check finite numeric identity, NaN bits, infinity, negative
zero and object order. This is component evidence, not root-handling acceptance.

The formats lifecycle API is also under review. A further issue was found in
root and array-index ancestor type notifications: scalar and array roots could
be reported as the same missing-field fallback as an empty object. Integration
must retain enough type information to reproduce the corresponding VM errors.
Callback tests must distinguish source-order replacement notifications from
the final first-insertion-order iteration of duplicate object keys.

Recoverable runtime diagnostics must be emitted before effects from the next
root, without printing the final diagnostic twice. A reported-runtime error
variant is being integrated for ordinary input streams. Null-input and slurped
execution have no next top-level root and must retain their existing embedded
error-reporting contract. These changes still require focused and full tests.

### Native component checkpoint, 23:11 UTC

Later clean-target verification exposed stale artifacts when this shared Cargo
target was reused across extracted source snapshots. The component results
below are diagnostic observations, not final source-to-executable proof.
Final acceptance uses fresh target directories.

Frozen source archive
`4758717980d2e58618a5a7eb50b4b80c7d3010b33e937b58be98e959905e71c5`
was tested on Ironhide with Rust 1.98. Native parser unit suites passed:
16 core tests and 27 formats lifecycle tests. CLI unit tests had 89 passes
and two failures. Process continuation tests had one pass and one failure.
The catalog schema check rejected a new root-boundary fixture, and all-target
Clippy rejected new test syntax. Complete logs are retained under
`target/linux-review/root-components-20260909/evidence/`.

Primary review identified an indexed-spool bounds check performed after seeking,
an embedded capability-denial diagnostic regression, and an assertion expecting
the word `parse` although the actual second diagnostic says `invalid JSON`.
Corrections are in progress. No failed check is counted as accepted evidence.
The root-stage integration is not part of this frozen checkpoint.

The indexed-spool correction was reviewed and retested against the same frozen
base. Source SHA-256
`191931d6e22f89e9826610d3a76e30edef67d91c68919883eaf389dac415e50a`
passes all 18 spool tests, including bounds failures for both memory and file
storage. An initial rerun hit the system temporary-directory quota; the identical
source passed with an isolated `/var/tmp` directory. Both logs are retained.
The corrected process-test source
`62fba6cc26ae697389814e3eb814a37a89f64e4802a959cb804aa28917b0aa35`
passes both diagnostic-order tests. The ambient capability-denial correction
remains separate and unverified at this checkpoint.

Pinned native jq 1.8.1 observations for all six runtime-continuation witnesses
are retained under `target/linux-review/runtime-reference-20260909/`, including
exact input, argv, output, diagnostics, and status. They confirm continuation
after ordinary runtime errors, raw-line continuation, fatal halt behavior, and
runtime-before-parse diagnostic order. Acceptance remains 62/69.

The corrected component archive
`45817a25c188aaeadb15d5775ee80e63f991e802cfd2d676ce34d83f7a9378f8`
isolates the capability-denial fix from unfinished automatic integration. It
passes 16 parser, 13 core capability/effect, 27 formats lifecycle, 93 CLI unit,
three runtime-continuation, and one catalog-schema test on Ironhide. Its spool
source `677c8279` includes 20 passing unit tests, including exact physical-write
accounting. Clippy remains unsuccessful at this checkpoint due to two reporting
code lint classes; fixes are pending verification. Raw evidence is under
`target/linux-review/capability-boundary-20260909/evidence/`. This assembled
component checkpoint is not a frozen copy of the entire current worktree and
does not validate the new automatic runner or prefix-access integration.

The assembled prefix-access checkpoint
`cc519ee86ae39b246e1fdf13574e2dc3b570394bf731d691d3a6f110eb8b4e49`
adds the one-step prefix programs and matching comparison dependencies. It
passes all selected native suites: 16 parser, 13 capability/effect, four prefix
access, seven comparison, seven normalization, 27 formats lifecycle, 93 CLI
unit, three continuation, and one catalog-schema test. Rust 1.88 also passes
the workspace all-target/all-feature locked offline check on this assembled
source. Logs are under `target/linux-review/prefix-components-20260909/`.
This is component evidence, not full-worktree or final MSRV acceptance.

An earlier prefix checkpoint omitted the matching normalization helper and
failed compilation. That assembly error is retained in `evidence/`; the
corrected dependency set passes in `evidence-v2/`. The earlier Clippy run also
flagged the length of `automatic_plan`; a helper extraction is awaiting
verification. Automatic root integration remains under primary review,
including shared preparation-memory accounting and nested duplicate keys.
Acceptance remains 62/69.

Prefix checkpoint `e1af9aa80925c34e24e8ad98d204cd45a84b1ccc4a24a06e8d039f047cd47a6b`
passes the same 171 selected tests, with raw logs in `evidence-v3/`. Clippy
still fails because the extracted helper returns a complex tuple. The next
revision uses a private named result struct instead of suppressing the lint.
It awaits whole-worktree verification with the corrected automatic runner.

### Clean-target integration checkpoint, 2026-09-10

Identical source archive `826cbacf833d1c3422b1186057d5de8cd529aa59fb737a3a3c4a83fc39a959a0`
produced four passing boundary tests with the shared target and 21 with a
fresh target. The remaining clean-target failure was a stale single-root
type-error expectation, subsequently corrected. Both logs are retained under
`target/linux-review/root-integration-20260910/evidence/`. The cached result
does not describe the current executor's behavior.

Updated archive `ba459049a69a7ddb5ad14442b1cde14abc102362678a787cf69d2074f9b89cd9`
was built in a second fresh target directory. All 32 root-boundary, memory,
disk-spool, and continuation tests pass, along with workspace formatting.
The broader workspace run stopped on two outdated TOON expectations, for
default trailing LF and preserving a valid root before a malformed later
root. Clippy exposed remaining helper and signature cleanup. These are being
corrected without suppressing lints. Raw logs are in `evidence-v2/`.
Acceptance remains 62/69; this is not full workspace or campaign completion.

## Scope

The starting worktree contained only the proposed change. The spec was committed
as `90b231be35fe6339ed440494aafe76e4e68e54b2`, `docs: specify complete jq manual parity`.
Implementation review uses that commit as its fixed point. No implementation
commits follow it and no unrelated starting edits required exclusion.

The primary agent reviewed both standards and spec compliance, as requested.
Implementation agents used Luna X-high. Review used
`git diff 90b231be35fe6339ed440494aafe76e4e68e54b2`, including staged changes, plus
the in-scope files listed by `git ls-files --others --exclude-standard`.

Reviewed code covers CLI argument parsing, core grammar, manual gate validation,
the manual comparison command, and their new public-interface tests. Reviewed
support files include the archive-safe gap inventory, test documentation, and
standalone dependency probes. Generated root `target/` artifacts are excluded.
The source is this change's proposal, design, eight delta specs, and task list.
Standards include the workspace lint policy, the repository-management Rust and
test guidance, and the review skill's code-smell baseline.

## Standards

No unresolved standard violations were found in the completed slices. Review
corrected an early-exit test's broken-pipe race and moved generated probe build
output outside the fixture tree. The TOON-only fixture guard remains unchanged.
The unsafe-code prohibition is unchanged. Production dependencies now include
pure-Rust `libm` 0.2.16 and `fancy-regex` 0.19.1; no native regex engine or direct
FFI bridge was added. Historical standalone probes are not production dependencies.

Grammar parsing retains distinct entry points for expressions with different
separator rules. Their similar pipe loops were reviewed as grammar context,
not treated as a mandatory abstraction opportunity.

## Spec

The checkboxes in
[tasks.md](tasks.md) are the acceptance inventory; every unchecked task remains
unresolved.

Review reopened tasks 6.3 and 10.3 after the expanded composition checks exposed
the legacy `IN` empty-right-stream failure and unresolved executable witnesses.
Task 6.3 subsequently passed the focused SQL review below. The live checklist
records final acceptance; the dated checkpoints retain earlier incomplete states.

Review reopened task 3.11 after the evidence-refresh query exposed lost assignment
provenance across a nested binding. With an approval array followed by a report as
two inputs, `. as $approvals | input as $report | $approvals | map(. as $approval |
($report.cases[] | select(.id == $approval.case_id)) as $case | .evidence = {x: 1})`
succeeds under pinned jq 1.8.1. The frozen macOS candidate with SHA-256
`b3376165a85e897a8b37dfebb2e486e6a26eec535d292b82beca60c7719489ee`
instead exits 5 with `assignment left side is not a path`. This was an
implementation regression, not an accepted safe-library disparity. The corrected
binding route passed the renewed acceptance checks below; earlier failed attempts
remain historical evidence.

Review corrections included compact-output conflict bypass through
`--pretty-output`, catch expressions consuming trailing comma/pipe expressions,
object-value pipes, quoted empty keys, reused non-manual-prefixed case IDs,
unknown review references, and deferred catalog entries escaping the gate.
Focused regression tests now cover those behaviors.

The strict campaign fails the current report instead of accepting former policy
differences. Source/executable identity pinning now protects the imported
inventory, fourteen source documents, every original case ID, and all 303 original
matches. Changed bytes, build configuration, missing sections, and replacement IDs
fail validation; a relocated reference executable remains valid. Compact-byte
campaigns, the rest of the language/built-ins, I/O, modules,
platform contracts, and final release evidence are not complete.

## Verification

### Final verification, 2026-09-09

Final review reopened task 7.2 and the dependent acceptance tasks after a direct
strict-JSON input witness failed. With `NaN Infinity -Infinity` plus LF on
stdin, pinned jq 1.8.1 under `-c '.'` exits 0 and emits `null`, positive finite
maximum, and negative finite maximum. Candidate SHA-256 `e9478b0...` under
`-i json -c '.'` instead exits 5 with `expected value at line 1 column 1`.
The core delta explicitly requires jq-compatible non-finite strict-JSON input.
This is an implementation gap, not a safe-library disparity. The 905-case
campaigns below remain valid checkpoint evidence but did not cover this input.
Their green results do not establish full-spec completion. A correction and
renewed executable-bound campaigns are required before final acceptance.

The feasibility review found that `serde_json` rejects these tokens before a
custom value visitor or raw-value hook can observe them. Document, line,
sequence, structural-event, selected/parallel, and `fromjson` routes must agree.
NaN and infinities must retain numeric identity for `type`, `isnan`, and
`isinfinite`; substituting `null` or finite maxima before evaluation is wrong.
String/sentinel rewriting also risks collisions with ordinary input and cannot
replace a bounded incremental parser. No parser code changed during this review.
The user subsequently approved one shared safe-Rust incremental JSON parsing
boundary used by every route. The value and event readers will share grammar
without forcing whole-document allocation for streaming consumers. Task 7.2
remains open until implementation and regression verification are complete.
No parser disparity, unsafe code, or FFI change has been approved.

Parser integration review also reopened task 9.4. The test runner compares
serialized output, so runtime NaN can incorrectly satisfy an expected JSON
null. It must compare bounded captured runtime values as well, without running
the query twice or changing effects and process status. Non-finite input with
numeric predicates is the positive public witness. The negative witness uses
NaN input and expects null, which the pinned jq test runner rejects. Reference
tests with non-finite expected values trigger a jq assertion failure and are
not contracts for tq to reproduce.

The parser integration review is still open. It also checks numeric token
boundaries and UTF-8 repair, physical input lines despite buffered read-ahead,
exact-one consumers that do not parse a second root, and cancellation during
discarded values. JSON sequence truncation and recovery must survive the parser
replacement. Stream error arrays must retain jq's messages, paths, and partial
output rather than merely return a successful status. In particular, pinned jq
emits no earlier leaf for `[1 2]`, and reports the error at path `[0]`.
The shared numeric conversion must cover `tonumber` too. Pinned jq accepts
`"NaN"`, `"Infinity"`, `"+01.200"`, and `"1."`, but rejects `" 1 "`.
These are implementation checks, not additional approved disparities. Focused
test results and the renewed full campaigns are required before acceptance.

Task 9.4 was accepted again after primary review of a fresh native checkpoint.
Archive `d88af8f09443f5970bc28f9d364bd0a5b89270911d93a0a961a917582c3b2d77`
compiled the reviewed runner `1a9f447e...` into debug executable `ca323824...`.
The CLI library passed 67 tests and extended CLI passed 36, including typed
capture bounds, missing/overflowed capture rejection, passing test files,
compile diagnostics, non-finite input predicates, and rejection of NaN as
expected null. Primary verified the raw logs and hashes under
`target/linux-review/parser-checkpoint-20260909/`. Effects passed 14 tests,
argument handling 17, modules 19, and resource guards 2 on this same checkpoint.
The later runner diff only changes sequence resource-error propagation; it does
not alter the reviewed capture or test-runner code. Parser task 7.2 remains open.

The same checkpoint is not a full parser pass. Its input-stream suite passed
33 of 35 tests; the two new compound-result assertions incorrectly expected
output before array construction finished. Independent primary limit probes
verified compound-result atomicity and preserved earlier independent results,
but found sequence resource failures returning exit 0. That is a real defect,
not an acceptable recovery behavior. The primary's 57-case differential probe
matched 47 cases and found three null-slot path-shape failures plus seven
sequence diagnostic failures. These results remain checkpoint evidence.

Task 7.2 passed the subsequent parser acceptance review. Frozen source archive
`3722f4232e539e51a349c7d6ad318e098880e8e7c800fc3a0d092eaf70eae8f2`
built native Linux debug executable
`edc12b4d84ad963aac6d8fe4e8602c6e14260c3b03848733c5d4949192eb9b17`.
The complete workspace run passed 1,022 top-level tests, with two reference tests
ignored by default. The nested resolver child contributes one additional test
summary, not a separate top-level test. All 61 pinned-jq parser probes and all
six independent resource-failure probes passed. Primary verified source and
executable hashes and unfiltered logs under
`target/linux-review/parser-clean-20260909/evidence/`.

Review corrected nested null-slot paths, preserved partial output, EOF/RS
numeric diagnostics, sequence resource exit status, early depth/token limits
inside unfinished records, and whitespace compaction with source positions.
Selected/skipped-token tests now reach the intended values rather than failing
on an earlier key. Native TOON/YAML/JSON5 regression tests remain passing.
The 41-row prose ledger now preserves the literal string `"null"` in its input
mapping. Clippy cleanup, explicit reference tests, final release campaigns,
approval renewal, and benchmark evidence remain separate pending acceptance
work; this checkpoint does not make the old 905-case reports current.

Primary review rechecked all eight delta specs, initially 31 requirements and
110 scenarios, against the implemented behavior and the focused evidence below.
The final sync review also found stale default-framing statements in the older
CLI spec and a missing rename declaration for standalone output. Those artifact
corrections must preserve the already-approved standalone default and explicit
`--seq` contract before archival. They do not require a behavior change.

| Delta | Requirements / scenarios | Implementation and behavior evidence |
| --- | ---: | --- |
| cross-tool-compatibility | 6 / 17 | `manual.rs`, `manual_pin.rs`, comparator and normalization; `compatibility_manual_gate`, `manual_pin`, `manual_completeness`, `manual_portability`, and `manual_reference_provision` tests. Source-linked cases, protected original matches, negative arities, stale approvals, missing source/reference evidence, and strict/completion separation remain enforced. |
| extended-jq-cli-parity | 4 / 9 | CLI args and runner; `manual_arguments`, `extended_cli`, `colors`, `modules`, and ambient-policy tests. Includes invalid-option no-read behavior, controlled environment, PTY colors, test files, and pipe handling. |
| jq-core-language | 8 / 25 | Parser/resolver, numeric model, managed evaluator, math and collection/string adapters; `manual_grammar`, `runtime_numbers`, math/collection/string/SQL/regex tests, path/update and origin tests. Campaign retains exact decimal and compact observations without a blanket tolerance. |
| jq-reduce-foreach | 2 / 6 | Managed fold continuations; `user_fold_composition`, `user_composition_resources`, and advanced manual cases verify destructuring, initializer/update/extraction cardinality, independent state, and partial errors. |
| jq-regex-date-platform | 2 / 9 | Safe regex/date adapters and shared source metadata; core/CLI regex and date tests, live input/effect tests, and both native campaigns. Longest matching remains a specifically reviewed restriction. |
| jq-user-functions-modules | 2 / 15 | Managed user calls, filter/value closures, explicit origin tokens and resolver module lookup; composition family/resource suites, `composition_inventory`, CLI `modules`, and original approval-renewal helper replay. The 220 per-arity composition witnesses execute in the campaign; admission-only checks are not counted as semantic proof. |
| toon-stream-io | 2 / 6 | Writer and replay preparation; writer conformance/cardinality/spool tests and independent TOON campaign. Standalone output publishes only after exactly-one-result success; explicit sequences retain complete prior records. |
| tq-cli | 5 / 23 | Output option resolution and shared cursor/effects; `compact_selection`, `input_streams`, `effects`, `user_effect_composition`, and `platform_shell`. JSON sequence recovery, stream reconstruction, non-catchable halt, and live unbuffered writes are observable process contracts. |

Final production evaluator SHA-256 is
`51626821acbabdd2664d78efbeff8515bd4a375ab23de4dc62e3fa06f54852c5`.
The ordinary binding route evaluates values normally, retains the original
input origin, and records bounded destructuring aliases. Actual path evaluation
retains its own context. Foreign constructed roots remain invalid assignment
targets. Review rejected the earlier continuation-truncation workaround.

The final macOS completion gate exited 0 with 905 cases, 900 exact matches,
five reviewed disparities, and zero unresolved failures. Compact JSON is
871/876 exact; TOON preserves 876/876 JSON execution contracts. The report
`target/parity-macos-bind-completion-20260909.toon` has SHA-256
`dcedf6f3da6c51663535b8453a077801412a9c4db1e10a4c4df026758e929f22`
and is copied unchanged to the checked-in manual comparison report.
The frozen CLI executable SHA-256 is
`e9478b0ad46e951a1654efbe4d9d52440da49b8b57283754a0c86fb69f1d4f0c`.
The comparator is `8601f6170d18c6865d4d09fbfd60f366fe6919e4fb1144035c488c0ba7bb9bbe`.

Native Linux ran the immutable source archive with SHA-256
`2dfe30f384e1ffd832e58f7c85cbb267cc8c4cabc6d2e2d0b40159e98ef12400`.
The final CLI SHA-256 is
`fda6cb6843aedeac9d9777b48d23a44a3a283c86e5a5f5c0b7fb36d98b07b74f`.
Its completion campaign exited 0 with 896 exact matches and nine reviewed
disparities out of 905, zero failures, compact JSON 867/876, and TOON 876/876.
The report SHA-256 is
`2c746e2cfac91ab6f8767517719e46e466e163b9bcad59dc66357dc35bae6c72`.
After publishing the final macOS report fixture, the affected tests passed:
24 gate, nine compatibility, two completeness, three inventory, and three
storage tests. Raw fixture-test log SHA-256 is
`7d211df3a4a3f1181b58809238832e40b15a2ced67a6c5a5867fe35bd712bd09`.
The nine approvals comprise the earlier six observations and three separately
reviewed wrappers: `manual.composition.arity.scalbln.2`,
`manual.composition.arity.y0.0`, and `manual.composition.arity.yn.2`.
Each wrapper has its own fingerprint and exactly the same observations as its
parent witness. Integer exponent conversion is a semantic restriction, not ULP
rounding. No broader exception was approved.

The final Linux workspace run passed 981 top-level tests across 100 suite
summaries, with zero failures and two ignored tests. Both ignored reference and
relocated-corpus tests passed when explicitly enabled. A nested resolver child
prints one additional success summary; it is not double-counted. The raw
workspace log SHA-256 is
`43edd5b3e21367b0715dd09aba64b87763218c8b6bd1eaa9c5fb7ef26ecbf794`.
Test-only raw-string delimiter cleanups followed the source archive and passed
the renewed 12 core path/update tests and two CLI binding tests. Query bytes
and production behavior were unchanged. Rust 1.88 separately passed
`cargo check --workspace --all-targets --locked --offline`; stable Rust 1.98
ran the workspace tests. Native Windows remains explicitly unverified.

Final workspace formatting passed. Clippy with `-D warnings` exited 0; the
pre-existing removed `clippy::assert_is_empty` lint warning from global
configuration remains. Raw Clippy log SHA-256 is
`5432164d2a4e20a994d8b1f0e38612a25742fdf7d31e206ff50effb6e3c2c54f`.
The attempted full macOS workspace and MSRV reruns were stopped after pre-main
loader stalls and are not passing evidence. Native Linux supplies the complete
workspace/MSRV checks; the final macOS release binary and campaigns ran normally.

The final nine-workload paired benchmark passed every correctness and regression
limit. Wall ratios range from 0.956 to 1.176 and peak-RSS ratios from 1.032 to
1.182. Both revisions use explicit `--unframed` for comparable standalone TOON
bytes. The companion report `2026-09-09-jq-parity-bind-final1.md` publishes the
measurements; raw report SHA-256 is
`0badb9cf077bf17fc994c0e7197c7efa88eb1b8230d134fed0b62739fa800b92`.
These are measured changes on one host, not a universal performance guarantee.

The final six-function math probes each cover 92 inputs and preserve executable
and runtime-library identities before and after execution. macOS records
77 exact and 15 differing samples; Linux records 81 exact and 11 differing
samples, with no process issues. Function-specific observed maxima support the
disparity document, not global tolerances or additional campaign passes.

All sections below are historical checkpoint evidence unless explicitly cited
by this final verification. Their pending states do not supersede final results.

### Binding-source correction accepted

Ordinary bindings now evaluate their entire source as a value expression and
preserve origin information for aliases. True selector contexts retain path
evaluation. The fix does not truncate continuations, discard expression steps,
replay effects, or suppress path errors. Relative alias copies enforce path
limits, charge their work, and reserve memory fallibly.

Primary reviewed the clean diff, verified matching local/native source hashes,
and inspected the 57 passing adjacent core tests and two passing CLI regressions.
The final `eval.rs` SHA-256 is
`51626821acbabdd2664d78efbeff8515bd4a375ab23de4dc62e3fa06f54852c5`.
The bounded core log SHA-256 is
`de2a4ebd005161020b2cf6063692685fb8fd0fe0e210e25c262aec87e985f6d7`;
the CLI log SHA-256 is
`a28d9351469524ba83b70a1723b389473aec15be9647c1915987d373106edb5a`.
Both logs are under `target/linux-review/composition-20260909-current-checkpoint/`.

The original full evidence-refresh query also completed under native tq and
pinned jq with empty stderr. Primary independently compared the retained JSON
outputs byte for byte; both have SHA-256
`b4a47b5244932799a587132826b92419919315fff9c3b635987224e7d77ea961`.
Those generated outputs are diagnostic evidence, not renewed disparity approvals.
Workspace formatting and whitespace checks pass. This accepts task 3.11 again;
the final release campaigns, approvals, and performance checks remain open.

### Prior checkpoint, before assignment acceptance

The next macOS candidate is `target/release/tq-composition-final-6Td8pQ`,
SHA-256 `b3376165a85e897a8b37dfebb2e486e6a26eec535d292b82beca60c7719489ee`.
Its source-audited 905-case report has 900 exact JSON matches, five unrenewed
library-disparity observations, compact JSON 871/876, and independent TOON
876/876. Report `target/parity-macos-composition-final-20260909.toon` has SHA-256
`eb9d234347d5282e28e287668dd90b383446083ed2430201ba229e92c9b34f8a`.
The 92-input math probe retains 77 exact samples, 15 differing samples, and zero
process issues. Its SHA-256 is
`058f0ae57247f5d35d9b79be4c499821516f25c20e5def23c5007577473bae65`.

The current Linux checkpoint has 896 exact JSON matches and nine unrenewed
library-disparity observations across 905 cases. Compact JSON matches 867/876;
independent TOON matches 876/876. The release binary SHA-256 is
`f468b33be31cc9233675467691d1625ae8a6678de15597c967602c8909c9b991`.
Raw logs and source/archive identities are preserved in
`target/linux-review/composition-20260909-current-checkpoint/`. The full workspace
run had 972 passing tests, two ignored tests, and one historical traceability-title
failure. An explicitly recorded TSV-only overlay corrected that title; its
focused test and the separately invoked pinned manual reference test each passed.
The latest four-file comparison-test overlay also passed all six fake-executable
and nine normalization tests. Primary reviewed their raw output and overlay diff.

Primary Clippy passed with `--workspace --all-targets --locked --offline -- -D warnings`.
The preexisting global allowance for removed lint `clippy::assert_is_empty` still
prints an unknown-lint warning. Raw log SHA-256 is
`24ecbe1772ffc1f5956842d19444904c789c860596ba190da0ff2d54b30bca6e`.
Primary stopped the macOS workspace run and independent physical Rust 1.88 check
after repeated pre-main startup stalls; their retained logs are incomplete, not passes.
A sampled stalled test process was still at `_dyld_start`, before Rust test
startup; no startup attempt or listing-only warmup counts as a passing test.

These results repair the twelve comparison-setup failures recorded below, but
do not accept task 3.11's newly confirmed assignment regression. Final binaries,
campaigns, approvals, and performance evidence must follow its reviewed fix.

The first assignment fix did not pass native verification: the update-composition
target had eight passes and three generator-binding failures. The four adjacent
origin/path/resource targets passed 9/5/14/9 tests. Raw evidence is retained in
`target/linux-review/composition-20260909-current-checkpoint/eval-path-focused-tests.log`.
Primary review also corrected invalid new CLI test framing and a duplicate
definition terminator. A mid-path variable fallback is insufficient because
already-scheduled path continuations still interpret its ordinary value as a
path. The binding-source routing correction remains under implementation.

### Expanded release campaign, acceptance pending

The earlier macOS release candidate has SHA-256
`e80eda34a889ada258733a2c36a93607cc95e5be370d32067a9dddda99540609`,
preserved as `target/release/tq-composition-review-IDjscD`. Its source-verified
905-case campaign has 900 JSON matches and five known, not-yet-renewed disparity
observations. Compact JSON matches 871/876 cases. Independent TOON matches
864/876; those twelve failures keep the gate red. The report is
`target/parity-macos-composition-20260909.toon`, SHA-256
`aa6f8126c88e0ce6ad2ae88d02ba18d0819b25392cdcf56df303408768173275`.

Primary traced the TOON failures to the comparison setup: pre-inserting `--seq`
bypasses its input-mode preservation helper, successful empty standalone TOON
is misread as zero results instead of an empty object, and late-error cases
with one prior result incorrectly select atomic standalone output. Corrections
and regression tests are in progress. No comparison is suppressed or approved
as a library disparity.

The same candidate's 92-input math probe retains 77 exact samples, 15 last-bit
differences, no process issues, and the previous per-function observed maxima
of 1/1/2/2/1/1 ULP. The report is
`target/dependency-probes/math-composition-20260909.toon`, SHA-256
`965de5b356d6ec410d23d80c15b724d342af55623e88278a5b849877bac1ceff`.
This supports witness review, not a general numerical tolerance.

The frozen Linux checkpoint has 895 JSON matches and ten failures across 905
cases. Nine are the six earlier library observations plus three composition
witnesses that still require individual review. The remaining module-origin
failure comes from running both executables outside the expected checkout
locations. Its independent TOON campaign has the same twelve setup failures.
Raw evidence is retained under
`target/linux-review/composition-20260909-final-checkpoint/`; no failed run is
rewritten. The native test rerun must use disk-backed temporary storage and the
verified PTY utility, as the first run hit temporary-file quota and PATH errors.

Workspace formatting, whitespace checks, strict OpenSpec validation, and the
docs OKF bundle pass at this checkpoint. The bundle has no errors or warnings;
its TSV link is informational. Full workspace tests and final Clippy remain
pending after test and comparison-runner corrections.

The first new paired benchmark stopped at its correctness gate before collecting
timed pairs. The starting revision emits RS-framed TOON by default; the current
revision emits standalone TOON. Primary verified that the 9,227,458-byte baseline
and 9,227,456-byte candidate outputs differ only by the baseline's RS and LF.
The retained attempt is `.work/jq-parity-perf-20260909-composition1/` in the
companion repository. The rerun must request `--unframed` on both revisions for
the single-document TOON workload, preserving exact byte comparison and all
performance thresholds. No mismatched sample counts as performance evidence.

### Managed admission and regex checkpoint

The compile-only guard now attempts every one of the 220 documented signatures
inside a called definition. It preserves arities, rejects duplicate inventory
entries, and reports failures by compiler stage without executing the VM.
Primary source review confirmed that this tests admission, not semantic parity.
The inventory lists `INDEX/2`; the separately tested `INDEX/1` overload is not
part of this 220-signature denominator.

The initial native Linux guard reported 21 compile-admission failures, with no
parse or resolution failures. Input, effects, SQL, and path callbacks remain
implementation work. These failures must not become approved disparities.
Primary inspected the raw build and run logs in
`target/linux-review/composition-20260909-admission-checkpoint2/` and verified
their hashes against the recorded provenance. The bounded test exited 101 with
the complete compile-failure list; it did not time out.

Primary independently ran all eight `user_regex_composition` tests successfully
after correcting pattern/flag ordering and split-piece admission before copying.
The tests cover replacement scope and cardinality, late errors, catchable type
errors, replacement quotas, and early termination without copying an unused tail.
Task 3.13 remains open for the other callback families and final verification.

The next primary checkpoint passed ten pull-composition tests and two truncation
unit tests. Truncation preserves retained null events and extra fields and
checks copy bounds. Ten independent path-accumulator unit tests also passed
after review removed a retained-width limit incorrectly tied to pending forks,
corrected missing-index padding, and replaced a changing sort comparator with
fallible in-place sorting. These helpers still require dispatcher integration.

Further primary review added CLI regressions for `halt_error/0` and `/1`, plus
the different evaluation orders of scalar filter arguments and user value
parameters. Pinned jq confirms their exact status, output, and effect payloads.
The first macOS `halt_error` attempt timed out during process startup before
semantic assertions. It is unverified, not a passing or differing observation.

A later primary run passed all eight then-current CLI effect tests, including
both `halt_error` forms, scalar argument-effect ordering, and live early output.
The core effect suite also passed eight tests, including debug messages retained
before a late callback error. Two called-definition numeric-path tests passed.
An additional CLI identity regression remains pending: `debug(empty)` preserves
its input's path identity, but a source line number must receive a fresh identity
even when it equals that input. These are focused checkpoints, not final native
campaign or full-spec acceptance.

Primary subsequently passed all 197 then-current core library tests. A new
native macOS admission run reduced the compile failures from 21 to 11, all in
SQL and path callbacks, with no parse or resolution failures. Its raw log is
`target/diagnostics/composition-admission-native-11-20260909.log`, SHA-256
`a6503c3adda9ed9d9f60c59790139493dab941201d718d76470e2c82a8b29fb8`.
The test binary's observed SHA-256 was
`77b65b11ea84be132a027b97aab64373068d0a360edb28396208adec9b68a293`.
The additional CLI identity test hit the child startup deadline; a direct
controlled-home probe of the metadata case produced the pinned jq result.
That direct probe does not replace the pending complete CLI rerun.

The next source-review round found eager optional-argument indexing in JOIN,
SQL error paths that could leave a buffered finish task pending, and path
traversal root-exclusion and depth-accounting issues. Fixes and focused
regressions are in progress; the earlier 197-test result predates these edits.
The caught buffered JOIN lookup regression was checked against pinned jq 1.8.1
and requires only the caught result, with no trailing partial collection.

The completeness ledger now retains its 220 signature rows and links 71 clause
evidence entries across 17 clauses. Added links cover called path, SQL,
recursive, and streaming tests, including SQL resource tests. Primary checked
all 68 named test references in the ledger against their source files; none
was missing. These links establish executable coverage, not passing acceptance.

Primary subsequently passed all 41 focused tests across path origins (9), path
builtins (14), relational composition (8), SQL resources (4), and recursive
composition (6). Review corrected two new identity-test expectations against
pinned jq: binding a constructed result against the original input is not an
assignment alias, while binding the constructed current input after a pipe is
assignable. The tests now check both forms and invoke called definitions.

All three composition-inventory tests and both completeness tests passed on
macOS. The compile-only guard now admits all 220 documented signatures, with
zero parse, resolution, or compile failures. The raw inventory rerun is
`target/diagnostics/composition-inventory-all-20260909.log`; its observed test
binary SHA-256 is
`5184b166ef36e26df6ef5832a1aa6b799eb2cb7bf4d54ac44650b57ea747d8f2`.
This is admission and structural evidence, not a 220-case semantic parity claim.

The independent Ironhide checkpoint also passed 197 library tests and the same
41 focused tests. Primary inspected their raw result lines and verified source
archive SHA-256
`7f869c5d4fd4781014353fc22afcad69739c4041e414abe4cc60bafb3d76ccef`
against recorded remote provenance in
`target/linux-review/composition-20260909-checkpoint2/`. Its frozen `sql_compat`
suite passed four tests and failed one stale expectation: jq returns one global
boolean from two-stream IN, not one per right-side result. Primary verified
`IN(1; (1,2))` emits only `true` and empty-right IN emits `false`, then corrected
both local expectations. The frozen Linux test remains recorded as failed.
Linux support-test builds hit a temporary-filesystem quota; a separate disk
temporary directory is being evaluated without removing source or saved evidence.

Broader core regressions and warnings-as-errors checks remain in progress.
The first clippy run found style findings and two test calls to a removed
production-unused helper; those require correction and a fresh verification run.

The broad primary core run finished with 474 passing tests and one failure
across 32 top-level suites (excluding a filtered resolver child reexecution).
Its raw log is `target/diagnostics/core-composition-reviewed-20260909.log`,
SHA-256 `a38df93ed8884e22e77be9b3ca122b70fb329eddc357feec1533e6bd74d95b97`.
The corrected direct SQL suite passed all five tests, alongside eight relational
composition and four SQL-resource tests. This accepts task 6.3. Captured
environments, scalar/generator composition, and their resource suites passed,
supporting acceptance of tasks 3.4 and 3.12. Final campaign acceptance remains
separate under tasks 3.14 and 10.3.

The one broad-run failure was `.[] |= empty` on `[1,2,3]` returning `[2]` rather
than jq's `[]`: immediate deletion shifted later original indices. The revised
implementation uses the reviewed bounded path accumulator for deferred deletion,
including overlapping slice unions, and reads prior replacements from the
current document. New direct/called, overlapping-slice, retained-width, and
tight-work regressions subsequently passed the primary 74-test run across
manual grammar, path updates, path resources, and pull composition. Primary
then reran all six path-update tests with two additional pinned RHS-alias cases;
all passed. This accepts task 3.11 together with the earlier fold and origin
review, without replacing the pending final campaign.

All nine CLI effect-composition tests subsequently passed independently with
unchanged two-second child deadlines, including source-metadata identity and
live output before EOF. The preceding run had nine startup timeouts and remains
failed evidence. The successful run used the exact Cargo-built test executable
after warming the CLI, without relinking. Together with the earlier core effect,
regex, recursive, and pull suites, this accepts task 3.13. Workspace type-checking
is clean; final Clippy still has test-only raw-string style findings to resolve.

### Composition inventory review

Task 3.9 passed primary source review after correcting nominal family links that
did not exercise their named operations. The supplemental inventory contains
220 per-arity definition cases, 29 executable operation witnesses, and 48
family/context cases. The remaining bytecode variants have explicit kernel-only
rationales. Guards preserve original queries, fixtures, invocation modes, and
adapter setup, and deleting a supplemental review mapping fails the strict gate.
The Luna implementer passed the inventory test, eight catalog tests, the mapping
deletion regression, and pinned jq audits. The four nonzero jq arity contracts
remain intentional error/process witnesses. This accepts the audit and failing
regression inventory, not their unfinished tq implementation or final campaign.

### Fold and path checkpoint, not accepted

The primary independently passed the five fold and five path tests after the
first whole-slice and user-path implementation. Task 3.11 remains open. Review
found ordinary-value bindings misclassified as paths, stale alias metadata after
shadowing, and a negative test that asserted only one result instead of proving
failure. The implementer is correcting these cases against pinned jq. Passing
the focused suite is not final acceptance of path composition.

Further pinned-reference probes accept both a conditional root alias and a
root passed through a value parameter:

```jq
{a:true,x:1} | ((if .a then . else 7 end) as $p | $p.x) = 2
{x:1} | (def p($v): $v.x; p(.) = 2)
```

The results are `{a:true,x:2}` and `{x:2}`. A static classification of the whole
binding expression is insufficient: the selected branch may produce either a
path or an ordinary value. Review requires runtime provenance without repeating
the expression's effects. These regressions remain part of task 3.11 acceptance.

### Built-in helper review in progress

The scalar helper's five focused unit tests pass. Public composition integration
is still pending, so this is not acceptance of task 3.12. Primary review also
found that `range(nan; 3)` must remain pull-driven and emit NaN values, not an
empty stream. Pinned jq emits `[null,null]` for
`[limit(2; range(nan; 3))]`. The bounded generator regression is being corrected.
Regex cursor cancellation documentation distinguishes checkpoints surrounding
an engine search from the engine's own work limits; cancellation does not
interrupt a search already in progress.

An independent core-library run passed 135 tests and failed two existing resource
contracts after folds and assignments entered the managed evaluator. A fold with
`value_stack: 0` did not fail, and `.a.b = 2` failed to record path-stack usage.
Both require implementation fixes, not weaker assertions. A separate independent
run passed all 27 regex, fold-composition, and path-composition tests at that
checkpoint. Later provenance changes still require a fresh run.

The subsequent independent core-library run passed all 152 tests, including
both resource regressions. A fresh run of the value-origin, wrapper, path, and
fold integration suites passed 24 tests. This verifies the checkpoint, not
the remaining builtin integration or full task 3.11 acceptance.

The separate builtin-composition suite still fails all nine tests at the
managed-admission guard. The public composition-resource suite passes six
tests and rejects the two witnesses requiring `sqrt` and `debug/1`. These are
unfinished dispatch paths, not accepted disparities. Strict OpenSpec validation
and the working diff's whitespace check pass at this checkpoint.

After concrete scalar/generator dispatch was connected, the primary reran the
builtin-composition suite: all nine tests passed. This is the first integrated
checkpoint for task 3.12, including argument order, decimal range output,
bounded generators, combinations, and stream-event output. Broader regressions,
legacy helper delegation, and the remaining callback families are still pending.

The next independent checks passed 159 core-library tests and seven of eight
public composition-resource tests; only the deferred `debug/1` witness failed
admission. The origin/fold/path run passed 24 tests and retained the known
dynamic-index failure. Legacy `transpose` and `bsearch` now delegate to their
shared helpers; pinned jq confirms `bsearch` evaluates its argument before
rejecting a non-array input.

A frozen debug-binary diagnostic covered all 220 arity rows: 155 diagnostic
matches, 64 rejected rows, and one full-list `builtins/0` mismatch, without
timeouts. This is not strict campaign evidence: its Python JSON parser uses
binary64 numbers, and expected-error comparisons check status rather than the
full process contract. Its binary SHA-256 is
`205677d30c9b9fe55bfe99a930a7d01d5f8034c345b98d5a3d132f8621424d79`.
The rejected rows are not 64 distinct missing dispatches: `first/1` and
`last/1`, for example, share a witness containing the missing `nth/2` call.
The supplemental `builtins/0` witness is being corrected to the manual's
availability contract rather than exact jq-private registry enumeration.

Pinned argument-order probes also distinguish built-in and user-call dispatch.
`[pow((2,3);(2,3))]` yields `[4,9,8,27]`; `range` and user-function value
parameters use first-argument-outer order instead. The public composition test
now includes the direct `pow` witness inside an invoked definition. Tests also
cover decimal representation through `tojson`, no-progress ranges,
`combinations(empty)` on null, object iteration, and nested stream closes.
They remain failing admission tests until task 3.12 is integrated.

The composition admission audit currently accounts for 220 documented arities:
42 admitted, 116 rejected with scalar/generator helpers ready, and 62 rejected
without helper coverage. Helper coverage totals 138 rows, including 22 already
admitted rows; it is not 138 additional implemented arities. The remaining
callback work includes collection predicates/grouping, pull consumers, path
built-ins, relational streams, stream reconstruction, regex, input/effects, and
platform access. Final acceptance must execute these cases after integration.

### Runtime-origin and stream-allocation review

The path work remains incomplete while runtime identities replace the remaining
syntax-derived alias decisions. An initial address-keyed environment registry
was rejected in review because allocation reuse could associate stale metadata
with a new environment, and retaining historical entries would grow state.
The implementation now keeps origin metadata with its lexical environment.
Scalar identities and values that later become the assignment input still need
complete regression evidence.

Primary review accepted the new allocation-aware `eval/path.rs` helper after an
independent ten-test run. It checks resulting container lower bounds before
allocation, uses checked growth and fallible reservation, and charges traversal,
copying, and filling. Review corrected the initial per-call quota, which could
be bypassed through repeated small growth. Tests cover indexed overflow,
incremental array/object growth, mid-copy/mid-growth failure, unchanged roots,
key order, and structural sharing.

The pull-fed stream helpers now use this replacement helper. Their nine focused
generator tests also passed independently before the final quota refinement.
Legacy and managed assignment call-site delegation is still pending. These
checks accept the helper slice, not tasks 3.11 or 3.13 as a whole.

The expanded path helper passed 16 tests independently. Review then caught an
inherited slice-assignment mismatch: jq rejects a scalar replacement, while the
old code inserted it as one element. The helper now requires an array and has
rejection regressions; all 17 tests also passed independently. Legacy call
sites still need delegation. A new nested dynamic-index origin witness also
remains wrong (`1` instead of jq's `20`), despite the earlier focused origin
checkpoint passing. Both corrections are required before path acceptance.

Read-only diagnosis traced the index failure to a normal binding body entering
path-selection mode. The continuation needs an explicit normal/path context,
not another expression-syntax exception. Retained-input operands also need to
carry the caller's origin through slice bounds and other deferred evaluations.
Builtin dispatch must distinguish identity-preserving branches from computed
results: pinned jq preserves identity through `numbers`, nonnegative `abs`
(including negative zero), and string `tostring`, but not through `fabs`.

The subsequent independent path checkpoint passed all 37 tests across value
origins, path resources, path updates, folds, and builtin composition. The
normal/path-context correction fixes the dynamic-index witness. Assignment,
deletion, and slice replacement now delegate to bounded helpers. Review still
found an uncharged `to_vec` when materializing a selected old slice. The
replacement uses fallible reservation and charges each copied element; the
primary independently passed all seven path-resource tests, including the new
partial-slice regression. Pre-allocation quota admission is receiving a final
check. This is not acceptance of the remaining path-function and callback
composition scope.

### Collection composition review in progress

The first independent collection run passed five tests and failed three.
`any`/`all` leaked intermediate predicate values, and `max_by` selected the
minimum for unequal keys. Tests now separate extrema error checks, verify
distinct-key grouping and stable ties, and exercise all predicate arities through
called definitions. These failures remain implementation work.

Scalar resource review rejected using the VM's default 4,096-slot value stack
as a blanket limit on ordinary input containers. Allocation and work bounds
must preserve admitted large-input behavior. The new pull/stream and input/effect
composition tests also require successful execution before acceptance.

The subsequent independent run passed all 46 tests across builtin, collection,
fold, path-resource, scalar-resource, and value-origin suites. This verifies
the corrected predicate delivery, extrema selection, scalar materialization
guards, and the covered origin branches. The scalar guards preserve large flat
inputs and duplicate-key reductions. Interior-whitespace trim origins,
cumulative grouped-output admission, shared helper delegation, and the remaining
pull, relational, regex, and input/effect families still require completion.

The next independent run passed 28 tests across pull/stream composition,
collection composition, and value origins. It includes cumulative grouped
materialization admission and interior-whitespace trim identities. Pull result
origins and nonnumeric `truncate_stream` behavior remain under review; passing
the output-value examples alone does not accept those paths.

The primary independently passed all six concrete path-builtin composition
tests, including captured `getpath`/`setpath`, argument ordering, `delpaths`,
empty and late-error streams, depth bounds, and reconstruction work limits.
This checkpoint used `user_path_builtin_composition-4f68bc974e12d759`.
The broader path task remains open. Review of ambient composition corrected an
extra array nesting in a test oracle against pinned jq. Regex integration still
needs catchable setup errors and pull-driven `splits` before acceptance.

The primary also passed all three resolver-stack regressions from the copied
`resolve-stack-primary-rUyTv2` executable. They cover an explicit 2 MiB worker
stack, a long imported module with location preservation, and paired-operator
ordering. The bounded run completed outside the sandbox in 0.03 seconds after
sandboxed startup attempts timed out. Later pull-test launches also timed out
outside the sandbox, so sandboxing alone is not an established explanation for
the intermittent startup stalls. These timeouts are not passing test evidence.
Strict OpenSpec validation passed again; implementation acceptance is unchanged.

The original focused Cargo run subsequently completed successfully. It passed
29 distinct tests across resolver-stack, pull composition, concrete path
builtins, and value origins. Cargo also reports the nested resolver child
execution, producing 30 reported passes across five harness runs. The long
launch delays are separate from the reported 0.07-second test-body duration.
Nonnumeric truncation, iteration origins, and remaining callback families are
still unfinished.

The primary reran the complete `composition_inventory` integration target after
the resolver correction. Both tests passed, including the original 220-signature
availability witness that previously exhausted the small stack. This verifies
the inventory contract, not execution parity for every inventoried builtin.

The following independent checkpoint passed 197 tests: 175 core-library tests,
19 regex compatibility tests, and three ambient composition tests. This includes
the SQL helper's eight unit tests and the corrected ambient oracles. Managed
regex acceptance still needs explicit called-function coverage across all forms
and bounded copying of split slices; legacy regex tests alone do not establish
user-filter composition. SQL helpers are tested but dispatcher integration is
not yet complete.

The next independent local checkpoint passed 213 tests across the core library,
pull composition, value origins, and concrete path builtins. This includes
bounded `fromstream` path decoding, iterator child identities, split-piece helper
limits, and effect-buffer no-partial-write protection. Managed regex ordering
and split-helper wiring still have explicit failing regressions. Truncation's
extra-field handling and bounded path construction remain under review.

The fresh Ironhide composition checkpoint used source archive
`24fa14db8e00e06c60eb737a318d18806f1bd4edfd08b9aa26d8c27aeab9b29d`
and explicit stable Rust 1.98 compiler paths. The primary inspected the copied
logs and provenance under `target/linux-review/composition-20260909-checkpoint1/`.
They record 207 distinct passing tests across the library and five focused
suites. All five input/effect tests remain admission failures. All six relational
tests fail, including an existing `IN` empty-right-stream bug that returns no
value rather than `false`. These are unresolved implementation gaps, not approved
disparities. This checkpoint does not replace the final native manual campaigns.

### Regex substitution state checkpoint

The new owned substitution state separates regex pulls from replacement-filter
execution. It preserves the existing zip/branch semantics and exposes explicit
pending-stream and terminal-error states. Review replaced infallible input-slice
copies with borrowed slices through an owned `Arc`, added immediate replacement
size admission, and made bounded string growth fallible. The primary passed all
183 core-library and regex-compatibility tests after these corrections. Managed
callback integration is still pending under task 3.13.

### Builtins availability test investigation

The supplemental availability witness now subtracts all 220 documented
signatures from the registry and produces exact `[]` in both frozen binaries.
Its public VM inventory test nevertheless ran for over ten minutes; an isolated
five-second invocation reproduced the timeout. The existing inventory-only
control passed in 0.10 seconds. The owned long-running process was stopped, and
a fresh bounded probe is being minimized. The new inventory test is not counted
as passing while this failure remains unexplained.

### Bounded constructor and slice review

Task 3.10 passed primary review. Objects and slices now use explicit resumable
continuations, including computed keys, Cartesian field and bound generators,
captured inputs, partial errors, and short-circuit consumers. Review corrected
runtime negative-zero slice bounds without losing negative fractional bounds.
Independent checks passed 13 public core composition tests, 125 core library
tests, and the CLI composition test. CLI assertions also require empty stderr.
This does not approve the remaining fold, path, built-in, or effect composition
tasks. Final native campaigns and performance evidence remain pending.

### Approval serialization and ordinary token counting

Primary review accepted exact decimal-value comparison in structured approval
observations. Equivalent expanded and scientific decimals compare equally;
stdout, stderr, status, result order, types, and signed zero remain protected.
The regression uses a self-contained synthetic report, not an untracked
`target/` artifact. The primary independently passed all 24 approval-gate tests.
Native Linux completion still requires a fresh run.

Fixture token counting now uses ordinary-text encoding for both tokenizers.
Literal special-token markers are data, not protocol tokens. The previous
encoding undercounted a marker-containing witness. Final generated reports must
use the corrected counter.
The primary independently passed all seven comparison unit tests. The corrected
single-result fake fixtures use standalone TOON; multiple-result fixtures retain
explicit sequence framing.

### Approved composition implementation revision

The user approved revising the plan to remove the managed-composition
restriction. Proposal, design, user-function requirements, and tasks now cover
all admitted operations in definitions and around calls. Tasks 3.9 through
3.14 separate inventory, constructors/slices, folds/paths, built-in families,
effects/callbacks, and integrated verification. No unsafe bridge, native-stack
recursion, or eager fallback is authorized by this revision. Implementation
continues with Luna X-high and primary reviews.

The Linux checkpoint approval round-trip also remains unresolved. Completion
with `disparities-x86_64-linux.toon` exited 2 on a stale
`manual.audit.math.integer-scale-boundary` observation and emitted no report.
Raw evidence is retained in `target/linux-review/linux-v2-raw-compare.toon`.
Human review of the six disparities does not override that validator failure;
lossless typed deserialization is under investigation. No successful Linux
completion gate is claimed for that checkpoint.

### Current-source native checkpoint and reopened composition gap

The current macOS CLI, SHA-256
`ef0fc5555acbf373a6680f14b0d8d5b67c504586afc18dc16fdbf468ce1d80b2`,
passes the nine-workload benchmark in companion archive
`.work/jq-parity-perf-20260908-review6/`. Median wall-time ratios range from
0.972 to 1.125 and peak-RSS ratios from 0.984 to 1.128. All outputs match
the baseline exactly. The numeric-array ratios are 1.020 wall and 1.055 RSS.
Builds and tests were paused during measurement. Formatting now passes after
concurrent report edits were formatted by their owner.

Primary independently reran the pinned macOS campaign and compared all stable
observations against the previously accepted executable. Every fingerprint,
ordered value, stdout/stderr payload, status, compact-byte observation, and
TOON result for the five disparities is unchanged. The independently rerun
completion gate exits 0 with 603 exact matches and five disparities. The
concurrently refreshed macOS registry matches those observations exactly.
This comparison does not approve unrelated token-renderer edits.

The current-source Ironhide build uses explicitly selected Rust 1.98 compiler
and rustdoc. Local and remote source archive SHA-256 matched
`b2d2b89c17423f55ab3ace8d583c81568373cbd27c0c9ec7cae722c0ce4732a7`,
with critical file hashes checked separately before compilation. Native tq
SHA-256 is `f774395e856f103f13eb0d0cf3fc7357d1be2097dc442fd566f5be92136d5e3b`.
The full workspace log `target/linux-review/final-v2-stable-workspace.log`
contains 720 passes, zero failures, and two ignored reference tests across
77 suites. It includes native controlled date/ambient-policy, POSIX shell,
PTY color, math, and corrected CPU/RSS benchmark tests. Optional non-Windows
PowerShell tests are not native Windows evidence.

All fourteen companion manual files were transferred and their pinned digests
verified. `target/linux-review/manual-linux-v2-source-verified.toon` records
608 cases, 602 exact matches, six differences, compact 575/581, and TOON
581/581. The signed-zero min/max case now matches exactly. Primary reviewed
the six remaining witnesses and the 92-input native CLI probe before creating
the narrow Linux registry. Linux probe results are 81 exact and 11 differing
samples, with no process issues and all five reference runtime hashes checked
before and after. The same macOS probe records 77 exact and 15 differing
samples. Sample-specific ULP maxima and practical restrictions are documented
in `docs/jq-compatibility-disparities.md`.

Final review then found a separate language implementation gap:
`def f: {tool}; f` succeeds in pinned jq but fails with exit 3 and
`TQ-CAP-USER-FUNCTIONS` in tq. The managed user-function execution path rejects
object constructors, including objects around otherwise valid user calls.
Task 3.4 is reopened. This is not a safe-library disparity and blocks final
acceptance. The above binaries and reports remain checkpoint evidence; they
must not be described as the post-fix release evidence.

The follow-up inventory shows this is a broader execution-design issue, not
just one missing object branch. Primary independently reproduced these four
programs with controlled HOME, `LC_ALL=C`, and `TZ=UTC`:

| Program | Input | jq compact stdout | tq |
| --- | --- | --- | --- |
| `def f: {tool}; f` | `null` | `{"tool":null}` plus LF | Exit 3, no stdout |
| `def f: .[0:1]; f` | `[1,2]` | `[1]` plus LF | Exit 3, no stdout |
| `def f: sqrt; f` | `4` | `2` plus LF | Exit 3, no stdout |
| `def f: reduce .[] as $x (0; .+$x); f` | `[1,2]` | `3` plus LF | Exit 3, no stdout |

Every jq invocation exits 0 with empty stderr. Every tq invocation emits
`tq: query compilation failed: TQ-CAP-USER-FUNCTIONS: user filter composition is not executable by the managed runtime`
plus LF on stderr. `managed_operation_supported` in `eval.rs` excludes these
operations or built-ins, and `user_execution_gap` converts that execution
restriction into a compilation error whenever a user call is reachable.
The specification says unsupported optimization must not become unsupported
language and promises jq-compatible user-defined filter composition.

No evaluator workaround or collector fallback was added. The OpenSpec apply
workflow pauses here for a design decision: revise the implementation plan to
close the managed-composition restriction with bounded continuations and
cross-operation tests. Merely whitelisting operations without implementing
their managed execution would not fix the issue. Passing all concrete imported
examples is insufficient evidence for the general composition requirement.

### Seven-delta evidence map

Primary review reread all seven deltas, including the explicit Windows deferral.
The implementation and executable evidence are grouped below. The platform
rows remain provisional until native Linux failures are resolved; this map does
not replace the task checklist or approve an unexplained discrepancy.

| Delta | Implementation | Executable evidence |
| --- | --- | --- |
| `cross-tool-compatibility` | `manual.rs`, `manual_pin.rs`, comparison normalization, and `tq-manual-compare` | `compatibility_manual_gate`, `manual_pin`, `manual_completeness`, `manual_portability`, and the pinned campaigns |
| `extended-jq-cli-parity` | CLI option resolution, capability policy, color writer, and test-file runner | `extended_cli`, `manual_arguments`, `colors`, `effects`, and independent compile-error fixtures |
| `jq-core-language` | Parser/resolver, VM generator and path execution, numeric values, safe math, collection/string built-ins | `manual_grammar`, `runtime_numbers`, `math_compat`, `manual_signature_matrix`, collection/string/recursive/SQL tests, and boundary witnesses |
| `jq-reduce-foreach` | Shared binding patterns and VM accumulator continuations | Advanced manual cases, grammar tests, and `advanced_execution_equivalence` |
| `jq-regex-date-platform` | Bounded fancy-regex integration, Jiff date conversion, capability-controlled ambient reads | Core regex/date tests, CLI regex/date/resource tests, and native platform campaigns |
| `jq-user-functions-modules` | Lexical filter capture and canonical module/data-import resolution | `modules`, advanced grammar/equivalence tests, and manual module cases including executable-relative lookup |
| `tq-cli` | Output selection, shared input cursor, sequence scanner, stream reconstruction, and non-catchable process termination | `compact_selection`, `input_streams`, `path_streams`, `effects`, native-format regressions, and independent JSON/TOON campaigns |

Coherence review retains one evaluator across formats, separate exact and
reviewed-disparity gates, safe Rust libraries, immutable generator branches,
bounded engine work, and CLI-versus-embedded capability policy. No manual case
ID controls production evaluation, and jq is not a production subprocess.

### User decisions and current acceptance scope

The user authorized a Rust upgrade and scoped Linux testing on Ironhide,
including the source transfer needed for that testing. Rust 1.88 is sufficient;
the workspace declaration and installation documentation now name that floor.
Explicit Rust 1.88 compiler and rustdoc checks and the full workspace tests pass.
Together with the final nine-workload benchmark below, this resolves the macOS
and minimum-Rust portions of task 10.4. The initial Linux run subsequently
reopened the task for missing PTY tooling and benchmark CPU/RSS sampling failures.
The post-declaration verification used `target/msrv-188-msrv-bump` and retained
`/tmp/msrv-188-msrv-bump-workspace-tests.log`: 72 suites, 717 passes, zero
failures, and two reference-dependent tests ignored in that run. Their separate
successful pinned-reference execution is recorded below.

The authoritative repository standards are the user-level bundle at
`/Users/reno/.agents/memory/repo-man/`, as confirmed by the user. Review read its
applicable Rust, Bash, documentation, OpenSpec, and release-target standards.
The complete `docs/` bundle passes OKF validation with zero errors or warnings.
New helper and probe documentation filenames follow the naming standards.

Native Linux testing is in progress. Native Windows testing is explicitly
deferred until a runner is available. Its tests remain, but neither their
presence nor non-Windows PowerShell execution counts as Windows evidence.
The proposal, design, compatibility delta, task checklist, and release workflow
record this deferral. Final synchronization and archival still await Linux and
the final integrated review. No implementation commit or push was performed.

### Earlier acceptance blockers, superseded by the user decisions above

Native Linux and Windows evidence remains missing for tasks 8.5, 10.1, and
10.2. macOS PowerShell tests are not Windows evidence. The scoped private-source
transfer to the available Linux host was denied; no alternative transfer was
attempted. Continuing that route requires explicit user authorization and a
native Windows runner must be identified.

Task 10.4 awaits the minimum-Rust decision. Keeping the declared 1.87 floor
requires addressing pre-existing let chains and the pinned JSON5 dependency;
correcting the floor to 1.88 requires user approval. The actual 1.88 compiler
and rustdoc pass, but that does not make the declared 1.87 floor truthful.

Task 10.6 cannot synchronize/archive an incomplete change. The configured
parent standards checkout is absent. Final standards review requires either
restoring `/Users/reno/Development/commandzero/repo-man` or approval to treat
the cached `/Users/reno/.agents/memory/repo-man` bundle as authoritative.
OpenSpec strict validation passes; planning-artifact completeness is not task
or release acceptance. No implementation commit, push, or archive was performed.

### Final macOS completion checkpoint

Task 10.5 is also accepted, bringing the final local checklist to 57/62.
After report regeneration, the complete test-support suite passed 168 tests
with two reference-dependent tests ignored; both had passed explicitly above.
The renderer matches the checked-in Markdown. Historical-verdict tests now
use the immutable execution ledger rather than treating the rolling report as
the old 303-match baseline. The replacement-ID test selects an actual pinned
original case by ID instead of relying on report row order. These corrections
change tests only; the production gate and final executable are unchanged.

The final release executable has SHA-256
`ebda721eb84f8a728ee397125fa7d707c8564dc7525931eadd24e92af7b71eeb`.
The regenerated checked-in manual comparison records 608 cases, 603 exact
matches, five reviewed disparities, and zero unresolved failures. It preserves
all 518 original cases and all 303 original exact matches. Compact JSON has
576 of 581 exact matches and the same five reviewed differences; TOON preserves
all 581 successful JSON execution contracts. The pinned source checkout and
jq executable were verified. Exact mode exits 1 on those five differences;
completion mode exits 0 without suppressing their observations.

Primary review renewed only the existing five approvals. Case fingerprints,
ordered values, compact bytes, stdout/stderr, and process statuses were unchanged
from their reviewed predecessors. No new input or numeric tolerance was added.
The final 66-input end-to-end math probe also reproduces 55 exact and 11
last-bit differences, with unchanged sample-specific maxima of 1/1/2/2 ULP.

The final Rust 1.88 compiler and rustdoc were explicitly selected. Its
locked/offline all-target workspace run passed 717 tests with zero failures;
the two reference-dependent tests subsequently passed when run explicitly.
The executable-origin test now works with an alternate Cargo target directory.
Primary process review generated 37 compile-failure fixtures from pinned jq,
then both jq and tq passed all 37 with identical accounting and empty stderr.
Workspace formatting, Clippy, and debug/release builds passed. Clippy retains
the baseline unknown-lint warning described below.

Tasks 9.4, 9.6, 10.3, and 10.7 are accepted, bringing the checklist to 56/62.
This is macOS evidence, not native Linux or Windows acceptance. Final report
renderer checks, the final-build benchmark, the declared minimum Rust decision,
and remaining platform/standards gates are tracked separately.

The final-build nine-workload benchmark subsequently passed every limit.
Wall-time ratios ranged from 0.965 to 1.137; peak-RSS ratios ranged from
0.977 to 1.140. The numeric array measured 0.965 times baseline wall time and
1.026 times baseline RSS. Every measured output matched the baseline exactly.
Evidence for the same final executable is retained in the companion archive
`.work/jq-parity-perf-20260908-review5/`. All builds/tests were paused during
measurement. Task 10.4 still awaits the declared minimum-Rust decision; these
measurements do not resolve the pre-existing Rust 1.87 compilation failure.

### Integrated diagnostic verification

Workspace formatting, the locked/offline release build, and Clippy completed.
Clippy reports the existing workspace allowance for unknown
`clippy::assert_is_empty`; no code lint failed with `-D warnings`.
The latest explicitly pinned Rust 1.88 all-target test log records 716 passes,
one diagnostic-rendering assertion failure, and two ignored reference-dependent
tests. The corrected executable-origin module test passes. This captured log
supersedes the earlier agent summary reporting two diagnostic failures.
Task 9.4 remains unchecked while those failures are corrected. Earlier focused
oracle checks do not supersede the integrated test failures.

### Direct-value input performance review

The primary nine-workload paired benchmark passed every 50% wall-time and
20% peak-RSS limit for candidate SHA-256
`154a0f4924e1f3812a85c43436d824b98f6fe137b9b76b4a4f897a39f1b1531a`.
Exact output equality was required for every warmup and measured sample.
The numeric-array workload measured 1.060 times baseline wall time and
1.022 times baseline RSS, resolving the earlier 1.27-times RSS regression.
Direct deserialization into the core value avoids an intermediate JSON tree;
the preceding integer-serialization optimization alone did not resolve it.

The other eight wall-time ratios range from 0.993 to 1.073 and RSS ratios
from 1.023 to 1.125. Builds and tests were paused during the measurements.
The companion archive `.work/jq-parity-perf-20260908-review4/` retains the
script fingerprint, executable identities, fixture hashes, individual outputs,
timing samples, and TOON report. This candidate predates the final diagnostic
edits; the final integrated build still needs its own acceptance evidence.

### MSRV verification correction

Earlier commands invoked Cargo 1.87 through rustup but still selected the
Homebrew `rustc` from PATH. Those successful checks are not Rust 1.87 compiler
evidence. Pinning `RUSTC` explicitly to
`/Users/reno/.rustup/toolchains/1.87.0-aarch64-apple-darwin/bin/rustc` exposes
unstable-let-chain errors in first-party code and `json5` 1.3.1. The original
revision already contains let chains in `ValueVisitor` and the JSON5 dependency;
the declared minimum and its resolution need separate review. No minimum-version
bump or dependency downgrade has been made. Task 10.4 remains open, and every
earlier claim of a passing Rust 1.87 check in this chronological log is superseded
by this correction.

### Reopened runtime-zero min/max boundary

Resolved in candidate SHA-256
`154a0f4924e1f3812a85c43436d824b98f6fe137b9b76b4a4f897a39f1b1531a`.
Primary six-output process probes match exactly, including both-negative-zero
operands. The new twelve-result campaign witness also covers both operand
orders and NaN fallback. The expanded 608-case report has 603 exact matches,
576 of 581 exact compact matches, and 581 of 581 TOON matches. Every stable
observation and case fingerprint for the five previously reviewed disparities
is unchanged; their executable-bound approvals await the final build.
Task 5.6 is accepted again, restoring 52/62 completed tasks.

The evidence is `target/parity-direct-value-origin-checkpoint.toon`. An earlier
run placed the candidate in an extra Cargo target-directory level and failed
the executable-origin module fixture. Copying the identical hashed executable
under `target/release/` restored the fixture's declared `$ORIGIN` layout and
preserved every original match; no module behavior was changed.

Original finding:

Task 5.6 is reopened. The query
`[fmin(copysign(0;-1);0),fmin(0;copysign(0;-1)),fmax(copysign(0;-1);0),fmax(0;copysign(0;-1))]`
produces `[-0,-0,0,0]` in pinned jq and `[0,0,0,0]` in release checkpoint
`71eaa6cdf880b438e4621551742d05c302ef0c034337253dde1463883a3366ce`.
Both processes exit 0 without stderr. Earlier literal `-0` tests normalized
their input to positive zero before the function and did not cover this case.
This is a fixable implementation failure, not an approved library disparity.
The current count is 51/62 until the implementation and a compact-byte campaign
witness pass review. The 607-case completion result below remains evidence for
that frozen corpus, not proof that this newly found boundary works.

### Reviewed-disparity completion checkpoint

The identity-bound macOS approval registry now covers only the four recorded
math observations and the explicit longest-regex restriction. Primary review
accepted those exact witnesses under the user's safe-Rust policy; supporting
66-input math probes establish sample-specific observations, not global ULP
bounds. No source correction, missing feature, timeout, or former baseline
match received an exemption.

`target/parity-approved-checkpoint.toon` passed the completion validator with
602 exact matches, five reviewed disparities, and zero unresolved failures
across 607 cases. Compact output has 575 exact matches and the same five
reviewed disparities; TOON preserves all 580 applicable executions. The exact
gate remains failing by design. Reference/source identities and all original
518 cases and 303 matches remain protected.

This accepts tasks 1.6, 5.4, 5.5, 5.7, 5.8, 6.7, 8.2, and 8.4 on the verified
host, bringing the checklist to 52/62. The 220-signature inventory and all
fourteen section ledgers have executable links; focused completeness tests
reject claimed coverage without cases. Final reports and approvals must be
regenerated after the pending diagnostic and memory changes. Other native
targets, final performance, and release acceptance remain open.

### Release checkpoint and numeric memory regression

Task 5.6 passes primary review on the matched macOS reference. Rounding,
sign, remainder, neighboring-value, min/max, and fused-operation witnesses
match in the 607-case campaign. Focused math tests cover ties, signed runtime
zero versus literal unary-zero normalization, adjacent subnormal values,
remainder signs, and fused evaluation. The four unresolved last-bit witnesses
belong to inverse trigonometric, exponential, error, and Gamma functions;
they remain separately measured, not silently tolerated by this acceptance.

The integrated workspace passed 715 tests (two reference/relocation tests are
explicitly ignored by the default run), the release build, and the Rust 1.87
all-target check. Release executable SHA-256
`71eaa6cdf880b438e4621551742d05c302ef0c034337253dde1463883a3366ce`
produced `target/parity-release-review3.toon`: 602 of 607 semantic/process
matches, 575 of 580 exact compact matches, and 580 of 580 TOON matches.
The five remaining differences are four measured math witnesses and regex
longest matching. No disparity approvals were supplied. All original 518 cases
and 303 baseline matches remain protected; the pinned companion sources were
verified. The strict validator correctly reported failure.
Both ignored tests subsequently passed when explicitly enabled with pinned jq:
the published-manual reference check and the fixture-only relocated checkout.
Primary process probes also passed counted TOON arrays/tables, JSON auto-detection,
and invalid-UTF8 proxy passthrough on this release executable. Strict OpenSpec
validation passed.

The nine-workload paired benchmark against the exact starting revision passed
eight workloads. The new 32,768-integer JSON identity workload measured 1.078x
wall time and 1.278x peak RSS, exceeding the 1.20x RSS limit. Evidence is retained
in the companion repository at `.work/jq-parity-perf-20260908-review2/`.
Task 10.4 remains open while this regression is investigated; the limit has not
been raised. Common missing-delimiter `--run-tests` diagnostics also remain an
implementation gap under task 9.4.

The subsequent integer-canonicalization fast path preserves explicit numeric
limits after a primary-review correction and improves numeric-array wall time
to 0.955x baseline. It does not resolve peak RSS (1.271x), so the hypothesis that
temporary coefficient allocation explained the memory increase was rejected.
Evidence is in `.work/jq-parity-perf-20260908-review3/`. Supplemental five-sample
startup probes measured roughly 0.38–0.49 MB additional peak RSS, compared with
2.02 MB additional RSS for the array. The JSON stream's intermediate serde
value tree is being investigated as a retained-allocation source.

The four changed legacy compatibility guides now carry required OKF metadata;
they and both new compatibility guides pass individual validation. Four
unrelated older documentation files still lack metadata and are left unchanged.
Repository-standard authority remains pending the missing parent-bundle decision.

### Sequence diagnostic acceptance

Task 7.6 also passes primary review. The effects integration suite passed all
14 tests, including a live stdout consumer receiving two complete results while
stdin remains open. Timeout paths kill and reap the child. The expanded
campaign executed all 57 I/O/invoking cases without skipped, unsupported,
timed-out, or crashed observations; final release reports will be regenerated
after the remaining unrelated numeric and test-file fixes.

Task 7.3 passes primary review. The immutable executable
`target/debug/tq-review-luspcc`, SHA-256
`a4621173055ec30f9da4745416be8b277c4237ee4f2431ba6de12d165023ffd4`,
matches pinned jq for seven malformed-sequence probes, including numeric
truncation, invalid literals, non-JSON whitespace, unmatched braces, trailing
array commas, and missing object separators. Full stdout, stderr, and status
match with only the product-name substitution. Probes use `--seq -c` without
an input-format workaround and run under controlled HOME, locale, and timezone.
The reviewed input-stream tests also cover native/default TOON, explicit JSON,
framing, recovery, physical lines, slurp, and emission before input EOF.

### Expanded campaign review

The expanded checkpoint in `target/parity-54case-check-2.toon` contains 607
cases, including the 54 additional catalog witnesses referenced by the manual
coverage ledgers. It reports 602 semantic/process matches and five unaccepted
math/regex differences. Compact output matches 573 of 580 cases; its two
additional failures are signed-zero spelling and the exponent spelling from
`tonumber`. TOON preserves all 580 applicable JSON executions. These counts are
intermediate evidence, not acceptance or approved disparity counts.

The subsequent primary workspace run completed with three failures: one numeric
output fixture and two fake-executable timing tests. All other suites passed.
The focused input/effects/extended-CLI rerun passed 74 tests. Direct comparison
still found incorrect JSON-sequence warning envelopes and classifications, and
review found jq compile-expectation and embedded-policy gaps in `--run-tests`.
Those tasks remain unchecked until corrected and reverified.

### Identity contract acceptance

Task 9.5 passes primary review. Fixed typed identity contracts validate their
specific invocation and successful reference execution, then check truthful tq
version/build/help output. Help assertions cover the documented option inventory,
TOON default, and compact JSON selection. Arbitrary assertions cannot exempt
ordinary comparison cases. Negative tests cover invalid associations, missing
help options, failed reference execution, and stale observations. The primary
rerun of support library, manual gate, and fixture storage tests passed 36 tests.

### Fourth integrated test checkpoint

Tasks 7.2 and 7.4 now pass review. The saved executable
`target/debug/tq-review-AhbTG4`, SHA-256
`8bc494aedf03a33162da336b43329dec958e7c6a5b9774c0f86296b4d0ffb185`,
matches seven previously failing malformed-stream probes exactly, including
source positions and partial output. Ordinary valid JSON streams match through
automatic detection; explicit strict JSON rejects malformed input with matching
statuses and committed output. Native-format fallback is intentionally excluded
from strict malformed-JSON probes. The format stream suite passed 54 tests and
input-stream integrations passed 31. Sequence framing/recovery probes also
match stdout and status, but their warning positions/classification remain
task-7.3 work rather than an accepted identity adaptation.

Task 7.5 passes review after the live-delivery corrections. The 13 effects
integration tests and the injected flush-failure/open-input regression pass.
The opaque effect handle enforces bounded writes. Value delivery waits for an
acknowledgment before requesting further input; effect delivery waits until
both write and flush succeed. Failed delivery cancels the producer, and empty
effect polls do not flush or wait for acknowledgment. Ordinary execution stays
synchronous. Explicit halt payload/status and retained earlier output remain
covered independently of runtime diagnostic formatting.

The explicit relocated-corpus test passed after controlled empty-HOME handling
was added. Every catalog case executed from a temporary checkout containing
only committed fixtures and compatibility data, with no companion manual.
Source fingerprint auditing remains a separate operation. Task 1.7 is accepted.

Task 9.3 passes review: six process tests cover forced/default colors, seven-slot
key styling, NO_COLOR/ordered overrides, empty containers, and actual POSIX PTY
detection. Three independent full-type palette probes against pinned jq match
stdout, stderr, and status exactly, including seven- and eight-entry palettes.
The validated palette parser no longer uses a public-path `expect`.

Dependency-selection tasks 5.1 and 8.1 are accepted independently of family
completion. The standalone probe and integrated public tests establish safe
`libm` 0.2.16 and `fancy-regex` 0.19.1 APIs under the unsafe-code prohibition,
document their licenses and verified macOS target, and test fused arithmetic,
signed zero, arities, scoped flags, Unicode, and empty matches. Actual hostile
regex inputs exhaust budgets of 1 and 100; cancellation tests cover checks
before matching and between result operations. Bessel work is capped and charged
to VM steps. These findings do not promise mid-match cancellation, accept the
longest-mode restriction, approve any numeric disparity, or validate other
targets. Those obligations remain in the family and release tasks.

Module tasks 9.1 and 9.2 pass primary review. The focused module/date run passed
19 tests. Module evidence covers namespace aliases, appended suffixes, ordered
search and metadata termination, prefix substitutions, distinct startup/module
locations, data slurping, and bounded reads. Dynamic metadata now retains a
shared bounded loader instead of resetting its resource state for every call;
cancellation checks bracket loading. Module fixtures use temporary-directory
ownership rather than deleting predictable preexisting paths. Embedded
filesystem denial suppresses implicit roots and startup loading. The complete
module catalog matched at the preceding campaign checkpoint.

Eight paired release benchmarks passed the 50% wall-time and 20% peak-RSS
regression limits against the exact starting source (`90b231be`). Every sample
required identical output bytes; sequence projections also had independently
generated expected output. One warmup and five alternating measured pairs used
controlled HOME/locale/timezone, single-threaded Rayon, and macOS `time -lp`
outside the sandbox, with agent builds paused. Median wall-time ratios ranged
from 0.989 to 1.047; peak-RSS ratios ranged from 0.994 to 1.122. Workloads covered
JSON identity, JSON/TOON projection, selection, sorting, regex, and both retained
sequence formats. These are performance measurements, not token savings.

Evidence is retained in the companion benchmark repository under
`.work/jq-parity-perf-20260908-review1/report.toon`, alongside every sample's
output and timing. Baseline executable SHA-256:
`2d6ef92e7db033f3bab141558bf50a47660dca4ae92dd07eec7b473ac78dac36`;
candidate: `624a0c1102868cf7749c9accb9a808ef969c27ef3a6e8c931c9f28f2bfbc240e`.
Later implementation changes still need final regression verification.

The workspace `--no-fail-fast` run completed all suites. Runtime suites passed;
three repository-evidence tests failed: an unresolved manual case reference,
the fixture-storage guard's outdated fixed catalog count, and a traceability
reference to a renamed numeric test. The strict gate's 20 tests passed. Rust
1.87.0 passed the workspace/all-targets check. Clippy still rejected two needless
raw-string delimiters in math tests; the palette API also needs lint review.

Primary review found a potential join deadlock in the unbuffered worker when
output fails before its zero-capacity channel is drained. This is assigned for
correction with failing-writer tests. The exposed mutable effect-buffer handle
also needs an encapsulated drain-only interface. These findings keep effect and
final-verification tasks open despite the passing ordinary execution tests.

Additional task-5.6 boundary probes matched fused multiply-add but exposed
compact signed-zero spelling (`-0.0` versus jq's `-0`) and different treatment
of the `-0` query literal through rounding functions. These remain numeric
integration fixes, not accepted library-accuracy disparities.

### Third shared checkpoint

With implementers paused, `target/parity-safe-checkpoint.toon` records 553
executed cases: 542 matches, 11 failures, and no approved disparities. All 518
original cases and all 303 protected matches remain present and unchanged in
verdict. Compact JSON matches 521 of 526 structured cases; TOON preserves all
526 JSON execution contracts. Source checkout verification passed. The strict
command exits 1, also identifying a missing supplemental arity source mapping.

The preserved executable is `target/debug/tq-review-CVaKiy`, SHA-256
`40a3fbac2136b5bd9fa90176a30ff064f7fc784ad59aeefe21ee47ad89ce7b67`.
Future probes can use this copy without depending on agents' subsequent builds.

Workspace tests passed the CLI, core, and formats suites before stopping at a
supplemental regex ledger query-spacing mismatch. That metadata was corrected;
the ledger then passed nine tests, with one reference-only test ignored. Gate
tests still fail three assertions for the missing arity mapping. Separate checks
passed seven reference-pin tests, nine normalization tests, and 27 TOON tests.
Formatting, whitespace validation, and the Rust 1.87.0 workspace/all-targets
check passed. Clippy reports three errors in the new math tests; they remain
implementation work, not baseline exceptions.

Tasks 5.2, 7.1, and 8.3 pass primary review and their expanded regression tests.
Remaining review findings include halt-error null stderr on the ordinary input
route, bounded effect formatting and live flushing, incomplete test-file
semantics, module search termination/substitution and dynamic `modulemeta`, and
coverage inventory rows that prove only callable availability. The four measured
math ULP differences and longest regex mode are still unapproved disparities.
Native Windows acceptance and full matched-platform campaign evidence remain
outstanding. Implementation continues; this checkpoint is not release acceptance.

### Second shared checkpoint

All three implementers paused source edits for this run. The current expanded
campaign, `target/parity-shared-review.toon` and its Markdown/stderr siblings,
contains 552 cases: 531 semantic matches, 21 failures, and no approved
disparities. All 303 original matches are preserved. Compact JSON matches
515 of 525 structured cases; TOON preserves the JSON execution contract in
all 525. The strict command exits 1.

Primary verification passed 239 core tests, all 102 CLI integration tests,
44 focused ledger/gate/normalization/pin tests (one reference test ignored),
and 27 TOON tests. The ledger run initially found an incorrectly quoted
supplemental scan input; correcting its source record made the focused suite
pass. Workspace tests still fail two CLI execution-plan assertions and two
formats default numeric-envelope assertions. Workspace Clippy still fails;
new findings were assigned to their implementation owners.

Accepted tasks in this batch are 1.8, 3.2, 3.8, 4.1–4.3, 6.4, and 6.6.
Task 1.7 is reopened because newly enabled startup-file loading requires an
explicit controlled HOME in the campaign, not the developer's inherited HOME.
Focused probes also found sequence line-number loss after 64 KiB compaction,
incorrect form-feed acceptance, and halt-error null/status boundaries. Those
remain implementation failures even where the current manual examples match.
The agents have resumed work on the remaining checklist.

### Follow-up review in progress

The next implementation batch remains under review, not accepted. CLI-specific
checks are required for `walk` callback cardinality, filtered path generators,
module imports, and input reconstruction; passing the direct VM route is not
sufficient. Regex review also found scalar output for a single captured group
in `scan`, unbounded retained match vectors outside substitution, and missing
cancellation checkpoints between global engine operations.

Ad hoc reference commands must use the executable named by the pinned report.
The system `jq` here is an older Apple build. Explicit jq 1.8.1 reruns confirmed
the eight supplemental path outcomes, one-group `scan` array output, and the
four documented math reference values. They also corrected an earlier numeric
assumption: jq 1.8.1 preserves unary-negated decimal literal scale and tiny
values, including `-1e-400`; unary negation must not force binary64 projection.

An independent minimum-version diagnostic passed with Rust 1.87.0:
`CARGO_TARGET_DIR=target/msrv-review rustup run 1.87.0 cargo check --workspace --all-targets --locked --offline`.
This was not a frozen source checkpoint; repeat the workspace check for final
acceptance. The new library manifests declare lower minimum Rust versions.

### Expanded shared checkpoint

With all three agents' source edits paused, primary verification passed 74 core
public tests, 15 normalization/reference-pin tests, five TOON writer/runtime
tests, two advanced-mode tests, and nine manual-ledger tests (one reference test
ignored in that batch). The input-stream suite passed all 29 tests when rerun
serially with unchanged deadlines; three live-pipe tests had timed out in the
concurrent run. Core library tests remain 122 passed, one failed for an extra
error after `first(fromstream(...))`. Disparity gate tests remain 18 passed, one
failed for stale independent-output approval evidence.

The first full expanded campaign is
`target/parity-expanded-checkpoint.toon` and its Markdown/stderr siblings:
538 cases, 504 semantic matches, 34 failures, no accepted disparities.
Compact JSON matches 489 of 511 structured cases; TOON preserves all 511 JSON
execution contracts. All fourteen pinned source files were verified. The strict
gate correctly exits 1 and identifies the original variable-key case as a
regression after its query-rewriting adapter was removed.

Eight source-linked path boundary witnesses were added after this run and
verified against pinned jq, raising the required denominator to 546. They are
not included in the 538-case checkpoint's counts.

Lossless normalization and independent campaigns passed review again, completing
1.5. Broader execution reopened 3.2 for explicit variable-valued object keys,
6.4 for `join` on object values, and 7.2 for auto-detection of JSON Unicode
escapes. Tasks 6.6 and 7.3–7.4 remain open: CLI `walk`, optional `repeat`, scalar
sequence token validation, truncated later numbers, retained buffers, and
stream early termination still need corrections. The current scanner also
retains two obsolete implementations and rescans incomplete tokens; these are
not accepted implementation or resource contracts.

### Earlier independent-output checkpoint

The independent-output run reached 446 semantic matches and 72 failures out of
518. Compact JSON matched 431 of 491 structured cases; TOON preserved all 491
JSON execution contracts, including matching error outcomes. These are distinct
counts, not a completion claim. Evidence:
`target/parity-independent-encodings.toon` and its Markdown sibling. Its gate
remains red. Broader execution reopened task 7.2 after finding that the mixed
scalar stream `1 true [2]` was misdetected as TOON despite object-stream tests
passing. That regression was fixed, independently verified, and task 7.2 checked
again. Sequence recovery and source-line behavior remain under review.

The subsequent source audit adds seventeen source-linked prose-boundary cases,
plus a source-linked 220-signature arity check, bringing the required campaign to
536 cases without changing the original pin.
It also removes the sole tq query rewrite and adds a gate regression rejecting
such rewrites. These additions have not yet received a full campaign result.
See [the behavior audit](source-behavior-audit.md) for the remaining evidence.

Task 3.6 passed primary code review and all 34 grammar tests, including direct
index versus generator arities, fractional counts, empty streams, and bounded
early termination. JSON-sequence review subsequently reproduced newline-loss
data corruption, discarded-record contamination, and acceptance of truncated
numbers. Task 7.3 remains unchecked pending fixes and independent verification.

The next stable integration checkpoint passed 47 core public tests and 38
test-support tests (one external-reference test remained ignored in that batch).
Typed errors and optional suppression passed review, completing task 3.7.
The expanded numeric-normalization test then proved that runtime projection
collapsed distinct enormous decimal exponents to the same finite maximum.
It also exposed changed canonicalization of ordinary integer output. Task 1.5
was reopened and task 5.2 remains unchecked until lossless comparison and native
numeric contracts are restored. No new report is accepted while that guard is red.

The first normalization repair passed all nine tests, then a wider exponent
boundary exposed another false match: distinct exponents beyond `i64::MAX`
were saturated to the same representation. That regression is now executable.
The 147 official TOON encode fixtures pass, but an additional literal-provenance
test fails because `1.000` is emitted unchanged instead of canonical TOON `1`.
Both remain implementation failures, not accepted numerical disparities.

A subsequent checkpoint independently passed 31 focused core tests, nine
normalization tests, and four TOON writer/runtime tests. Review of its new
decimal renderer then found incorrect leading-zero placement for fractional
values; additional fraction/exponent equivalence guards were added before any
campaign was accepted. Runtime transcode admission must also remain separate
from unbounded report normalization. The math supplement adds two source-linked
cases, bringing the current required catalog to 538.

Path review found a CLI-only dispatch gap despite passing direct VM tests:
the cursor-backed emitter called unfiltered descendant traversal for `paths/1`.
For example, `paths(.==2)` on `[1,2]` emitted both paths instead of only `[1]`.
The path tasks remain unchecked pending process-level filtering, cardinality,
and partial-output verification in addition to the VM tests.

Task 1.8 is reopened after adding independent encoding observations: its approval
fingerprint still captured only semantic jq/tq results. A focused regression
proves that changing TOON output after approval does not invalidate the approval.
Completion must bind all observed output contracts, while exact mode continues
to reject every disparity and all original matches remain protected.

The latest pinned-source run reached 423 exact matches and 95 unresolved failures
out of 518, with no accepted disparities or original-match regressions. Evidence:
`target/parity-pinned-sources.toon` and its Markdown sibling. The source checkout
audit verified all fourteen files against the committed pin. The published-table
reference test passed with the pinned jq build.

A separate relocation test copied only fixtures and compatibility data into a
temporary checkout and executed all 518 cases without the companion repository.
Portable `env_paths` replace developer-specific module-fixture paths. Focused
schema, harness, pin, and gate tests passed (35 tests), as did the 25 grammar tests
and 9 input-stream tests. Input and destructuring-alternative review corrections
remain under verification and do not yet mark their tasks complete.

Review corrected mixed-type array indexing expectations, consolidated query
equality, added bounded collection scans and binding work, and capped binding
pattern nesting. Later review found multiline source-line and eager-input
prefetch problems; those fixes and their live-pipe tests are being verified.

### Safe-library checkpoint

The resumed workspace suite passed 478 tests, with one intentionally ignored
reference test; that pinned jq reference test passed when explicitly enabled.
Workspace formatting, Clippy (zero errors, one pre-existing warning), OpenSpec
strict validation, and the new disparity document's OKF validation passed.
Broader docs validation still has eight pre-existing metadata errors.

The resumed comparison reached 395 exact matches and 123 unresolved failures,
with zero accepted disparities. All original 303 matches were preserved. Every
Math case matched, including both negative arity probes. The only remaining
Regex case was longest-match mode. The command exited 1, as required; evidence
is in `target/parity-safe-rust-reviewed.toon` and its Markdown sibling.
The full implementation remains in progress; release evidence must be
regenerated after the remaining work.

Primary review of resumed work corrected argument-generator ordering, nonnumeric
predicates, negative-zero projection, nested TOON projection of NaN, regex flag
mapping, unequal replacement-generator cardinality, and runtime error status.
CLI positional arguments now use typed records rather than parallel vectors;
invalid UTF-8 raw files follow the pinned jq replacement behavior.

Additional review fixed invalid slurpfile input's exit status, later regex
matches producing more replacements than the first, and stale numeric-test
traceability links. Generated probe binaries were moved under root `target/`;
the fixture-storage guard was not weakened.

The reviewed-disparity gate has focused tests for stale/unknown approvals,
forged embedded approvals, and attempted exceptions for original passing cases.
The exact gate remains separate. The provisional longest-match and Bessel-order
witnesses are documented but have not been turned into completion approvals.

### Earlier checkpoint

- Starting workspace suite: 399 passed, 1 ignored.
- Final workspace suite: 421 passed, 1 ignored, using `rtk cargo test --workspace`.
- Focused compact selection: 3 passed; grammar: 11 passed; strict gate: 8 passed.
- Dependency feasibility probes: 4 passed with `--offline --locked --release`.
- Workspace formatting and `git diff --check`: passed.
- Workspace Clippy: zero errors; the pre-existing unknown
  `clippy::assert_is_empty` warning remains.
- Strict OpenSpec validation: passed.
- Original and archive-safe 215-row gap inventory files are identical.

The pinned jq 1.8.1 manual comparison still contains all 518 cases. Matches rose
from 303 to 310, with no regression among the original 303 matches. The remaining
report contains 191 failures, 15 former policy differences, and 2 reference
discrepancies. The command exits 1, as required. Local observations are retained
in `target/parity-start.toon` and `target/parity-paused.toon`; these are working
reports, not replacement release baselines.

## Dependency decision resolved

Tasks 5.1 and 8.1 now pass the selection review recorded above. See the reproducible
[dependency probes](../../../tests/compatibility/probes/dependencies/readme.md).
Onig 6.5.3 provides safe longest-match and limit-setting APIs, but no safe
cooperative cancellation callback. Its raw callout APIs require an unsafe FFI
boundary. API availability alone does not prove cancellation or bounded engine
execution. Pure-Rust libm probes establish available functions and some numeric
properties, not exact equality to jq's platform math. Other targets are unverified.

These probes did not establish that unsafe or FFI code was necessary. The
revised design excludes new native engines and unsafe bridges. Safe Rust math
and regex implementation is proceeding, with measured limitations recorded in
`docs/jq-compatibility-disparities.md` and kept distinct from exact matches.
Missing functionality and uninvestigated failures remain incomplete. The
implementation remains uncommitted until the required work and checks are complete.
