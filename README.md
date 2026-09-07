# tq

`tq` runs jq 1.8.x-style queries over TOON, YAML, JSON, JSON5, and JSON Lines. It
writes TOON Text Sequences by default and can stream JSON, JSON Lines, and TOON
without loading the complete input.

`tq` supports common jq filters, including navigation, pipes, generators,
conditionals, operators, variables, path updates, user filters, modules, and
the common built-ins. Arrays and ordered objects retain their jq semantics.
The language also includes `empty`, `error`, optional access, `try/catch`,
`reduce`, and `foreach`. See [jq compatibility](docs/compatibility.md) for
supported syntax and known differences.

## Install and use

Rust 1.87 or newer is required.

```console
cargo install tq-cli
tq '.features[] | {id, magnitude: .properties.mag}' feed.json
```

To build from a checkout instead, run `cargo build --release` and use
`target/release/tq`.

With no file argument, `tq` reads stdin. It processes files and `-` in argument
order. Recognized `.toon`, `.yaml`, `.yml`, `.json`, `.json5`, `.jsonl`, and
`.ndjson` extensions select the parser. Other sources use bounded content
detection. For ambiguous input, select a parser with
`--input-format toon|yaml|json|json5|jsonl`. `ndjson` is an alias for `jsonl`.
JSON5 is document-at-a-time input and accepts the literal triple-double-quoted
multiline strings used by kibana-sync. Files ending in `.json` remain strict
JSON, so select `-i json5` when a JSON5 producer uses that extension.

```console
printf 'name: Ada\nactive: true\n' | tq '.name'
tq --input-format json --output-format json -c '.features | length' feed.json
tq --input-format yaml -r '.people[].name' people.yaml
tq --input-format json5 -r '.attributes.title' saved-object.json
tq -i jsonl -o jsonl '.event' events.ndjson
unpredictable-command | tq -x -i json
```

`-x` or `--proxy-on-error` handles sources whose format is uncertain. `tq`
keeps the bounded source before parsing it. If parsing rejects the source,
`tq` writes the original bytes unchanged and treats that source as successful.
Resource, I/O, query, runtime, and output errors still fail.

For multiple sources, the fallback applies to each source separately. With
`--slurp`, a rejected source proxies the complete ordered source set because
slurp evaluates the set as one input. `--proxy-on-error` cannot be used with
`--stream-errors`.

By default, each structured result contains an ASCII RS byte, one canonical
TOON document, and LF. This framing distinguishes zero, one, and many results.
If a later result fails, earlier complete records remain valid. Use
`--output-format json` for jq-style JSON or `--output-format jsonl` for one
compact, LF-terminated JSON value per result. `-r` writes raw strings, `-j`
joins raw output, and `--unframed` is available when the query must return
exactly one TOON value.

## Streaming and memory

`--stream` emits jq-compatible `[path,value]` records and container-end
`[path]` records from JSON, JSON Lines, or TOON decoder events. JSON Lines resets
the root path for every physical record. YAML and JSON5 decode one document at a
time. On large inputs, streaming avoids retaining the whole document:

```console
tq --stream --input-format json \
  'select(length == 2 and (.[0] | length) == 1)' buildings.geojson
```

`--explain` and `--explain-json` show the query plan and its memory limits.
Plans fall into six classes: transcode, event, subtree, document, whole-input,
and blocking.

For the identity query `.` with JSON or strict TOON input and canonical TOON
output, the planner selects `transcode`. This path bypasses jq bytecode and uses
one bounded preparation arena. The JSON decoder stages object members as they
arrive. If a later member repeats a name, transcode rejects and discards the
current record instead of applying jq's last-value normalization. Unframed
output stays empty as well.

Safe key folding, sorted keys, raw or joined output, slurp, explicit jq stream
mode, non-TOON output, and non-identity queries use the existing plans.

