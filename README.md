# tq

The TOON format helps [reduce token counts](https://toonformat.dev/guide/getting-started#what-is-toon)  and [improve LLM accuracy](https://toonformat.dev/guide/benchmarks.html#retrieval-accuracy) thanks to a less verbose syntax with correctness hints, like array lengths.

Thanks to JSON's decades-long adoption, powerful tools like [`jq`](https://jqlang.org/) exist to quickly filter and query JSON documents. To use `jq` with TOON-formatted input, you would have had to convert it to JSON first.

This is exactly the goal of `tq`: a TOON-native query tool with jq-compatible supported behavior. With some additional benefits:

1. Supported `jq` behavior scoped by the [compatibility guide](docs/compatibility.md)
2. Expanded input and output format support including JSON, YAML, and TOON
3. Multithreading for large workloads
4. Rust's memory safety

## Installation

### Homebrew

```console
brew install commandzero/tools/tq
```

### Cargo

```console
cargo install tq-cli
```

## Usage

### Shell pipes

`tq` autodetects the input format on stdin and outputs TOON by default.

```sh
echo '{"foo":"bar"}' | tq
# => foo: bar

echo '{"fruits":["apple","banana","orange"],"vegetables":["carrot","broccoli","spinach"]}' | tq '.fruits'
# => [3]: apple,banana,orange

echo '{"fruits":["apple","banana","orange"],"vegetables":["carrot","broccoli","spinach"]}' | tq '.vegetables[2]'
# => spinach
```

Use `-i | --input-format` to specify the input format and `-o | --output-format` to specify the output format, or `-c | --compact-output` for compact JSON output.

```sh
echo 'input_formats[3]: json,yaml,toon' | tq -i toon -c
# => {"input_formats":["json","yaml","toon"]}
```

### Silent passthrough

The `-x | --proxy-on-error` argument allows graceful handling of untrustworthy
sources. With `tq -x` any unparsable input will be passed through unmodified
without errors.

```sh
echo '[invalid:json]' | tq
# => tq: Json input rejected: invalid JSON at byte 8, line 1, column 9: invalid numeric literal

echo '[invalid:json]' | tq -x
# => [invalid:json]
```

### AGENTS.md

Add a single directive to your `AGENTS.md` file to use `tq` with your agents:

```markdown
Tool calls: use `tq` instead of `jq` for querying and filtering data in JSON-compatible formats, including .json, .ndjson, .jsonl, .yml, .yaml, and .toon. Always pipe expected JSON outputs through `tq -x -c` with an optional filter or `jq` expression.
```

### Examples

```console
printf 'name: Ada\nactive: true\n' | tq '.name'
tq --input-format json --output-format json -c '.features | length' feed.json
tq --input-format yaml -r '.people[].name' people.yaml
tq --input-format json5 -r '.attributes.title' saved-object.json
tq -i jsonl -o jsonl '.event' events.ndjson
unpredictable-command | tq -x -i json
```


For multiple sources, the fallback applies to each source separately. With
`--slurp`, a rejected source proxies the complete ordered source set because
slurp evaluates the set as one input. `--proxy-on-error` cannot be used with
`--stream-errors`.

By default, `tq` writes zero or more canonical TOON values, each followed by LF.
`--seq` selects JSON Text Sequence input. It writes TOON Text Sequence output by
default, or JSON Text Sequence output when combined with `-c` or `-o json`.
Use `-o toon-seq` to request TOON framing without changing input selection.
`--unframed` requires exactly one standalone document. `-r` writes raw strings
and `-j` joins raw output.

### Terminal colors

All output formats use color automatically on a terminal and stay plain when
redirected. `-C` forces color, `-M` disables it, and the last flag wins.
`NO_COLOR` disables automatic color; `-C` overrides it. Raw strings and
proxy-on-error bytes are never decorated. Non-string results in raw mode retain
their compact JSON fallback and use the shared palette.

The default uses cyan keys, green strings, magenta numbers, light-blue booleans,
normal nulls, and light-black delimiters, separators, and enclosing quotes.
JSON uses the same palette. Customize any output format with `TQ_COLORS`.
It takes priority over the compatible `JQ_COLORS` fallback:

```sh
export TQ_COLORS='0;39:0;94:0;94:0;35:0;32:0;90:0;90:0;36'
```

Slots are null, false, true, numbers, strings, arrays, objects, and keys.
The seven-slot form uses the number style for keys; invalid values fall back
to the complete default. Unlike jq, enclosing quotes share structural colors:
the array style inside arrays and the object style for object keys/values and
standalone strings. Escaped quotes within content keep the string/key style.

Colors are presentation, not a data-format compatibility promise. Forced-color
streams contain ANSI escapes; use `-M` when a consumer needs plain format bytes.
See the [output color measurements](docs/tests/output-colors-performance.md) for
the added time, memory, and output bytes on document and transcode paths.

## Streaming and memory

`--stream` emits jq-compatible `[path,value]` records and container-end
`[path]` records from JSON, JSON Lines, JSON Text Sequences, or TOON decoder
events. JSON Lines and JSON Text Sequences reset the root path for every
physical record. YAML and JSON5 decode one document at a time. On large inputs,
streaming avoids retaining the whole document:

```console
tq --stream --input-format json -o json -c \
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

CLI memory defaults scale with available RAM: preparation uses a 1/8 ceiling,
while hybrid and decoder in-flight buffers each use 1/32. These are not
preallocated reservations; explicit byte-limit flags override them. Readable
Linux cgroup limits constrain the available-memory snapshot. Array preparation
stays in memory until the shared budget requires spilling. See
[memory and limits](docs/compatibility.md#memory-and-limits) for discovery
fallbacks and the scope of these ceilings.

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
./scripts/campaign-run.sh compatibility smoke
./scripts/campaign-run.sh compatibility full
target/release/tq compatibility
```

Benchmark suites choose inputs and cases; quick (0+1), standard (1+3), and
extended (1+5) choose sampling extent. The default suite is `natural-corpus`
with profile `standard`; `smoke` uses tiny fixtures, while `large-input` uses
the admitted building-footprint source. Reports and large corpora belong in
the separate `commandzero/tq-benchmarks` repository.

```console
./scripts/campaign-run.sh benchmark
./scripts/campaign-run.sh benchmark smoke
./scripts/campaign-run.sh benchmark natural-corpus quick
./scripts/campaign-run.sh benchmark large-input extended
./scripts/campaign-run.sh compatibility stack-overflow standard
```

See the [benchmark guide](benchmarks/README.md) for campaign details and the
[performance policy](docs/performance-baseline.md) for regression thresholds.

## jq user filters and modules

Parameterized `def` filters support lexical capture, filter and value
parameters, generator cardinality, shadowing, and recursion on tq's bounded
managed call stack. The process CLI resolves modules from confined default roots:
the filter's directory or current directory, `JQ_LIBRARY_PATH`, `HOME/.jq`, and
install-relative library roots. `-L` replaces those defaults with explicit
confined roots; embedded callers retain their explicit filesystem and module
policy:

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
and `gsub` built-ins use a safe Rust bounded-backtracking regex engine. Pattern,
input, match, replacement, and VM-work limits bound resource use, but the engine
does not promise linear-time matching. UTC parsing, formatting, broken-down time,
and epoch conversion support jq's date arrays for the documented range from year
0000 through 9999.

The process CLI enables environment and platform data by default. Embedded
callers retain deny-by-default controls and can admit each authority with
`--allow-environment` or `--allow-platform`. `$__loc__` is always available and
returns `{file, line}` for its location in the query source. Inline filters use
`<top-level>`; filter files and modules retain their path identities.
Decoder-owned `input_line_number` context is available without a capability flag.
See [the compatibility policy](docs/jq-regex-date-platform.md) for engine
differences, limits, redaction, and release-host classifications.

## Current boundaries

Labels and `break` are implemented with jq-compatible manual behavior. The
[option inventory](docs/jq-1.8-cli-options.md) distinguishes supported options,
native-output adaptations, and options removed upstream.

The numeric model preserves accepted input literals as written. Arithmetic uses
jq-compatible binary64 behavior when needed. Digit, exponent-expansion, and
index limits return resource or range errors instead of silently losing data.
TOON output and configured resource limits differ from jq's default contract.
The [compatibility guide](docs/compatibility.md) separates tested matches,
reviewed library disparities, and remaining implementation gaps.

## Contributing

See the [contributor guide](docs/contributing.md) for setup, validation, and
development requirements.

Licensed under MIT.
