---
type: Report
title: jq compatibility disparities
description: Measured safe-library limitations and post-implementation reconsideration criteria.
generated: { by: codex/gpt-6-astra, at: 2026-09-13T23:50:52Z }
---

# jq compatibility disparities

## Implementation policy

tq targets the complete pinned jq manual using safe Rust and existing Rust
libraries. TOON remains the default output; `-c` selects compact JSON. Exact
parity does not justify adding unsafe code, a native FFI engine, or a maintained
unsafe bridge.

This document records measured limitations for reconsideration after the manual
spec is fully implemented. It is not a list of features we have chosen not to
implement. Missing functions, wrong argument handling, unexplained failures,
skipped tests, and regressions remain implementation work.

## Evidence and acceptance

Reports distinguish exact matches, reviewed disparities, and unresolved
failures. A disparity never raises the exact-match percentage. Keep the query,
input, both observed outputs and process statuses, jq build, tq revision,
dependency version, and target with each executable witness.

Each accepted disparity must explain its cause, practical impact, safe-library
tradeoff, regression test, and condition for future reconsideration. A numeric
bound must be justified for the specific function and tested at relevant
boundaries. Do not silently round values or apply a blanket tolerance. Different
types, result counts, or unknown functions are not numerical rounding issues.

## Current investigation status

The 905-case release reports below are historical checkpoints. New parser and
composition work expands the executable inventory to 952 cases and requires
fresh release builds, campaigns, and identity-bound approvals. The fresh v6
Linux debug campaign has 944 exact matches and eight math/regex differences,
with no skips or timeouts. It applies no approvals and does not establish
release acceptance. In that build the earlier `exp(1)` difference is exact.
The final release measurements must determine which approvals remain needed.

### Historical release approvals

Primary review accepted four specific numerical observations and one regex
restriction on the verified macOS build. The Linux registry now contains nine
renewed target-scoped observations, including the platform math and callback
boundaries. [The final Linux completion report](tests/comparison-x86-64-linux.md) records 896
exact results, nine reviewed observations, and zero unreviewed failures across
the 905-case inventory. This is a completion-campaign checkpoint for the
recorded x86_64 executable, not a claim that every remaining parity or release
gate is complete and not a waiver for another build or platform. The
[approval registry](../tests/compatibility/reviews/disparities-aarch64-macos.toon)
and [Linux registry](../tests/compatibility/reviews/disparities-x86_64-linux.toon)
bind each acceptance to exact executable identities, case fingerprints,
outputs, and process statuses. A changed build or observation requires renewed
review. Separate archive or framing reconciliation does not change the
target-scoped results recorded by either completion report.
Earlier `sqrt` and other math failures were missing implementations, not
evidence of platform precision differences.

The first integrated safe-Rust math comparison matched every executable Math
section witness. The two invalid-arity reference cases are retained as explicit
negative probes alongside their valid `/0` witnesses, with source provenance
separate from execution verdicts. These results provide no reason to introduce a rounding tolerance
for the manual's tested math inputs; they do not establish bit-for-bit agreement
for every input or platform.

### Current reproduction of the two math failures

The current macOS worktree reproduces both remaining mathematical failures with
pinned jq 1.8.1 and release tq 0.4.0. Run each query with `-c`, supplying the
listed JSON input on stdin:

| Manual case | Query | Input | jq stdout number | tq stdout number | tq relative to jq |
| --- | --- | --- | --- | --- | --- |
| `manual.audit.math.erfc-ulp` | `erfc` | `2` | `0.0046777349810472645` | `0.004677734981047266` | 2 binary64 ULP higher |
| `manual.audit.math.tgamma-ulp` | `tgamma` | `0.5` | `1.772453850905516` | `1.7724538509055159` | 1 binary64 ULP lower |

Both tools exit 0, emit one compact JSON number followed by LF, and emit no
stderr. These are numerical computation differences, not merely different
formatting of the same binary64 value:

| Query | jq binary64 value | tq binary64 value | Binary64 subtraction `tq - jq` |
| --- | --- | --- | --- |
| `erfc` | `0x1.328f5ec350e65p-8` | `0x1.328f5ec350e67p-8` | approximately `+1.7347234759768071e-18` |
| `tgamma` | `0x1.c5bf891b4ef6bp+0` | `0x1.c5bf891b4ef6ap+0` | approximately `-2.2204460492503131e-16` |

