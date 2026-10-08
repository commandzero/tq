---
type: Report
title: "jq manual coverage"
description: "Current jq manual test coverage, comparison results, and known differences."
generated: { by: "process:toon-4-1-evidence-review", at: "2026-10-07T05:53:56.657Z" }
benchmark_runs: [{"platform":"macos","target":"aarch64-macos","campaign_id":null,"captured_at":null,"binaries":{"jq":{"version":"jq-1.8.2","sha256":"2d75340ba57a4b4b4c8708a21c2dc8e958a48aaa8bba13b27f77f6e4c0eca07e","identity_status":"measured"},"tq":{"version":"tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)","sha256":"8a0ba42cf81881829b93ee950db0d4d3f808a22af574b210484be7833b238d00","identity_status":"measured"}}},{"platform":"linux","target":"x86_64-linux","campaign_id":null,"captured_at":null,"binaries":{"jq":{"version":"jq-1.8.2","sha256":"b1c22172dd303f3be49e935aa56aa48a8b7a46e0bc838b4997d3bb451495870f","identity_status":"measured"},"tq":{"version":"tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)","sha256":"474625b11f0ec3a1fa125bbeb4e55008efa50ec713b0c398822ada48f0289b91","identity_status":"measured"}}},{"platform":"windows","target":"x86_64-windows","campaign_id":null,"captured_at":null,"binaries":{"jq":{"version":"jq-1.8.2","sha256":"a6fc67fedaf9128a3309a1e2ebb8b986aeccf70122ee46d2cb4849e423f0c627","identity_status":"measured"},"tq":{"version":"tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)","sha256":"a1cec37e4c6e70bcd06647a5b5b5617ece0b88e41fe8a1ad9590bafc00992428","identity_status":"measured"}}},{"platform":null,"target":null,"campaign_id":null,"captured_at":null,"source_role":"historical-fix-local-performance","source":"#longest-match-regex-support","binaries":{"jq":{"version":"1.8.1","version_source":"authored historical timing description; exact version stdout not retained","sha256":null,"identity_status":"not-recorded"},"tq":{"version":null,"sha256":null,"identity_status":"not-recorded"}}}]
---

# jq manual coverage

The suite covers all **251 published input/output examples** in the jq manual,
accounts for **68 fenced snippets**, and adds tests for documented behavior,
composition, and edge cases. It runs **952 unique cases** against pinned jq 1.8.2.
A mapped example is covered, but coverage alone does not mean it passes.

## Results

The detailed historical comparison and token results are in the
[measured overview](overview.md).

The overview retains the frozen C9 macOS results (943/952 primary,
919/921 compact JSON, and 921/921 TOON), with strict exit 1. This coverage page
describes broader native-target scope below; it does not claim fresh manual
capture, complete parity, or new platform acceptance.

## Differences by test

