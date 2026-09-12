# jq, yq, and tq benchmark comparisons

Compare jq, yq, and tq on common data-processing tasks. Each page explains
what the query does and shows the latest available results for that workload.

Only outputs that pass the correctness check are timed. Failed, unsupported,
and unmeasured cases remain visible. This Linux review uses `tq-bench` native
measurements of jq 1.8.1, pinned yq 4.53.2, and the recorded tq release build.
This round includes Linux measurements only.

The [Linux worker-validation results](worker-validation.md) cover the new
native accounting path separately. They are helper controls, not jq/yq/tq
workload measurements. Every workload table below comes from the full native
rerun; no wrapper-based measurements are included.

## Findings

The standard review covers 846 adapter observations: 703 timed, 134
unsupported capability rows, 9 resource-limit rows, and 0 incorrect rows. It
records 10,579 primary samples: 10,570 valid timed samples and 9 samples from
failed resource-limit attempts. Another 180 instrumented samples check RSS
limits separately. Every recorded sample has positive native peak RSS. The
three additional smoke workloads are rendered on their own pages and complete
the 39-workload review with 18 timed rows and 540 measured samples.

The rapid check covers 30 observations: 27 timed, 3 unsupported, and 0
incorrect, with one positive-RSS sample for each timed row and three separate
instrumented limit checks. In the standard
review, the 134 unsupported observations are 130 yq adapters and 4 tq YAML
event-stream adapters. The rapid exclusions are the two yq event-stream
adapters and the tq YAML event-stream adapter.

On `usgs-all-month`, identity shows the main tradeoff. jq took 196.7 ms and
used 64.9 MiB; tq JSON took 533.6 ms and 15.4 MiB; tq TOON took 527.2 ms
and 15.3 MiB. Path update favored tq JSON at 168.6 ms versus jq at 196.8 ms,
while RSS was 69.5 MiB versus 64.9 MiB. Event streaming favored jq at
257.4 ms and 3.9 MiB versus tq JSON at 1,153.1 ms and 7.9 MiB. Recursive
scalar traversal also favored jq at 342.7 ms and 64.9 MiB versus tq JSON at
2,297.7 ms and 69.1 MiB. Object construction took 23,812.7 ms and 491.4 MiB
for yq JSON and 22,755.4 ms and 1,475.8 MiB for yq YAML; jq took 111.3 ms
and 65.8 MiB, while tq reached its resource limit.

The Linux self-regression gate evaluated 410 comparable standard tq rows. No
independent wall-time or peak-RSS increase exceeded 20%, so there are no
disclosures and no blocking increases above 50%. Thirteen rows were excluded
from evaluation: four unsupported tq YAML event-stream rows and nine
resource-limit attempts (three object-construction and six string-reduction
rows). The full gate evidence and unrounded supporting samples are retained
in the OpenSpec Linux regression review.

These examples describe workload-specific tradeoffs rather than an overall
winner. Unsupported and resource-limit rows do not support speed rankings.
The resource-limit rows record tq's classified resource exit, status 5.
These measurements do not establish a tq self-regression against earlier
wrapper-based reports, which use a different measurement method.

## Results

<!-- benchmark-results:start -->
Last updated: 2026-09-11
Profile: `standard` | Status: `observed-failures`

846 adapter observations across 36 workloads. Only correctness-checked outputs are timed; failed rows cannot support a speed ranking.

Environment: `linux` / `x86_64`, AMD Ryzen 7 7700 8-Core Processor, 16 logical CPUs, 61.9 GiB RAM; kernel `Linux 7.2.0-ogc4.1.fc44.x86_64 #1 SMP PREEMPT_DYNAMIC Thu Aug 20 16:15:37 UTC 2026`; compiler profile `release-benchmark`