Direct diagnostic calls to macOS's system `erfc` and `tgamma` reproduced jq's
values. tq dispatches to the existing pure-Rust `libm` 0.2.16 implementations in
[math.rs](../crates/tq-core/src/math.rs), which round differently at these inputs.
The diagnostic system-library calls were external Python probes, not an FFI
addition to tq. Neither result's proximity nor its agreement with jq establishes
which implementation is more mathematically accurate.

`erfc` computes a complementary error-function value used in tail probabilities;
`tgamma` computes the Gamma function. The last-bit differences can affect exact
number comparisons, serialized bytes, snapshots, or downstream calculations.
The observed 2-ULP and 1-ULP distances apply **only to these witnesses**, not to
all inputs, targets, or a global numerical error guarantee.

The user accepted these two measured witnesses as **low-significance rounding
differences**, rather than blockers requiring further numerical implementation
work. That judgment is scoped to `erfc(2)` and `tgamma(0.5)` and the observations
above; it is not a blanket numerical tolerance or an accuracy guarantee.

Both cases remain **failures** in the current strict campaign. No tolerance,
input-specific correction, output rounding, or machine-readable identity-bound
disparity approval was introduced by this acceptance. A principled exact fix would require a supported safe backend whose
results are verified against the selected reference; direct native FFI conflicts
with the implementation policy above.

This reproduction used `macOS-26.7-arm64-arm-64bit-Mach-O`, jq executable SHA-256
`a9fe3ea2f86dfc72f6728417521ec9067b343277152b114f4e98d8cb0e263603`,
and release tq executable SHA-256
`cf7549683a1ee004fe98ab4f7a0dc0dc133569d9e05a4296387ae4ca66e628b6`.
These observations are distinct from the historical approvals below.

### Reviewed math boundary observations

The current implementation uses the standard library's platform operations for
`acos` and `exp`. Their two macOS boundary witnesses below now have exact VM
regression checks in `math_compat.rs`. The table records the earlier `libm`
observations; it must not be read as the current result for those two functions.
`erfc` and `tgamma` still use `libm` and retain their measured differences.

Broader input probes found last-bit differences between the safe `libm` 0.2.16
implementation and the pinned macOS jq build. Each observation below used the
named query with `-c`; both processes exited 0, emitted compact JSON followed
by LF, and emitted no stderr.

| Query | Input | jq 1.8.1 stdout number | tq stdout number | Observed binary64 ULP distance |
| --- | --- | --- | --- | ---: |
| `acos` | `0.5` | `1.0471975511965976` | `1.0471975511965979` | 1 |
| `exp` | `1` | `2.718281828459045` | `2.7182818284590455` | 1 |
| `erfc` | `2` | `0.0046777349810472645` | `0.004677734981047266` | 2 |
| `tgamma` | `0.5` | `1.772453850905516` | `1.7724538509055159` | 1 |

The final macOS boundary corpus covers 92 inputs across the six functions,
with 77 exact outputs and 15 last-bit differences. Its largest observed
distance for each function was 1 ULP for `acos`, 1 ULP for `exp`, 2 ULP for
`erfc`, 2 ULP for `tgamma`, and 1 ULP for both `y0` and order-zero `yn`.
These are sample-specific observations, not global error bounds or approval
thresholds. The earlier 66-input probe and its derived artifacts are retained
only as historical evidence and are not current completion identities.

ULP distance counts adjacent representable binary64 values between the observed
results. It describes these inputs only, not other inputs or platforms.

Observed on `aarch64-apple-darwin`, Rust 1.98.0, with the final macOS
completion executable. The jq executable has SHA-256
`a9fe3ea2f86dfc72f6728417521ec9067b343277152b114f4e98d8cb0e263603`; the
immutable tq executable has SHA-256
`e9478b0ad46e951a1654efbe4d9d52440da49b8b57283754a0c86fb69f1d4f0c`.
The 92-input probe is historical evidence. Its raw report is not retained in
this checkout. The [macOS disparity registry](../tests/compatibility/reviews/disparities-aarch64-macos.toon)
retains the identity-bound witness observations and executable hashes. All
processes exited successfully with empty stderr and the expected result counts under controlled
HOME, `LC_ALL=C`, and `TZ=UTC`.

