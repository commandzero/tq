# jq, yq, and tq benchmark comparisons

Compare jq, yq, and tq on common data-processing tasks. Each page explains
what the query does and shows the latest available results for that workload.

Only outputs that pass the correctness check are timed. Failed, unsupported,
and unmeasured cases remain visible. The accepted Linux review below uses the
recorded Ironhide run with jq 1.8.1, pinned yq 4.53.2, and the recorded tq
release build. The later Brew yq 4.53.6 installation was not mixed into these
measurements. The earlier macOS captures remain historical correctness and
execution evidence only because they lacked verified RSS.

## Findings

The standard review covers 846 adapter observations: 703 timed, 134
unsupported capability rows, 9 resource-limit rows, and 0 incorrect rows. It
records 10,579 process samples: 10,570 valid timed samples and 9 samples from
failed resource-limit attempts. Every recorded sample has positive GNU-time
per-process RSS. The three additional smoke workloads are rendered on their
own pages and complete the 39-workload review with 18 timed rows, 540 measured
samples, and no incorrect or unsupported rows.

The rapid check covers 30 observations: 27 timed, 3 unsupported, and 0
incorrect, with one positive-RSS sample for each timed row. In the standard
review, the 134 unsupported observations are 130 yq adapters and 4 tq YAML
event-stream adapters. The rapid exclusions are the two yq event-stream
adapters and the tq YAML event-stream adapter.

On `usgs-all-month`, identity shows the main tradeoff. jq took 211.41 ms and
used 64.92 MiB; tq JSON took 543.79 ms and 15.27 MiB; tq TOON took 541.50 ms
and 15.22 MiB. Path update favored tq JSON at 175.26 ms versus jq at 210.44
ms, while RSS was 69.71 MiB versus 64.86 MiB. Event streaming favored jq at
282.40 ms and 4.00 MiB versus tq JSON at 1,138.87 ms and 8.15 MiB.
Recursive scalar traversal also favored jq at 360.75 ms and 64.86 MiB versus
tq JSON at 2,278.14 ms and 69.30 MiB. Object construction took 24,622.29 ms
and 490.74 MiB for yq JSON and 23,476.39 ms and 1,475.91 MiB for yq YAML;
jq took 138.44 ms and 65.82 MiB, while tq reached its resource limit.

These examples describe workload-specific tradeoffs rather than an overall
winner. Unsupported and resource-limit rows do not support speed rankings.
The resource-limit statuses are recorded; their engine-level cause remains
subject to stderr confirmation.

## Results

<!-- benchmark-results:start -->
Last updated: 2026-09-11
Profile: `standard` | Status: `observed-failures`

846 adapter observations across 36 workloads. Only correctness-checked outputs are timed; failed rows cannot support a speed ranking.

### Campaign coverage

| Outcome | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| timed | 141 | 76 | 76 | 138 | 134 | 138 |
| incorrect | 0 | 0 | 0 | 0 | 0 | 0 |
| unsupported | 0 | 65 | 65 | 0 | 4 | 0 |
| timeout | 0 | 0 | 0 | 0 | 0 | 0 |
| resource-limit | 0 | 0 | 0 | 3 | 3 | 3 |
| oom-or-signal | 0 | 0 | 0 | 0 | 0 | 0 |

### usgs-all-month

Largest recorded JSON input: 8013185 bytes, 11274 logical records. Each row compares the same workload across tools. Wall time is in milliseconds and captured peak RSS is in MiB. Lower is better. Missing RSS is not captured; compare matching formats and check each workload page for outcomes and sample counts.