A document plan keeps one decoded document, while slurp keeps every input
document. Sorting, uniqueness, and final reductions need blocking state. A fold
keeps one immutable accumulator plus bounded evaluator state. Transcode may
write array preparation, staged sequence records, or atomic unframed output to
private temporary files. Sequence output preserves completed records before a
later error. Unframed output publishes nothing until exactly one successful
result is known.

The resource controls are `--max-input-bytes`, `--max-depth`,
`--max-token-bytes`, `--max-line-bytes`, `--max-lookahead-bytes`,
`--max-vm-steps`, `--max-results`, `--max-output-bytes`,
`--prepare-memory-bytes`, and `--max-spool-bytes`. The evaluator checks for
SIGINT between units of work. A closed downstream pipe exits successfully.
`--report-file` records transcode preparation high-water bytes, object-index
spills, array preparations, spool bytes written and replayed, and the final
resource outcome. `--explain-json` includes the identity proof, decoder
duplicate policy, commitment mode, retained state, configured limits, and any
deterministic transcode fallback reason. JSON explanations also state the
duplicate-key limitation.

Each transcode report records the selected input format, whether selection came
from an override or detection, each bounded probe's inspected and commitment
bytes, and rejected probe candidates. Input staging counters are zero for the
single-pass transcode plan.

## Compatibility and benchmarks

The compatibility suite sends the same cases through jq, yq, and tq, then
compares result order, result count, failures, and raw framing.
See the [format compatibility matrix](docs/formats.md) for native input and
output support in each tool.

```console
./scripts/run-campaign.sh compatibility smoke
./scripts/run-campaign.sh compatibility full
target/release/tq compatibility
```

Benchmark campaigns generate and validate their input representations before
timing jq, yq, and tq. Reports and large corpora belong in the separate
`commandzero/tq-benchmarks` repository.

```console
./scripts/run-campaign.sh benchmark smoke
./scripts/run-campaign.sh benchmark standard
./scripts/run-campaign.sh benchmark large
```

See the [benchmark guide](benchmarks/README.md) for campaign details and the
[performance policy](docs/performance-baseline.md) for regression thresholds.

## jq user filters and modules

Parameterized `def` filters support lexical capture, filter and value
parameters, generator cardinality, shadowing, and recursion on tq's bounded
managed call stack. Modules load only from explicit roots:

```console
tq -L ./jq-libs 'import "metrics" as m; m::normalize' input.json
tq -L ./jq-libs 'include "shared"; shared_filter' input.json
```

Repeat `-L` to search multiple roots in order. The loader rejects absolute
paths and `..` escapes after canonicalization. `--explain` and
`--explain-json` include each loaded path and SHA-256 digest. Modules can
declare constant metadata with `module {...};`, which jq's `modulemeta` filter
reads.

## Regex, dates, and platform data

The Unicode-aware `test`, `match`, `capture`, `scan`, `split`, `splits`, `sub`,
and `gsub` built-ins use a bounded linear-time regex engine. UTC parsing,
formatting, broken-down time, and epoch conversion support jq's date arrays for
the documented range from year 0000 through 9999.

Environment and ambient platform data are opt-in. `--allow-environment` enables
both `env` and jq's `$ENV` startup snapshot. `$__loc__` is always available and
returns `{file, line}` for its location in the query source. Inline filters use
`<top-level>`; filter files and modules retain their path identities.
`--allow-platform` enables `now`, local timezone conversion, and `input_filename`.
Decoder-owned `input_line_number` context is available without a capability flag.
See [the compatibility policy](docs/jq-regex-date-platform.md) for engine
differences, limits, redaction, and release-host classifications.

## Current boundaries

`tq` supports lexical `label` and `break`, including early exit from a
generator. Some less common jq capabilities remain unsupported; see the
[compatibility guide](docs/compatibility.md) for the supported CLI switches
and known differences.

The numeric model preserves accepted input literals as written. Arithmetic uses
jq-compatible binary64 behavior when needed. Digit, exponent-expansion, and
index limits return resource or range errors instead of silently losing data.
These errors and TOON sequence framing are known differences from jq.

Licensed under MIT.