1. **Expected rounding difference — low significance.**
   [`manual.audit.math.erfc-ulp`](math-boundaries.md#manualauditmatherfc-ulp)

   `erfc` on `2`: jq returns `0.0046777349810472645`; tq returns
   `0.004677734981047266`. tq is 2 floating-point steps (ULP) higher.

2. **Expected rounding difference — low significance.**
   [`manual.audit.math.tgamma-ulp`](math-boundaries.md#manualauditmathtgamma-ulp)

   `tgamma` on `0.5`: jq returns `1.772453850905516`; tq returns
   `1.7724538509055159`. tq is 1 ULP lower.

3. **ANSI styles — expected presentation difference.**
   [`manual.colors.ansi-values`](colors.md#manualcolorsansi-values)

   The test checks custom colors and text styles across JSON types.
   tq styles enclosing quotes differently; the JSON data is unchanged.

4. **Bright-red palette — expected presentation difference.**
   [`manual.colors.custom-1-31`](colors.md#manualcolorscustom-1-31)

   Both tools use the requested bright-red color.
   ANSI escape/reset placement differs, not the JSON data.

5. **jq-style palette — expected presentation difference.**
   [`manual.colors.default-palette`](colors.md#manualcolorsdefault-palette)

   The test supplies jq's palette through `JQ_COLORS`.
   tq gives enclosing quotes the structure's style, not the key/string style.

6. **Forced color — expected presentation difference.**
   [`manual.invoking.color-output`](invoking-jq.md#manualinvokingcolor-output)

   Both tools emit colored JSON with `-C`.
   Quote styling differs, not the JSON data.

7. **Custom `JQ_COLORS` — expected presentation difference.**
   [`manual.invoking.jq-colors`](invoking-jq.md#manualinvokingjq-colors)

   Both tools apply the custom palette.
   tq styles quotes as structure rather than key content.

8. **Forced color with `NO_COLOR` — expected presentation difference.**
   [`manual.invoking.no-color-forced`](invoking-jq.md#manualinvokingno-color-forced)

   Both tools let `-C` override `NO_COLOR` and emit colored JSON.
   Quote styling differs; option precedence and JSON data agree.

9. **Test-runner messages — no effect on the supplied test's result.**
   [`manual.invoking.run-tests`](invoking-jq.md#manualinvokingrun-tests)

   jq 1.8.2 prints extra internal self-test messages that tq does not emit.
   Both tools pass the supplied test and exit 0.

On macOS, all 6 color cases are expected presentation differences.
They produce identical output after removing ANSI styling, even with the same palette.

The reports mark a color difference as expected only when the observed data and
process behavior agree. Exact colored-byte checks remain strict.

The existing user acceptance of these expected scoped behaviors applies only
to the measured inputs and presentation/test-runner contracts. The 6 ANSI
cases follow the main `output-colors` contract; `--run-tests` differs in jq's
internal diagnostic suffix, not failure of the supplied program. This update
creates no fresh approvals: the strict report records 9 failures and 0 reviewed
disparities. Every exact observation remains, without blanket tolerances.

The math cause is independently rounded safe Rust `libm::erfc`/`libm::tgamma`
versus the pinned jq implementation, not an inferred host-libc guarantee.
Exact witnesses remain in `tests/compatibility/cases/manual-math-boundaries.jsonl`;
`crates/tq-core/tests/math_compat.rs` supplies dispatch regression coverage, not
a blanket ULP bound. The six color witnesses remain in
`tests/compatibility/cases/manual-colors.jsonl` and
`tests/compatibility/cases/manual-invoking-jq.jsonl`,
with palette/quote/plain-byte regressions in `crates/tq-cli/tests/output_colors.rs`.
The supplied `--run-tests` success/failure contract is covered by
`crates/tq-cli/tests/extended_cli.rs::run_tests_executes_stdin_test_files_and_reports_failures`.
Reconsider math after library/reference changes, colors after presentation-contract
changes, and the runner suffix after runner/reference changes; rerun exact
witnesses and protected contracts rather than renewing approvals automatically.

## Native platform results

Current C9 manual evidence is retained in ignored `target/toon-4-1/candidate-c9/`.
The complete 903-file source snapshot is on integrated revision
`407b35d81681b8ca4ab85968f7186eea645f250f`, SHA-256
`b0a3e076abc748319859ea9810fd75bc93132b05c5b07cc116f97ffe27ecbc62`.
This is a reviewed working-tree snapshot, not a clean release commit.
C9 rebuilds only the native manual comparator to enforce required process
contracts; product sources and all frozen C8 tq/embedded binaries are unchanged.
Later documentation/checklist/spec synchronization is outside that snapshot.

| Native target | Primary exact / cases | Compact exact / cases | TOON matches / cases | Strict exit |
| --- | ---: | ---: | ---: | ---: |
| macOS ARM64, release/default | 943/952 | 919/921 | 921/921 | 1 |
| Linux AMD64, release/default | 937/952 | 913/921 | 921/921 | 1 |
| Windows AMD64 MSVC, native PowerShell, release/default | 912/952 | 879/921 | 921/921 | 1 |

All three runs retain all 518 original IDs and all 303 protected IDs.
macOS/Linux retain 303 protected primary and applicable-compact exact contracts;
Windows retains 291 primary and 289 primary-plus-compact protected matches,
not 303 exact. All 4,667 observations per target execute and exit; no mapping
is removed and no new primary failure appears. Baseline-to-C9 review records
41 changed cases on macOS/Linux and 43 on Windows. The changed Windows
`manual.modules.path-origin` captures now show tq's `origin-path` result while
pinned jq still reports a missing module; it remains a #69 failure, not a pass
or eligible size sample.

Windows's formerly eligible `manual.audit.streaming.json-root-boundary-nested-empty`
is now explicitly excluded: values match, but required jq stderr has CRLF and tq
has LF. This retained #69 process-contract failure cannot enter savings.
The other 857 Windows samples remain eligible; older 858-sample reports are
immutable historical evidence, not silently corrected or relabeled.

The macOS CLI SHA-256 is
`8a0ba42cf81881829b93ee950db0d4d3f808a22af574b210484be7833b238d00`;
Linux is `474625b11f0ec3a1fa125bbeb4e55008efa50ec713b0c398822ada48f0289b91`;
native Windows is `a1cec37e4c6e70bcd06647a5b5b5617ece0b88e41fe8a1ad9590bafc00992428`.
Native toolchains, all 15 executable identities, before/after unchanged reference
hashes, complete raw reports and 903-file source verification are retained per
target. The newly rebuilt macOS/Linux/Windows manual-comparator hashes are
`a581e66aa551e2c3397ae4507c6daf8b7a6def331e2ce5697664e3ed134e4cf4`,
`42567656150b80c0b0ccd18309839ba7d5ca0fdd3b8287f6b3fd25c16cfb88c0` and
`84889bf94101d48780cd888dbb4ec70ad6271165431118ae884d30df24223ea2`.
Windows executes native MSVC binaries in PowerShell 5.1; WSL is transport only.
The jq reference remains 1.8.2; no source/reference pin was changed.

| Native target | Eligible unique cases | `o200k_base` JSON → TOON | Signed % | `cl100k_base` JSON → TOON | Signed % |
| --- | ---: | ---: | ---: | ---: | ---: |
| macOS | 877 | 11,391 → 8,152 | −28.43% | 11,365 → 8,165 | −28.16% |
| Linux | 871 | 11,245 → 8,022 | −28.66% | 11,219 → 8,035 | −28.38% |
| Windows | 857 | 11,187 → 7,978 | −28.69% | 11,161 → 7,991 | −28.40% |

Both encodings count exact complete ordinary stdout, including terminating LF;
empty streams have zero tokens and unavailable zero-denominator percentages.
Each target retains 22 growth rows per tokenizer. Section overlap is
deduplicated in the overview. Separate sequence and ineligible captures do not
enter ordinary-output totals. This correctness corpus is not a workload
benchmark, and these totals are not all-platform savings guarantees.

The final macOS full compatibility campaign freshly executes 4,804 of 6,765
observations across 1,220 cases, retaining 1,961 unsupported observations and
218 pairwise differences. There are zero harness errors, non-exited executions
or declared-contract failures. Six actual embedded-denial observations use
the frozen embedded host across JSON/YAML/TOON, with runtime-policy exit 5
and empty output. `observed-differences` is not strict manual acceptance.

Final macOS full repository preflight passes with `RUST_TEST_THREADS=1`,
including formatting, locked workspace/all-target checking, all-feature Clippy
with warnings denied, workspace tests, microbenchmark smoke and 21 strict
main-spec validations. The preceding parallel run is retained as nonpassing:
two RSS tests hit OS `ps`-inspection timeouts. Serialization changes only test
concurrency; no RSS limit, inspection deadline or assertion is relaxed.
All 72 native TOON all-feature conformance tests pass separately. Of 11 ordinary
workspace ignored entries, four reference/relocation tests explicitly pass;
the remaining seven accounting/helper entries are not passes or calibration.
The earlier pre-commit associated-change boundary exited 1 and is retained;
committed archive correspondence remains a separate PR-boundary gate, not
waived by preflight. Forty actual final CLI scenarios cover
format discovery, layouts, framing, controls, colors, duplicate normalization,
depth/resource boundaries and a real keyed disk replay. All 96 live metadata
ledgers decode identically through the final native CLI. The actual 0.5.0
release-package smoke also passes using the real native binary; temporary
fixture tags are not an official release or publication.

The separate same-host performance guard belongs to the comparison PR, not
this manual report. That PR reported a user-approved 20% hard limit and a
14.32% keyed-transcode slowdown advisory; its earlier noisy run remains
nonpassing. This historical manual capture is not performance evidence.
No calibration, exact all-platform acceptance or #69/#70/#31 waiver is claimed.

## Historical tq 0.4.1 native platform closeout

Historical macOS/Linux closeout evidence dated 2026-10-06 is in
`target/closeout/macos/newhelp-final` and `target/closeout/linux/help-final`, using newly
built frozen release campaign executables. Both full source snapshots contain
884 files at base `3e0dedb` plus uncommitted changes, not clean-commit builds.
The macOS snapshot is unchanged; Linux separately verifies a 530-file executable
source manifest excluding docs/specs/reports and Markdown, with the full snapshot
retained. These labels describe the measured snapshots, not later documentation edits.

Since late-p2, `crates/tq-cli/src/args.rs` corrects the environment/platform help
wording and adds `help_distinguishes_cli_ambient_access_from_embedded_admission`.
It distinguishes CLI-enabled ambient access from embedded admission and removes
the false redaction claim; runtime policy logic is unchanged. Final-help product
and helper binary hashes differ from prior checkpoints, not renewed same-hash proof.
Both latest manual runs retain **518 original cases** and **303/303 protected
exact contracts**. macOS reruns all 297 CLI composition witnesses exactly and
its embedded inventory in the workspace suite. Linux reruns full all-feature
workspace, manual/full campaigns, help, guards and explicit reference/relocation
checks. Reconsider help after parser/capability changes by rerunning the wording
regression, release `--help`, and embedded authority guards.

Windows (`smokescreen`, Windows 11 Pro AMD64/MSVC) now has native release/default
renewal in `target/closeout/windows/final-286b5f6/native`. Source is base
`286b5f6` plus **one integration-test-only overlay**,
`crates/tq-test-support/tests/compatibility_catalog_contracts.rs`, SHA-256
`a86772509c2e279b823e6f02529e976c73b8c75ed30d23bea1620acf828a6c30`.
Structured jq probes use `--binary` while retaining exact stdout/stderr assertions
and authored raw arguments; `compact_reference_args` extraction changes tests,
not runtime product/helper code. Frozen release source/binary identities are
unchanged across the overlay: campaigns are mapped, **not a full release rebuild
or campaign rerun after the test fix**. macOS/Linux product/helper sources also
remain unchanged; this test is their only crate-source delta.

Actual system `/usr/bin/ssh` and `/usr/bin/scp` succeeded; tests execute in native
Windows PowerShell, with WSL only transport. Earlier macOS-client SSH failures
and blocked readiness summaries remain historical, not current host availability.
No security changes were made.

Windows retains all **518 original cases** and all **303 protected IDs**, but
only **291 protected primary matches** and **289 primary-plus-compact matches**.
That is **not 303 exact**; macOS/Linux still preserve 303 exact. All 297 Windows
CLI composition cases execute without tq gaps; 8 math/reference-availability
cases differ under #69. All 297 embedded witnesses and resource/recursion/
cancellation checks pass. Reference absence and deferred differences remain
observations, not successful matches.

| Platform | Primary exact / cases | Compact JSON exact / cases | TOON matches / cases |
| --- | ---: | ---: | ---: |
| macOS | 943/952 | 919/921 | 921/921 |
| Linux | 937/952 | 913/921 | 921/921 |
| Windows — current native release, mapped test overlay | 912/952 | 879/921 | 921/921 |

The primary and compact campaigns overlap. Their combined difference counts
are **9 on macOS, 15 on Linux, and 60 on Windows**; do not add the columns.
All three retained native strict commands exit 1. Windows counts happen to equal
the historical v6 counts, but are now tied to different current release hashes
and mapped source proof. Renewal is evidence, not additional approvals or an
all-platform compatibility pass.

### Additional platform differences

1. **Math rounding:** Linux `y0`/`yn` differ by 1 ULP; Windows by 2 ULP.
   This affects `manual.math.y0`, `manual.math.yn`, and their
   `manual.composition.arity.*` witnesses. Linux/Windows `erfc(2)` differs
   by 1 ULP; `tgamma(0.5)` differs by 1 ULP.
2. **Large/non-finite scale exponents:**
   `manual.audit.math.integer-scale-boundary` and
   `manual.composition.arity.scalbln.2` produce different values on Linux
   and Windows. jq returns `[0,M,0,0,0]`; tq returns `[M,M,0,2,2]`, where
   `M` is the largest finite binary64 value. **This is not rounding.**
   tq uses safe Rust conversion rules; jq's out-of-range C conversions are
   platform-dependent.
3. **Unavailable Windows jq math functions:** `drem`, `exp10`, `gamma`,
   `scalb`, and `significand` are absent from the pinned Windows jq build.
   tq implements them. Their manual/composition witnesses and the `scalb`
   boundary witness account for 11 differences.
4. **Windows line endings:** 32 raw-input, streaming, I/O, and CLI cases
   observe jq CRLF versus tq LF. Structured jq programs use explicit
   `--binary`; raw contracts keep their original arguments. No captured
   bytes are normalized into an exact match.
5. **Windows ANSI and test-runner output:** the 6 color cases and
   `manual.invoking.run-tests` also retain native newline differences.
6. **Windows module lookup:** `manual.modules.path-tilde` and
   `manual.modules.path-origin` fail in the pinned jq build while tq resolves
   the supplied paths. These remain unapproved differences.

[Current closeout metadata](../../../tests/compatibility/reviews/parity-closeout.toon)
maps source manifests, profiles, executable/report SHA-256 hashes, retained raw
evidence, and follow-up scope. [Historical v6 metadata](../../../tests/compatibility/reviews/native-platform-acceptance.toon)
remains unchanged. Its four referenced Windows summary/decoded/ID/gate files and
all 81 retained v6 manifest entries are available and hash-matched, including the
full raw manual report; retention integrity is not current-candidate proof.
Raw evidence is retained locally under ignored `target/`; hashes alone are not
an off-host archive or a substitute for retaining those files.

Live issue bodies #69/#70/#31 were checked on 2026-10-06 and confirm the
user-approved transfer, with all three issues open. The superseding #69/#70
status headers record repaired shared harness debt; older unchecked checklist
wording remains historical. Transfer separates implementation/evidence closeout
from strict or reviewed-completion acceptance:
- [Windows — #69](https://github.com/commandzero/tq/issues/69) owns the enumerated
  60 target/case differences: 6 rounding, 2 scale conversion, 11 unavailable
  reference-function, 32 newline, 6 ANSI/newline, 1 internal-runner/newline,
  and 2 module-lookup witnesses. Native renewal is complete; differences remain
  unresolved, including the non-exact protected contracts.
- [Linux — #70](https://github.com/commandzero/tq/issues/70) owns the enumerated
  15 differences: 6 rounding, 2 scale conversion, 6 ANSI, and 1 internal-runner
  witness. Current execution retains these as unresolved, not approved.
- [Calibration/performance — #31](https://github.com/commandzero/tq/issues/31)
  and platform follow-ups own unfinished calibration and affected comparisons.
  Windows accounting checks do not prove macOS calibration or acceptance.

The exact target/case map is retained in the closeout metadata. Reconsider each
witness after implementation, reference/library changes, or platform fixes;
rerun its exact contract and protected baseline before changing a verdict.
No transfer normalizes bytes/values, raises match counts, or relaxes a gate.

## Longest-match regex support

The manual's `match("a|ab"; "l")` example passes. The implementation uses the
existing safe-Rust `fancy-regex` library, with no added dependency or FFI.
Conservative match-length bounds avoid unnecessary endpoint searches.

Local release-build measurements against jq 1.8.1 include startup, execution,
and output. These timings are separate from the jq 1.8.2 correctness run:

| Workload | jq median | tq median | tq / jq |
| --- | ---: | ---: | ---: |
| One manual example | 4.69 ms | 5.31 ms | 1.13× |
| 1,000 manual examples in one process | 9.37 ms | 54.93 ms | 5.87× |
| `match("a"; "l")` on `"a"` followed by 500 `"b"` characters | 4.67 ms | 6.66 ms | 1.43× |

These historical fix-local timings are not calibrated closeout performance
acceptance. Repeated compilation remains a cost
for batched inputs. Complex searches can reach resource limits; whole-pattern
recursion with `l` is unsupported, and some engine-specific edge cases remain.
These measurements are not general performance guarantees.

The separate historical timing entry records the documented jq 1.8.1 version;
the measured tq version and executable digests were not recorded with this
table and remain unknown. These rows are not measurements of the C9 binaries.

## Historical tq 0.4.1 details and verification

- [Comparison results](overview.md): measured results and token measurements.
- [Source inventory](../../../tests/compatibility/reviews/jq-manual/source-examples.toon): manual examples and source-to-test mappings.
- [Test setup](../../../tests/compatibility/readme.md#jq-manual-coverage): reference setup and coverage checks.

- Final-help macOS full workspace/all-feature tests, full preflight, final
  release build and explicit two-test manual relocation pass; help, embedded
  composition and resource regressions pass. Three initial disk-full preflight
  attempts are retained; the successful full script changes only incremental/
  dev-test debug storage settings, not checks or final release/default settings.
  The separate latest OpenSpec PR-boundary check exits 1 on uncommitted OpenSpec
  changes; preflight does not clear the active-change merge boundary.
- Final-help Linux workspace/all-feature tests: **1,891 passed, 0 failed,
  11 ignored**. Four of those ignored entries explicitly passed separately:
  corrected-catalog jq reference, published manual reference, and full/focused
  relocated manual tests. The remaining seven accounting/worker/helper entries
  are not passes or calibration proof. Strict all-target/all-feature Clippy,
  10 fakehost regressions, 43 embedded/catalog guards, help regression and release
  help checks pass. Full validator provisioning/preflight remains a #70 follow-up,
  distinct from the passing full Cargo workspace.
- Windows final-overlay all-feature workspace, strict all-target/all-feature
  Clippy and formatting pass; native shell/byte/newline/stack and release campaign
  evidence are retained and source/hash-mapped. **10 ordinary workspace entries
  are ignored**; all **5 relevant reference tests explicitly pass separately**
  (catalog, published manual, native raw/binary bytes, full/focused relocation).
  The catalog also passes targeted release exact assertions. No workspace pass
  total is asserted here; remaining ignored helpers/accounting are not passes.
  Historical CRLF-reference and 101/100 Clippy failures remain in red logs.
- Corrected shared full campaigns cover **1,220 cases**, **4,804 executed** and
  **1,961 unsupported observations** each, with **0 harness errors and 0 declared
  contract failures**. Embedded environment/platform denials now use the embedded
  host for JSON/YAML/TOON; the stale denial/clock harness-error claim is superseded
  for all three retained source-bound native campaigns. Windows additionally
  retains 12 exact release denial byte observations with runtime-policy exit 5.
- macOS reuses the separately source-bound
  `target/compatibility/embedded-denial-p2-closeout-3e0dedb/full.json` campaign:
  93 cases retain the P2 checkpoint's 224 pairwise differences. This copied/
  retained campaign **predates the help fix**, not a final-help macOS full rerun.
  Its source equals the earlier late-p2 snapshot only; the sole crate-source delta
  to final-help is diagnostic help wording and its test. Runner/policy/catalog
  logic is unchanged, but CLI/helper/embedded hashes differ—never same-hash proof.
  Linux reruns the full campaign with the final-help CLI/helper/embedded host and
  retains 99 pairwise-difference cases. Windows's current mapped release full
  campaign retains **146 difference cases**, with zero harness/declared-contract
  failures and 4,804 executed observations; its unsupported rows stay unsupported.
  All recorded full campaigns exit 0 with `observed-differences`, **not strict
  manual acceptance**.
- P2 regressions in `crates/tq-test-support/tests/compatibility_fake_executables.rs`
  verify selected-authority denial without output/disclosure and independently
  enforce the JSON companion contract. Unrelated errors, permissive host results
  and partial output no longer qualify as policy-denial success. The latest raw
  campaigns retain the exact runtime-policy exit 5 and empty-output observations
  for both denial fixtures across JSON/YAML/TOON.
- No fresh calibration or calibrated performance acceptance is established on
  any platform. Historical Windows controls exceed the unchanged 20 ms CPU
  tolerance; unfinished macOS calibration and affected benchmark comparisons
  remain unpublished as accepted performance evidence under #31/platform follow-ups.

Run a new comparison from the repository root after configuring the reference
and building tq:

```sh
cargo run -p tq-test-support --bin tq-manual-compare -- \
  --markdown-dir target/jq-manual-review target/manual-comparison.toon
```

The command regenerates the [overview](overview.md) and all 21 section Results
blocks. The [index](index.md) remains frontmatter-free navigation, without a
generated Results block.

For `--render-only`, supply a saved report with the complete reviewed/catalog
case inventory. Missing, duplicate, unexpected or malformed case IDs are
rejected before page updates. Complete reports with differences still render;
rendering is not strict acceptance.

Capture identity uses only explicit `campaign_id` and `captured_at` fields.
Live execution records capture completion; historical missing fields stay null.
Document `generated_at` does not supply a missing capture timestamp.

The historical **newhelp-final** report was rendered into all 21 section Results
blocks. At that checkpoint, twenty section bodies were byte-identical;
invoking-jq changed the help capture, not match counts. Authored blocks remain
preserved and repeated rendering is idempotent.

The current overview retains source-bound C9 provenance for macOS, Linux and
Windows. The earlier newhelp-final/help-final reports below are historical
checkpoints, not source/version labels for the C9 capture.
The [implementation/evidence change](../../../openspec/changes/archive/2026-10-06-achieve-jq-manual-parity/tasks.md)
is verified, synchronized and archived with 69 tasks complete under the
approved scope; #69/#70 and calibrated performance follow-ups remain open.
Current published-reference and source/pin/strict-gate checks pass in the retained
campaigns; this does not turn the manual strict exit 1 into success.

Historical macOS raw evidence is in
`target/closeout/macos/newhelp-final/manual-full-final.json`; Linux is in
`target/closeout/linux/help-final/evidence/manual-strict.json`. Earlier late-p2,
`20261006T071209Z`, `run-tgrv3qmn`, and catalog-contract shared artifacts remain
referenced as checkpoints in the metadata, not current-source labels. See the durable
closeout record for hashes and retention limits. macOS retains 9 exact-contract
differences and Linux 15.
