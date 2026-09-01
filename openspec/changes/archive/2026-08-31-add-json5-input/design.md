## Context

See [proposal.md](proposal.md) for the motivation. `tq-formats` owns the `InputFormat` enum and document adapters. `tq-cli` owns option parsing, extension selection, execution planning, and user-facing help. JSON and TOON have decoder-event paths, while YAML uses a complete-document Serde adapter.

The JSON5 grammar is larger than JSON and includes non-finite numbers that tq cannot store. The esdiag files add a non-standard `"""..."""` string form. The current kibana-sync implementation normalizes that form before calling the `json5` crate, but its helper is private, reports parser locations in normalized text, and does not apply tq's token or depth limits.

## Goals / non-goals

Goals:

- Keep JSON5 decoding inside `tq-formats` and return the same ordered `tq_core::Value` used by every other adapter.
- Match kibana-sync's literal triple-quoted string content while improving malformed-input and resource-limit handling.
- Preserve useful source positions after normalization.
- Keep JSON5 out of event, transcode, and automatic content-probing paths.

Non-goals:

- A JSON5 writer or source-preserving round trip.
- Incremental JSON5 events or multiple JSON5 roots in one source.
- Changing strict JSON, `--argjson`, or the format selected for `.json` files.

## Decisions

### Parse with `json5` through tq's existing Serde value visitor

Add the maintained `json5` crate to `tq-formats` and deserialize the normalized source directly into `tq_core::Value`. This keeps insertion order because tq's visitor builds its ordered object type as map entries arrive. It also receives signed and unsigned integers through the exact integer visitor methods and decimal forms through `visit_f64`. The existing number conversion rejects non-finite binary64 values.

Deserializing through `serde_json::Value` was rejected because it adds an unnecessary value conversion and may narrow integer behavior. Writing a complete JSON5 parser was rejected because the standard grammar has enough lexical edge cases that a local implementation would be costly to audit.

### Normalize triple-quoted strings in one bounded lexical pass

Add an internal preprocessor in `tq-formats` that recognizes comments, standard quoted strings, and triple-double-quoted strings. It converts each triple-quoted value to an escaped standard JSON5 double-quoted string. Content is literal, so the preprocessor escapes backslashes, quotes, control characters, and physical line endings instead of interpreting them.

The pass rejects an unterminated triple-quoted string. It tracks source bytes for string, key, and numeric tokens and structural depth before the downstream parser allocates decoded values. The runner's bounded reader remains responsible for the whole-source byte limit.

Reusing kibana-sync was rejected because JSON5 parsing is a small part of a much broader client crate, its helper is private, and its current normalizer does not expose tq's limits or diagnostic data.

### Carry an offset map through normalization

The preprocessor records the original source byte associated with each emitted normalized span. When the JSON5 parser reports an error location, the adapter translates it back to the original source before formatting the diagnostic. Errors detected by the preprocessor already use original offsets.

Reporting normalized locations was rejected because a multiline string can change both byte columns and later line numbers when physical newlines become escape sequences. Users need locations that match the file they opened.

### Keep selection explicit or extension-based

Add `InputFormat::Json5`, parse `-i json5`, and map the case-insensitive `.json5` extension to it. Do not add JSON5 to `probe_format`. A source beginning with `{` or `[` keeps the existing strict JSON commitment, and `.json` remains strict JSON.

This avoids turning a strict JSON syntax error into a permissive retry and avoids adding another overlapping candidate to bounded probing. It also makes the esdiag `.json` workflow explicit: users run `tq -i json5 ...` for those files.

### Route JSON5 through document execution only

`decode_bytes` gains a JSON5 branch that returns one document. Planner predicates for decoder events and direct transcode continue to admit only their current formats. Explicit `--stream -i json5` fails during option validation, and a `.json5` file in automatic stream mode fails during planning before its contents are consumed.

Treating JSON5 as JSON events after normalization was rejected. Normalization currently requires a bounded complete source, and claiming event behavior would create a false streaming guarantee.

### Test the public boundary and the dialect edge cases

Adapter tests cover standard JSON5 features, numeric conversion, order, malformed input, triple-quoted literal semantics, translated locations, and each resource limit. CLI tests cover explicit selection, `.json5` selection, `.json` strictness, mixed files, help, and stream rejection. A trimmed esdiag saved-object fixture proves the issue's markdown form works without importing a large upstream asset.

## Risks / trade-offs

- [Normalization temporarily retains another source-sized buffer and an offset map] -> Keep JSON5 classified as document-at-a-time, enforce the source limit before normalization, and document the memory behavior.
- [A lexical preprocessor can disagree with the downstream grammar] -> Limit it to quote and comment state needed for safe normalization and bounds, then leave grammar acceptance to the `json5` crate. Add tests around comment delimiters, quote runs, escapes, and braces inside strings.
- [Parser upgrades can change accepted grammar or diagnostics] -> Pin the dependency to a reviewed compatible release and keep behavior tests at tq's adapter boundary.
- [JSON5 decimals use binary64 semantics unlike strict JSON's exact decimal literals] -> Keep strict JSON unchanged and test the format-specific numeric behavior stated in the spec.

## Migration plan

No stored data changes. Add the dependency and adapter first, then wire the enum through exhaustive matches, CLI selection, planning, help, documentation, and tests. A rollback removes the new selector and dependency; existing input formats are unaffected because probing and `.json` selection do not change.
