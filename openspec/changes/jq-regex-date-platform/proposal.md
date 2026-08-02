## Why

Regex, date/time, environment, and platform built-ins account for a substantial
part of real jq script compatibility, but bring portability, determinism,
security, and dependency choices that deserve an explicit contract.

## What Changes

- Add jq-compatible regex compilation, matching, capture, split, scan, and substitution.
- Add UTC date parsing, formatting, epoch conversion, and documented platform ranges.
- Add explicitly governed environment and platform I/O built-ins.
- Define dependency identity, Unicode behavior, determinism, limits, and error mapping.

## Capabilities

### New Capabilities

- `jq-regex-date-platform`: Regex, date/time, environment, and platform-dependent jq built-ins.

### Modified Capabilities

None.

## Impact

The built-in registry, evaluator/VM, dependencies, sandbox and resource policy,
cross-platform tests, compatibility baselines, and release manifests are affected.
