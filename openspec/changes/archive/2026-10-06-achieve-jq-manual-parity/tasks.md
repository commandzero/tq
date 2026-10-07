## 1. Freeze coverage and make failures enforceable

Matching tasks target jq behavior subject to the reviewed safe-library disparity contract. Reviewed-completion acceptance still requires exact contracts or specifically reviewed, reproducible disparities. The user-confirmed implementation/evidence closeout instead follows the authoritative target/case map in `specs/cross-tool-compatibility/spec.md`: only enumerated Linux/Windows refinements move to #70/#69, and calibrated performance acceptance remains under #31/platform follow-ups. Transfer is not acceptance, approval, a gate pass, or permission to omit cases. Unresolved implementation gaps and retained evidence obligations stay unchecked. MacOS compatibility differences are not transferred; unfinished macOS calibration/comparisons remain explicitly unpublished as accepted performance evidence.

- [x] 1.8 Add reviewed disparity validation and separate exact/completion gate modes; reject stale or unknown approvals, preserve exact observations and all prior matches, and keep skips/timeouts/crashes as failures.
- [x] 1.1 Pin the manual fingerprint, section inventory, and matched-platform jq 1.8.2 build identities; verify a changed source or executable fingerprint fails reference validation.
- [x] 1.2 Connect `gap-inventory.toon` to the executable catalog and review model; verify exactly 198 `failure` rows, 15 `expected-difference` rows, and 2 `reference-discrepancy` rows are accounted for, with no duplicate or missing IDs.
- [x] 1.3 Separate source corrections from execution verdicts; verify both invalid arity probes, corrected `/0` witnesses, and both whitespace corrections retain original provenance and execute.
- [x] 1.4 Add the strict manual campaign exit policy; test injected mismatch, skip, missing reference, timeout, normalization error, and deleted source mapping all cause nonzero exit.
- [x] 1.5 Add semantic JSON, exact compact-byte, and independent TOON-output campaigns; verify numeric changes, changed result order, and changed explicit stderr payloads cannot normalize into a pass.
- [x] 1.6 Audit every section's documented arities, flags, error/empty behavior, and runnable snippets beyond table examples; deliver a source-linked completeness inventory and executable cases for missing behaviors.
- [x] 1.7 Remove developer-specific paths from case execution and record reproducible environment setup; verify the committed corpus runs without the companion repository while source auditing verifies its pinned checkout separately.

## 2. Output selection and CLI option resolution

- [x] 2.1 Add failing process tests for default TOON, pretty JSON, `-c`, long compact output, bundled `-cr`, and both orders of explicit JSON/TOON combinations; verify they express the design matrix.
- [x] 2.2 Normalize output selection before input consumption; make those tests pass without changing automatic input-format selection.
- [x] 2.3 Verify compact/raw/join/sort/ASCII combinations and retained native-format options against jq or their native contract; include JSON Lines restrictions and invalid-option no-read tests.
- [x] 2.4 Implement the remaining named/positional argument and argument-file contracts; verify `manual.invoking.args-named` and every documented option form with valid and invalid arguments.
- [x] 2.5 Make LF-terminated TOON values the default for zero or more results; reserve RS framing for explicit `--seq` or `-o toon-seq` and strict single-document cardinality for explicit `--unframed`. Update output-byte, documentation, and regression contracts.

## 3. Grammar, binding, and control flow

