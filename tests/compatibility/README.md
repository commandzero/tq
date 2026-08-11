# tq feature comparison

Use tq now for ordinary jq-style selection and transformation. Check the
examples below before you move a filter with advanced jq language features.

| Feature | jq or yq example | tq |
| --- | --- | --- |
| Select and filter | `.items[] \| select(.enabled) \| .name` | Supported |
| Build output | `[.items[] \| {id, name}]` | Supported |
| Map, sort, unique | `.items \| map(.name) \| unique \| sort` | Supported |
| Update | `.items[] \|= .count += 1` | Supported |
| JSON, YAML, TOON input | `tq --input-format yaml '.items[]' file.yaml` | Supported |
| Common CLI modes | `-n`, `-R`, `-s`, `-r`, `-j`, `--stream`, `-e` | Supported |
| Output controls | `-a`, `-S`, `-C`, `-M`, `--raw-output0`, `--unbuffered` | Supported/adapted |
| CLI values | `--arg`, `--argjson`, `--rawfile`, `--slurpfile`, `--args`, `--jsonargs` | Supported |
| User filters and modules | `def f: .x; f`, `include "lib"` | Supported with explicit `-L` roots |
| Folds | `reduce .items[] as $x (0; . + $x)` | Supported |
| Recursive descent | `.. \| scalars` | Supported |
| Interpolation | `"name=\(.name)"` | Supported |
| Regular expressions | `test("^prod-")` | Supported with documented engine differences |
| UTC date/time | `fromdateiso8601`, `gmtime` | Supported |
| Local time and environment | `now`, `env.HOME` | Supported with explicit capability flags |

Important migration differences:

- tq structured output defaults to TOON Text Sequence. Use `--output-format json` for JSON consumers.
- tq has documented digit, exponent, and index limits for large numbers.
- jq JSON formatting switches require `--output-format json` because tq's
  structured default is TOON Text Sequence.
- YAML output uses exact-number-preserving YAML 1.2 flow syntax.
- `-L` is repeatable and searches only explicit, confined module roots.
- `env` requires `--allow-environment`; clock, timezone, and input metadata
  require `--allow-platform`.

The generated report records current capability counts and all raw-byte
adaptations. Its next priority is labels and breaks.

The complete evidence is in [coverage-v1.json](reviews/coverage-v1.json). The
older [reference candidate](baselines/jq-yq-mvp-v1.json) compares jq and yq
only.
