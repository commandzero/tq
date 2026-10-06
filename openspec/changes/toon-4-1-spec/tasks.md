## 1. Establish the stacked migration baseline

- [ ] 1.1 Reconcile the current PR68 head with recorded base `286b5f6690bc1bee260cba007f3953c52178bf68` and its active parity deltas before applying this change; verify inherited fixes, original 518 IDs, protected 303 exact contracts, and deferred issue ownership remain intact, recording any base movement.
- [ ] 1.2 Preserve the complete current manual case mapping and available PR68 stdout/token/provenance evidence in ignored migration storage; verify 952 mapped unique cases and 877 eligible published samples or document a reviewed superseding baseline, without calling overlay evidence a clean-commit build.
- [ ] 1.3 Vendor official fixtures from TOON spec tag `v4.1.0`, recording resolved commit, file hashes and MIT attribution; verify fixture membership and upstream expected values/errors/bytes, replacing the active v3 fixture set and removing obsolete folding/expansion fixtures.
- [ ] 1.4 Upgrade both `toon-format` pins to exact `0.6.0` with default features disabled and update applicable lockfiles; verify resolved package/feature identities and adapt oracle calls to the documented new API without substituting another package named `toon-rust`.
- [ ] 1.5 Before implementation, freeze a clean reconciled PR68 release/default binary and the focused performance workload matrix, covering flat controls, decoding, document encoding, identity transcode, nested/keyed shapes, late invalidation and spool-triggering preparation; verify baseline/source/input hashes, semantic correctness, toolchain/features and same-host measurement settings, distinguishing any version-specific input representations.

## 2. Implement native 4.1 parsing and semantic events

- [ ] 2.1 Implement recursive field-list and keyed-header parsing with bounded state and per-group uniqueness; verify official valid/malformed fixtures, nested leaf widths, keyed counts including zero, delimiter mismatches and source-positioned errors.
- [ ] 2.2 Reconstruct nested row values and keyed object entries through the existing semantic event contract; verify DOM/event/`--stream` values, header-order normalization, literal dotted keys, prototype-like ordinary keys, and that the observed flat-key misdecode no longer occurs.
- [ ] 2.3 Implement root/object-field `[]` and legacy empty-header input, anonymous nested-array restrictions and first-field list-item depth rules; verify exact values and errors for root, field, list, row-cell and keyed-entry contexts under official fixtures.
- [ ] 2.4 Implement BOM/CRLF, full-line comment exclusion, U+0020-only trimming, quoted-token boundaries, BMP/control escapes and surrogate rejection; verify fragmented byte reads, original source positions, comments between rows, hash data, Unicode whitespace and malformed UTF-8/escape boundaries.
- [ ] 2.5 Enforce strict sibling duplicates, scope/blank-line/header/count/width checks even for discarding event consumers, with bounded duplicate/schema bookkeeping; verify hostile huge declarations do not preallocate and configured limits fail before exceeding bounds.
- [ ] 2.6 Implement non-strict document-order last-write-wins, no declared-count truncation, absent short-row leaves and ignored surplus cells; verify official non-strict fixtures and route consumers that cannot retract duplicate values to a sound staged/document path before consumption.

## 3. Implement canonical shape-driven writing

- [ ] 3.1 Introduce shared ordered recursive schema detection for arrays and keyed objects, including first-value key order and later shape invalidation; verify same-key-set/different-order values and null/object, array, empty-object, missing/extra-key and nested mismatch boundaries.
- [ ] 3.2 Emit recursive table headers/leaf rows and keyed entry rows at eligible root/object-field positions; verify official exact encoder bytes for comma/tab/pipe, nested groups and quoted field/entry keys, retaining exact supported numbers.
- [ ] 3.3 Apply position-specific empty-array, anonymous nested-array and list-item first-field indentation rules; verify exact output and strict round trips, including arrays of empty objects and single-entry objects that must remain expanded.
- [ ] 3.4 Update primitive/key quoting and control/BMP escaping in shared plain/colored sinks; verify hash-leading data, ASCII-only unquoted keys, numeric-looking strings, delimiters and Unicode, and that stripping ANSI yields identical canonical bytes.

## 4. Preserve bounded transcode and publication

- [ ] 4.1 Extend the existing replay preparation to shape-dependent objects and recursive schemas without duplicate DOM/replay retention; verify wide root/object-field keyed candidates spill securely or fail under the aggregate memory/spool/output/nesting budgets.
- [ ] 4.2 Charge nested object/array candidates, schema names/indexes and transient composites to the shared preparation arena; verify nested threshold exhaustion, spool denial, temporary-file cleanup, cancellation and sink failures do not leak resources or allocate beyond limits.
- [ ] 4.3 Replay each final object/array body once after canonical layout selection; verify late keyed/tabular invalidation emits expanded output without speculative headers, lost entries, duplication or reordering, and document/transcode bytes match.
- [ ] 4.4 Preserve early sequence RS, default LF result terminators, prior complete results on late evaluation errors and atomic exactly-one unframed commitment; verify live sink observations for zero/one/many values, late input/duplicate/resource errors and sequence/unframed differences.
- [ ] 4.5 Update transcode proof, eligibility and `--explain`/run-report retention descriptions for whole-object shape preparation; verify real JSON/TOON identity paths remain bounded and report actual high-water/spool observations rather than claiming immediate object-member publication or upstream streaming semantics.

## 5. Cut over callers, options and metadata consumers