- [x] 3.1 Repair comma indexing, quoted identifiers, namespace errors, and boolean/pipe precedence; verify the nonnumeric `basic` inventory entries and `manual.condcomp.not-array` against jq.
- [x] 3.2 Implement nested array/object destructuring and variable object shorthand with lexical scope; verify manual pattern cases plus missing-field, shadowing, and invalid-type cases.
- [x] 3.3 Implement destructuring alternatives and their backtracking bindings; verify all manual `?//` examples and errors from later alternative bodies against jq.
- [x] 3.4 Repair captured filter environments and filter/value parameter behavior; verify nested `addvalue`, post-author lookup, and callback tests with multiple branches.
- [x] 3.5 Repair reduce/foreach destructured bindings, optional extraction, and accumulator isolation; verify expanded manual examples and zero/multiple initializer/update results.
- [x] 3.6 Repair pull-driven first/last/nth/skip/isempty and short-circuit consumers; verify empty generators and early termination do not evaluate later erroring branches.
- [x] 3.7 Preserve typed error values and optional suppression scope; verify manual error examples, nested try/catch, and previously passing label/break cases.
- [x] 3.8 Run all `advanced` inventory entries and parser/control-flow regressions across DOM and eligible optimized modes; require identical ordered results and process contracts.
- [x] 3.9 Audit composition admission for every operation and documented built-in arity; add source-linked failing witnesses for operations inside definitions, around calls, and through filter/value parameters without altering the pinned original inventory.
- [x] 3.10 Implement bounded object-constructor and slice composition, including computed keys, field generators, lexical captures, empty branches, partial errors, and early termination.
- [x] 3.11 Implement bounded fold and path/update composition with user calls in generators, initializers, updates, extraction, selected paths, and RHS filters; preserve branch state, ordering, and cancellation.
- [x] 3.12 Integrate documented scalar and generator built-ins into user-filter execution without a restrictive arity whitelist or an unbounded collector fallback; verify math, string, collection, and generator family witnesses.
- [x] 3.13 Integrate callback-driven regex, recursive utilities, input, and process effects with composed user filters; verify lexical replacement scope, shared input consumption, debug/stderr ordering, non-catchable halt, and live early termination.
- [x] 3.14 Verify the full 297-case composition inventory through CLI and public embedded interfaces, including recursion, cancellation, and tight resource limits, with no deferred execution gaps. Reconcile final-candidate source/executable/profile identities and refresh native campaigns where identity cannot be proved; retain mapped #69/#70 differences as unresolved. Renew approvals only for independently reviewed acceptance, not deferred contracts; calibrated performance acceptance is follow-up work.
  - Performed — macOS `newhelp-final`: 297/297 CLI composition matches; embedded 297 witnesses (249 direct comparisons, 48 independent expectations), eight composition-resource and six recursive-composition tests pass in the all-feature workspace run.
  - Performed — Linux `help-final`: fresh manual execution retains the same three transferred composition differences, with no new manual failure IDs; embedded inventory and resource/recursion/cancellation tests pass in the fresh all-feature workspace run.
  - Performed — Windows `final-286b5f6/native`: all 297 CLI cases execute without tq execution gaps; eight semantic-difference IDs are explicitly mapped to #69, not accepted. All 297 public embedded witnesses and resource/recursion/cancellation tests pass in the final-overlay native workspace run. Cross-host product/helper source identities reconcile; the sole Windows overlay is an integration test, not product/helper code or a new release campaign. No deferred execution, blanket approval or calibration claim follows.

## 4. Path selection and assignment

- [x] 4.1 Preserve path identity through recursive traversal and selection; verify `path`, `paths`, `pick`, `del`, and `delpaths` against manual and nested-path cases.
- [x] 4.2 Separate plain assignment RHS branching from update assignment first-result and empty-deletion behavior; verify range assignments and independent branches against jq.
- [x] 4.3 Verify all `assignment` inventory entries, nested posts/comments, immutable roots, and unaffected key order; require the complete assignment campaign to pass.

## 5. Numeric model and math

