## Why

`tq` can sometimes consume multiple JSON texts and can emit compact JSON one result per line, but it has no JSON Lines contract. `.jsonl` and `.ndjson` files are not detected, automatic event execution can reject multiple records, and the CLI cannot request record-framed JSON directly.

## What changes

- Add JSON Lines input with `jsonl` as the documented name and `ndjson` as an alias.
- Detect `.jsonl` and `.ndjson` files as JSON Lines when input format is automatic.
- Parse one JSON value per non-empty physical line, preserve record order, and attach line context to parse errors.
- Apply document, slurp, and explicit or automatic event plans consistently across JSON Lines records.
- Add JSON Lines output that writes exactly one compact JSON value followed by LF for each result.
- Reject pretty-print and indentation controls that would break one-record-per-line output.
- Document the distinction between JSON documents and JSON Lines records in help and compatibility material.

## Capabilities

### New capabilities

- `json-lines-io`: Strict JSON Lines decoding, record processing, output framing, aliases, and file-extension inference.

### Modified capabilities

- `tq-cli`: Add JSON Lines format selection, option validation, help text, and automatic file-format selection.
- `automatic-stream-planning`: Execute eligible event and subtree plans independently across ordered JSON Lines records.

## Impact

The change affects format enums and adapters in `tq-formats`, CLI parsing and source dispatch in `tq-cli`, automatic execution planning, diagnostics, executable compatibility tests, and user documentation. It should not require a new runtime value type or an external dependency.
