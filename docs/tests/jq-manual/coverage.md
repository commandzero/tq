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

The macOS comparison reports:

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

The math acceptance applies only to the 2 measured inputs.
The suite retains every exact observation without blanket numerical tolerances.

## Native platform results

Optimized-build comparisons ran on local macOS, Linux (`ironhide`), and native
Windows 11 Pro (`smokescreen`). Linux and Windows used release builds; the final
macOS run used the optimized bench-profile binary left by preflight.
WSL was transport only, not Windows test execution.

| Platform | Primary exact / cases | Compact JSON exact / cases | TOON matches / cases |
| --- | ---: | ---: | ---: |
| macOS | 943/952 | 919/921 | 921/921 |
| Linux | 937/952 | 913/921 | 921/921 |
| Windows | 912/952 | 879/921 | 921/921 |

The primary and compact campaigns overlap. Their combined difference counts
are **9 on macOS, 15 on Linux, and 60 on Windows**; do not add the columns.
All three strict commands exit 1. These are observed differences, not additional
approvals or an all-platform compatibility pass.

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

[Platform witness metadata](../../../tests/compatibility/reviews/native-platform-acceptance.toon)
records every difference ID, exact math observations, binary hashes, and the
shared source snapshot. Raw reports remain under ignored `target/` storage.

Remaining refinements are tracked separately:
- [Windows compatibility — #69](https://github.com/commandzero/tq/issues/69).
- [Linux compatibility — #70](https://github.com/commandzero/tq/issues/70).

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

This performance is accepted for the fix. Repeated compilation remains a cost
for batched inputs. Complex searches can reach resource limits; whole-pattern
recursion with `l` is unsupported, and some engine-specific edge cases remain.
These measurements are not general performance guarantees.

## Details and verification

- [Comparison results](index.md): test-by-test outputs and token measurements.
- [Source inventory](../../../tests/compatibility/reviews/jq-manual/source-examples.toon): manual examples and source-to-test mappings.
- [Test setup](../../../tests/compatibility/readme.md#jq-manual-coverage): reference setup and coverage checks.

- All-target/all-feature Clippy and workspace tests passed on all three hosts.
- macOS final preflight passed, including formatting and strict OpenSpec validation.
- Native PowerShell/cmd, binary-output, resource, and large-query stack tests passed.
- Windows release builds and both formerly overflowing arity queries passed.
- Windows CPU/RSS collection passes same-child counter checks, but timing
  calibration remains incomplete. Separate control runs exceed the unchanged
  20 ms CPU tolerance; no cross-platform performance acceptance is claimed.
- The full compatibility campaign still has stale embedded-denial fixtures
  running through the permissive CLI. Its three clock-comparison harness errors
  remain unresolved; the manual campaign is a separate check.

Run a new comparison from the repository root after configuring the reference
and building tq:

```sh
cargo run -p tq-test-support --bin tq-manual-compare -- \
  --markdown-dir target/jq-manual-review target/manual-comparison.toon
```

The jq 1.8.2 comparison regenerates the [index](index.md) and all 21 section
Results blocks. The 251 published-example reference check and 55 reference-pin,
workflow, coverage, and strict-gate tests pass.

Full local evidence is in `target/manual-comparison-jq-1.8.2.toon`, ignored by
Git. The strict comparison exits 1 because the 9 exact-contract differences remain.
