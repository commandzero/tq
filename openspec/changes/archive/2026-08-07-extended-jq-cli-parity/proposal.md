## Why

The MVP intentionally covers the high-value jq command shape, leaving many
production scripting switches and integration behaviors unsupported. A separate
change can extend parity without destabilizing the core language release.

## What Changes

- Baseline and prioritize the remaining jq 1.8.x CLI options and exit behaviors.
- Add supported input/output modes, argument/file helpers, color/sort/ASCII controls, and diagnostics.
- Preserve tq's explicit TOON/YAML format and framing rules when jq options overlap.
- Add shell-level compatibility, broken-pipe, encoding, and multi-file regressions.

## Capabilities

### New Capabilities

- `extended-jq-cli-parity`: The post-MVP jq command-line surface and scripting integration contract.

### Modified Capabilities

None.

## Impact

CLI parsing/help, format adapters, process statuses, filesystem-facing built-ins,
compatibility adapters, documentation, and packaging tests are affected.