- [x] 5.1 Establish pure-Rust math dependency feasibility using safe APIs under `unsafe_code = forbid`; test arities, fused operations, signed zero, rounding, availability, and license/resource behavior. Record verified targets and measured disparities; exact C-library equality is not an integration prerequisite.
- [x] 5.2 Preserve decimal identity and observable literal representation without expanding exponents; verify numeric `basic` entries, `have_decnum`, and `tojson` precision/representation cases against pinned jq.
- [x] 5.3 Add runtime non-finite values and jq-compatible numeric predicates, arithmetic, and JSON projection; verify valid native TOON projection and unchanged native parser rejection rules.
- [x] 5.4 Implement trigonometric and hyperbolic math families, including inverse forms; verify every corresponding Math-section witness and domain boundary against matched-platform jq.
- [x] 5.5 Implement exponent, logarithm, roots, powers, and scaling functions; verify all corresponding witnesses plus overflow, underflow, and large-exponent cases.
- [x] 5.6 Implement rounding, sign, remainder, neighboring-value, min/max, and fused math functions; verify witnesses and signed-zero/rounding boundaries, recording measured disparities with function-specific bounds separately from exact matches.
- [x] 5.7 Implement error, gamma, Bessel, decomposition, and remaining platform math functions; verify the full Math inventory has no missing arity and both `frexp`/`modf` probes and valid witnesses execute.
- [x] 5.8 Run every `math` inventory entry and existing numeric/resource tests; verify explicit tight limits fail boundedly and normal manual cases no longer use numeric-envelope exceptions.

## 6. Remaining built-in families

- [x] 6.1 Repair collection containment and string/array index variants; verify all manual `contains`, `inside`, `indices`, `index`, and `rindex` cases plus type and empty cases.
- [x] 6.2 Repair entry conversion, add/filter overloads, and all any/all forms; verify generator cardinality, key aliases, empty collections, and short-circuit behavior against jq.
- [x] 6.3 Implement all documented `IN`, `INDEX`, and `JOIN` forms; verify every overload with duplicate keys, empty streams, and generated results.
- [x] 6.4 Repair string division, trim variants, starts/ends predicates, joining, ASCII case, UTF-8 byte length, boolean conversion, and `@urid`; verify manual witnesses plus Unicode and invalid-input cases.
- [x] 6.5 Repair combinations, transpose, and binary search; verify all manual cases and empty/ragged/insertion-position behavior against jq.
- [x] 6.6 Repair while/repeat/until/walk and remaining recursive utilities using shared generator semantics; verify manual witnesses and bounded early termination.
- [x] 6.7 Run all `builtins` inventory entries, including cross-workstream math/path/regex/I/O cases; verify no unknown documented arity or unresolved behavioral failure remains.

## 7. Input streams and process effects

- [x] 7.1 Share the ordered cursor between top-level input, `input`, and `inputs`; verify null-input mode, EOF, multiple files/stdin, and changing source location against jq.
- [x] 7.2 Accept ordinary JSON whitespace streams and match strict JSON error behavior using one shared safe-Rust incremental parser, including jq non-finite input and `fromjson`; preserve numeric identity, event streaming, source positions, cancellation, and resource limits across JSON routes without broadening native-format admission. Verify automatic detection separately so `-i json` cannot hide a valid-input regression.
- [x] 7.3 Implement JSON sequence recovery and JSON/TOON output-mode separation; verify malformed records, RS/LF framing, diagnostics, and `--seq` with default output, `-o json`, and `-c`.
- [x] 7.4 Repair stream/error events and reconstruction functions; verify all `streaming` entries plus stream reduce, empty containers, malformed input, and partial output.
- [x] 7.5 Implement debug/stderr payloads and non-catchable halt termination; verify exact payload bytes, statuses, preserved earlier output, and abandoned pending branches.
- [x] 7.6 Verify all `io` entries and invoking stream/halt cases; add a live producer test proving `--unbuffered` output arrives before input EOF.

## 8. Regex and date/platform behavior

