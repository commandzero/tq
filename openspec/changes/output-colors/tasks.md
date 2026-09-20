## 1. Presentation contract and shared theme

- [x] 1.1 Add tq-owned color fixtures for keys, every scalar type, punctuation, enclosing quotes, escaped content, and exact number spellings; distinguish the new default from PR #33's jq defaults while preserving JQ_COLORS overrides.
- [x] 1.2 Introduce the eight-slot default palette and bounded presentation sink with a plain pass-through path; verify slot-to-SGR mappings, structural quote ownership including root fallback, resets, empty output, and byte-identical output after removing generated styling.
- [x] 1.3 Replace internal JSON-only output options with format-independent color policy and audit public API callers; verify default library writers remain plain and any required compatibility bridge compiles.

## 2. Native writer coverage

- [x] 2.1 Integrate semantic spans into JSON and YAML writers; verify pretty/compact JSON, JSON Lines aliases, JSON sequences, YAML documents, ASCII escaping, sorted keys, and embedded line breaks retain exact undecorated bytes.
- [x] 2.2 Integrate TOON document, table, folded-key, delimiter, framed/unframed, and direct transcode paths; verify stripping generated styling matches plain output and color alone preserves the transcode plan and bounded state.
- [x] 2.2a Keep TOON colon separators unstyled across native, direct, and prepared/spooled paths; preserve the pre-color direct scalar quoting context.
- [x] 2.3 Integrate CSV/TSV headers and typed fields; verify quoted numeric strings, null versus empty string, doubled quotes, tabs, embedded newlines, header order, and row publication against plain-byte fixtures.
- [x] 2.4 Cover raw strings, structured non-string fallback, query formatter strings, and proxy sources; verify raw/proxy bytes are unchanged and no tq style leaks across those boundaries.

## 3. CLI selection and capability policy

- [x] 3.1 Generalize Auto/Always/Never and once-per-invocation palette resolution to all output formats; verify seven/eight-slot JQ_COLORS, invalid/unset fallback, distinct boolean styles, injected terminal states, redirected stdout, nonempty/empty NO_COLOR, last-explicit-flag precedence, and terminal/environment capability denial without ambient reads.
- [x] 3.2 Replace JSON Lines, JSON sequence, CSV/TSV, YAML, and TOON color rejections with the shared policy; verify forced color is accepted across the catalog while unrelated pretty/raw/folding conflicts still fail before input consumption.
- [x] 3.3 Update help and color option descriptions; verify the displayed help states coverage, JQ_COLORS compatibility, default terminal detection, raw string behavior, and how to obtain plain bytes.

## 4. Resource and compatibility verification

- [x] 4.1 Account for SGR bytes in output and applicable spool limits, reserve normal span-closing bytes, and reset where possible on errors; verify limit-boundary, partial-write, late-failure, empty-result, atomic-publication, cancellation, and broken-pipe cases.
- [x] 4.2 Extend the versioned compatibility matrix with tq presentation cases and review PR #33's jq color cases individually; verify jq semantic/plain-output comparisons and JQ_COLORS mappings remain intact and expected default-palette and enclosing-quote differences are explicitly classified.
- [x] 4.3 Run the complete native-output matrix through monochrome and forced color, including streamed event results and custom palettes with distinct array/object/string/key styles; verify root/nested/table quote ownership, escaped-content roles, and exact stripped-byte equality for each writer route and alias.
- [x] 4.4 Measure representative plain/colored document and identity-transcode output outside the restricted sandbox; record time, actual output bytes, and peak memory and verify coloring does not introduce result-sized buffering or growth with completed frames. See [release measurements](../../../docs/tests/output-colors-performance.md).

## 5. Documentation and delivery

- [x] 5.1 Update README, format/compatibility documentation, and release notes with the built-in palette and non-breaking presentation classification; verify docs describe JSON's tq palette, seven/eight-slot JQ_COLORS overrides, structural quote styling, forced-color streams, and raw/proxy exceptions consistently.
- [x] 5.2 Run repository preflight and strict OpenSpec validation after implementation; verify required checks pass and record any unrelated existing failures separately.