Only the four exact case observations in the table were approved for the
earlier 608-case completion campaign. Their executable has SHA-256
`ef0fc5555acbf373a6680f14b0d8d5b67c504586afc18dc16fdbf468ce1d80b2`.
Primary review renewed these approvals only after confirming that every case
fingerprint, result, stderr payload, and status was unchanged from the earlier
reviewed executable. The campaign records 603 exact matches and five reviewed
disparities, not 608 exact matches.
This is historical approval evidence. The final macOS completion campaign
renewed the same five reviewed observations against tq SHA-256
`e9478b0ad46e951a1654efbe4d9d52440da49b8b57283754a0c86fb69f1d4f0c`.
It records 900 exact matches and five reviewed disparities across 905 cases;
the disparities remain separate from exact matches.
The larger boundary corpus provides supporting evidence, not additional
campaign matches or a general error tolerance. Decimal and byte comparisons
continue to expose every difference. Nearby probes also exposed a unary
zero-normalization bug, which was fixed separately. Literal `-0` normalizes
to positive zero under jq unary negation, whereas parsed input `-0.0` retains
its sign. Neither behavior is a library disparity.

The practical impact is function-specific: `acos` affects angular results,
`exp` affects exponential growth, `erfc` affects tail probabilities, and
`tgamma` affects Gamma-function results. The table does not establish a safe
maximum error for any of those functions. The accepted restriction is therefore
the exact recorded input/output pair, not every result within a numeric window.
Supporting tests cover domain edges, signed zero, non-finite projection, and
representative magnitudes, with exact result counts, stderr, and exit statuses.
Reconsider these entries after implementation when a newer `libm`, jq build,
or release target changes the measurements. No decimal-normalization or blanket
comparison tolerance is permitted.

### Native Linux math observations

The verified `x86_64-unknown-linux-gnu` build uses Rust 1.98.0 and
`libm` 0.2.16. The reference is jq 1.8.1 on Ironhide, Bazzite 44 with glibc
2.43. Each numerical witness exits 0 with empty stderr and one LF-terminated
compact JSON result.

| Query | Input | jq result | tq result | Observed binary64 ULP distance |
| --- | --- | --- | --- | ---: |
| `exp` | `1` | `2.718281828459045` | `2.7182818284590455` | 1 |
| `tgamma` | `0.5` | `1.772453850905516` | `1.7724538509055159` | 1 |
| `y0` | `1` | `0.08825696421567698` | `0.08825696421567697` | 1 |
| `yn(0;1)` | `null` | `0.08825696421567698` | `0.08825696421567697` | 1 |

The final standalone CLI probe covers 92 inputs across six functions, with
81 exact samples, 11 differing samples, and no process issues. Its observed
maxima are 0 ULP for `acos`, 1 for `exp`, 1 for `erfc`, 2 for `tgamma`, and
1 each for `y0` and order-zero `yn`. Bessel probes include the domain boundary,
subnormal inputs, values around 1, and large magnitudes. These sample maxima
are not global guarantees or approval thresholds. The four Linux numerical
witnesses in this table receive direct numerical approvals. The final Linux
registry also records three separately fingerprinted called-definition
wrappers, each an exact clone of its parent witness:

- `manual.composition.arity.scalbln.2` → `manual.audit.math.integer-scale-boundary`
- `manual.composition.arity.y0.0` → `manual.math.y0`
- `manual.composition.arity.yn.2` → `manual.math.yn`

Those approvals cover only the named wrappers and do not waive other composed
calls. The integer-scale conversion restriction is a target-specific semantic
boundary, distinct from the ULP observations above. The practical impact of
the numerical observations is last-bit variation in exponential, Gamma, and
Bessel calculations.

The same 92-input probe on the final macOS executable produced 77 exact and
15 differing samples, with maxima of 1/1/2/2/1/1 ULP in the same function
order. Its extra Bessel samples do not add macOS campaign approvals.

Linux executable SHA-256 identities for the historical completion report:

- jq: `136748786226819bf582738e8be963638c9d721aa0c5d1d650b506a2a52ddb97`
- tq: `fda6cb6843aedeac9d9777b48d23a44a3a283c86e5a5f5c0b7fb36d98b07b74f`

