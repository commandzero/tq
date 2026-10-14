## Why

`toon-format` 0.7.0 implements TOON v4.4. Upgrading the pinned oracle exposed a real disagreement with the native v4.1 decoder: non-strict malformed headers were accepted as literal key/value lines. Native implementation and official fixtures must migrate together instead of excluding the failing oracle comparison.

## What Changes

- Upgrade both exact `toon-format` pins from 0.6.0 to 0.7.0, retaining `default-features = false`, and update the lockfile.
- **BREAKING:** Target TOON v4.4 in native decoding, canonical output, CLI version output, and active contracts. Non-strict decoding permits only the five recoveries for counts, duplicate keys, indentation, blank lines, and initial nested-scope depth jumps. Malformed headers, row-width mismatches, trailing root content, and out-of-scope lines fail in both modes.
- Correct header classification, quoted-span scanning, SP/HTAB handling, empty keys, Unicode escapes, oversized declarations, and nested-scope depth recovery without proportional allocation.
- Emit quoted DEL and U+0080–U+009F literally in document and replay/transcode writers; escape only U+0000–U+001F controls.
- Replace the complete official v4.1 fixture suite with unmodified v4.4.0 fixtures and auditable provenance. Preserve exact native/reference assertions and existing consumer regressions.
- Preserve exact supported numbers, key and result order, resource limits, event semantics, and existing result/sequence framing.

## Impact

Affected capabilities: `toon-stream-io`, `streaming-transcode`, and `tq-cli`. Production changes are confined to native TOON parsing/escaping and the CLI version declaration. The fixture oracle remains test/support-only; no upstream full-document decoder or writer replaces the bounded native path.

The main specification edits were already applied during implementation. The associated deltas record their complete requirements and scenarios for review and archive-time correspondence checks. Keep this change active during PR review; archive only after clean current-head review and post-review implementation verification.
