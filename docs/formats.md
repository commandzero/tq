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
| CSV | - | R/W | R/W |
| TSV | - | R/W | R/W |
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
  7464 mode. Use `--seq` for JSON sequence input and output, or `-i json-seq`
  and `-o json-seq` to select either direction. `jsonseq` is an alias.
  Existing scripts using `--seq` for TOON output must switch to `-o toon-seq`.
- CSV and TSV require `-i csv`, `-i tsv`, or a matching file extension. tq does
  not guess them from content. Each row after the unique-key header is one
  Document. Quoted fields remain strings, including empty and numeric-looking
  strings. Unquoted JSON numbers and booleans become their corresponding types;
  empty fields and missing trailing fields become null. Literal `null` text
  remains a string. Excess fields and duplicate header keys are rejected.
- CSV and TSV output accept object Results with scalar or null fields. The
  first Result defines the ordered header across all input sources. Later
  Results may omit keys, which become empty fields, but cannot add keys.
  Nested arrays and objects are rejected. Quoting preserves scalar types,
  doubled quotes escape quotes, and quoted fields may contain newlines.
- `--strict-conversion` rejects a Result whose declared output normalization
  would change its value when decoded again. For example, it rejects a missing
  CSV key becoming an explicit null. Earlier completed rows remain published.
- `--max-line-bytes` bounds JSON Lines physical lines and CSV/TSV logical rows.
  `--max-token-bytes` bounds decoded fields; `--max-fields` bounds row width.
  `--max-frame-bytes` bounds RS recovery segments. CSV/TSV output uses the same
  row, field, and width limits and validates before writing an affected row.
- jq provides formatters such as `@csv`, `@tsv`, `@base64`, `@uri`, and `@sh`.
  They return strings from filters and do not add native document parsers or
  output modes for those formats.

XML, TOML, Properties, and INI remain follow-on adapters. HCL and Lua are
deferred entirely for now.

See the [jq manual](https://jqlang.org/manual/), the
[yq documentation](https://mikefarah.gitbook.io/yq/), and
[tq's compatibility guide](compatibility.md) for details.
