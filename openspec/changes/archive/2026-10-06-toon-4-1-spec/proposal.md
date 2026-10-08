## Why

`tq` targets TOON v3, while the official Rust implementation now implements TOON 4.1 with new mandatory shape-driven layouts and revised parsing rules. This change delivers the native format migration; historical jq-manual totals remain historical and are not release evidence for the new format.

## What Changes

- Start from PR #68's frozen `286b5f6690bc1bee260cba007f3953c52178bf68` baseline and preserve its compatibility fixes, catalog coverage, strict gates, and unresolved platform obligations. PR68 merged during implementation; the user approved integrating its final main ancestry and publishing this migration as a normal follow-on PR to `origin/main`, replacing the now-impossible new native stack on merged PR68.
- Upgrade the upstream `toon-format` package (repository `toon-format/toon-rust`) from exact `0.5.0` to exact `0.6.0`, retaining `default-features = false`. Migrate the native `tq-toon` implementation, not merely its test oracle, to the pinned TOON 4.1 specification and official fixtures.
- **BREAKING**: Canonical output gains recursive nested field groups and keyed tabular objects; root/object-field empty arrays become `[]`; nested arrays in anonymous list positions stay list-form; first-field list-item indentation, quoting, escapes, comments, duplicate keys, and strict/non-strict decoding follow 4.1.
- **BREAKING**: Remove key folding/path expansion and their public options, CLI switches `--fold-keys` / `--flatten-depth`, proof fields, aliases, and obsolete tests. Dotted keys remain literal keys.
- Preserve incremental input events, exact supported numbers, output colors, existing result/sequence framing, and bounded retention. Generalize existing secure replay/spooling to shape-dependent objects and recursive schemas instead of adopting upstream's full-document JSON streaming convenience APIs.
- **BREAKING**: Release all coordinated workspace packages as `0.5.0`, with migration notes and accurate help/version/release metadata. This proposal does not publish or tag the release.
- Deliver fresh jq-manual execution, token accounting, and a focused migration performance guard as separate sibling acceptance work. These requirements and their evidence are not part of PR #71's native implementation delivery; no helper, campaign, or measurement is claimed here.

## Capabilities

### New Capabilities

None; extend the existing format, CLI, streaming, and comparison contracts.

### Modified Capabilities

- `toon-stream-io`: TOON 4.1 canonical forms, interpretation and validation, literal dotted keys, conformance fixtures, and shape-dependent bounded preparation.
- `streaming-transcode`: Object as well as array shape preparation, recursive schema selection, header-order equality, and truthful publication/retention guarantees.
- `tq-cli`: Remove obsolete folding switches and detect new canonical TOON root forms without changing framing or JSON interoperability.
- `cross-tool-compatibility`: Preserve exact process, result-order, framing, and complete-consumption boundaries for native TOON 4.1 behavior; fresh manual regeneration and token accounting are separately delivered acceptance.

## Impact

- `crates/tq-toon`: decoder, DOM consumer, writer, replay/spool/transcode, public options, official fixture runners and provenance.
- `crates/tq-formats`, `crates/tq-core`, `crates/tq-cli`: format probing, event adapters, transcode proof, resource explanations, colors, CLI parsing and consumer-visible regressions.
- `crates/tq-test-support`: upstream dependency, corpus conversion, native TOON evidence serializer/readers, ordered stdout-boundary recovery, and correctness probes for the native implementation.
- Workspace/package manifests and lockfiles, release/install guidance, CHANGELOG, compatibility/migration documentation, and native implementation documentation. Fresh generated jq-manual pages and comparison campaign reports are outside this delivery.
- Existing checked-in TOON metadata must remain readable where 4.1 accepts the old syntax. Re-encode only incompatible or newly generated artifacts; preserve byte-pinned source inventories/reference hashes unless an independently reviewed pin change is necessary.
- PR68's independently owned `achieve-jq-manual-parity` archive is inherited from its merged final tree, not completed or archived by this migration. Issues #69/#70 and calibrated-performance issue #31 remain separately owned; absent candidate-native evidence must remain explicitly absent.

## References

1. [Rust 0.6.0 release](https://github.com/toon-format/toon-rust/releases/tag/v0.6.0).
2. [Pinned TOON 4.1 specification](https://github.com/toon-format/spec/blob/v4.1.0/SPEC.md).
3. [Spec 4.1 release](https://github.com/toon-format/spec/releases/tag/v4.1.0).
4. [Merged dependency PR #68](https://github.com/commandzero/tq/pull/68).
