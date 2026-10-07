## Why

`tq` targets TOON v3, while the official Rust implementation now implements TOON 4.1 with new mandatory shape-driven layouts and revised parsing rules. Existing output examples and jq-manual token totals therefore cannot describe the planned `tq` 0.5.0 release without a native format migration and complete fresh comparison campaign.

## What Changes

- Start from PR #68's frozen `286b5f6690bc1bee260cba007f3953c52178bf68` baseline and preserve its compatibility fixes, catalog coverage, strict gates, and unresolved platform obligations. PR68 merged during implementation; the user approved integrating its final main ancestry and publishing this migration as a normal follow-on PR to `origin/main`, replacing the now-impossible new native stack on merged PR68.
- Upgrade the upstream `toon-format` package (repository `toon-format/toon-rust`) from exact `0.5.0` to exact `0.6.0`, retaining `default-features = false`. Migrate the native `tq-toon` implementation, not merely its test oracle, to the pinned TOON 4.1 specification and official fixtures.
- **BREAKING**: Canonical output gains recursive nested field groups and keyed tabular objects; root/object-field empty arrays become `[]`; nested arrays in anonymous list positions stay list-form; first-field list-item indentation, quoting, escapes, comments, duplicate keys, and strict/non-strict decoding follow 4.1.
- **BREAKING**: Remove key folding/path expansion and their public options, CLI switches `--fold-keys` / `--flatten-depth`, proof fields, aliases, and obsolete tests. Dotted keys remain literal keys.
- Preserve incremental input events, exact supported numbers, output colors, existing result/sequence framing, and bounded retention. Generalize existing secure replay/spooling to shape-dependent objects and recursive schemas instead of adopting upstream's full-document JSON streaming convenience APIs.
- **BREAKING**: Release all coordinated workspace packages as `0.5.0`, with migration notes and accurate help/version/release metadata. This proposal does not publish or tag the release.
- Fully re-execute every jq-manual case and every output campaign with newly built release/default binaries; regenerate all comparison sections and unique-case token totals for both `o200k_base` and `cl100k_base`. Review output/eligibility changes against the PR68 baseline; never reuse old stdout or token counts as candidate evidence.
- Add a focused performance regression guard against frozen PR68 release/default binaries: each selected workload must avoid a confirmed median wall-time increase greater than 20%, with an advisory review note above 10%. The user approved this revised limit and deferred review of those advisory regressions. Cover decoding, document encoding, identity transcode, nested/keyed shapes and spooling with repeated same-host measurements; noisy or incomplete evidence at the hard limit cannot pass. RSS remains diagnostic, and this local migration gate does not replace calibrated performance acceptance.

## Capabilities

### New Capabilities

None; extend the existing format, CLI, streaming, and comparison contracts.

### Modified Capabilities

- `toon-stream-io`: TOON 4.1 canonical forms, interpretation and validation, literal dotted keys, conformance fixtures, and shape-dependent bounded preparation.
- `streaming-transcode`: Object as well as array shape preparation, recursive schema selection, header-order equality, truthful publication/retention guarantees, and a focused migration-wide +20% hard wall-time regression guard with advisory notes above +10%.
- `tq-cli`: Remove obsolete folding switches and detect new canonical TOON root forms without changing framing or JSON interoperability.
- `cross-tool-compatibility`: Complete release-bound jq-manual comparison regeneration with exact capture token accounting and explicit baseline/eligibility/provenance changes.

## Impact

- `crates/tq-toon`: decoder, DOM consumer, writer, replay/spool/transcode, public options, official fixture runners and provenance.
- `crates/tq-formats`, `crates/tq-core`, `crates/tq-cli`: format probing, event adapters, transcode proof, resource explanations, colors, CLI parsing and consumer-visible regressions.
- `crates/tq-test-support`: upstream dependency, corpus conversion, native TOON evidence serializer/readers, ordered stdout-boundary recovery, manual comparisons/renderer, and a correctness-gated focused baseline/candidate performance check.
- Workspace/package manifests and lockfiles, release/install guidance, CHANGELOG, compatibility/migration documentation, generated `docs/tests/jq-manual` results and related current summaries.
- Existing checked-in TOON metadata must remain readable where 4.1 accepts the old syntax. Re-encode only incompatible or newly generated artifacts; preserve byte-pinned source inventories/reference hashes unless an independently reviewed pin change is necessary.
- PR68's independently owned `achieve-jq-manual-parity` archive is inherited from its merged final tree, not completed or archived by this migration. Issues #69/#70 and calibrated-performance issue #31 remain separately owned; absent candidate-native evidence must remain explicitly absent.

## References

1. [Rust 0.6.0 release](https://github.com/toon-format/toon-rust/releases/tag/v0.6.0).
2. [Pinned TOON 4.1 specification](https://github.com/toon-format/spec/blob/v4.1.0/SPEC.md).
3. [Spec 4.1 release](https://github.com/toon-format/spec/releases/tag/v4.1.0).
4. [Merged dependency PR #68](https://github.com/commandzero/tq/pull/68).