| Workload | Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| array-construction | Wall (ms) | 138.626 | 569.176 | 1119.989 | 176.037 | 215.935 | 175.361 |
| array-construction | Peak RSS (MiB, source) | 64.67 (gnu-time-v) | 474.99 (gnu-time-v) | 1306.37 (gnu-time-v) | 74.12 (gnu-time-v) | 94.23 (gnu-time-v) | 83.05 (gnu-time-v) |
| blocking-sort | Wall (ms) | 139.322 | 337.389 | 944.942 | 172.518 | 215.201 | 170.535 |
| blocking-sort | Peak RSS (MiB, source) | 60.90 (gnu-time-v) | 332.33 (gnu-time-v) | 1299.50 (gnu-time-v) | 9.30 (gnu-time-v) | 90.80 (gnu-time-v) | 8.69 (gnu-time-v) |
| comma-generator-sort | Wall (ms) | 250.113 | 995.106 | 1516.708 | 215.988 | 291.662 | 254.006 |
| comma-generator-sort | Peak RSS (MiB, source) | 64.96 (gnu-time-v) | 738.21 (gnu-time-v) | 1524.91 (gnu-time-v) | 72.84 (gnu-time-v) | 93.03 (gnu-time-v) | 81.75 (gnu-time-v) |
| dead-sort-length | Wall (ms) | 101.633 | 336.211 | 927.943 | 135.675 | 178.732 | 134.135 |
| dead-sort-length | Peak RSS (MiB, source) | 60.48 (gnu-time-v) | 332.97 (gnu-time-v) | 1302.80 (gnu-time-v) | 8.51 (gnu-time-v) | 90.09 (gnu-time-v) | 8.08 (gnu-time-v) |
| event-stream | Wall (ms) | 282.396 | unsupported | unsupported | 1138.871 | unsupported | 1097.745 |
| event-stream | Peak RSS (MiB, source) | 4.00 (gnu-time-v) | unsupported | unsupported | 8.15 (gnu-time-v) | unsupported | 7.91 (gnu-time-v) |
| format-base64-roundtrip | Wall (ms) | 138.752 | 492.760 | 1089.118 | 395.119 | 250.638 | 209.940 |
| format-base64-roundtrip | Peak RSS (MiB, source) | 60.31 (gnu-time-v) | 453.43 (gnu-time-v) | 1305.14 (gnu-time-v) | 11.70 (gnu-time-v) | 89.42 (gnu-time-v) | 8.30 (gnu-time-v) |
| format-csv | Wall (ms) | 137.877 | 1120.377 | 1749.505 | 248.434 | 289.065 | 250.909 |
| format-csv | Peak RSS (MiB, source) | 60.68 (gnu-time-v) | 478.52 (gnu-time-v) | 1440.90 (gnu-time-v) | 69.40 (gnu-time-v) | 89.43 (gnu-time-v) | 78.07 (gnu-time-v) |
| format-json | Wall (ms) | 251.400 | 997.104 | 1526.081 | 288.026 | 329.652 | 287.628 |
| format-json | Peak RSS (MiB, source) | 63.67 (gnu-time-v) | 832.21 (gnu-time-v) | 1528.47 (gnu-time-v) | 69.53 (gnu-time-v) | 89.61 (gnu-time-v) | 78.13 (gnu-time-v) |
| format-shell | Wall (ms) | 137.446 | unsupported | unsupported | 249.396 | 287.150 | 250.443 |
| format-shell | Peak RSS (MiB, source) | 60.68 (gnu-time-v) | unsupported | unsupported | 69.35 (gnu-time-v) | 89.54 (gnu-time-v) | 78.18 (gnu-time-v) |
| format-template | Wall (ms) | 137.901 | unsupported | unsupported | 393.692 | 288.551 | 244.650 |
| format-template | Peak RSS (MiB, source) | 60.28 (gnu-time-v) | unsupported | unsupported | 11.71 (gnu-time-v) | 89.50 (gnu-time-v) | 8.24 (gnu-time-v) |
| format-tsv | Wall (ms) | 137.263 | 528.732 | 1119.732 | 246.251 | 290.156 | 248.085 |
| format-tsv | Peak RSS (MiB, source) | 60.67 (gnu-time-v) | 460.48 (gnu-time-v) | 1304.88 (gnu-time-v) | 69.42 (gnu-time-v) | 89.61 (gnu-time-v) | 78.26 (gnu-time-v) |
| format-uri-html | Wall (ms) | 135.571 | unsupported | unsupported | 393.517 | 287.263 | 211.285 |
| format-uri-html | Peak RSS (MiB, source) | 60.30 (gnu-time-v) | unsupported | unsupported | 11.64 (gnu-time-v) | 89.31 (gnu-time-v) | 8.27 (gnu-time-v) |
| identity-reencode | Wall (ms) | 211.411 | 760.572 | 1297.316 | 543.793 | 215.493 | 541.504 |
| identity-reencode | Peak RSS (MiB, source) | 64.92 (gnu-time-v) | 483.57 (gnu-time-v) | 1361.74 (gnu-time-v) | 15.27 (gnu-time-v) | 88.91 (gnu-time-v) | 15.22 (gnu-time-v) |
| issue5-collection | Wall (ms) | 139.129 | 574.232 | 1128.168 | 175.655 | 215.812 | 176.321 |
| issue5-collection | Peak RSS (MiB, source) | 61.42 (gnu-time-v) | 618.60 (gnu-time-v) | 1528.16 (gnu-time-v) | 72.40 (gnu-time-v) | 92.89 (gnu-time-v) | 81.12 (gnu-time-v) |
| issue5-json-conversion | Wall (ms) | 100.061 | 258.304 | 932.736 | 136.829 | 174.562 | 174.827 |
| issue5-json-conversion | Peak RSS (MiB, source) | 60.18 (gnu-time-v) | 293.32 (gnu-time-v) | 1302.04 (gnu-time-v) | 69.39 (gnu-time-v) | 89.40 (gnu-time-v) | 78.12 (gnu-time-v) |
| issue5-paths | Wall (ms) | 99.451 | unsupported | unsupported | 136.736 | 177.092 | 173.982 |
| issue5-paths | Peak RSS (MiB, source) | 60.27 (gnu-time-v) | unsupported | unsupported | 69.27 (gnu-time-v) | 89.31 (gnu-time-v) | 78.15 (gnu-time-v) |
| issue5-predicate | Wall (ms) | 100.031 | unsupported | unsupported | 136.101 | 176.332 | 175.238 |
| issue5-predicate | Peak RSS (MiB, source) | 60.25 (gnu-time-v) | unsupported | unsupported | 69.46 (gnu-time-v) | 89.31 (gnu-time-v) | 78.00 (gnu-time-v) |
| issue5-scalar-utilities | Wall (ms) | 213.758 | unsupported | unsupported | 175.484 | 213.125 | 210.055 |
| issue5-scalar-utilities | Peak RSS (MiB, source) | 60.61 (gnu-time-v) | unsupported | unsupported | 70.47 (gnu-time-v) | 90.59 (gnu-time-v) | 79.18 (gnu-time-v) |
| label-early-break | Wall (ms) | 100.436 | unsupported | unsupported | 135.022 | 174.734 | 174.181 |
| label-early-break | Peak RSS (MiB, source) | 60.29 (gnu-time-v) | unsupported | unsupported | 69.35 (gnu-time-v) | 89.38 (gnu-time-v) | 78.10 (gnu-time-v) |
| multi-result-projection | Wall (ms) | 136.729 | 375.233 | 1040.836 | 169.334 | 251.576 | 132.898 |
| multi-result-projection | Peak RSS (MiB, source) | 60.54 (gnu-time-v) | 331.81 (gnu-time-v) | 1302.93 (gnu-time-v) | 7.77 (gnu-time-v) | 89.38 (gnu-time-v) | 7.23 (gnu-time-v) |
| numeric-reduction | Wall (ms) | 136.095 | 336.009 | 926.003 | 157.481 | 212.229 | 176.177 |
| numeric-reduction | Peak RSS (MiB, source) | 60.29 (gnu-time-v) | 344.36 (gnu-time-v) | 1296.59 (gnu-time-v) | 69.21 (gnu-time-v) | 89.41 (gnu-time-v) | 78.05 (gnu-time-v) |
| object-construction | Wall (ms) | 138.439 | 24622.290 | 23476.393 | resource-limit | resource-limit | resource-limit |
| object-construction | Peak RSS (MiB, source) | 65.82 (gnu-time-v) | 490.74 (gnu-time-v) | 1475.91 (gnu-time-v) | resource-limit | resource-limit | resource-limit |
| parse-discard | Wall (ms) | 100.554 | 260.953 | 926.953 | 137.826 | 175.416 | 173.940 |
| parse-discard | Peak RSS (MiB, source) | 60.34 (gnu-time-v) | 292.48 (gnu-time-v) | 1300.65 (gnu-time-v) | 68.70 (gnu-time-v) | 88.78 (gnu-time-v) | 77.20 (gnu-time-v) |
| path-update | Wall (ms) | 210.435 | 761.375 | 1327.983 | 175.259 | 215.352 | 209.443 |
| path-update | Peak RSS (MiB, source) | 64.86 (gnu-time-v) | 485.11 (gnu-time-v) | 1308.05 (gnu-time-v) | 69.71 (gnu-time-v) | 89.57 (gnu-time-v) | 78.30 (gnu-time-v) |
| recurse-bounded | Wall (ms) | 293.298 | unsupported | unsupported | 250.351 | 288.916 | 252.353 |
| recurse-bounded | Peak RSS (MiB, source) | 64.86 (gnu-time-v) | unsupported | unsupported | 69.47 (gnu-time-v) | 89.36 (gnu-time-v) | 78.05 (gnu-time-v) |
| recursive-scalars | Wall (ms) | 360.748 | unsupported | unsupported | 2278.141 | 2316.437 | 2322.709 |
| recursive-scalars | Peak RSS (MiB, source) | 64.86 (gnu-time-v) | unsupported | unsupported | 69.30 (gnu-time-v) | 89.25 (gnu-time-v) | 78.18 (gnu-time-v) |
| regex-test | Wall (ms) | 136.136 | unsupported | unsupported | 249.018 | 288.128 | 249.071 |
| regex-test | Peak RSS (MiB, source) | 60.73 (gnu-time-v) | unsupported | unsupported | 71.22 (gnu-time-v) | 91.10 (gnu-time-v) | 79.79 (gnu-time-v) |
| scalar-extraction | Wall (ms) | 98.934 | 259.637 | 922.644 | 136.457 | 176.583 | 172.616 |
| scalar-extraction | Peak RSS (MiB, source) | 60.23 (gnu-time-v) | 293.19 (gnu-time-v) | 1301.94 (gnu-time-v) | 69.27 (gnu-time-v) | 89.49 (gnu-time-v) | 78.29 (gnu-time-v) |
| selective-filter | Wall (ms) | 138.513 | 375.164 | 970.181 | 171.315 | 215.421 | 207.912 |
| selective-filter | Peak RSS (MiB, source) | 60.30 (gnu-time-v) | 380.99 (gnu-time-v) | 1296.11 (gnu-time-v) | 9.09 (gnu-time-v) | 89.55 (gnu-time-v) | 8.16 (gnu-time-v) |
| string-reduction | Wall (ms) | 137.102 | 336.334 | 945.400 | resource-limit | resource-limit | resource-limit |
| string-reduction | Peak RSS (MiB, source) | 60.86 (gnu-time-v) | 332.57 (gnu-time-v) | 1293.37 (gnu-time-v) | resource-limit | resource-limit | resource-limit |
| user-filter-call | Wall (ms) | 120.784 | unsupported | unsupported | 211.928 | 249.910 | 248.918 |
| user-filter-call | Peak RSS (MiB, source) | 60.54 (gnu-time-v) | unsupported | unsupported | 69.29 (gnu-time-v) | 89.36 (gnu-time-v) | 77.99 (gnu-time-v) |
| user-filter-map | Wall (ms) | 137.681 | unsupported | unsupported | 141.069 | 215.710 | 175.232 |
| user-filter-map | Peak RSS (MiB, source) | 60.54 (gnu-time-v) | unsupported | unsupported | 70.21 (gnu-time-v) | 90.07 (gnu-time-v) | 78.92 (gnu-time-v) |
| user-filter-select | Wall (ms) | 137.483 | unsupported | unsupported | 175.120 | 215.022 | 176.126 |
| user-filter-select | Peak RSS (MiB, source) | 60.33 (gnu-time-v) | unsupported | unsupported | 69.60 (gnu-time-v) | 89.80 (gnu-time-v) | 78.38 (gnu-time-v) |
| user-filter-sort-by | Wall (ms) | 138.519 | unsupported | unsupported | 176.911 | 215.619 | 180.022 |
| user-filter-sort-by | Peak RSS (MiB, source) | 61.42 (gnu-time-v) | unsupported | unsupported | 72.45 (gnu-time-v) | 92.93 (gnu-time-v) | 81.25 (gnu-time-v) |
| walk-structural | Wall (ms) | 952.231 | unsupported | unsupported | 695.054 | 740.195 | 734.899 |
| walk-structural | Peak RSS (MiB, source) | 86.61 (gnu-time-v) | unsupported | unsupported | 107.47 (gnu-time-v) | 127.68 (gnu-time-v) | 116.25 (gnu-time-v) |
<!-- benchmark-results:end -->