- [x] 8.1 Select an existing pure-Rust regex dependency through safe APIs; test scoped flags, Unicode, empty matches, actual bounded-work failure, and cancellation between bounded operations. Investigate longest matching and document demonstrated limitations and verified targets without unsafe bridges or native FFI engines.
- [x] 8.2 Implement all documented array/null pattern/flag forms and flag combinations; verify argument interpretation and inline, extended, longest, multiline, and empty-match behavior against jq.
- [x] 8.3 Preserve capture scope and replacement generator results in sub/gsub; verify zero/multiple replacement choices, unmatched captures, Unicode offsets, and errors.
- [x] 8.4 Run every `regex` inventory entry and existing regex resource tests; replace historical policy labels with exact results or specifically reviewed safe-library disparities and verify bounded hostile-pattern failures.
- [x] 8.5 Verify date, environment, clock, filename, and line behavior under controlled locale/timezone/files; test CLI default access and embedded denial independently on local macOS, ironhide Linux, and native smokescreen Windows. WSL execution does not count as Windows evidence.

## 9. Modules, colors, and remaining CLI contracts

- [x] 9.1 Implement jq-compatible module search, substitutions, startup file, repeated-component rejection, and search termination; verify controlled-home/default/explicit-path cases and embedded confinement/cycle tests.
- [x] 9.2 Implement JSON data imports and dependency metadata; verify every `modules` inventory entry with namespace, ordering, parsing, and bounded file-read cases.
- [x] 9.3 Implement JSON color palette, JQ_COLORS, NO_COLOR, terminal detection, and ordered force/disable flags; verify every `colors` entry and invoking color difference with byte and PTY tests.
- [x] 9.4 Implement `--run-tests` through tq evaluation; verify passing/failing test files, compile-error expectations, comments, stdin, option interactions, diagnostics, and status against jq.
- [x] 9.5 Verify help/version/build commands against explicit truthful tq contracts and complete option documentation; replace unconditional identity differences with executable assertions.
- [x] 9.6 Run every remaining `invoking` inventory entry; require jq-compatible JSON/process behavior without tq-specific allow flags.

## 10. Cross-platform acceptance and release evidence

- [x] 10.1 Reconcile final-candidate strict campaign execution and matched jq 1.8.2 identities on local macOS, ironhide Linux, and native smokescreen Windows, refreshing campaigns where source/executable/profile identity cannot be proved. Retain every non-match and missing-evidence limitation without passing acceptance claims. Record automated Windows release-host wiring under #69 and Linux full validator preflight under #70 as unexecuted follow-ups; the current workflows do not establish all advertised targets.
  - Performed — macOS `newhelp-final` release/default strict execution: 943/952 primary, 919/921 compact, 921/921 TOON; strict exit 1, nine unchanged differences, zero approvals applied. Product/helper source hashes match the current implementation at review.
  - Performed — Linux `help-final` fresh locked release: 937/952 primary, 913/921 compact, 921/921 TOON; strict exit 1 with exactly the same 15 #70 IDs. Source, reference and frozen binary stability are recorded.
  - Performed — native Windows PowerShell/MSVC release/default execution at `286b5f6` plus the hash-verified test-only overlay: 912/952 primary, 879/921 compact, 921/921 TOON; strict exit 1, zero applied disparities and exactly the same 60 #69 IDs. jq 1.8.2 reference/build, source, frozen binary/profile, PowerShell/cmd, raw binary/newline, stack and workspace evidence are retained and reconciled against current product/helper sources. This supersedes the earlier SSH-unavailable checkpoint, not its historical failure; automated Windows host wiring and Linux full validator preflight remain unexecuted platform follow-ups.
