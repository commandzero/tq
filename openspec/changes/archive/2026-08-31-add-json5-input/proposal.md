## Why

Issue #13 shows that tq cannot read the JSON5 saved-object files used by esdiag and kibana-sync. Those files also use kibana-sync's triple-double-quoted multiline string extension, so a standard JSON5 parser alone does not make them queryable.

## What changes

- Add document-at-a-time JSON5 input with comments, trailing commas, unquoted keys, single-quoted strings, JSON5 number syntax, and the rest of the standard JSON5 grammar.
- Accept kibana-sync-compatible `"""..."""` multiline strings and preserve their content as tq strings.
- Add `json5` to `-i/--input-format` and select JSON5 automatically for `.json5` files. Keep `.json` files and `-i json` strict JSON.
- Reject unsupported non-finite numbers, malformed triple-quoted strings, invalid UTF-8, and values outside tq's number or resource limits with JSON5 input diagnostics.
- Treat JSON5 as a document-at-a-time format. It does not gain decoder-event streaming, automatic content probing, JSON5 output, or JSON5 parsing for `--argjson`.
- Add format-adapter, CLI, fixture, and hostile-input coverage, including a representative esdiag saved object.

## Capabilities

### New capabilities

- `json5-input`: Defines the accepted JSON5 profile, kibana-sync multiline strings, value conversion, diagnostics, and document-at-a-time limits.

### Modified capabilities

- `tq-cli`: Adds explicit and extension-based JSON5 input selection and defines its interactions with strict JSON, automatic detection, and stream mode.

## Impact

The change affects `tq-formats` decoding, the CLI format enum and selection paths, runner planning and diagnostics, help and README text, and compatibility fixtures. It adds a JSON5 parser dependency. There are no changes to tq query semantics, output formats, or the strict JSON parser.
