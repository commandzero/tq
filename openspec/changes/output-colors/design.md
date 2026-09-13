## Context

See [proposal.md](proposal.md) for motivation and scope. The existing CLI resolves automatic color at the process boundary, limits it to ordinary JSON, and rejects forced color for several framed and delimited formats. PR #33 exposes `color_json` and `JsonColorPalette`, supports seven/eight-slot `JQ_COLORS`, and colorizes buffered JSON bytes. TOON identity transcode has its own streaming publication path. The inherited jq manual compatibility coverage must distinguish jq color bytes from semantic compatibility.

## Goals / Non-Goals

**Goals:** Give every output writer typed token styling, preserve byte-identical undecorated output, and keep color state bounded on document and streaming paths. Make terminal policy a caller decision so library writers remain deterministic.

**Non-Goals:** New input/output formats, syntax changes, an interactive viewer, a theme-file loader, config discovery, a new RGB theme interface, or recoloring raw/proxy bytes. The eight-slot default and existing `JQ_COLORS` interface require no user configuration file.

## Decisions

### Semantic spans at the serializer boundary

Introduce a small role vocabulary and a presentation sink usable by all writers. Writers emit existing encoded byte spans with roles; the sink either passes bytes directly or decorates spans with SGR. Preserve the current escaping and formatting algorithms. Integrate at token emission, where the writer already knows key versus value, string versus number, and quote versus content.

Whole-output regular expressions would confuse escaped punctuation and numeric-looking strings. Reparsing output would add buffering and fail to preserve all existing spellings. A second parallel colored serializer would drift from plain serialization. Semantic spans keep one serialization path and make the stripping invariant testable.

The format-independent options replace the internal use of `color_json`. Audit public callers before removing or deprecating that field; the palette's presentation classification does not authorize an unrelated Rust API break. Keep a compatibility bridge if published API commitments require it.

### Exact palette and role precedence

Use the eight-slot palette in [the color spec](specs/output-colors/spec.md) as the source of truth. Its SGR entries reset emphasis/background and select fixed ANSI foreground slots, letting terminal themes choose RGB. Retain the inherited seven/eight-entry parser and seven-entry number-to-key fallback. Invalid or absent overrides select the complete new default. Custom entries are not restricted to the default foreground choices.

Quotes share the structural entry of their immediate enclosing container: array entry for array elements, object entry for object keys/values, and object entry for root scalar quotes. Nested containers own their delimiters; parent separators retain parent styling. Mapping and sequence syntax follow the same distinction in YAML/TOON. Table field quotes and separators use object context, including TOON tables and CSV/TSV. The spec defines these contexts so custom palettes with distinct array/object styles remain deterministic. Do not add a ninth slot or hardcode quote color.

Role precedence is transport boundary, syntactic quotation mark, key text, scalar content, then syntax punctuation/container delimiters. Escaped quotes inside a scalar are content, not enclosing quotes. CSV/TSV null fields remain empty. TOON counts are numbers. The noninteractive writers do not emit indices or focused tokens, and no serializer is changed to add them.

### Policy once per invocation

Resolve Auto/Always/Never and the palette once at the CLI process boundary using the existing capability policy. Read `JQ_COLORS` only when environment access is permitted; otherwise use the built-in default. Apply the resolved choice to every supported writer. A custom palette does not enable color. Retain last-explicit-flag precedence, allow `-C` to override `NO_COLOR`, and allow `-M` for every format. Do not couple color to input extensions, formatter builtins, or output indentation families.

This preserves plain redirected output while allowing deliberate terminal presentation through a pipe with `-C`. Such a stream needs SGR removal before direct format parsing. No writer probes environment variables or terminal state on its own.

### Framing and publication

Treat native serialization and ANSI presentation as separate contracts. JSON, JSON Lines, RFC 7464, YAML, TOON, and delimited framing requirements describe the serialized bytes before styling. SGR decorates payload token spans; RS, LF, and CR are written outside active styles. Embedded newlines close and reopen the current style without inserting or removing any text.

Close styles before handing control to raw/proxy output and at each complete result boundary. Emit no prefix until payload publication begins, preserving empty output and first-result validation failures. Reset at normal completion and make a best-effort reset after partial failures only when the sink and remaining byte allowance permit it. Do not replace the original error or retry a broken pipe.

Retain existing writer-specific commitment guarantees: atomic writers stage and validate the complete record as before, incremental writers may already have exposed partial data on failure, and completed earlier records remain intact. Count ANSI bytes in actual output accounting. Reserve enough byte allowance for a span's normal close before opening it so a resource-limit failure does not require writing beyond the configured limit.

### Streaming and resource behavior

Use constant-size current-style state plus the writer's existing bounded state. Stream long strings in styled chunks without collecting a result-sized string. Reuse the same presentation sink for document output and TOON event/transcode emission; enabling color alone must not force whole-document materialization.

Apply output accounting after decoration. If publication stages styled bytes, account for them in spool storage too. Preserve exact plain output paths with no allocation per token when styling is disabled. Choose a suitable crate/module boundary during implementation that does not introduce a dependency cycle between native-format and TOON crates.

### Compatibility evidence

Keep jq semantic comparisons and plain-byte compatibility cases. Update inherited default-palette snapshots and color rejection cases with tq-owned role assertions. Preserve `JQ_COLORS` acceptance and mapping coverage, changing enclosing-quote expectations to structural styling. Audit PR #33's colored jq examples individually, classifying default-palette and enclosing-quote differences as expected presentation differences while retaining result, error, and framing checks. Test seven/eight-slot overrides, invalid fallback, capability denial, distinct true/false styles, and custom structural styles across root and nested values in every output format.

For all native formats and writer routes, verify that removing only generated styling produces the exact plain bytes. Do not strip arbitrary escape sequences from raw input or proxy output to make a test pass. Include forced output through pipes and terminal-policy tests without requiring the test runner's actual stdout to be a TTY.

## Risks / Trade-offs

- ANSI changes bytes in explicitly colored streams. Automatic redirection stays plain, and docs explain `-M` and forced terminal presentation.
- A missed writer route would leave inconsistent colors. Enumerate native writers, raw/non-string fallback, JSON event output, TOON tabular/folded output, and direct transcode in acceptance coverage.
- Extra SGR increases output size and runtime. Merge adjacent compatible spans, retain a plain fast path, and measure representative document/transcode runs; colors do not grant extra resource allowance.
- Bright-black contrast varies by terminal theme. Preserve the supplied default and document terminal-palette dependence; do not invent adaptive RGB behavior.
- Custom array/object styles make quote context visible. Specify and test immediate-container ownership and the root fallback; preserve the seven-slot key fallback even when its number style differs from the default key color.

## Migration Plan

Land implementation with matching writer tests, CLI help, the format compatibility matrix, and a changelog entry describing a non-breaking presentation change. No new legacy-palette switch or data migration is required; users retain `JQ_COLORS` customization, with tq's structural quote styling. Update only tq presentation expectations in PR #33's test suite. Users needing plain bytes can continue to redirect automatic output or request `-M`.

The planning PR targets `test/jq-manual-coverage`, the head branch of PR #33. Retarget after its parent stack lands. Rollback can restore the previous color policy without changing serialized data or removing accepted query behavior.
