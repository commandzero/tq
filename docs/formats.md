---
type: Report
title: Format compatibility
description: Input and output format support in jq, yq, and tq.
generated: { by: codex/gpt-6-astra, at: 2026-09-09T01:24:47Z }
---

# Format compatibility

This table compares native document input and output. It does not count formats
that require a filter to parse or assemble raw strings.

The comparison uses jq 1.8.x, Mike Farah's yq 4.53.x, and the current tq
checkout. `R` means read, `W` means write, and `-` means no native support.

| Format | jq | yq | tq |
| --- | :---: | :---: | :---: |
| JSON | R/W | R/W | R/W |
| JSON5 | - | - | R |
| JSON Lines / NDJSON | R/W | R/W | R/W |
| JSON Text Sequences, RFC 7464 | R/W | - | R/W |
| YAML | - | R/W | R/W |
| TOON | - | - | R/W |
| TOON Text Sequences | - | - | R/W |
| CSV | - | R/W | - |
| TSV | - | R/W | - |
| XML | - | R/W | - |
| Properties | - | R/W | - |
| TOML | - | R/W | - |
| HCL | - | R/W | - |
| Lua | - | R/W | - |
| INI | - | R/W | - |

## Notes

- jq reads a stream of whitespace-separated JSON values. With compact output,
  that covers JSON Lines. `--seq` selects RFC 7464 JSON Text Sequences.
- yq has an explicit JSON parser and supports multiple JSON documents, including
  NDJSON. Its YAML parser accepts some constructs that resemble JSON5, but yq
  has no JSON5 input mode. Do not rely on it for JSON5 syntax or tq's
  triple-double-quoted strings.
- tq keeps `.json` strict. Use `-i json5` or a `.json5` extension for JSON5.
  JSON5 is input-only and document-at-a-time.
- tq writes zero or more LF-terminated TOON values by default. `--seq` explicitly
  selects an RS-framed TOON Text Sequence with unambiguous record boundaries. This is not jq's RFC 7464 mode. `-c` selects compact JSON;
  `-o json` selects pretty JSON. `--seq` reads JSON Text Sequences. Combine it
  with `-c` or `-o json` to write RFC 7464 JSON records. Use `-i toon-seq` to
  read TOON Text Sequences explicitly. Native format support does not imply
  complete jq language or malformed-input parity; see the [compatibility
  guide](compatibility.md).
- jq provides formatters such as `@csv`, `@tsv`, `@base64`, `@uri`, and `@sh`.
  They return strings from filters and do not add native document parsers or
  output modes for those formats.

See the [jq manual](https://jqlang.org/manual/), the
[yq documentation](https://mikefarah.gitbook.io/yq/), and
[tq's compatibility guide](compatibility.md) for details.
