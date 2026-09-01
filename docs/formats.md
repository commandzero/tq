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
| JSON Text Sequences, RFC 7464 | R/W | - | - |
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
- tq writes an RS-framed TOON Text Sequence by default. This is not jq's RFC
  7464 mode. RFC 7464 frames JSON payloads; tq frames TOON payloads.
- jq provides formatters such as `@csv`, `@tsv`, `@base64`, `@uri`, and `@sh`.
  They return strings from filters and do not add native document parsers or
  output modes for those formats.

See the [jq manual](https://jqlang.org/manual/), the
[yq documentation](https://mikefarah.gitbook.io/yq/), and
[tq's compatibility guide](compatibility.md) for details.