That historical reference also bound all five Linux runtime libraries. The
probe verified their hashes before and after execution. The active
[reference pin](../tests/compatibility/reviews/jq-manual/reference-pin.toon)
now selects the official statically linked jq 1.8.1 Linux artifact. Changing
the reference requires fresh observations and individual disparity review;
the historical approvals do not transfer to it.
These results do not approve another Linux libc,
architecture, or Windows build. The raw boundary probe and final manual
report are historical evidence and are not retained in this checkout. The
[Linux disparity registry](../tests/compatibility/reviews/disparities-x86_64-linux.toon)
retains the identity-bound witness observations and executable hashes.
Reconsider each witness when the safe library or target reference changes.

### Target-aware gamma and scalb

The pinned Linux jq probe is a platform contract rather than evidence for a
portable disparity: on Linux, scalb(2;0.5) and scalb(infinite;-infinite)
project as null, while the pinned macOS build produces 2 and finite maximum.
Linux also projects the verified infinite-exponent sequence
[finite maximum, 0, null, 0, finite maximum, null] for
[scalb(2;infinite), scalb(2;-infinite), scalb(0;infinite),
scalb(0;-infinite), scalb(infinite;infinite), scalb(infinite;-infinite)].
The pinned Linux gamma(2) projects as 0 while macOS projects as 1. tq now
selects the safe Rust behavior for the target at compile time (gamma uses the
Linux lgamma contract and Linux scalb follows these IEEE infinite cases while
rejecting fractional exponents). The reproducible observations, host library
identities, and remaining Windows unverified status are recorded in
platform-reference-investigation.md in the OpenSpec change directory.
These target mappings are implementation work, not an accepted disparity.

Integer-exponent conversion is a separate target boundary. For
[ldexp(2;2147483648), ldexp(2;2147483647), ldexp(2;-2147483649),
ldexp(2;nan), scalbln(2;nan)], the pinned macOS jq result is
[finite maximum, finite maximum, 0, 2, 2], while the pinned Linux jq result is
[0, finite maximum, 0, 0, 0]. tq deliberately uses a deterministic,
safe saturating conversion for this wrapper rather than target-dependent
undefined C floating-to-integer conversion. The executable witness remains
an exact, target-scoped approval in the Linux registry. Both processes exit 0
with empty stderr; `finite maximum` means the compact JSON number
`1.7976931348623157e+308`. tq returns the macOS sequence on both targets.
Scripts depending on those extreme or NaN conversions are not interchangeable.
This is a semantic restriction, not a small numerical error or blanket
rounding exemption. In-range finite exponent conversions remain exact
obligations. The regression witness is
`manual.audit.math.integer-scale-boundary`. Reconsider the conversion policy
after implementation if a portable defined contract or safe-library API
justifies changing it.

Signed-zero `fmin`/`fmax` behavior was fixed rather than approved. The pinned
x86_64 GNU/Linux reference preserves the first operand for equal zeros; the
macOS reference uses its canonical min/max zero rule. The twelve-result
compact-byte witness tests both operand orders, both-negative zeros, and NaN
fallback and now matches on both verified targets.

### Longest regex matching

The current implementation supports jq's `l` flag through bounded endpoint
searches using `fancy-regex` 0.19.1. The manual witness `match("a|ab"; "l")`
on `"ab"` now matches the pinned macOS jq 1.8.1 compact stdout and process contract.
It selects the longest consumed byte length across starting positions, retains
engine-order ties, and reports offsets and lengths in Unicode scalars. Endpoint
assertions preserve the complete input for anchors and lookarounds.

This is bounded support, not universal Oniguruma equivalence. A cursor-wide,
size-weighted work budget and per-search backtracking allowance can reject large
or complex searches with `regex-backtrack`. Whole-pattern recursion under `l`
is explicitly unsupported because endpoint wrapping would change its recursion
target; ordinary-mode recursion is unchanged. Known engine differences remain
for `\\K` resets in abandoned alternatives and global empty matches around
multibyte characters. These are not approved disparities or blanket waivers.

The observations below are **historical**, from the implementation that rejected
`l`. Its identity-bound approvals do not apply to the changed executable.

Witness: input `"ab"`, query `match("a|ab"; "l")`, compact JSON output.

| Observation | jq 1.8.1 | tq working build |
| --- | --- | --- |
| stdout | `{"offset":0,"length":2,"string":"ab","captures":[]}` followed by LF | Empty |
| Exit status | 0 | 2 |
| stderr | Empty | `tq: bytecode operation is not executable in this language wave: regex flag 'l' (longest-match mode)` followed by LF |