RSS collector provenance (outside measurement tables): `instrumented linux-wait4`, `linux-wait4`.
Measurement method (outside measurement tables): `tq-bench` native measurement: RSS scope `wait4 child lifetime including pre exec waited descendants and threads`; residual RSS floor `2.5 MiB` retained, not subtracted; observed control excess `1.1 ms`; primary timing is sampler-free; instrumented `tq-bench` native measurement: RSS scope `wait4 child lifetime including pre exec waited descendants and threads; sampled worker process group`; residual RSS floor `2.5 MiB` retained, not subtracted; observed control excess `1.0 ms`; RSS-limit enforcement uses a separate worker process-group sampler and is not pooled with primary timing.

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

Largest recorded JSON input: 8013185 bytes, 11274 logical records. Each row compares the same workload across tools. Wall time cells use milliseconds and peak RSS cells use MiB, with one decimal place. Lower is better. A `-` cell means no valid comparable measurement, not zero; compare matching formats and check each workload page for outcomes and sample counts.

| Workload | Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| array-construction | Wall time | 114.7 ms | 559.5 ms | 1157.8 ms | 147.5 ms | 190.6 ms | 166.6 ms |
| array-construction | Peak RSS | 64.6 MiB | 474.4 MiB | 1310.3 MiB | 74.2 MiB | 94.2 MiB | 82.9 MiB |
| blocking-sort | Wall time | 106.0 ms | 320.5 ms | 932.5 ms | 146.2 ms | 180.8 ms | 139.8 ms |
| blocking-sort | Peak RSS | 60.9 MiB | 332.9 MiB | 1303.5 MiB | 9.3 MiB | 90.7 MiB | 8.6 MiB |
| comma-generator-sort | Wall time | 216.3 ms | 983.0 ms | 1512.5 ms | 207.1 ms | 249.8 ms | 226.5 ms |
| comma-generator-sort | Peak RSS | 64.7 MiB | 851.4 MiB | 1528.0 MiB | 72.7 MiB | 93.3 MiB | 81.8 MiB |
| dead-sort-length | Wall time | 99.3 ms | 304.0 ms | 957.7 ms | 125.7 ms | 171.4 ms | 117.4 ms |
| dead-sort-length | Peak RSS | 60.5 MiB | 331.4 MiB | 1296.0 MiB | 8.7 MiB | 90.1 MiB | 8.1 MiB |
| event-stream | Wall time | 257.4 ms | - | - | 1153.1 ms | - | 1081.8 ms |
| event-stream | Peak RSS | 3.9 MiB | - | - | 7.9 MiB | - | 7.7 MiB |
| format-base64-roundtrip | Wall time | 106.4 ms | 484.1 ms | 1082.3 ms | 369.7 ms | 245.1 ms | 202.9 ms |
| format-base64-roundtrip | Peak RSS | 60.2 MiB | 454.4 MiB | 1305.3 MiB | 11.6 MiB | 89.6 MiB | 8.1 MiB |
| format-csv | Wall time | 117.8 ms | 1093.8 ms | 1709.8 ms | 217.6 ms | 258.0 ms | 235.9 ms |
| format-csv | Peak RSS | 60.6 MiB | 476.7 MiB | 1439.7 MiB | 69.3 MiB | 89.4 MiB | 78.1 MiB |
| format-json | Wall time | 225.3 ms | 974.4 ms | 1489.7 ms | 256.2 ms | 299.1 ms | 274.5 ms |
| format-json | Peak RSS | 63.7 MiB | 832.6 MiB | 1528.0 MiB | 69.6 MiB | 89.6 MiB | 78.3 MiB |
| format-shell | Wall time | 116.6 ms | - | - | 217.7 ms | 260.4 ms | 240.5 ms |
| format-shell | Peak RSS | 60.5 MiB | - | - | 69.4 MiB | 89.4 MiB | 78.1 MiB |
| format-template | Wall time | 120.5 ms | - | - | 374.0 ms | 258.9 ms | 212.9 ms |
| format-template | Peak RSS | 60.2 MiB | - | - | 11.6 MiB | 89.4 MiB | 8.0 MiB |
| format-tsv | Wall time | 118.1 ms | 506.4 ms | 1052.9 ms | 215.0 ms | 258.2 ms | 236.5 ms |
| format-tsv | Peak RSS | 60.5 MiB | 463.3 MiB | 1309.7 MiB | 69.5 MiB | 89.4 MiB | 78.0 MiB |
| format-uri-html | Wall time | 114.6 ms | - | - | 371.0 ms | 250.0 ms | 203.7 ms |
| format-uri-html | Peak RSS | 60.2 MiB | - | - | 11.6 MiB | 89.5 MiB | 8.1 MiB |
| identity-reencode | Wall time | 196.7 ms | 728.9 ms | 1318.5 ms | 533.6 ms | 210.9 ms | 527.2 ms |
| identity-reencode | Peak RSS | 64.9 MiB | 487.3 MiB | 1362.2 MiB | 15.4 MiB | 88.8 MiB | 15.2 MiB |
| issue5-collection | Wall time | 113.0 ms | 564.5 ms | 1090.4 ms | 146.7 ms | 187.2 ms | 164.0 ms |
| issue5-collection | Peak RSS | 61.4 MiB | 615.3 MiB | 1357.8 MiB | 72.3 MiB | 93.0 MiB | 81.5 MiB |
| issue5-json-conversion | Wall time | 94.4 ms | 257.3 ms | 912.9 ms | 120.1 ms | 161.5 ms | 137.8 ms |
| issue5-json-conversion | Peak RSS | 60.3 MiB | 293.4 MiB | 1299.4 MiB | 69.3 MiB | 89.5 MiB | 78.3 MiB |
| issue5-paths | Wall time | 94.1 ms | - | - | 121.5 ms | 163.5 ms | 139.5 ms |
| issue5-paths | Peak RSS | 60.3 MiB | - | - | 69.3 MiB | 89.5 MiB | 78.1 MiB |
| issue5-predicate | Wall time | 94.4 ms | - | - | 121.2 ms | 164.6 ms | 139.4 ms |
| issue5-predicate | Peak RSS | 60.3 MiB | - | - | 69.3 MiB | 89.4 MiB | 78.1 MiB |
| issue5-scalar-utilities | Wall time | 211.6 ms | - | - | 154.7 ms | 198.5 ms | 172.7 ms |
| issue5-scalar-utilities | Peak RSS | 60.7 MiB | - | - | 70.4 MiB | 90.6 MiB | 79.3 MiB |
| label-early-break | Wall time | 95.2 ms | - | - | 122.6 ms | 164.7 ms | 141.1 ms |
| label-early-break | Peak RSS | 60.3 MiB | - | - | 69.4 MiB | 89.5 MiB | 78.0 MiB |
| multi-result-projection | Wall time | 100.8 ms | 370.6 ms | 1005.5 ms | 134.0 ms | 229.6 ms | 118.7 ms |
| multi-result-projection | Peak RSS | 60.5 MiB | 332.4 MiB | 1303.8 MiB | 7.9 MiB | 89.3 MiB | 7.2 MiB |
| numeric-reduction | Wall time | 100.5 ms | 327.5 ms | 894.5 ms | 136.3 ms | 175.9 ms | 152.7 ms |
| numeric-reduction | Peak RSS | 60.1 MiB | 335.4 MiB | 1296.8 MiB | 69.5 MiB | 89.3 MiB | 78.1 MiB |
| object-construction | Wall time | 111.3 ms | 23812.7 ms | 22755.4 ms | - | - | - |
| object-construction | Peak RSS | 65.8 MiB | 491.4 MiB | 1475.8 MiB | - | - | - |
| parse-discard | Wall time | 93.5 ms | 261.7 ms | 906.2 ms | 120.3 ms | 161.3 ms | 137.0 ms |
| parse-discard | Peak RSS | 60.3 MiB | 294.4 MiB | 1301.3 MiB | 68.6 MiB | 88.7 MiB | 77.4 MiB |
| path-update | Wall time | 196.8 ms | 737.7 ms | 1337.6 ms | 168.6 ms | 210.4 ms | 184.6 ms |
| path-update | Peak RSS | 64.9 MiB | 481.8 MiB | 1362.4 MiB | 69.5 MiB | 89.6 MiB | 78.0 MiB |
| recurse-bounded | Wall time | 284.7 ms | - | - | 226.8 ms | 274.3 ms | 247.5 ms |
| recurse-bounded | Peak RSS | 64.9 MiB | - | - | 69.4 MiB | 89.3 MiB | 78.1 MiB |
| recursive-scalars | Wall time | 342.7 ms | - | - | 2297.7 ms | 2314.8 ms | 2336.4 ms |
| recursive-scalars | Peak RSS | 64.9 MiB | - | - | 69.1 MiB | 89.1 MiB | 77.9 MiB |
| regex-test | Wall time | 113.8 ms | - | - | 194.6 ms | 235.6 ms | 211.4 ms |
| regex-test | Peak RSS | 60.7 MiB | - | - | 70.9 MiB | 90.9 MiB | 79.8 MiB |
| scalar-extraction | Wall time | 94.1 ms | 260.0 ms | 848.6 ms | 121.3 ms | 162.3 ms | 137.3 ms |
| scalar-extraction | Peak RSS | 60.2 MiB | 294.1 MiB | 1301.3 MiB | 69.2 MiB | 89.3 MiB | 78.2 MiB |
| selective-filter | Wall time | 101.4 ms | 365.8 ms | 969.3 ms | 163.4 ms | 202.3 ms | 197.2 ms |
| selective-filter | Peak RSS | 60.3 MiB | 344.6 MiB | 1296.4 MiB | 8.8 MiB | 89.4 MiB | 8.0 MiB |
| string-reduction | Wall time | 104.5 ms | 314.8 ms | 886.1 ms | - | - | - |
| string-reduction | Peak RSS | 60.9 MiB | 333.4 MiB | 1296.1 MiB | - | - | - |
| user-filter-call | Wall time | 99.3 ms | - | - | 195.2 ms | 238.2 ms | 213.8 ms |
| user-filter-call | Peak RSS | 60.7 MiB | - | - | 69.2 MiB | 89.4 MiB | 78.1 MiB |
| user-filter-map | Wall time | 101.2 ms | - | - | 135.3 ms | 178.7 ms | 153.7 ms |
| user-filter-map | Peak RSS | 60.5 MiB | - | - | 69.9 MiB | 90.0 MiB | 79.1 MiB |
| user-filter-select | Wall time | 103.5 ms | - | - | 145.5 ms | 186.8 ms | 162.9 ms |
| user-filter-select | Peak RSS | 60.4 MiB | - | - | 69.6 MiB | 89.7 MiB | 78.4 MiB |
| user-filter-sort-by | Wall time | 118.1 ms | - | - | 156.0 ms | 199.7 ms | 174.2 ms |
| user-filter-sort-by | Peak RSS | 61.5 MiB | - | - | 72.8 MiB | 93.1 MiB | 81.4 MiB |
| walk-structural | Wall time | 940.8 ms | - | - | 690.3 ms | 734.7 ms | 708.0 ms |
| walk-structural | Peak RSS | 86.5 MiB | - | - | 107.4 MiB | 127.3 MiB | 116.2 MiB |
<!-- benchmark-results:end -->

## Method

`tq-bench` checks correctness, then measures each executable from launch to
exit observation. Input preparation, worker startup and cleanup are excluded.
Native `wait4` records CPU time and lifetime peak RSS, including launch,
threads and waited descendants. The launch RSS floor is not subtracted.
First-output latency uses the earliest observed output, or completion if output
was only visible at exit.

Primary measurements run without RSS sampling; separate passes enforce RSS
limits. Compare tools on the same host and OS using repeated samples and
dispersion. The observed 1.1 ms control excess is not a timer-error guarantee.
See the [measurement details](../benchmark-harness.md).

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
