## 1. JSON5 adapter

- [x] 1.1 Add a reviewed `json5` dependency to `tq-formats`, expose `InputFormat::Json5` and `decode_json5`, and verify the crate builds with `cargo check -p tq-formats`.
- [x] 1.2 Implement the bounded lexical preprocessor for comments, standard strings, kibana-sync triple-quoted strings, token limits, nesting limits, and original-offset mapping; verify focused unit tests cover literal backslashes, quote runs, braces in strings and comments, unterminated delimiters, oversize tokens, and excess depth.
- [x] 1.3 Deserialize normalized JSON5 directly into `tq_core::Value`, translate parser errors to original locations, and verify adapter tests cover standard JSON5 syntax, complete-source consumption, object order, exact hexadecimal integers, finite decimals, non-finite rejection, and invalid UTF-8.
- [x] 1.4 Add a trimmed esdiag saved-object fixture containing multiline markdown and verify an identity decode preserves its exact text.

## 2. CLI selection and planning

- [x] 2.1 Accept `json5` for `-i/--input-format`, list it in generated help, and select it for case-insensitive `.json5` paths; verify CLI argument and path-selection tests pass.
- [x] 2.2 Route JSON5 stdin and files through document execution while leaving probe, decoder-event, and transcode eligibility unchanged; verify explicit input, mixed-file ordering, `.json` strictness, and explicit override tests pass.
- [x] 2.3 Reject explicit and extension-selected JSON5 stream mode before source consumption and verify CLI tests assert the document-at-a-time diagnostic and untouched input.

## 3. Documentation and regression coverage

- [x] 3.1 Update README examples, format lists, package descriptions, compatibility metadata, and `CHANGELOG.md`; verify searches for the old input-format list find no stale user-facing entries.
- [x] 3.2 Run `cargo test -p tq-formats` and `cargo test -p tq-cli` and verify all adapter and end-to-end JSON5 cases pass.
- [x] 3.3 Run the workspace formatting, lint, and test checks used by CI and verify the full suite passes without changing strict JSON, YAML, TOON, or JSON Lines behavior.
