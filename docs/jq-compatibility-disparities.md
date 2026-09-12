---
type: Report
title: jq compatibility disparities
description: Measured safe-library limitations and post-implementation reconsideration criteria.
generated: { by: codex/gpt-5.6-luna, at: 2026-09-12T17:42:14Z }
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
The 92-input report is
`target/dependency-probes/math-bind-final-20260909.toon`. All processes exited
successfully with empty stderr and the expected result counts under controlled
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

Linux executable SHA-256 identities for the final completion report:

- jq: `136748786226819bf582738e8be963638c9d721aa0c5d1d650b506a2a52ddb97`
- tq: `fda6cb6843aedeac9d9777b48d23a44a3a283c86e5a5f5c0b7fb36d98b07b74f`

The [reference pin](../tests/compatibility/reviews/jq-manual/reference-pin.toon)
also binds all five Linux runtime libraries. The probe verified their hashes
before and after execution. These results do not approve another Linux libc,
architecture, or Windows build. Evidence is retained at
`target/dependency-probes/math-boundaries-e2e-bind-final.toon` and the final
manual report at
`target/linux-review/bind-final-20260909/completion/strict-bind-final-completion.toon`.
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

The replacement implementation using `fancy-regex` 0.19.1 rejects jq's `l` flag.
It does not silently return the first alternative, which would give the wrong
match for patterns such as `a|ab`. Primary review accepts this explicit
safe-library restriction for the exact witness in each target's approval registry.

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

Scripts requiring longest matching cannot currently substitute tq for jq.
Other regex behavior is not exempted. The regression witness is
`longest_match_remains_an_explicit_safe_engine_limitation` in
[regex_compat.rs](../crates/tq-core/tests/regex_compat.rs).
After implementation, reconsider this restriction if a safe Rust library offers
longest matching with captures and bounded work. Revalidate the witness and
refresh its identity-bound approval against the final executable before release.

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