Observed on `aarch64-apple-darwin`, Rust 1.98.0, against the final macOS
completion executable. The implementation was based on commit
`90b231be35fe6339ed440494aafe76e4e68e54b2` plus uncommitted changes.
SHA-256 executable identities for this observation:

- jq: `a9fe3ea2f86dfc72f6728417521ec9067b343277152b114f4e98d8cb0e263603`
- tq: `e9478b0ad46e951a1654efbe4d9d52440da49b8b57283754a0c86fb69f1d4f0c`

The Linux executables identified above produce the same stdout/stderr and exit
statuses for this witness. Its approval additionally binds Linux runtime libraries.

The current regression witness is `regex_longest_match_selects_longer_alternative`
in [regex_compat.rs](../crates/tq-core/tests/regex_compat.rs), with additional
capture, flag, recursion-rejection, and resource-limit tests. The CLI regression
also checks exact compact output. Revalidate broader engine behavior and target
contracts before release; do not renew the historical unsupported-flag approval
for a witness that now matches.

#### Longest-match dependency and performance comparison

The longest-match fix adds **no new library, foreign-function call, or unsafe
bridge**. It uses the existing safe Rust `fancy-regex` 0.19.1 dependency; neither
`Cargo.lock` nor the core dependency manifest changed. jq's pinned reference
uses its native Oniguruma engine. tq instead constrains candidate endpoints and
runs repeated bounded Rust-engine searches, sometimes compiling endpoint probes.
This extra work is a compatibility tradeoff, not a performance optimization.

The **pre-optimization** local comparison used the release tq and pinned jq 1.8.1
executables identified in the current math reproduction above. Both received piped JSON stdin and `-c`;
stdout was captured and required to match exactly, with status 0 and empty stderr
on every run. Each scenario had three warmups and nine measured rounds, alternating
tool order. The table shows median end-to-end wall time, **including process
startup, parsing, query execution, and output**, not isolated regex-engine time.

| Scenario | Query | jq median | tq median | tq / jq |
| --- | --- | ---: | ---: | ---: |
| Startup control, one `"ab"` input | `.` | 5.00 ms | 6.00 ms | 1.20× |
| Manual witness, one `"ab"` input | `match("a\|ab"; "l")` | 5.10 ms | 6.37 ms | 1.25× |
| Manual witness, 1,000 `"ab"` inputs in one process | `match("a\|ab"; "l")` | 9.73 ms | 58.85 ms | 6.05× |
| 501-character input: `"a"` followed by 500 `"b"` characters | `match("a"; "l")` | 7.19 ms | 196.21 ms | 27.29× |
| Same 501-character input, ordinary-mode control | `match("a")` | 5.82 ms | 7.28 ms | 1.25× |

The startup control shows why the single-example ratio hides much of the extra
regex work. Batched inputs amortize startup, while the longer literal-search
probe exposed the cost of endpoint enumeration and compilation. Ordinary
matching does not use this endpoint-search path.

The subsequent optimization derives conservative maximum **consumed byte and
Unicode-scalar lengths** from the parsed pattern. It skips endpoints beyond
those bounds and stops checking later starts once the byte maximum is reached.
This preserves byte-based longest ranking, earlier-match ties, captures, and
full-input context. Case-insensitive bounds account for changing UTF-8 widths;
`\\K` bounds count consumption before the reported-span reset. Unknown constructs,
unbounded repeats, and bound arithmetic overflow retain the original fallback.
No new dependency, FFI, cache, or relaxed work limit was introduced.

The same five scenarios were measured again with the same method and pinned jq,
using optimized release tq executable SHA-256
`e5fc798027c0a3d52a59728a455229ba0b811dedb0edeb7ae7da68d14b7c547f`.
All captured outputs and process contracts matched exactly.

| Scenario | Earlier tq median | Optimized tq median | Fresh jq median | Optimized tq / jq |
| --- | ---: | ---: | ---: | ---: |
| Startup control | 6.00 ms | 5.13 ms | 4.59 ms | 1.12× |
| One manual longest-match input | 6.37 ms | 5.31 ms | 4.69 ms | 1.13× |
| 1,000 manual inputs in one process | 58.85 ms | 54.93 ms | 9.37 ms | 5.87× |
| 501-character literal longest search | 196.21 ms | 6.66 ms | 4.67 ms | 1.43× |
| 501-character ordinary-mode control | 7.28 ms | 6.15 ms | 4.76 ms | 1.29× |

