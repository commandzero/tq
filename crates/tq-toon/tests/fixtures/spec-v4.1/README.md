# Pinned TOON 4.1 conformance fixtures

The `encode/` and `decode/` directories are the complete, unmodified official
JSON fixture set from [toon-format/spec tag v4.1.0](https://github.com/toon-format/spec/tree/v4.1.0/tests/fixtures),
resolved to commit `be5c1495987aad5a3f54636d8179139cd824c7bb` (annotated tag
object `43f1143ec36e8f345a8d18a26d10d468922b0e0c`). This cutover replaces the
active `spec-v3` suite, including obsolete folding/expansion fixtures.

[`manifest.json`](manifest.json) inventories every upstream path, Git blob,
SHA-256, byte length and per-file case membership. The vendored set has 14 decode
files (358 cases) and 9 encode files (179 cases). These totals describe this pin;
the runners verify the manifest's complete file set and content hashes instead
of hard-coding incidental totals or accepting minimum coverage. Case identity
is upstream path plus zero-based array index; names are diagnostic prose only.
Upstream files retain their `version: "4.0"` baseline and per-case
`minSpecVersion` fields exactly as published in the 4.1 tag.

## License

Fixtures are Copyright (c) 2025-PRESENT Johann Schopplich, licensed under MIT.
The complete upstream attribution and permission notice is in [`LICENSE`](LICENSE).

## Execution contract

- Every official decoder fixture asserts its expected typed value or error
  directly, including non-strict outcomes and optional leniencies represented
  by this complete suite. No fixture is silently skipped.
- Valid decode fixtures additionally pass through the public semantic events
  into `DomBuilder`, with one-byte input buffering. Error fixtures also drain
  events without retaining values, so DOM-only validation cannot pass them.
- Every encoder fixture asserts exact canonical document bytes for `encode`,
  plain/colored sinks and semantic-event transcode, with memory/spool thresholds
  of 4096 and 0 bytes. ANSI-stripped bytes must match; framing is not conflated
  with document-internal bytes.
- Official options are `indentSize`, `strict` and `delimiter`. Unknown options
  fail explicitly. Folding and path-expansion options no longer exist.
- Exact `toon-format =0.6.0`, default features disabled, independently asserts
  official expectations in both directions. The resolved Rust release is
  [commit `54efbdf8eaffdbec0d5eb463501fbc184eb02a87`](https://github.com/toon-format/toon-rust/tree/54efbdf8eaffdbec0d5eb463501fbc184eb02a87)
  (annotated tag object `bb7a3dd95a381c1129c9d2d99aa1ba379aa2ffb1`). An oracle
  disagreement fails; it does not change official expectations.
- [`../migration-v4.1.json`](../migration-v4.1.json) contains separately authored
  consumer regressions for recursive header ordering, keyed entry order,
  anonymous lists, first-field depth, empty-array delimiter contexts and late
  invalidation. They run through the same byte/event/color/spool checks, but
  are not represented as upstream fixtures or included in upstream hashes.
- Object encounter order is asserted recursively, not delegated to unordered
  JSON/`Value` equality. Encoder round trips allow only the specified table
  normalization: recursive row/entry-value keys follow the first value's
  schema. Outer keyed entries and ordinary object fields keep encounter order;
  anonymous list arrays and anonymous list objects cannot acquire a table
  merely to excuse reordering.

## Numeric and depth domains

Native decimal literals retain exact mathematical values within `NumberLimits`:
4096 coefficient digits, absolute decimal exponent at most 2,000,000,000,
4096 plain-expansion digits and 8192 rendered bytes. Canonical exact native
spelling outside the plain-expansion envelope may use exponent notation. The
fixture runner compares numbers exactly, never with an epsilon, binary64
projection, or type coercion.

The [0.6.0 oracle's documented domain](https://github.com/toon-format/toon-rust/blob/54efbdf8eaffdbec0d5eb463501fbc184eb02a87/src/lib.rs)
is exact `i64`/`u64` integers plus finite binary64 fractions/exponent forms.
Out-of-range integral tokens decode as a float only if they equal that float's
canonical spelling; otherwise they remain strings. Non-finite exponent results
remain strings. Its nesting limit is 256, including recursive field groups;
the native fixture config uses the same structural-depth bound and finite
line/token/lookahead limits.

There are **no official fixture exclusions**. Local numeric regression cases
separately assert the native exact policy and the oracle's string policy for
`18446744073709551616`, `-9223372036854775809` and `1E1234567890`.
These are documented implementation-domain differences, not official/oracle
conformance approvals. Shared-domain integer-boundary and decimal cases assert
exact agreement. Native bounds and host-only non-finite output also have their
own regression tests. No cross-domain tolerance is applied to official cases.
