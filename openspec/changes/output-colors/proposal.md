## Why

tq currently treats color as a JSON-only option. PR #33 supplies jq-compatible JSON token colors and `JQ_COLORS` handling; a shared tq semantic palette will extend readable terminal presentation to every supported output format, including JSON.

## What Changes

- Make `0;39:0;94:0;94:0;35:0;32:0;90:0;90:0;36` tq's built-in eight-slot palette: normal nulls, light-blue booleans, magenta numbers, green strings, light-black structure, and cyan keys.
- Apply the same roles to JSON, YAML, TOON, JSON Lines/NDJSON, JSON sequences, TOON sequences, CSV, and TSV, independent of input format.
- Generalize automatic terminal color, `-C/--color-output`, and `-M/--monochrome-output` across those formats. Keep automatic redirected output plain and honor the existing terminal/environment capability boundaries and `NO_COLOR`.
- Define color as presentation around existing serialization. Removing only tq-generated ANSI styling must recover byte-identical plain serialization, including sequence framing, escaping, whitespace, and number spellings.
- Keep raw strings and proxy bytes verbatim. Do not invent array indices, focus state, delimiters, or trailing commas merely to exercise theme roles.
- Treat the JSON palette difference from jq as a non-breaking presentation change. Document and test tq's colors separately from jq's data compatibility.
- Initially provide one built-in default theme. User theme files, theme-selection flags, and interactive focus are deferred.
- Preserve seven/eight-slot `JQ_COLORS` overrides and apply them across all output formats; keep `NO_COLOR` and explicit color/monochrome selection.
- Color syntactic quotation marks with their enclosing structure's delimiter/separator style, including custom palettes, instead of jq's string/key style. Use the object entry for root scalar quotes; introduce no extra palette slot.

## Capabilities

### New Capabilities

- `output-colors`: Shared semantic theme, token roles, terminal policy, reversible ANSI decoration, and bounded rendering for every supported output format.

### Modified Capabilities

- `tq-cli`: Permit color controls across output formats, replacing the JSON Lines and delimited-output color restrictions while preserving unrelated formatting conflicts.

## Impact

Implementation will touch CLI option validation and terminal detection, the native-format catalog and output writers, TOON streaming/transcode output, and compatibility fixtures including PR #33's jq manual examples. Plain library serialization stays the default; the shared output options need a format-independent replacement for `color_json`. Query evaluation and input decoding do not change. No new output format or configuration-file interface is introduced.

This proposal is stacked on PR #33, branch `test/jq-manual-coverage`.
