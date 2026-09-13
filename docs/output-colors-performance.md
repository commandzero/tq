---
type: Report
title: Output color performance
description: Release measurements of monochrome and colored document and transcode output.
generated: { by: codex, at: 2026-09-13T21:06:06Z }
---

# Output color performance

Color preserves the measured outputs after removing generated SGR. It is not
free: on the month corpus, JSON identity takes 2.43× as long and TOON identity
takes 3.46× as long. TOON sequence identity takes 3.80× as long.

The month transcode paths stay near 15 MiB peak RSS with color. JSON identity
uses 103.9 MiB versus 77.3 MiB plain because its existing document buffer holds
the larger decorated serialization. No additional whole-result color buffer
was introduced.

## Results

Wall time and process peak RSS are medians of 5 samples after 2 warmups.
Each plain/color pair first passed exact stripped-byte comparison. All 80 timed
samples exited successfully and reported positive native RSS.

| Corpus | Case | Plain ms | Color ms | Time ratio | Plain RSS MiB | Color RSS MiB |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Week | JSON identity | 37.2 | 69.7 | 1.88× | 20.3 | 25.2 |
| Week | JSON projection | 80.5 | 84.4 | 1.05× | 6.8 | 6.8 |
| Week | TOON identity | 94.4 | 124.2 | 1.32× | 8.0 | 11.2 |
| Week | TOON sequence identity | 86.5 | 117.2 | 1.35× | 8.0 | 11.3 |
| Month | JSON identity | 154.1 | 373.8 | 2.43× | 77.3 | 103.9 |
| Month | JSON projection | 440.9 | 431.5 | 0.98× | 11.8 | 11.9 |
| Month | TOON identity | 1,199.5 | 4,156.4 | 3.46× | 14.5 | 14.9 |
| Month | TOON sequence identity | 1,008.2 | 3,828.8 | 3.80× | 14.6 | 15.1 |

The month projection ranges overlap: 422.9–464.2 ms plain and 429.8–535.8 ms
colored. Its lower colored median is not evidence that coloring improves speed.
The largest observed colored RSS was 15.14 MiB for TOON sequence identity and
103.94 MiB for JSON identity.

Exact stdout bytes include SGR and framing.

| Corpus | Case | Plain bytes | Color bytes | Byte ratio |
| --- | --- | ---: | ---: | ---: |
| Week | JSON identity | 2,419,795 | 8,050,332 | 3.33× |
| Week | JSON projection | 185,194 | 748,119 | 4.04× |
| Week | TOON identity | 1,872,399 | 4,737,316 | 2.53× |
| Week | TOON sequence identity | 1,872,400 | 4,737,317 | 2.53× |
| Month | JSON identity | 12,263,577 | 40,789,404 | 3.33× |
| Month | JSON projection | 937,004 | 3,789,326 | 4.04× |
| Month | TOON identity | 9,490,993 | 24,012,808 | 2.53× |
| Month | TOON sequence identity | 9,490,994 | 24,012,809 | 2.53× |

## Memory interpretation

The corpus grows 5.07× from week to month. Transcode color overhead over plain
RSS does not grow with decorated output: it falls from about 3.2 MiB to
0.4–0.5 MiB. This is consistent with the existing 8 MiB preparation budget
and spill-to-disk path, not an unbounded colored-output buffer.

Projection emits 2,225 and 11,274 completed results. Color adds at most 0.11 MiB
to median RSS, although output grows from 0.75 MB to 3.79 MB. Total process RSS
is not constant across these inputs; this comparison isolates the small extra
cost of presentation rather than claiming the existing input plan has no state.

Code and limit tests complement these finite measurements. The presentation
sink uses a 4 KiB chunk for the default palette; custom styles can allocate in
proportion to the style, not the result. JSON keeps its existing serialization
buffer. TOON reuses bounded preparation and spool accounting, including SGR
bytes. Color preserves the document/subtree/transcode plan selection.

The extra bytes and token writes are a real cost for bulk terminal output.
These runs do not establish a universal overhead bound, a parent-commit
regression result, or a jq/yq speed comparison. Redirected automatic output
remains plain; `-M` explicitly selects plain output.

## Method

Measured on 2026-09-13 with an Apple M4 Pro, 14 logical CPUs, 48 GiB RAM,
macOS 26.6.2 build 25G83, Darwin 25.6.0, arm64. The binary is release-built
tq 0.3.0 with thin LTO and 1 codegen unit. Release compilation and the full
preflight finished before timing began.

The cached USGS week input contains 1,580,876 bytes and 2,225 records; month
contains 8,013,185 bytes and 11,274 records. Both original JSON files were
rehashed against their admitted manifests. No corpus was resized or refreshed.

Each invocation uses strict JSON input by filename, `-M` or `-C`, and the
default palette with `JQ_COLORS` and `NO_COLOR` unset. Identity uses `.` with
`-o json`, `-o toon`, or `-o toon-seq`. Projection uses `-o json` and:

```jq
.features[] | {id, mag: .properties.mag, place: .properties.place}
```

The existing isolated benchmark worker collects `darwin-wait4` process RSS,
CPU, bounded stdout/stderr, and spawn-to-exit wall time. Its native allocation
preflight passed before each corpus. No recurring process-group sampler ran.
The coordinator supplies empty stdin; the workload reads the explicit corpus
filename. Each invocation has a 120-second timeout and 512 MiB capture limit.
Cases and corpora ran sequentially, plain before color.

An independent macOS `/usr/bin/time -l` control for colored month TOON identity
reported 15,745,024 bytes peak RSS, consistent with native samples near 15 MiB.
Its 3.64-second wall time is not included in the table because stdout went to
`/dev/null` instead of the benchmark capture pipe.

Raw reports, the measurement driver, the time control, compatibility evidence,
and the preflight log are retained in the separate `tq-benchmarks` checkout at
`.work/output-colors-20260913/`. The principal reports are `week.json` and
`month.json`. The driver uses the repository's production resource collector;
it does not replace it with shell timing.

Measurements were built at `ea83d71` before the parent PR was concurrently
restacked. Replaying the implementation onto that stack changed no files in
`tq-cli/src`, `tq-core`, `tq-formats`, or `tq-toon`. The measured runtime code
therefore matches the delivered runtime. The incoming stack did change the
benchmark collector; these samples retain their original collector identity
and do not claim to measure that newer collector.

Recorded collector source SHA-256:

```text
19064656b75953c4d0da8e670b4a212a8a3743d90faa490bb2674e76551d2dc5
```

Binary SHA-256:

```text
a943425168c26e7fb5aa397be3d3bb0ad0bf11dc45a87b78bc59799a4a0d8ddd
```

Week and month input SHA-256, respectively:

```text
00a7b7382678bd96255bffb69254dbc7c121181d704b91303d6eac2e0be8fe6c
459af9ee1e8b067d8e3602454ff2c7f47baf60f3575d3915143855c333f923d2
```

## Validation

`RUST_TEST_THREADS=1 ./scripts/preflight.sh` passed with elevated process
permissions after restacking: formatting, workspace check, strict Clippy,
1,528 tests, OKF docs,
and all 22 OpenSpec items. The 10 ignored tests retain their explicit opt-in
requirements. An earlier concurrent run exceeded deadlines in 2 inherited
worker timing tests; both passed in the serialized run without worker changes.

The [compatibility review](../tests/compatibility/reviews/output-colors.md)
records 29 successful observations and distinguishes ANSI differences from
existing delimited-format differences.
