# Compatibility feature comparison

This table summarizes the 154-case compatibility candidate in
[reference-candidate-v2.json](reference-candidate-v2.json). It compares jq
`1.8.2-8-g603db3f` with yq `4.53.2` by feature area. A **match** has no recorded
semantic difference after result sequence, exit status, and error-class
normalization. A **divergence** has one or more recorded differences.

| Feature area | Cases | Both run | Match | Diverge | jq only | yq only | Neither run |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Arithmetic | 3 | 3 | 1 | 2 | 0 | 0 | 0 |
| Builtins | 33 | 33 | 10 | 23 | 0 | 0 | 0 |
| Cardinality | 3 | 3 | 2 | 1 | 0 | 0 | 0 |
| CLI behavior | 26 | 8 | 6 | 2 | 9 | 2 | 7 |
| Comparisons | 2 | 2 | 0 | 2 | 0 | 0 | 0 |
| Composition | 5 | 5 | 4 | 1 | 0 | 0 | 0 |
| Construction | 5 | 5 | 2 | 3 | 0 | 0 | 0 |
| Control flow | 6 | 6 | 4 | 2 | 0 | 0 | 0 |
| Deferred jq features | 12 | 0 | 0 | 0 | 12 | 0 | 0 |
| Errors | 6 | 6 | 1 | 5 | 0 | 0 | 0 |
| Identity | 7 | 7 | 7 | 0 | 0 | 0 | 0 |
| Navigation | 11 | 11 | 10 | 1 | 0 | 0 | 0 |
| Numeric behavior | 19 | 16 | 5 | 11 | 3 | 0 | 0 |
| Updates | 9 | 9 | 6 | 3 | 0 | 0 | 0 |
| Variables | 7 | 4 | 2 | 2 | 2 | 0 | 1 |
| **Total** | **154** | **118** | **60** | **58** | **26** | **2** | **8** |

`jq only` and `yq only` mean the other reference tool was marked
`unsupported` for that case; `Neither run` covers cases not executed by either
reference tool. These are coverage gaps, not semantic matches or failures.

## tq status

This baseline predates the tq executable. Every tq observation has status
`unavailable` (`executable not found`). The table does not state tq feature
support. Use it as the jq/yq reference comparison for the MVP target. jq is the
semantic target when jq and yq differ.

The source report is the record of individual cases, observed output, and
error classifications; this README is only its grouped view.
