---
type: Report
title: "jq manual coverage"
description: "Current jq manual test coverage, comparison results, and known differences."
---

# jq manual coverage

The suite covers all **251 published input/output examples** in the jq manual,
accounts for **68 fenced snippets**, and adds tests for documented behavior,
composition, and edge cases. It runs **952 unique cases** against pinned jq 1.8.2.
A mapped example is covered, but coverage alone does not mean it passes.

## Results

The latest 2026-10-06 macOS **newhelp-final** comparison reports (release/default,
locked; pinned jq 1.8.2; base `3e0dedb` plus uncommitted implementation/catalog,
embedded-denial helper and final CLI help fixes):

| Check | Exact matches | Differences | Cases |
| --- | ---: | ---: | ---: |
| Values and process behavior, including CLI contracts | 943 | 9 | 952 |
| Compact JSON output | 919 | 2 | 921 |
| TOON values and process behavior | 921 | 0 | 921 |

JSON value comparison ignores whitespace and object key order, but preserves
array and result order. Compact JSON and CLI contracts compare exact bytes.
Raw CLI cases are not included in the JSON/TOON output checks.

**9 differences remain:** 2 expected math rounding differences,
6 expected presentation differences, and 1 test-runner message difference.
The all-cases exact gate does not pass.
These results describe the measured build, not every platform or release.

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

Latest macOS/Linux closeout evidence dated 2026-10-06 is in
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

## Details and verification

- [Comparison results](index.md): test-by-test outputs and token measurements.
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

The jq 1.8.2 comparison can regenerate the [index](index.md) and all 21 section
Results blocks. The parent has rendered the **newhelp-final** report into all
21 section Results blocks and updated index provenance. Twenty section bodies
are byte-identical; invoking-jq changes the help capture, not match counts.
Authored blocks remain preserved and repeated rendering is idempotent. Final
help/version/documentation proof reports no blocking mismatches. The manual
index now records renewed Windows provenance alongside the macOS Results.
The [implementation/evidence change](../../../openspec/changes/archive/2026-10-06-achieve-jq-manual-parity/tasks.md)
is verified, synchronized and archived with 69 tasks complete under the
approved scope; #69/#70 and calibrated performance follow-ups remain open.
Current published-reference and source/pin/strict-gate checks pass in the retained
campaigns; this does not turn the manual strict exit 1 into success.

Current macOS raw evidence is in
`target/closeout/macos/newhelp-final/manual-full-final.json`; Linux is in
`target/closeout/linux/help-final/evidence/manual-strict.json`. Earlier late-p2,
`20261006T071209Z`, `run-tgrv3qmn`, and catalog-contract shared artifacts remain
referenced as checkpoints in the metadata, not current-source labels. See the durable
closeout record for hashes and retention limits. macOS retains 9 exact-contract
differences and Linux 15.
