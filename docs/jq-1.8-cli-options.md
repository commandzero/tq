# jq 1.8.x CLI option inventory

This inventory is the admission registry for jq-shaped command-line options.
It is based on jq 1.8.x help/manual behavior and executable probes. `tq` keeps
its TOON-first defaults, so jq output switches apply to JSON output unless an
adaptation is called out below.

| Option | Classification | tq contract |
| --- | --- | --- |
| `-n`, `--null-input` | supported | Run once with `null`; do not consume input. |
| `-R`, `--raw-input` | supported | Read UTF-8 physical lines, or one bounded string with `--slurp`. |
| `-s`, `--slurp` | supported | Retain ordered inputs and run once. |
| `-c`, `--compact-output` | adapted | Compact JSON; requires `--output-format json`. |
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
`--argtoon`, TOON writer controls, explain/trace/report controls, and resource
limits are intentionally tq-specific. YAML output is admitted explicitly with
`--output-format yaml`; it never follows filename inference.

## Environment, terminal, and filesystem policy

Library callers may disable environment, terminal, or filesystem integration
through `CapabilityPolicy`. The process CLI permits them, but ambient color is
never used by injectable test I/O. Explicit `-C` is deterministic, `-M` wins,
the last explicit color flag wins, `NO_COLOR` disables ambient color, and `JQ_COLORS` is not interpreted because
tq publishes one stable palette. Every filter/input/argument file is read under
the per-source byte limit, and diagnostics identify the path without including
file contents or argument values.

## Evidence scope

Executable shell cases cover argv parsing, stdin, ordered files, output bytes,
stderr, and exit classes. The published full campaign records the exact
reference executable identity and currently exercises these cases against a jq
1.8.2 reference build, including 1.8-only behavior such as `--raw-output0`.
Platform-dependent behavior remains explicitly classified in the inventory.
