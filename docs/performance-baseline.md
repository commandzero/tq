# MVP performance baseline review

The accepted local baseline is evidence, not a claim that tq wins every row.
All reports below were collected on machine identity
`08b8ce91ba80003df6ac9fb3f9dc39685746dff93f169c1aa39af744e1ea5ba7`.
Direct regression comparisons additionally require identical corpus and tool
manifests; reports from another host or refreshed snapshot remain informative.

## Standard campaign

`baselines/2026-08-01/performance/tq-standard-mvp-v1.json` contains 288 rows
over four natural USGS snapshots and all JSON, YAML, and TOON representations.
All 276 applicable rows passed their ordered semantic correctness gate and
timed; 12 YAML/event-stream rows are explicitly unsupported.

The one-sample campaign is broad release evidence, not a statistical
regression baseline. On the natural 7.7 MB JSON month feed:

- identity/re-encode took about 177 ms for tq JSON, 316 ms for tq YAML, and
  200 ms for tq TOON, versus 305 ms for jq JSON and 627 ms for yq YAML;
- parse/discard took about 115 ms, 242 ms, and 137 ms for tq JSON, YAML, and
  TOON, respectively, versus 88 ms for jq JSON and 322 ms for yq YAML;
- the explicit stream took 2.70 s for tq JSON and 0.48 s for tq TOON versus
  0.30 s for jq JSON, but tq produced its first byte in roughly 10 ms/7 ms
  while jq did so at 271 ms;
- object construction exposed an unfavorable jq comparison: tq JSON was
  about 6.0 times jq, although it was far faster than the equivalent yq rows.

Startup-dominated small rows and single observations are not used to infer a
winner. The full JSON report preserves CPU, first-result, throughput, output,
command, and unavailable-RSS fields.

## Natural-large campaign

The frozen Microsoft US building-footprint snapshot contains 3,981,792
features. Its natural files are 1,119,571,788 bytes of JSON/YAML and
1,608,222,424 bytes of TOON; they were not padded, sliced, or repeated.

`tq-large-parse-discard-mvp-v1.json` is the current jq 1.8.2 / yq / tq
correctness-gated parser campaign over all three representations. Every
applicable row timed successfully: jq JSON took 14.1 s, yq JSON/YAML took
82.8 s/209.9 s, and tq JSON/YAML/TOON took 19.2 s/32.0 s/20.7 s. It is a
low-output workload by design, so these measurements do not recreate the
previous multi-gigabyte correctness-capture failure.

The correctness-gated explicit-stream report
`large-event-stream-v1.json` records:

| Adapter | Wall time | First result | Physical throughput | Comparison |
| --- | ---: | ---: | ---: | --- |
| jq JSON | 59.7 s | 59.7 s | 17.9 MiB/s | reference |
| tq JSON | 464.4 s | 6.5 ms | 2.3 MiB/s | 7.8x jq wall time |
| tq TOON | 126.4 s | 16.7 ms | 12.1 MiB/s | 2.1x jq wall time |

Because the restricted harness could not read RSS, exact equivalent release
commands were remeasured with macOS `/usr/bin/time -p -l` and archived in
`large-event-stream-rss.json`. tq JSON peaked at 3,309,568 bytes and tq TOON
at 3,522,560 bytes: 2.5% and 2.6% of the 128 MiB release objective. The result
demonstrates bounded memory and early output, while also showing substantial
JSON throughput work remains.

The earlier all-workload large attempt ended with exit 137 before it could
atomically write a report. That outcome remains in `large-full-attempt.json`;
it is attributed to the benchmark harness's unbounded correctness capture for
multi-gigabyte output, not silently discarded or labeled as a tq timing. The
release uses low-output correctness-gated rows plus the explicit event stream
for safe natural-large coverage, while output-heavy large harness redesign is
separate follow-up work.

## Accepted self-regression policy

The first stable local baseline establishes these tq-only defaults:

- median wall time may increase by at most 50%;
- peak RSS may increase by at most 20%;
- a row needs at least five measured samples before it can fail the gate.

These accommodate local scheduler noise without making jq/yq ratios into tq
pass/fail thresholds. The initial five-sample `parse-discard` replay had one
45.1% YAML/USGS-week median swing while all other tq rows stayed within 5%, so
the 50% wall bound is the smallest rounded bound that accepts observed local
noise. The accepted baseline and its comparable passing replay are
`tq-standard-parse-discard-stable-v1.json` and
`tq-standard-parse-discard-regression-v1.json`. `tq-bench` stores the policy
and manifest comparison in the candidate report:

```console
TQ_BIN="$PWD/target/release/tq" cargo run -p tq-test-support --bin tq-bench --release -- \
  run --profile standard --origin frozen --manifest PATH \
  --output reports/local/candidate.json \
  --baseline baselines/local/accepted.json \
  --wall-regression-percent 50 --rss-regression-percent 20 \
  --minimum-regression-samples 5
```

The gate is not evaluated when profile, machine, corpus artifacts, or tool
identities differ. A comparable candidate with any tq row beyond threshold is
reported as `regression`; reference-tool changes remain comparison metadata.