- [ ] 5.1 Discover references to public folding/expansion types and fields with available language-server references before removal; remove the types/config/re-exports, DOM helpers, CLI flags, proof fields and obsolete tests, migrating all callers without shims; verify both removed flags fail before input consumption and surviving options retain behavior.
- [ ] 5.2 Update bounded format probing for keyed headers, BOM/comments and shared root `[]`; verify fragmented automatic input, `.toon`/explicit overrides, JSON-first shared-array tie-break and no-faildown/lookahead-limit behavior through the actual CLI.
- [ ] 5.3 Update corpus conversion, fixture metadata serialization/readers, semantic witnesses and TOON stdout-boundary recovery; verify actual keyed/nested/empty result streams consume all bytes and corpus representations preserve the ordered model except specified recursive header reordering.
- [ ] 5.4 Decode every live checked-in TOON ledger/fixture using 4.1 and minimally migrate incompatible syntax; verify typed values, literal markers, catalog membership and byte-pinned source inventories/reference hashes remain unchanged unless an explicit reviewed pin diff is required, retaining historical evidence as historical.
- [ ] 5.5 Update fixture/differential regression coverage across DOM/events/transcode and supported numeric domains; verify official expectations directly, replace incidental wording/count/name-skip tests instead of re-pinning them, and record any evidence-backed reference-domain exclusions.

## 6. Prepare coordinated tq 0.5.0 metadata

- [ ] 6.1 Set workspace/package versions and explicit inter-crate constraints to `0.5.0`, updating applicable lockfiles and live release/install identities; verify package metadata and actual `tq --version` while leaving historical version evidence untouched.
- [ ] 6.2 Update help, README, compatibility/streaming/migration guidance and changelog under the repository policy; verify documentation explains removed switches, literal dotted keys, new layouts, header-order equality and delayed/spooled object publication without unsupported compatibility or savings claims.

## 7. Fully regenerate jq-manual comparisons and token savings

- [ ] 7.1 Build and freeze fresh release/default tq, embedded host and comparison helpers; provision the unchanged pinned jq 1.8.2 reference and record source snapshot, native target, profile/features, versions and before/after executable hashes; verify identities are immutable and not debug/bench substitutes.
- [ ] 7.2 Execute the complete manual comparator into ignored `target/toon-4-1` evidence and staged Markdown, capturing all primary/compact/default-TOON/applicable-sequence campaigns; verify complete unique-case mapping, original 518 IDs, protected 303 contracts and no omitted skips/crashes/timeouts/missing references, recording the actual strict exit status.
- [ ] 7.3 Run affected full compatibility/corpus correctness and boundary-recovery paths using the frozen candidate, including embedded-denial companion observations; verify representation correctness and independent TOON/process gates without treating shared `observed-differences` status as exact manual acceptance.
- [ ] 7.4 Produce and review a case-by-case PR68-to-candidate output/eligibility/token diff; verify both `o200k_base` and `cl100k_base` ordinary-text counts include exact LF-terminated stdout, signed percentages and growth, exclude ineligible and separate sequence captures, and deduplicate section overlap in index totals.
- [ ] 7.5 Promote every reviewed generated jq-manual section and its index, refreshing current coverage/provenance and related comparison summaries; verify no page claims stale 0.4.1 captures or new savings without candidate evidence and retained exact differences still have their nonzero strict outcomes.
- [ ] 7.6 Renew supported-target manual evidence using native candidate builds where available; verify reports retain actual platform scope and mark unavailable targets unrenewed without waiving #69/#70 or claiming calibration under #31. Unavailable evidence is not a completed execution or release-wide acceptance.

## 8. Guard focused performance regressions

- [ ] 8.1 Implement the focused guard using existing native measurement/correctness/report primitives without relaxing the one-sample quick profile or calibrated publication gates; verify one warmup plus three alternating measured pairs, bounded seven-pair confirmation, unrounded per-workload `candidate / baseline > 1.10` decisions, and retained identities/samples/dispersion.
- [ ] 8.2 Add behavior regressions for the guard's deterministic decision logic: verify exactly +10% with zero dispersion passes, confirmed greater-than-10% slowdown fails, improvements elsewhere cannot offset it, MAD bands crossing the threshold and disagreement between rounds are inconclusive, and invalid/missing/zero-baseline/interrupted evidence cannot pass.
- [ ] 8.3 Exercise the actual baseline/candidate guard on the fixed matrix and final frozen release/default candidate, verifying shared semantic/process correctness despite changed canonical bytes, a real spool path, same-host comparability and an explicit deadline; retain initial/confirmation rounds and fail or leave acceptance blocked for every significant or unresolved noisy result without omitting rows or shrinking inputs.
- [ ] 8.4 Record the final native target, per-workload wall-time ratios, samples/dispersion, output sizes and actual gate outcome with CPU/RSS diagnostics when available; verify the report makes no calibrated publication or all-platform claim and document any mitigation followed by a fresh candidate verification.

## 9. Verify and close the implemented change

- [ ] 9.1 Run the actual CLI through nested/keyed input and output, empty arrays, comments/control escapes, automatic detection, non-strict duplicate behavior, colors, resource thresholds and all framing modes; verify observed values/bytes/errors and bounded preparation, not just compilation or mocked forwarding.
- [ ] 9.2 Run relevant conformance/regression suites and the repository preflight on the final candidate; verify and report individual gates, preserving the expected associated active-change merge-completion failure without waiving other checks or unrelated parity debt.
- [ ] 9.3 Run the existing release-package smoke for the 0.5.0 candidate and validate documentation/OpenSpec deltas; verify package identity, links, migration examples and artifacts without tagging or publishing a release.
- [ ] 9.4 Verify implementation against all four deltas and complete review/fix/re-verification; then synchronize and archive only `toon-4-1-spec` under the adopted OpenSpec lifecycle, verifying delta-to-main correspondence and final checks while leaving PR68's independently owned parity change alone.
