# Performance review policy

Keep corpus files, generated formats, and full sample collections in
`benchmarks/.work/`. Git ignores that directory. Do not commit these files.

After review, add one concise `YYYY-MM-DD.md` report to `benchmarks/`. Record the
input size, tool versions, commands, timing, peak RSS, comparison method, and
important failures. Leave out the full corpus and per-sample data.

The current reviewed report is
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

The gate skips comparisons when the profile, machine, corpus artifact, or tool
identity differs. A reference-tool change is metadata, not a regression.
