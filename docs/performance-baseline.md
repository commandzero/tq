# Performance review policy

Keep corpus files, generated formats, full sample collections, and reviewed
reports in the separate `commandzero/tq-benchmarks` checkout. The campaign
runner writes to its `.work/` directory there when it discovers the sibling
checkout. Set `TQ_BENCHMARK_ARCHIVE_ROOT` to select another archive location.

After review, add one concise `YYYY-MM-DD.md` report to the archive checkout.
Record the input size, tool versions, commands, timing, peak RSS, comparison
method, and important failures. Leave out the full corpus and per-sample data
from the report itself.

The current reviewed reports are kept in the `commandzero/tq-benchmarks`
repository alongside their raw campaign outputs.

## Self-regression policy

The local tq-only defaults are:

- Median wall time may increase by at most 50%.
- Peak RSS may increase by at most 20%.
- A row needs at least five measured samples before it can fail the gate.

Run self-regression checks against JSON reports in the archive checkout's
`.work/` directory:

```console
export TQ_BENCHMARK_ARCHIVE_ROOT=/path/to/tq-benchmarks
TQ_BIN="$PWD/target/release/tq" cargo run -p tq-test-support --bin tq-bench --release -- \
  run --profile standard --origin frozen --manifest PATH \
  --output "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/candidate.json" \
  --baseline "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/accepted.json" \
  --wall-regression-percent 50 --rss-regression-percent 20 \
  --minimum-regression-samples 5
```

The gate skips comparisons when the profile, machine, corpus artifact, or tool
identity differs. A reference-tool change is metadata, not a regression.