- [x] 10.2 Verify actual POSIX invocation on macOS and Linux, including quoted filters, variables, paths, and outputs. Verify native PowerShell/cmd and Windows binary/newline tests on smokescreen; do not count non-Windows shell execution as Windows evidence.
- [x] 10.3 Reconcile all original 518 cases and added witnesses against final-candidate executable evidence, source mappings, and the explicit target/case follow-up map. Preserve exact matches, genuinely approved disparities, and unresolved transferred differences as separate outcomes; verify the 303 protected baseline contracts without silently changing the pin or gate. Require no missing mappings, skips, timeouts, crashes, unowned failures, or unexplained regressions. Resolve/review macOS's nine non-transferred contracts before closeout; claim neither exact nor reviewed-completion acceptance for deferred differences.
  - Performed — macOS `newhelp-final` and Linux `help-final` reports preserve all 518 original cases, 303/303 protected exact contracts, 952-case execution and source/reference mappings. No new failure IDs or hidden manual execution failures are recorded.
  - Performed — macOS's nine scoped expected behaviors were reviewed against actual observations: erfc(2) 2 ULP higher, tgamma(0.5) 1 ULP lower, six output-colors presentation contracts with diagnostically identical plain bytes/process status, and the run-tests internal suffix with the supplied test passing in both tools. These are existing input/contract-scoped expected behaviors, not fresh executable-bound approvals; all nine strict failures remain.
  - Performed — Windows raw release report retains all 518 originals and all 303 protected IDs; 291 protected primary and 289 primary-plus-compact contracts match, with 14 unique protected difference IDs all explicitly mapped to #69. Presence is not 303 exact acceptance. All 4,667 manual observations execute and exit; the raw primary/compact union is exactly the 60 owned #69 IDs, with no new/unowned difference or missing case. Linux's raw union likewise equals the 15 #70 IDs. Inventory/pin and strict/reviewed-completion gates remain unchanged and failing; this checkbox completes revised-scope bookkeeping, not the protected-baseline acceptance condition.
- [x] 10.4 Verify final-candidate workspace formatting, Clippy, tests, native-format and bounded-resource regressions, and protected baseline evidence. Disclose performance observations and explicitly defer calibration/calibrated benchmark acceptance to #31/platform follow-ups; unfinished macOS calibration and affected comparisons remain unpublished as accepted performance evidence. Historical controls and microbench smoke do not establish current calibration. Reconcile the full-campaign denial-fixture/clock harness debt as explicit #69/#70 follow-ups, not product disparities or a passing full campaign.
  - Performed — macOS `newhelp-final` full preflight, all-feature workspace tests and release build exit 0; three earlier disk-full preflight attempts remain recorded failures. The successful run disabled incremental compilation/dev-test debug symbols, not checks; named native-format, parser/resolver, composition, resource, CLI and gate suites are present in the retained logs. Microbench smoke is correctness/smoke evidence only.
  - Performed — Linux `help-final` fresh all-feature workspace execution passes 1,891 tests (zero failures, 11 ignored in that command); the pinned/published-reference and both relocation tests pass explicit separate runs. Strict workspace/all-target/all-feature Clippy and release build exit 0. Linux full validator preflight remains the explicit unexecuted #70 follow-up; ignored native performance controls are not calibration proof.
  - Performed — repaired macOS/Linux shared full campaigns cover 1,220 cases with zero declared-contract failures and zero harness errors. MacOS retains pre-help P2 diagnostic evidence, not a final-help full-campaign rerun or whole-source/executable equality claim; only help strings/regression changed in crate sources. Linux `help-final` is a fresh rerun (4,804 executed, 1,961 unsupported observations, 99 differing cases). Unsupported observations and pairwise differences remain, so exit 0 is not exact manual acceptance.
  - Performed — Windows final-overlay native fmt, strict all-target/all-feature Clippy and all-feature workspace exit 0 (1,728 passed, zero failed, 10 ordinarily ignored). The catalog reference test passes explicitly in debug and release; four other ignored reference/relocation/raw-byte tests pass explicitly. Native shell, stack, native-format and bounded-resource guards pass. The mapped release full campaign retains 1,220 cases, 4,804 executed/1,961 unsupported observations, 146 differing cases and zero declared-contract/harness failures (`observed-differences`, not exact acceptance); 12 exact release embedded-denial byte captures pass. Calibration stays deferred under #31/platform follow-ups; remaining ignored accounting controls are not accepted performance evidence.
