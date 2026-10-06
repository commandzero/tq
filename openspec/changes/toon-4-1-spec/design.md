## Context

See proposal.md for motivation and scope. The worktree starts on PR68 commit `286b5f6690bc1bee260cba007f3953c52178bf68`, with workspace version 0.4.1. `toon-format =0.5.0` is a test oracle in `tq-toon` and a runtime dependency of `tq-test-support`; production parsing and writing are native `tq-toon` implementations. Updating the package alone cannot update tq output.

The pinned [4.1 specification](https://github.com/toon-format/spec/blob/v4.1.0/SPEC.md), rather than upstream `main`, governs conformance. [Rust 0.6.0](https://github.com/toon-format/toon-rust/releases/tag/v0.6.0) removes folding/expansion, rewrites both directions, and replaces bounded JSON streaming encode with full-document convenience functions. Registry inspection also found 0.6.1; its release notes list dependency/workflow updates only. Keep exact 0.6.0 as requested, not an incidental latest-version/spec upgrade.

### Exercised evaluation

An isolated Rust executable linked the PR68 native libraries with published `toon-format =0.6.0`, encoded five shapes through both, decoded new output through both, and probed five parser boundaries. The upstream output round-tripped in each encoding case. Observations:

| Shape | PR68 native output | Upstream 4.1 output / native interpretation |
| --- | --- | --- |
| Empty object-field array | `empty[0]:` | `empty: []`; native decoder rejects it |
| Uniform nested object columns | Expanded list objects | `orders[2]{id,customer{name,country}}:`; native accepts but incorrectly creates flat keys `customer{name` and `country}` |
| Uniform keyed object | Expanded `users/alice/bob` nesting | `users[2:]{age,city}:`; native rejects keyed marker |
| Hash-leading text, U+0001, non-ASCII key | Unquoted `#data`, literal control, unquoted `é` | Quoted hash text, `\u0001`, quoted key; native rejects the escape |
| Anonymous nested array of uniform objects | Keyless tabular header after hyphen | Nested list form, which native decodes correctly |

Additional probes: comment plus object, root `[]`, and `\u0001` decode upstream but fail natively; strict duplicate sibling keys and an over-indented scalar fail in both. These are migration observations, not full conformance or release evidence. No new savings figures were measured.

### Existing constraints

1. `Decoder` retains bounded lines/tokens and emits ordinary structural events. The actual public `Event` does not expose a flat table schema; internal decoder frames reconstruct values before events reach DOM/query consumers.
2. `PreparationArena` already shares memory, spool, output and nesting budgets; replay storage, external object-key indexing and atomic publication exist. Current table detection and preparation primarily target arrays and flat primitive schemas.
3. `WriterConfig` exposes `KeyFolding`/`flatten_depth`; `DecoderConfig` exposes `PathExpansion`. CLI `--fold-keys`/`--flatten-depth` and `TranscodeProof::key_folding_disabled` perpetuate removed behavior.
4. Canonical output feeds CLI results, corpus conversion, fixture metadata and comparison reports. Existing TOON ledgers and pinned byte hashes must not be silently rewritten.
5. Current published macOS manual evidence: 952 unique cases, 943 primary exact, 919/921 compact exact, 921/921 TOON contracts, 877 eligible token examples. Totals are 11,391 JSON / 8,204 TOON tokens (-27.98%) for `o200k_base`, and 11,365 / 8,214 (-27.73%) for `cl100k_base`. These belong to the documented 0.4.1 source overlay, not a clean 0.5.0 build. Windows final-candidate renewal remains absent; Linux/Windows differences and performance calibration are independently owned.

## Goals / Non-Goals

**Goals:**

- One native canonical 4.1 contract across document, transcode, plain and colored sinks, corpus generation and evidence readers.
- Recursive table schemas with explicit position-dependent eligibility; lossless supported numbers and bounded parser/preparation state.
- Clean removal of folding/expansion, truthful resource/explanation behavior, and coordinated 0.5.0 metadata.
- Fresh full manual comparisons with reproducible captured-byte token totals and an explicit baseline-to-candidate review.
- A focused same-host wall-time guard that blocks confirmed per-workload regressions greater than 10% without averaging them away or claiming calibrated publication.

**Non-Goals:**

- Adopting TOON 4.2/4.3 or upgrading jq/yq/reference pins as a side effect.
- Replacing native streaming with upstream DOM parsing, changing jq query semantics, general JSONL metadata migration, new CLI formatting preferences, or broad benchmark campaigns.
- Completing PR68's active parity change, repairing its deferred platform differences, claiming calibration or all-platform acceptance, or publishing a release during proposal/apply without release authorization.

## Decisions

### 1. Spec authority and conformance

Vendor fixtures from tag `v4.1.0`, recording resolved commit, upstream paths, hashes and MIT attribution; replace the active `spec-v3` suite and remove obsolete folding/expansion fixtures. Verify official expected values, expected errors, and encoder bytes directly. Differential execution against exact 0.6.0 is an independent oracle within its documented numeric/depth domain, not permission to bless disagreements with official fixtures. Replace brittle incidental fixture-count assertions and name-based exclusions with auditable fixture membership/coverage and behavior checks.

Keep tq's exact-literal numeric domain and resource limits documented. Upstream has different integer/float out-of-range rules, so compare numeric fixtures against official expectations and explicitly documented domain differences, not an assumed universal upstream equivalence. New 4.1 permission for exponent forms outside the canonical decimal range does not require changing otherwise conforming exact tq spelling.

### 2. Recursive schemas and position-driven output

Represent a schema as ordered leaf/group entries. Validate names uniquely per group, flatten leaf traversal depth-first for row cells, and retain first-element/first-entry encounter order at every level. Later objects can have the same key set in another order; their output follows the established header schema. A null/object mix, array column, empty-object column, missing/extra key, or any later recursive mismatch invalidates the complete table candidate.

Apply 4.1 rules uniformly:

- Uniform non-empty object arrays use recursive tabular form at root/object-field positions.
- Objects with at least two uniform non-empty object values use keyed tabular form at root/object-field positions; outer entry order is preserved.
- Anonymous array list-item positions prohibit keyless fields-bearing headers; nested object arrays there use list form. Anonymous objects never get a keyed header.
- Root and object-field empty arrays emit `[]`; empty inner array list items still emit `- [0]:` with the configured delimiter suffix. Decoders accept legacy empty-array headers and `- []` as specified.
- A tabular/keyed first field on a list-item hyphen line has its rows at hyphen depth +2 and sibling fields at +1.

Use the same eligibility/schema and primitive encoding logic for DOM writing and prepared replay. Plain and colored writers differ only at presentation spans: strip ANSI and the canonical bytes must agree. Do not expose a new user choice for form selection, since 4.1 makes it mandatory.

### 3. Generalize bounded replay rather than materialize

A root/object-field object can become keyed tabular only after its entry count and complete recursive shape are known. Therefore the old assumption that every completed object member can be emitted immediately is unsound. Extend existing preparation to objects, staging one replayable semantic representation while updating a bounded candidate schema. Commit headers only after the enclosing candidate closes. Nested candidates, schema names/indexes and transient composite state share `PreparationArena` charges and nesting limits; eligible preparations spill securely, otherwise fail before exceeding the bound. Do not keep simultaneous DOM and replay copies or construct a result-sized output string.

Replay the selected final body in encounter order once; output bytes use the final schema/position rules. An object that is proven ineligible can release completed prefixes when publication permits, but speculative table headers are forbidden. A still-eligible wide object may require spool space and delayed document bytes even when it ultimately uses expanded form. This is a deliberate latency/storage trade-off for canonical conformance, not full-root RAM materialization.

Retain early RS publication for direct sequence output, LF result terminators, earlier completed results on later evaluation failure, and atomic exactly-one unframed publication. Report shape-dependent object/array preparation and existing high-water/spool observations honestly. Update obsolete `Single-source array preparation`/duplicate-publication/wide-object wording to avoid promising bytes before shape determination.

Alternative rejected: disable transcode for all objects or use upstream `encode_json_stream`. The former loses a supported bounded conversion path; the latter reads the whole document and violates existing memory contracts.

### 4. Decoder, events, and non-strict behavior

Keep semantic `ObjectStart/Key/Scalar/ObjectEnd` and array events independent of physical table syntax. Parse nested field groups with bounded frame state; reconstruct each row's nested object events in header order. Keyed tables emit object boundaries and entry keys, not array events. Strict count/width/header/duplicate checks must execute even for event consumers discarding values; DOM-only duplicate detection is insufficient.

Lexically exclude BOM/line-ending CR and full-line comments before structural interpretation; this can be done incrementally per line without reading the full input. Comments never open/close scopes and arbitrary leading spaces on comment lines bypass indentation checks. Preserve positions from original input. Quote hash-leading strings and ASCII-pattern-invalid keys; implement control/BMP `\uXXXX` escaping and reject surrogate escapes, invalid quoted-token boundaries and misplaced scalars as required. Trim only U+0020 in token roles, not Unicode whitespace or active tab cells.

Implement strict duplicate sibling rejection and non-strict last-write-wins in document order. Non-strict count declarations must not truncate actual scopes; short rows omit missing fields and surplus cells contribute nothing. Any consumer that cannot retract values for duplicate resolution must choose a bounded staged/document path or reject unsupported planning before consumption, never claim an event path with different values. Retain finite configured bounds and fail closed rather than reserve attacker-declared counts.

### 5. Clean option and detection cutover

Remove `KeyFolding`, `PathExpansion`, their config fields/re-exports, DOM expansion helpers and obsolete CLI/proof branches. Migrate every caller/test after reference discovery. Removed CLI flags fail as unknown options before input consumption; do not silently ignore or alias them. Dotted names, quoted or unquoted, are literal keys.

Update bounded probing in `tq-formats` for keyed root headers, comments/BOM prefixes and new root empty-array ambiguity. Bare `[]` is valid JSON and TOON with the same value: preserve existing JSON-first shared-array tie-break without blocking explicit TOON or `.toon` parsing. New keyed root headers must not be misclassified as JSON arrays. Explicit format overrides and bounded no-faildown commitment remain intact.

### 6. Consumer, metadata, and release migration

Update both upstream pins and lock entries, corpus conversion, report serialization, semantic witnesses and boundary recovery. Read every live checked-in TOON fixture/ledger with the new decoder; migrate only syntax made incompatible by strict 4.1 rules. Legacy empty headers remain valid input. Preserve fixture literal markers unless a separate measured contract change justifies removing them. Do not mass re-encode historical evidence, byte-pinned inventories, or approved baseline records. If a pinned inventory truly must change, review its semantic and byte diff and update associated hashes explicitly, without changing case membership or reference executable identity.

Set workspace version and explicit local package dependency constraints to 0.5.0; verify fuzz/auxiliary locks and release scripts for live identities, retaining historical version records. Update installation/compatibility/migration docs, help and changelog under existing policy. Release packaging smoke must use the 0.5.0 candidate; actual publication remains separate authorization.

### 7. Full comparison regeneration and review

Use release/default builds, not debug or bench-profile evidence. Capture immutable product, embedded host and helper identities plus source revision/dirty manifest, target/profile, upstream/spec pins and reference identity. Run every manual case and all primary JSON, compact JSON, default TOON and applicable sequence observations. Preserve the current 952-case set (or explicitly reviewed superseding set), original 518 IDs and protected 303 exact contracts. Keep jq 1.8.2 source/reference pins unchanged.

Illustrative repository commands (actual paths come from provisioned references and frozen binaries):

```sh
cargo build --locked --release -p tq-cli -p tq-test-support --bins
TQ_JQ=target/reference-build/jq/jq TQ_BIN=target/release/tq \
TQ_EMBEDDED_HOST=target/release/tq-compat-embedded \
  target/release/tq-manual-compare \
    --markdown-dir target/toon-4-1/jq-manual-review \
    target/toon-4-1/manual-comparison.toon
```

The strict command can write complete reports and exit 1 for retained differences; preserve and explain that status. Do not hide it behind a pipe or equate a rendered report with acceptance. `--render-only` may render this newly captured report but cannot refresh old observations. Execute affected full compatibility/normalizer and corpus correctness paths too; avoid unrelated calibrated-performance work.

Review case-by-case changed TOON stdout, ordered-value/process contracts, eligibility additions/removals, both token counts and signed deltas, before promoting generated sections to every applicable `docs/tests/jq-manual` page. Count complete stdout via `encode_ordinary`, including LF and literal special markers. Use `(TOON - JSON) / JSON * 100`; negative means savings. Keep raw/error/non-equivalent/non-UTF-8 captures out of totals and sequence captures separate. Section overlap must not double-count unique index totals. Publish both savings and growth without a target savings threshold; this is a correctness corpus.

Refresh current provenance/coverage/related comparison summaries without rewriting historical platform evidence. Authoritative manual renewal for other supported targets uses native candidates and the same safeguards; if unavailable, mark that target unrenewed and do not claim release-wide acceptance. Issues #69/#70 and #31 are not waived by successful local regeneration.

### 8. Focused performance regression guard

Capture and freeze a clean PR68 release/default baseline before implementing the migration, recording its exact reconciled revision, executable hash, toolchain, features and measurement environment. Compare the final 0.5.0 candidate on the same native host with identical optimization, allocator, thread settings, input delivery, output sink and measurement boundaries. Historical overlay campaign reports are not a substitute for this baseline.

Use a small fixed matrix rather than the full corpus: TOON decoding to JSON, forced-document TOON encoding, identity transcode, recursive nested/keyed candidates, late shape invalidation and a preparation workload that genuinely crosses the spool threshold. Include ordinary flat data as a control. Reuse existing native runner, correctness normalization, measurement and report primitives where appropriate. Do not change the existing one-sample `quick` profile: repository policy rejects it as regression evidence. This is a focused local migration guard with repeated sampling, separate from calibrated regression/publication gates and issue #31.

Freeze workload identities, data sizes, queries, settings and correctness expectations before measuring. Use the same input bytes when both versions accept them. For new-format decoding that PR68 cannot parse, use each version's representation of the same fixed ordered semantic source, recording both byte counts/hashes and clearly labeling the comparison as end-to-end representation migration rather than an identical-byte parser comparison. Both versions must satisfy the same semantic/process contract; differing canonical TOON output bytes are expected and must not be rejected by an old byte-equality probe. Corpus preparation and correctness validation occur outside timed samples. Preserve unchanged baseline artifacts across reruns.

Run one warmup per binary and three measured baseline/candidate pairs per workload, alternating which binary runs first. Record individual direct spawn-to-exit wall times and dispersion. Compare unrounded medians with `candidate / baseline > 1.10`: exactly +10% is not a breach. Evaluate each workload independently; improvements elsewhere cannot offset a failing row. Baseline time must be positive and measurements comparable.

Any initial breach or noisy threshold result triggers a bounded confirmation of seven additional paired measurements for that workload. Retain initial and confirmation samples separately and report both medians and median absolute deviations. As a conservative noise screen, derive the ratio band from each median plus/minus its MAD (candidate lower divided by baseline upper, candidate upper divided by baseline lower); a nonpositive baseline lower bound is inconclusive. This band is a dispersion heuristic, not a confidence interval. A band straddling 1.10, or disagreement between initial and confirmation threshold classifications, is inconclusive rather than a pass. A stable confirmation above 1.10 with its dispersion band wholly above the threshold fails the gate; a stable result at or below 1.10 with no threshold-straddling dispersion passes. Exactly +10% with zero dispersion passes.

Bound initial and confirmation execution with an explicit campaign deadline and retain completed rows if it expires. Do not trim workload sizes or omit slow/failing rows to satisfy the deadline. Correctness failures, missing rows, timeouts, crashes, incompatible identities and unresolved noisy results block acceptance. Return success only when every selected row passes; distinguish observed regression/failure from incomplete/inconclusive evidence using existing status conventions. Keep reports and raw samples in ignored evidence storage, with binary/input hashes, host/profile/settings, output sizes, threshold, sample counts and per-workload ratios. Record CPU/RSS when available but apply the requested 10% gate to elapsed time only. Missing calibration remains explicit; this guard cannot populate calibrated publication approval.

This check is quick by workload selection and bounded confirmation, not by claiming a full benchmark campaign or one-sample result is statistically reliable. Run it against the final frozen candidate before closeout; report its actual native target scope.

## Risks / Trade-offs

- Whole-object shape staging increases first-document-byte latency and disk use. Preserve early sequence RS, charge all candidate/schema state, exercise late invalidation and spool-denial/cleanup paths, and document the changed retention boundary.
- Silent flat-key decoding is worse than rejection. Official nested/keyed fixtures must compare actual reconstructed types/paths, not merely successful parse or flat row widths.
- Header-driven key reordering is now conforming only inside table values, recursively. Preserve outer object/array/result order and do not globally weaken equality or benchmark witnesses.
- Strict/non-strict duplicate behavior can diverge between DOM and events. Guard plan selection and observe real paths for both; do not limit validation to the DOM fixture suite.
- Existing TOON metadata includes hashes and literal preservation. Automatic reformatting can break byte pins despite unchanged values; inventory readers first and review minimal migrations.
- The 0.6.0 oracle is not universal proof for tq's exact numeric domain. Fixtures/spec requirements take precedence; record narrowly scoped oracle exclusions with evidence.
- PR68 can continue changing. Reconcile its latest base, active deltas and candidate evidence before apply/final verification, preserving this proposal's base identity rather than pretending the inherited overlay reports prove the candidate.
- A 10% wall-time threshold is sensitive to host noise and process startup. Fixed workloads must be long enough to expose format work, with alternating repeated measurements and bounded confirmation; threshold ambiguity blocks acceptance rather than becoming a false pass. This focused gate does not authorize calibrated publication.
