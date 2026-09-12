# Dependency feasibility probes

This is a standalone Cargo workspace. It does not participate in the root
workspace and does not change production manifests.

The local Cargo config sends generated output to the repository-level
`target/dependency-probes` directory, outside the repository's fixture tree.
That keeps Cargo metadata from being mistaken for a stored fixture.

From the repository root, run the frozen host probe with an explicit target
directory (the nested Cargo config is not loaded for a root-level
`--manifest-path` invocation):

```text
CARGO_TARGET_DIR=target/dependency-probes rtk cargo test \
  --manifest-path tests/compatibility/probes/dependencies/Cargo.toml \
  --offline --locked --release -- --nocapture
```

The tracked math-boundary test derives each jq input expression from its
single input table, verifies the pinned jq SHA-256 at runtime, requires a
successful jq process with empty stderr and the expected result count, and
prints exact jq output plus per-input `libm` values and ULP distances. Capture
that evidence under the configured root target directory with:

```text
CARGO_TARGET_DIR=target/dependency-probes rtk proxy cargo test \
  --manifest-path tests/compatibility/probes/dependencies/Cargo.toml \
  --offline --locked --release pinned_jq_math_boundary_probe -- --nocapture \
  > target/dependency-probes/math-boundaries.tsv
```

The probe covers domain edges, subnormal/overflow/underflow behavior, poles,
non-finite values, and signed-zero inputs for `acos`, `exp`, `erfc`, `tgamma`,
`y0`, and `yn(0; .)`; the y0/yn samples include negative-domain, zero,
small/subnormal, near-one, and large finite inputs. The negative-zero `tgamma`
sample is computed with
`("-0.0"|tonumber)` so jq's unary-literal normalization is not mistaken for
a library result. The ULP column is measured evidence for those inputs only,
not a global tolerance or an approval.

## End-to-end math evidence

The separate CLI probe executes 92 inputs across six functions through both
pinned jq and an explicitly selected tq binary. It records twelve process
results, exact bytes, sample-specific ULP distances, and executable hashes
before and after execution. Each run uses an empty temporary HOME and a bounded
process timeout. Existing report files are rejected so a later run cannot
overwrite earlier evidence. Earlier four-function/66-input reports remain
historical evidence and are not rewritten.

```bash
CARGO_TARGET_DIR=target/dependency-probes rtk proxy cargo run \
  --manifest-path tests/compatibility/probes/dependencies/Cargo.toml \
  --offline --locked --release --bin math_end_to_end_probe -- \
  --jq "$PWD/target/reference-build/jq/jq" \
  --tq "$PWD/target/release/tq" \
  --output "$PWD/target/dependency-probes/math-boundaries-e2e.toon"
```

The probe accepts the pinned aarch64 macOS jq SHA-256
`a9fe3ea2f86dfc72f6728417521ec9067b343277152b114f4e98d8cb0e263603` on the
macOS host and the verified x86_64 GNU/Linux jq SHA-256
`136748786226819bf582738e8be963638c9d721aa0c5d1d650b506a2a52ddb97` on the
Linux host. On Linux, use the native `/usr/bin/jq` (or its verified immutable
copy) and the immutable tq binary explicitly, for example:

```bash
CARGO_TARGET_DIR=target/dependency-probes rtk proxy cargo run \
  --manifest-path tests/compatibility/probes/dependencies/Cargo.toml \
  --offline --locked --release --bin math_end_to_end_probe -- \
  --jq /usr/bin/jq \
  --tq "$PWD/target/release/tq" \
  --output "$PWD/target/dependency-probes/math-boundaries-e2e-linux.toon"
```

On x86_64 GNU/Linux the probe also requires the five reviewed dynamic runtime
files (`libjq.so.1`, `libonig.so.5`, `libc.so.6`, `libm.so.6`, and
`ld-linux-x86-64.so.2`) at their reviewed paths and SHA-256 values. Their
before/after identities are recorded in the report; a missing, changed, or
unexpectedly hashed file fails the probe. The macOS reference is statically
linked and has no runtime-library requirement in this probe.

Use a distinct immutable tq executable and a new report filename when recording
review evidence. Success means all processes completed with valid result types,
counts, statuses, and empty stderr. It does not mean their numbers matched;
the report preserves the exact and differing sample counts separately.

The recorded cache versions are `libm 0.2.16`, `fancy-regex 0.19.1`,
`onig 6.5.3` with `default-features = false`, `onig_sys 69.9.3`, and
`regex-automata 0.4.18`. The macOS probe host is `aarch64-apple-darwin`; the
reviewed native Linux pin is `x86_64-unknown-linux-gnu`. Other release targets
remain unverified.

## Verified

- The probe itself has `#![forbid(unsafe_code)]`.
- `libm` exposes most of the jq 1.8.1 math inventory. Its `fma` is fused,
  and the probe observes preservation of negative zero.
- `fancy-regex` supports scoped flags, a non-ASCII Unicode property match,
  empty-match iteration, and a configurable backtrack-count error.
- `onig` safely exposes Oniguruma longest matching plus callable match-stack
  and retry-limit setters. The bundled `onig_sys` build compiles without
  bindgen when its `generate` feature is disabled.

## Unresolved gates

The probes do not prove parity and must not be treated as implementation
approval.

- `fancy-regex` has a fixed internal VM stack (`MAX_STACK = 1_000_000`) and
  no cancellation callback or longest-match mode.
- `onig` has no safe progress/retraction callback or cancellation API. The
  raw `onig_sys` callout functions exist, but using them directly would add
  first-party unsafe FFI. The probe only verifies that the safe limit APIs are
  available; it does not prove a limit-exhaustion error. A safe wrapper/fork
  would be needed for that particular raw-hook route, but the probes do not
  establish that this route is necessary. The revised implementation policy
  excludes native engine integration and unsafe bridges; the Onig probe is
  historical feasibility evidence only.
- `regex-automata 0.4.18` supports only `All` and `LeftmostFirst`; its
  leftmost-longest mode is still a documented TODO. It also does not provide
  backreferences.
- `libm` lacks direct APIs for jq's `drem`, `logb`, `nearbyint`, `nexttoward`,
  `scalb`, `scalbln`, `significand`, and `gamma` names. jq's macOS source
  maps or synthesizes several of these. Safe wrappers need exact probes.
- Pure-Rust `libm` results have not been proven bit-for-bit equal to the
  pinned jq 1.8.1 macOS libSystem math. The revised policy permits measured,
  function-specific documented disparities, not blanket tolerances or claims
  that approximate values are exact matches.

Licenses are compatible in principle: `libm` MIT, `fancy-regex` MIT,
`regex-automata` MIT/Apache-2.0, `onig` MIT, and bundled Oniguruma BSD-style.
See the crate manifests and license files in the Cargo registry, plus
<https://raw.githubusercontent.com/jqlang/jq/jq-1.8.1/src/builtin.c> for
jq's macOS math mappings.
