# Benchmark evidence

The focused smoke campaign ran on 2026-08-31 on an Apple M4 Pro with macOS
26.6, using release tq and the manifest-recorded jq 1.7.1 binary. Each tool and
case used one warmup and 30 measured samples against the same generated JSON
input and output sink. The raw report is
`/Users/reno/Development/commandzero/tq-benchmarks/.work/jq-recursive-builtins-labels/smoke.json`
and remains intentionally outside the repository.

The campaign ran outside the restricted sandbox with elevated child-process
inspection permissions. The harness wrapped every measured jq and tq process
with `/usr/bin/time -l`; all accepted samples include its
`maximum resident set size` value. A run without those permissions or RSS data
is invalid benchmark evidence.

| Case | jq median | tq median | tq / jq | Goal |
| --- | ---: | ---: | ---: | ---: |
| Early label break | 74,793.5 us | 74,368.5 us | 0.994 | <= 2.0 |
| Bounded recurse | 73,728.5 us | 71,352 us | 0.968 | <= 2.0 |
| Structural walk | 73,818 us | 75,175.5 us | 1.018 | <= 2.0 |

| Case | jq peak RSS | tq peak RSS | tq / jq | Goal |
| --- | ---: | ---: | ---: | ---: |
| Early label break | 4,227,072 bytes | 4,030,464 bytes | 0.953 | <= 1.5 |
| Bounded recurse | 3,981,312 bytes | 4,554,752 bytes | 1.144 | <= 1.5 |
| Structural walk | 3,538,944 bytes | 3,424,256 bytes | 0.968 | <= 1.5 |

All measured wall-time and peak-RSS goals pass, so no profiling follow-up is
required.