## Method

The catalog compares `jq`, `yq`, and `tq` on JSON, YAML, and TOON where each
adapter applies. The harness checks semantic output before timing a row. Each
workload page has one table per dataset, with tool/input-format columns and
rows for wall time, first-output latency, CPU, authoritative RSS, logical and
physical throughput, output bytes, outcomes, diagnostics, sample counts, and
dispersion. Compare columns with the same input format to isolate tool
differences. Native-format rows show how each tool performs with its supported
input formats. Harness wall time includes process wrapping, polling, and sampler
shutdown, so startup overhead can matter for short workloads and the value is
not pure executable time. First-output latency uses the first captured output
when available and may fall back to completed output. Linux authoritative RSS
comes from per-process GNU `/usr/bin/time -v`; sampled process-group RSS is a
separate inspection signal. The raw report records the measurement source
snapshot and executable identities; the findings describe that frozen release
run and do not claim exact clean-commit equivalence with later renderer or
lint-only changes. Findings apply to the recorded machine and inputs only.

## Workloads

1. [Startup identity](startup.md)
2. [Discarding a stream](parse-discard.md)
3. [Scalar field extraction](scalar-extraction.md)
4. [Projecting many results](multi-result-projection.md)
5. [Selective filtering](selective-filter.md)
6. [Numeric reduction](numeric-reduction.md)
7. [String reduction](string-reduction.md)
8. [Array construction](array-construction.md)
9. [Object construction](object-construction.md)
10. [Deep object merge](object-deep-merge.md)
11. [Path update](path-update.md)
12. [Blocking sort](blocking-sort.md)
13. [Sort by multiple keys](comma-generator-sort.md)
14. [Identity re-encoding](identity-reencode.md)
15. [Event stream filtering](event-stream.md)
16. [Recursive scalar traversal](recursive-scalars.md)
17. [User function calls](user-filter-call.md)
18. [User function mapping](user-filter-map.md)
19. [User function selection](user-filter-select.md)
20. [User function sorting](user-filter-sort-by.md)
21. [Regular-expression testing](regex-test.md)
22. [Sort before counting](dead-sort-length.md)
23. [Grouping a collection](issue5-collection.md)
24. [Enumerating paths](issue5-paths.md)
25. [JSON round trip](issue5-json-conversion.md)
26. [Any-match predicate](issue5-predicate.md)
27. [String and scalar utilities](issue5-scalar-utilities.md)
28. [Reading additional inputs](issue5-inputs.md)
29. [Base64 startup formatting](format-base64-startup.md)
30. [JSON text formatting](format-json.md)
31. [HTML and URI escaping](format-uri-html.md)
32. [CSV formatting](format-csv.md)
33. [TSV formatting](format-tsv.md)
34. [Shell quoting](format-shell.md)
35. [Base64 round trip](format-base64-roundtrip.md)
36. [Templated URI output](format-template.md)
37. [Bounded recursion](recurse-bounded.md)
38. [Structural walking](walk-structural.md)
39. [Early break with a label](label-early-break.md)
