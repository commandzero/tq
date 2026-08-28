## Why

Identity conversion currently pays for a full JSON tree, a second `tq_core::Value`
tree, and a complete TOON output string. On the natural benchmark corpus this used
1,678.7 MiB RSS for a 73.6 MiB wide object, while the streaming `toon` path used
20.8 MiB. `tq` needs a bounded conversion plan that preserves its faster document
VM for filters that actually require a document.

## What changes

- Add an output-aware `transcode` plan for semantic identity with JSON or TOON
  input and canonical TOON output.
- Consume object members incrementally without constructing the root document.
  Write directly when the decoder's duplicate-key policy permits it; otherwise
  normalize members through bounded replay storage.
- Prepare unknown-length arrays through one bounded replay store. Move from memory
  to a secure temporary file when the shared preparation budget is exhausted.
- Preserve the document, whole-input, and blocking plans for filters or output
  options that need retained values.
- Expose transcode eligibility, retained state, preparation bytes, spool bytes,
  and fallback causes through explain and benchmark reports.
- Remove result-sized TOON string assembly from every output plan.
- Deserialize and serialize `tq_core::Value` directly so retained document plans
  no longer build transient `serde_json::Value` trees.
- Add correctness-gated memory and throughput benchmarks for wide objects,
  nested objects, root arrays, nested arrays, and tabular candidates.

## Capabilities

### New Capabilities

- `streaming-transcode`: Output-aware identity conversion over structural events,
  including eligibility, duplicate-key handling, bounded container preparation,
  output commitment, and exact equivalence with document-mode encoding.

### Modified Capabilities

- `automatic-stream-planning`: Select the transcode plan before semantic input
  consumption and reject it without speculative output when its proof fails.
- `query-runtime`: Record semantic-identity capability in analyzed programs and
  construct a typed transcode plan without entering the document VM.
- `toon-stream-io`: Write canonical TOON to `Write` incrementally and replay
  prepared arrays from a single bounded representation.
- `resource-governance`: Explain and report transcode retention, preparation,
  spooling, limits, and partial-output behavior.
- `performance-benchmarks`: Add correctness-gated identity-transcode memory and
  throughput objectives on natural large files of several shapes.

## Impact

The change affects query analysis and plan types in `tq-core`, JSON and TOON event
adapters in `tq-formats`, the encoder and array spool in `tq-toon`, plan dispatch
and explain output in `tq-cli`, and benchmark reporting in `tq-test-support`.
It changes no jq expression semantics and adds no required dependency. The spool
record format is private and has no compatibility guarantee.