- [x] 10.5 Regenerate compatibility reports and update help, formats documentation, numeric/security migration notes, and changelog; verify TOON remains the documented default, `-c` selects compact JSON, and token savings remain separate from verdicts.
  - Performed — current coverage/compatibility prose and witness metadata distinguish exact failures, scoped expected macOS behaviors, unresolved #69/#70 transfers, historical Windows evidence, and deferred performance acceptance. Help/option assertions, output defaults, compact selection and migration/changelog behavior have implementation/test mappings.
  - Performed — `target/closeout/macos/newhelp-final/task-10.5-final-proof.json` records zero blocking documentation mismatches. `crates/tq-cli/src/args.rs` help and `help_distinguishes_cli_ambient_access_from_embedded_admission` distinguish process ambient access from embedded admission without claiming redaction. The manual/options reference is pinned to jq 1.8.2; migration targets 0.4.1 while retaining the historical 0.4.0 API boundary and separately scoped native-format jq 1.8.1.
  - Performed — the newest frozen report is rendered into all 21 section Results blocks and the index identifies its source/release provenance; only the help capture changes, 20 section files are byte-identical and repeat rendering is idempotent. README/help/formats, numeric/security migration and changelog guidance agree; token savings remain separate from verdicts. All eight proof-document hashes matched at the completed 10.5 checkpoint; later Windows reader-document updates are separately scoped, not asserted hash-equal to that proof. Documentation validation exits 0 and 147 local links across 31 files pass (external URLs not fetched). This completes documentation publication, not Windows renewal or acceptance.
- [x] 10.6 Validate OpenSpec and affected documentation and verify implementation/evidence against all eight revised delta specs. Only after retained closeout obligations have proof, synchronize and archive under repository policy; retain source-bound campaign results and gap/follow-up reconciliation without changing original inventory or gate outcomes. No synchronization or archival is performed by this scope revision.
  - Performed — final implementation/test/evidence reconciliation for all eight deltas (32 requirements, 123 scenarios) is recorded in `implementation-review.md`, including native Windows proof, exact #69/#70 sets, protected-baseline outcomes and platform-conditional scenario qualifications. All 368 Windows artifact entries hash-match locally; current native code/fixture identities and test-only overlay reconcile without rerunning unchanged frozen release campaigns.
  - Performed — final help/report/documentation publication is verified under 10.5; witness reconciliation remains complete under 10.7. Current macOS/Linux manifest checks verify all 121/65 entries locally; this is not off-host archival proof. Strict OpenSpec validation, `scripts/docs-check.sh` and scoped artifact `git diff --check` exit 0 with docs/review metadata unchanged during validation. This checkpoint is not final native/platform or PR-boundary verification.
  - Performed — corrected the manual index's stale Windows-offline paragraph, independently verified all 32 requirement descriptions and 123 scenarios after merging eight main specs, and preserved 57 older unmentioned requirements and all Purpose sections. Strict main-spec validation passes 21/21. Archived to `openspec/changes/archive/2026-10-06-achieve-jq-manual-parity/` after native and revised-scope verification; the committed PR-boundary check is verified separately, not inferred from this checkbox.
