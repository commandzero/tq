# Output color compatibility review

The reviewed runs cover 12 output-color cases and 3 inherited jq color cases.
All 29 executed observations exit successfully. Unsupported adapters remain
explicitly unsupported.

Both reports retain `observed-differences`. The raw-byte runner does not
classify ANSI differences as semantic agreement. This review records the
differences separately; it does not suppress them or change baseline bytes.

## Reviewed differences

| Cases | Comparison | Classification |
| --- | --- | --- |
| `output-colors.default-json-roles` | jq/tq | Equal bytes after removing generated SGR. Enclosing quote ownership and SGR span boundaries differ. Both use the explicitly supplied tq palette in this fixture. |
| `output-colors.default-json-roles` | yq versus jq/tq | Palette and JSON whitespace differ. Parsing the stripped fixture confirms equal values. yq's `-c` controls YAML sequence indentation, not compact JSON output. |
| `output-colors.custom-eight-nested-quotes` | jq/tq | Equal stripped bytes. tq assigns quotes to their container and retains separate key/string content styles. |
| `output-colors.custom-seven-key-fallback` | jq/tq | Equal stripped bytes. Object-key content retains the number-slot fallback. Quote styling differs. |
| `output-colors.yaml-output` | yq/tq | Equal stripped bytes. Palettes and span boundaries differ. |
| `output-colors.csv-output`, `output-colors.tsv-output` | yq/tq | Existing native-profile adaptation, not ANSI-only. tq quotes numeric-looking strings and emits null as an empty field. yq emits unquoted `42` and literal `null` for this fixture. |
| `output-colors.jsonl-alias`, `output-colors.json-sequence` | jq/tq | Equal stripped bytes, including every LF and RS boundary. |
| `output-colors.raw-formatted-string` | jq/tq | Exact raw-byte agreement, with no inserted ANSI. |
| `output-colors.raw-non-string-json-fallback` | jq/tq | Equal stripped bytes. tq retains compact JSON for the non-string despite a YAML selector; the trailing string stays raw. Quote ownership and SGR span boundaries differ. |
| `output-colors.proxy-bytes`, `output-colors.toon-sequence` | tq only | No cross-tool claim. Proxy bytes remain exact; stripped TOON sequence framing matches the expected records. |
| `manual.colors.default-palette`, `manual.colors.ansi-values` | jq/tq | Equal stripped bytes with inherited custom palettes. Enclosing quote ownership and span boundaries differ. |
| `manual.colors.custom-1-31` | jq/tq | Equal stripped bytes and the same requested red style. SGR span boundaries differ. |

The CSV/TSV differences match the existing
[native-format review](native-formats-v1.md). They are not new serialization
changes introduced by coloring.

## tq's plain-byte contract

The CLI color matrix requires successful colored and monochrome executions,
requires colored bytes to contain SGR, and checks exact stripped-byte equality.
CSV and TSV fixtures select each row object with `.[]`; an invalid array root
cannot pass merely because both invocations fail.

Separate tests cover custom seven/eight-slot palettes, quoted header escapes,
root/nested quote contexts, raw/proxy bytes, frame boundaries, resource failures,
empty output, and atomic cardinality errors. Library tests check typed token
roles rather than treating raw cross-tool differences as proof of correctness.

## Replay

The reference tools are jq 1.8.2 and Mike Farah yq 4.53.2 on macOS arm64.
The corrected eight-slot fixture uses `31:32:33:34:35:36:37:90`; the seven-slot
fixture uses `31:32:33:34:35:36:37`.

```sh
cargo build -p tq-cli -p tq-test-support --bins
TQ_BIN="$PWD/target/debug/tq" target/debug/tq-compat run --profile full \
  --case-prefix output-colors --json /private/tmp/output-colors-compat-reviewed.json
TQ_BIN="$PWD/target/debug/tq" target/debug/tq-compat run --profile full \
  --case-prefix manual.colors --json /private/tmp/manual-colors-compat-reviewed.json
```

These reports retain executable identities, arguments, environment, raw stdout,
stderr, exit status, and all differences. The earlier exploratory reports are
not the reviewed evidence. Successful raw observations have no decoded result
list; the stripped-byte and JSON-value checks described above are separate
review checks, not claims made by the raw-byte runner.

Reviewed copies and the full preflight log are retained in the separate
`tq-benchmarks` checkout under `.work/output-colors-20260913/`.
The `*-compat-restacked.json` replays retain the same raw stdout observations
and difference counts after the parent PR restack.
