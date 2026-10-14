## Decision

Migrate native TOON to the pinned v4.4 specification rather than retaining a v4.1 decoder behind a v4.4 oracle. Official expected values, errors, and canonical bytes remain authoritative; there are no fixture exclusions or tolerances.

## Native decoder

Keep the existing line-oriented semantic event decoder and bounded sibling/schema bookkeeping. Non-strictness changes only the five v4.4 recovery classes. All other syntax, width, scope, UTF-8, escape, and resource errors remain errors even when consumers discard events.

A frame retains its original scope-closing depth separately from its adopted content depth. Initial depth-jump recovery can adjust content depth without swallowing shallower sibling fields or permitting later under-indentation. Semantic event depth, not physical whitespace depth, still controls nesting budgets.

Canonical length declarations outside `u64` remain syntactically valid headers and carry no representable event count. Strict completion rejects the unmet count; non-strict mode decodes the actual scope. No declaration drives proportional reservation or wraps a counter.

## Canonical writers

Both document and prepared-tape rendering use the same v4.4 escape boundary: U+0000–U+001F. DEL, C1 controls, and other BMP scalars remain literal UTF-8 inside quoted strings. Existing delimiter, root BOM, structural-character, key, and numeric-looking-string quoting remains unchanged.

## Fixture provenance

Official suite: spec tag `v4.4.0`, tag object `4bf71f491ba8474b405710e8926ea8268700d36f`, commit `d09c3d33e37389345a74bd96981eb96dd9be6c5e`. The complete suite contains 404 decode cases and 160 encode cases. Manifest records retain Git blob, SHA-256, byte length, and case membership for every upstream file.

Oracle: exact `toon-format =0.7.0`, tag object `651a878554a3f22dfc38d35256992289929b00ae`, commit `a1172f1489efc3b5571f475c06847874fb1a37a9`, default features disabled. The native exact-number domain remains distinct from the oracle's documented binary64 domain; official fixtures are not skipped.

## Exercised evidence

- Complete official decode expectations pass through DOM, one-byte-buffered event construction, and discarded-event validation, with independent oracle assertions.
- Complete encode expectations pass through document/color sinks and semantic-event transcode with in-memory and forced-spool thresholds.
- Workspace run: 1,985 tests passed, 11 ignored.
- CLI smoke: delimiter mismatch fails in both modes; valid initial nested depth jump preserves siblings; short rows and trailing root content fail in non-strict mode; canonical output preserves literal DEL/C1/separator bytes.
- Full release compatibility campaign: 1,220 cases, 4,804 executed observations, 219 observed cross-tool differences, zero TOON differences and zero harness errors. Historical jq/yq disparities are not relabeled as new exact matches.
- Main OpenSpec validation: 21 passed. Documentation validation passed with the existing unrelated frontmatter warning.

## Review lifecycle

The main specs already contain these requirement changes. Before archival, verify the deltas against the reviewed implementation and verify exact delta-to-main correspondence. Apply any needed synchronization before moving the active change; do not reapply a rename whose source is already absent merely to manufacture a synchronization mutation. The repository's committed-head archive-completion gate remains required.
