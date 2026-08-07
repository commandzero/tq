# Performance review policy

Benchmark corpus files, generated formats, and full sample collections are
local working data. They stay in `benchmarks/.work/` and Git ignores them.
Do not add them to the repository.

After review, add one concise `YYYY-MM-DD.md` Markdown artifact in `benchmarks/`.
It must state the input size, tool versions, commands under review, timing,
peak RSS, comparison method, and important failures. It must not include the
full corpus or per-sample collection.

The current reviewed artifact is
[2026-08-06.md](../benchmarks/2026-08-06.md).

## Self-regression policy

The local tq-only defaults are:

- Median wall time may increase by at most 50%.
- Peak RSS may increase by at most 20%.
- A row needs at least five measured samples before it can fail the gate.

Run self-regression checks against an ignored local JSON report. Do not commit
the report:

```console
TQ_BIN="$PWD/target/release/tq" cargo run -p tq-test-support --bin tq-bench --release -- \
  run --profile standard --origin frozen --manifest PATH \
  --output benchmarks/.work/candidate.json \
  --baseline benchmarks/.work/accepted.json \
  --wall-regression-percent 50 --rss-regression-percent 20 \
  --minimum-regression-samples 5
```

The gate is not evaluated when the profile, machine, corpus artifact, or tool
identity differs. Reference-tool changes remain comparison metadata.