The user accepted the optimized longest-match fix at the measured performance
above, closing the original unsupported-flag witness. This scoped acceptance
includes the disclosed batched-input overhead; it is not a general performance
waiver for other workloads or future regressions.

The long literal probe improved approximately **29.45×** between captures.
Startup and ordinary-mode controls also vary between runs, so their small timing
changes are not evidence of optimization gains. The repeated manual witness
still spends time compiling regexes and endpoint probes per input: **5.87× jq**
in this capture. This change primarily removes impossible long-input probes;
it does not solve repeated compilation or improve unknown/unbounded patterns.

A deterministic regression makes `a` and `a|ab` succeed on the 501-character input
under a 4,000-unit work budget, where the earlier implementation exhausted it.
Additional tests cover Unicode byte ties, case folding, bounded repetition,
lookaround, newline matching, and search/reset anchors. Larger or more complex
fallback searches may still exhaust tq's conservative work budget rather than
produce a successful result. Resource-limit failures must not be counted as
fast successful matches.

These are local diagnostic measurements, not cross-platform or representative
workload claims. No isolated engine-time, CPU, or peak-memory measurement was
made. Samples, queries, input byte counts, method, and executable identities are
retained locally in ignored `target/regex-longest-performance.json` (before)
and `target/regex-longest-performance-optimized.json` (after).

### Cancellation and resource bounds

The safe math adapter using `libm` 0.2.16 currently caps the absolute `jn`/`yn` order at 1,024 and
charges accepted orders against the VM step budget. This conservative cap bounds
the library's recurrence work. It also rejects some cheap special cases: with
null input, `jn(1025; 0)` produces `0` plus LF and exits 0 in the pinned jq build;
the tq build identified above emits no stdout and exits 5 with
`tq: VM resource limit exceeded: math-bessel-order` plus LF on stderr.
The public regression is `bessel_work_is_charged_and_bounded` in
[math_compat.rs](../crates/tq-core/tests/math_compat.rs).

This is a resource restriction, not a floating-point accuracy difference or an
approved completion exception. After implementation, reconsider a configurable
budget and proven constant-time special cases before raising the cap. No
unbounded recurrence or native FFI is needed to revisit it.

Engine cancellation is a separate resource contract. Tests must establish
actual work-limit exhaustion and cancellation checks around bounded matching.
Do not claim immediate mid-match cancellation or a wall-clock deadline without
evidence. No unsafe callback is required merely because a library lacks one.

## Reconsideration after implementation

### Existing behavior outside the manual corpus

The JSON object `{"$serde_json::private::Number":"1"}` was read as the
number `1` by both the starting tq revision and the earlier implementation.
Pinned jq preserves the object. All three processes exit 0 with empty stderr
under compact JSON output. This collides with the serde arbitrary-precision
number marker; it is not a math rounding difference, a new regression, or an
approved completion exception. The manual campaign does not cover this key.

This historical observation used starting executable SHA-256
`2d6ef92e7db033f3bab141558bf50a47660dca4ae92dd07eec7b473ac78dac36`
and candidate SHA-256
`7fdf250fbc89cae92e5dd75f7e4989238991c577f9266be4d8f0ceb6dfcf0be8`
on the macOS target and jq build identified above. The shared incremental
JSON parser now preserves that object. A direct compact-output probe of macOS
release executable `1c7a77c13b8658a86fd4b16afccfb53a8febf296a151a57acf07e5403bfc3b99`
and pinned jq produced the same object, empty stderr, and exit 0. This resolves
the observed ordinary-JSON collision without unsafe or FFI. It does not prove
that every other deserialization route is collision-free. Manual coverage must
not be described as proof of
equivalence for every possible jq input.

Once every spec task has execution evidence or a reviewed disparity, revisit
the accepted entries against newer safe Rust libraries and measured user impact.
Keep each witness as a regression test. Remove an entry when exact behavior is
implemented and verified; preserve its history in version control. Introducing
unsafe or native FFI remains outside this implementation policy and would need
a separate decision.