- [x] 10.7 Complete the witness reconciliation for `docs/tests/jq-manual/coverage.md` and supporting metadata in `tests/compatibility/reviews/`: observed jq/tq contracts, source/executable/profile and dependency identities, practical impact, regression evidence, owners, and reconsideration criteria. Accepted bounds/restrictions apply only to independently approved disparities; transferred #69/#70 entries remain explicitly unresolved/unapproved. Verify durable raw-evidence availability and hashes, and reconcile live issue acceptance wording with the confirmed parent closeout scope.
  - Performed — read and reconciled current coverage prose and `parity-closeout.toon` witness impacts, expected scopes, follow-up owners and reconsideration criteria; reviewed the late-p2 observations independently. Historical approvals are not renewed.
  - Performed — all 81 macOS and 60 Linux late-p2 artifact-manifest entries are locally present and SHA-256 matching. Source snapshots, logs, reports and frozen binaries are retained under ignored `target/`; no off-host durable archive has been verified.
  - Performed — live #69/#70 explicitly approve parent scope transfer without acceptance; #31 now states that the runner/backend exist but calibration/native renewal remain incomplete. All three issues remain open.
  - Performed — reviewed the completed late-p2 identity promotion in coverage prose and `parity-closeout.toon`: 16 directly referenced current/historical/shared artifact hashes match, as do 12 shared P2 and 81 historical Windows manifest entries. Historical Windows retention is explicitly not candidate renewal. Metadata records current helper/product profiles and identities, causes, regression witnesses, practical impact, precise expected scopes, owners and reconsideration criteria; no deferred approval is invented.
  - Performed — latest `parity-closeout.toon` promotion now records macOS `newhelp-final` and Linux `help-final` sources, reports and executable identities, retaining late-p2/v6 as history and macOS shared P2 as pre-help diagnostic evidence. Current manifest/source/document hash verification supports these labels; no whole-source identity is inferred across the help change.
  - Performed — `scripts/docs-check.sh` passes on the updated bundle. Retention is verified locally, with the no-off-host-archive limitation and preserve-before-cleanup obligation explicitly recorded. This checkbox completes witness/documentation reconciliation, not Windows execution, generated-page publication under 10.5, calibration, spec synchronization or archival.

All 69 original tasks are complete under the user-approved implementation/evidence closeout scope. Task 10.6 includes verified main-spec synchronization and archival; compatibility refinement and calibrated performance acceptance remain open in the named follow-ups.
Named plain subitems distinguish performed work from remaining obligations without adding
checkboxes or changing the 69 task IDs.
The current evidence is `target/closeout/macos/newhelp-final/` and
`target/closeout/linux/help-final/` (late-p2 retained as a previous checkpoint),
with base `3e0dedb` plus recorded uncommitted changes, stable source-bound
snapshots and frozen release binaries. Their strict reports retain 9/15 unique
differences and exit 1. Windows now uses
`target/closeout/windows/final-286b5f6/native/native-final-success-summary.json`
and `native-final-69-task-proof.json`: native release proof at `286b5f6` maps
to the sole integration-test overlay with SHA-256
`a86772509c2e279b823e6f02529e976c73b8c75ed30d23bea1620acf828a6c30`.
Product/helper code and frozen release binaries are unchanged. Its 60 mapped
#69 differences remain unresolved, strict exit 1; protected accounting is
303 IDs present / 291 primary matches / 289 primary-plus-compact matches,
never 303 exact. Historical v6/offline checkpoints remain retained.
No spec waiver is made. Current coverage/witness metadata and local-retention
reconciliation are complete under 10.7, including explicit retention limits;
there is no claim of an off-host durable archive. Generated-page/provenance-link
and final documentation reconciliation are complete under 10.5. Final state/metadata validation, synchronization and archival are complete under 10.6; the final commit's repository/PR-boundary check is recorded separately.

The confirmed transfer is limited to the explicit map in
`specs/cross-tool-compatibility/spec.md`:
- Windows refinements: https://github.com/commandzero/tq/issues/69.
- Linux refinements: https://github.com/commandzero/tq/issues/70.
- Calibrated performance acceptance: https://github.com/commandzero/tq/issues/31
  and platform follow-ups; #31 remains open. MacOS's unfinished calibration and
  affected comparisons remain explicitly unpublished as accepted performance
  evidence, not covered by an inferred Windows approval.

The corrected macOS/Linux shared full campaigns supersede their stale
denial-fixture/clock harness errors with zero declared-contract/harness failures,
while preserving unsupported observations and pairwise differences. Windows's
corrected native release full campaign likewise records zero declared-contract/
harness failures with 146 differing cases, not an exact compatibility pass.
No exact or reviewed-completion gate pass, newly approved difference, synchronization, archive, or issue closure
follows from this progress update.
