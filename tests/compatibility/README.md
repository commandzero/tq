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
| CLI values | `--arg name Ada '$name'` | Partial |
| User filters and modules | `def f: .x; f`, `include "lib"` | Deferred |
| Folds | `reduce .items[] as $x (0; . + $x)` | Deferred |
| Recursive descent | `.. \| scalars` | Deferred |
| Interpolation | `"name=\(.name)"` | Deferred |
| Regular expressions | `test("^prod-")` | Deferred |
| Time and environment | `now`, `env.HOME` | Deferred |

Important migration differences:

- tq structured output defaults to TOON Text Sequence. Use `--output-format json` for JSON consumers.
- tq has documented digit, exponent, and index limits for large numbers.
- `--arg`, `--argjson`, and `--argtoon` need more compatibility coverage.

The current matrix records 146 supported capabilities, 5 partial capabilities,
7 documented differences, 2 unsupported capabilities, 14 deferred
capabilities, and no untested capabilities. Its next priorities are user
functions/modules, `reduce`/`foreach`, recursive descent, interpolation,
regex, and CLI-value coverage.

The complete evidence is in [coverage-v1.json](reviews/coverage-v1.json). The
older [reference candidate](baselines/jq-yq-mvp-v1.json) compares jq and yq
only.
