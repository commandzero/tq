## Why

The native TOON 4.1 migration and the jq-manual comparison delivery have separate ownership. The manual comparison requires its own fresh release/default execution, exact token accounting and documentation promotion. This archive preserves that scoped delivery and its historical acceptance record without claiming that a new run was completed in this focused PR.

## What Changes

- Preserve the release-bound jq-manual regeneration and complete-stdout token-accounting requirements as this change's owned scope.
- Preserve the six historical completed `7.*` acceptance tasks from the original archived change, with their original wording and explicit source snapshot provenance.
- Retain the already captured C9 original-run observations as evidence produced from the original frozen binaries and source snapshot; this focused branch did not rebuild or rerun the campaign.
- Keep any newly required comparison run blocked until the host is idle; do not start manual execution or performance measurements in this change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `cross-tool-compatibility`: Release-bound manual regeneration and exact complete-stdout token accounting.

## Impact

- `openspec/specs/cross-tool-compatibility/spec.md`
- `docs/tests/jq-manual/index.md`
- Archived jq-manual acceptance history only; no fresh run is asserted here.
