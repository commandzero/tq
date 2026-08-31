# Performance review policy

Keep corpus files, generated formats, full sample collections, and reviewed
reports in the separate `commandzero/tq-benchmarks` checkout. The campaign
runner writes to its `.work/` directory there when it discovers the sibling
checkout. Set `TQ_BENCHMARK_ARCHIVE_ROOT` to select another archive location.

After review, add one concise `YYYY-MM-DD.md` report to the archive checkout.
Record the input size, tool versions, commands, timing, peak RSS, comparison
method, and important failures. Leave out the full corpus and per-sample data
from the report itself.

Run all benchmark and baseline commands outside restricted sandboxes with
elevated permission to inspect child processes. On macOS, collect every
authoritative peak RSS sample with `/usr/bin/time -l` and record the reported
`maximum resident set size`. Process-group sampling may supplement this value,
but it cannot replace it. Discard and rerun any campaign whose sandbox blocks
process inspection or leaves macOS RSS unavailable.

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

## jq-relative soft objective

Issue #5 workloads report an informational comparison against jq on the same
JSON input and recorded host. The target is at most 2.0 times jq's median wall
time and at most 1.5 times jq's maximum observed peak RSS. The report labels
each metric `met`, `missed`, or `not-comparable`. A miss remains visible but
does not fail the campaign or replace tq's self-regression gate.

The `inputs` workload uses a reviewed deterministic 65,536-document corpus
with separately identified JSON, YAML, and TOON sequence artifacts; natural
benchmark sources are never repeated or resized to construct that workload.
