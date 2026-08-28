# jq 1.8.x CLI option inventory

This table records which jq command-line options `tq` accepts. It comes from jq
1.8.x help text, the manual, and executable probes. `tq` keeps its TOON-first
defaults, so jq formatting switches apply to JSON unless the table says
otherwise.

| Option | Classification | tq contract |
| --- | --- | --- |
| `-n`, `--null-input` | supported | Run once with `null`; do not consume input. |
| `-R`, `--raw-input` | supported | Read UTF-8 physical lines, or one bounded string with `--slurp`. |
| `-s`, `--slurp` | supported | Retain ordered inputs and run once. |
| `-c`, `--compact-output` | adapted | Compact JSON; requires JSON or JSON Lines output. JSON Lines is always compact. |
| `-r`, `--raw-output` | supported | Write strings directly and other values as compact JSON. |
| `--raw-output0` | supported | Raw output separated by NUL; a string containing NUL is rejected. |
| `-j`, `--join-output` | supported | Raw output without separators. |
| `-a`, `--ascii-output` | adapted | Escape non-ASCII JSON output; requires JSON output. |
| `-S`, `--sort-keys` | supported | Recursively sort object keys before any structured encoding. |
| `-C`, `--color-output` | adapted | Force tq's stable ANSI JSON palette; requires JSON output. |
| `-M`, `--monochrome-output` | supported | Disable ANSI output, including environment defaults. |
| `--tab`, `--indent N` | adapted | Select JSON indentation; spaces also configure TOON indentation. YAML flow output is fixed. |
| `--unbuffered` | supported | Flush after every complete output value/frame. |
| `--stream`, `--stream-errors` | supported | Use bounded JSON/TOON event input; YAML remains document-at-a-time. |
| `-x`, `--proxy-on-error` | extension | Retain each bounded source and pass its original bytes through when structured parsing rejects it. Resource and execution errors remain failures. |
| `--seq` | divergent | tq emits TOON Text Sequence; JSON-seq input is not inferred. |
| `-f`, `--from-file` | supported | Load the filter from one bounded file. |
| `-L`, `--library-path` | supported | Add a repeatable explicit module root. Paths are canonicalized and imports are confined to those roots. |
| `--arg`, `--argjson` | supported | Bind named values and populate `$ARGS.named`. |
| `--slurpfile`, `--rawfile` | supported | Read one policy-approved, bounded argument file. |
| `--args`, `--jsonargs` | supported | Bind remaining argv values in `$ARGS.positional`. |
| `--argfile` | unsupported | Removed upstream; use `--slurpfile`. |
| `-e`, `--exit-status` | supported | Exit 0/1/4 for truthy, false-or-null, or no result. |
| `-b`, `--binary` | platform-dependent | Accepted as a no-op where Rust stdio is already binary-safe. |
| `-V`, `--version` | supported | Print tq, TOON, jq-target, and revision versions. |
| `--build-configuration` | adapted | Print stable tq target/capability information. |
| `-h`, `--help` | supported | Generated from the admitted option registry. |
| `--` | supported | End option parsing. |
| `--run-tests` | unsupported | jq-internal test runner; use tq's Cargo/compatibility campaigns. |

## tq extensions

`--input-format`, `--output-format`, `--toon-sequence-input`, `--unframed`,
`--argtoon`, the TOON writer settings, explain and report controls, and resource
limits belong to tq rather than jq. Request YAML with `--output-format yaml`.
The output format never follows the filename.

## Environment, terminal, and filesystem policy

Library callers can disable environment, terminal, or filesystem access through
`CapabilityPolicy`. The process CLI permits those integrations. Injectable test
I/O never uses ambient color. `-C` selects tq's fixed palette, `-M` wins, and
the last color flag wins. `NO_COLOR` disables ambient color. tq ignores
`JQ_COLORS`. The per-source byte limit applies to every filter, input, and
argument file. Diagnostics name the path but omit file contents and argument
values.

## Evidence scope

Executable shell cases cover argument parsing, stdin, ordered files, output
bytes, stderr, and exit classes. The full campaign records the exact reference
binary and runs these cases against jq 1.8.2, including 1.8-only behavior such
as `--raw-output0`. The table marks platform-dependent behavior.
