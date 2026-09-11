# Native worker proof results

## Result

The worker proof is not accepted. Two unchanged release runs on each native
host produced 480 paired observations. All RSS comparisons passed, but 11 CPU
comparisons exceeded the existing tolerance. No tolerance was widened, failed
pair dropped, or passing rerun substituted for a failed run.

Standard repository preflight passed after adding this opt-in test, including
formatting, compilation, strict all-feature Clippy, workspace tests,
documentation checks, and all 20 OpenSpec items. Native proof tests are
explicitly ignored by ordinary preflight and were executed separately here;
their failures remain acceptance blockers.

Each run used 20 repetitions of six cases: no-op targets with 0, 32 MiB, and
128 MiB retained in the coordinator, 32 MiB prepared stdin, a 32 MiB target
allocation, and a following low-memory target. Independent controls used BSD
`time -l` on macOS and GNU `time -v` on Linux, under elevated permissions.
Both stdin paths used prepared files and file captures. The echo bytes were
checked outside measurement, with the independent output digest retained.

The table counts independent pairs. A passing RSS comparison does not imply
that its CPU comparison passed or authorize campaign publication.

| Host and run | Pairs | RSS passed | CPU passed | CPU failed |
| --- | ---: | ---: | ---: | ---: |
| macOS first | 120 | 120 | 118 | 2 |
| macOS repeat | 120 | 120 | 119 | 1 |
| Linux first | 120 | 120 | 116 | 4 |
| Linux repeat | 120 | 120 | 116 | 4 |

## Failure scope

macOS failures occurred in system CPU for large stdin. One repeat observation
recorded 20,141 microseconds natively against the time tool's `0.01` seconds,
exceeding the 10,000-microsecond absolute tolerance by 141 microseconds.
Linux failures occurred in system CPU for the high-allocation target. The
first run included 10,393 microseconds natively against GNU time's `0.00`
seconds. RSS agreed in those pairs.

The raw time output has two decimal places for CPU seconds. Quantization and
ordinary differences between independent executions are plausible contributors,
not a verified diagnosis. The unchanged repeat reproduced failures on both
hosts. An accounting-scope error has not been excluded. Passing RSS checks
cannot override this failed CPU gate.

Task 1.4 remains open. Resolve the CPU validation discrepancy without silently
loosening the gate before adopting the worker. Coordinator-loss cleanup and
sampled RSS support remain separate implementation work in task 2.7. No full
matrix or baseline/candidate acceptance comparison has run with this worker.

## Retained evidence

Build before collecting measurements, then run outside the sandbox on each
native host. Set the worker path explicitly for nonstandard Cargo output
layouts. This command intentionally exits nonzero when any proof gate fails:

```sh
cargo test --release --locked -p tq-test-support --test benchmark_native_validation --no-run
TQ_BENCH_WORKER=/absolute/path/to/release/tq-bench-worker \
TQ_NATIVE_VALIDATION_OUT=/absolute/path/to/evidence \
TQ_NATIVE_VALIDATION_REPETITIONS=20 \
cargo test --release --locked -p tq-test-support --test benchmark_native_validation -- \
  --ignored --exact worker_isolation_validation_writes_retained_evidence --nocapture
```

The collector-source fingerprint in all four runs is
`29a2e0a6ecf17daa3c40c218e2fe7e5f996b1be4004c503d2677686741be482f`.
Every summary includes host, build, worker, probe, and time-tool identities;
every record retains its paired resource values and raw time-file references.
Raw filenames include coordinator allocation size to prevent collisions.

Paths below are relative to the external benchmark archive's `.work/` directory.

- macOS first: `native-worker-validation/worker-isolation-1789138950045755-64459/`.
  Summary SHA-256 `2346c65c6f56f97e9a4c75898e4056c3eb40350f8c598d195ef673b535d81ee2`.
- macOS repeat: `native-worker-validation/worker-isolation-1789139063395353-65465/`.
  Summary SHA-256 `0b9faccad5031fb434ef41169408e8ff4a540c4d924a0406bc3c6bc063fd8998`.
- Linux first: `native-worker-validation-linux/worker-isolation-1789138962460893-2258236/`.
  Summary SHA-256 `ea269da15fe718618e197ed4d822a3e2b5c6577d9197f06a186a09bc1e172658`.
- Linux repeat: `native-worker-validation-linux/worker-isolation-1789139064668946-2260640/`.
  Summary SHA-256 `46a6a10e2787cab0a07a405a8ec65afdb0909dfa6380764e0c8d3c7fd670df69`.

Linux originals remain in the authorized scratch directory's
`tmp/native-worker-validation/`. No source or benchmark data was published.
