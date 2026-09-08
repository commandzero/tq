# I/O coverage audit

Source: [jq manual, I/O](https://jqlang.org/manual/#io). The audit accounts for all five fenced blocks and the runnable prose examples in the section. jq and tq adapters remain enabled for every MVP case so missing tq `input`, `debug`, and `stderr` support, plus line-number differences, are visible in compatibility output. Debug/stderr cases compare raw stderr bytes.

| Source | Example | Cases | Disposition |
| ---: | --- | --- | --- |
| 16 | input prose | `manual.io.input` | covered |
| 16 | inputs prose | `manual.io.inputs` | covered |
| 18 | stdout/stderr prose | `manual.io.debug`, `manual.io.stderr` | covered |
| 26 | `-n` input note | `manual.io.input-null-input` | new |
| 28 | fenced `[., input]` shell example | `manual.io.input` | new |
| 38 | fenced `reduce inputs` shell example | `manual.io.inputs` | new |
| 44 | `debug` prose | `manual.io.debug` | new |
| 48 | debug output placeholder | `manual.io.debug` | covered |
| 54 | `debug(msgs)` definition | `manual.io.debug-msgs` | new |
| 58 | fenced debug expression | `manual.io.debug-example` | new |
| 64 | fenced debug stderr output | `manual.io.debug-example` | covered |
| 71 | `stderr` prose | `manual.io.stderr` | new |
| 75 | `input_filename` prose | `manual.io.input-filename` | new |
| 77 | `input_line_number` prose | `manual.io.input-line-number` | new |

The output placeholder at line 48 is intentionally linked to its exercising case; it is not executable on its own. The direct-process harness adapts shell pipelines to stdin bytes and uses `--allow-platform` for tq metadata builtins.
