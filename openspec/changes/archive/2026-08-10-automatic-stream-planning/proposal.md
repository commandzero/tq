## Why

The MVP provides bounded explicit `--stream`, while ordinary jq-shaped document
queries still materialize full inputs. Safe automatic planning can make common
large-file queries memory efficient without requiring users to rewrite them as
path/value stream programs.

## What Changes

- Derive event, subtree, document, whole-input, and blocking plans from analyzed HIR.
- Lower eligible navigation/projection/select pipelines onto decoder events.
- Add bounded subtree capture, deterministic fallback, and pre-input rejection where required.
- Make every retention decision observable through explain and benchmark reports.

## Capabilities

### New Capabilities

- `automatic-stream-planning`: Transparent bounded-memory execution plans for eligible jq-shaped queries.

### Modified Capabilities

- `tq-cli`: Auto-detected JSON container inputs commit to decoder events while YAML markers retain document fallback.

## Impact

Typestate plan APIs, effect analysis, compiler lowering, JSON/TOON event adapters,
CLI explain output, resource governance, compatibility, and large benchmarks are affected.
