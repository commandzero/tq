---
type: Guide
title: "Event-stream microbenchmarks"
description: "In-process attribution, correctness checks, and reproducible Criterion comparisons."
generated: { by: codex/gpt-6, at: 2026-09-16T03:37:48Z }
---

# Event-stream microbenchmarks

These developer benchmarks implement the event-stream slice of issue #32 to
support investigation of issue #48. They measure in-process operations.
Native `tq-bench` remains the source of spawn-to-exit timing and peak RSS.

## Run the suite

Run commands from the repository root with the pinned toolchain.
If a Homebrew compiler precedes rustup's shims, select the toolchain directory
explicitly so child processes use the same compiler:

```sh
toolchain_bin=$(dirname "$(rustup which cargo)")
export PATH="$toolchain_bin:$PATH"
```

```sh
# Optimized build and correctness smoke, also called by full preflight and CI.
./scripts/microbench-smoke.sh

# All 50 cases, 30 samples each, 1 second warmup and 3 second measurement target.
cargo bench --locked --bench event_stream \
  -p tq-core -p tq-toon -p tq-formats -p tq-cli

# Focused compiled execution comparison.
cargo bench --locked -p tq-core --bench event_stream -- \
  'event-stream/execute-events/mixed' --save-baseline before
cargo bench --locked -p tq-core --bench event_stream -- \
  'event-stream/execute-events/mixed' --baseline before
```

Criterion may increase measurement time to fit the requested samples. Shared CI
uses test mode only; timing differences never determine its pass/fail result.
Fixtures and correctness checks run before measured loops, including filtered runs.
Run the complete smoke suite before collecting a baseline.

## Inventory and timing boundaries

The query is `select(length == 2 and (.[0] | length) == 1)`.
Stable IDs start with `event-stream/<stage>/<format-or-record-class>/<shape>/<size>`.
Runner IDs include both input and output formats.

| Stage | Crate and public boundary | Included | Excluded | Throughput |
| --- | --- | --- | --- | --- |
| `decode-events` JSON | `tq-formats::decode_json_event_stream` | Fresh decoder, parsing, owned events, consuming callback, destruction | Input generation and file I/O | Input bytes |
| `decode-events` TOON | `tq-toon::Decoder::decode_into` | Fresh decoder, parsing, owned events, consuming callback, destruction | Input generation and file I/O | Input bytes |
| `stream-records` | `tq-formats::stream_json` and `stream_toon` | Parsing, path projection, record allocation, consumption and destruction | Query evaluation | jq stream records |
| `execute-events` | `tq-core::Vm::new_events` and `for_each_result` | Input handle clone, VM creation, synchronous evaluation, effect drain, result/VM destruction | Query compilation and record preparation | jq stream records |
| `encode-results` | `tq-formats::NativeOutputSequence` | Output selection, sequence creation, result write, finish, byte sink, destruction | Query evaluation | Output bytes |
| `runner` | `tq-cli::run_with_io` | Command clone, preparation, compilation, decode, execution, framing, encoding, destruction | Argument parsing, process startup and file I/O | Input bytes |

The core target contains accepted-leaf, rejected-leaf, container-end and mixed
cases. The primary measurement includes setup and destruction because production
creates a VM for every record. It uses synchronous consumption, as the runner does.
It does not benchmark the pull interface's worker-thread path.

The runner includes its effect drain and observations handling. The isolated core
case does not include runner observation aggregation or cancellation lookup.
Stage measurements overlap and use different consumers. Do not subtract or add
their medians to claim an exact breakdown of combined time.

The bounded output writer consumes every byte and retains only a byte count and
checksum. Its cost is included. Exact output checks use a separate buffer before
timing. Optimization barriers consume timed events, results, and sink state.

## Fixtures and correctness

Both shapes contain 16 or 1024 objects below `items` and one top-level `tail: 7`.
Nested items contain `id` and `meta.flag`; tabular items contain `id` and `flag`.
TOON inputs use nested lists or tabular rows. These are synthetic fixtures, not
modified natural campaign corpora, and require no downloads.

| Shape | Items | JSON bytes | TOON bytes | jq records | Selected results |
| --- | ---: | ---: | ---: | ---: | ---: |
| nested | 16 | 506 | 617 | 67 | 1 |
| nested | 1024 | 32702 | 39871 | 4099 | 1 |
| tabular | 16 | 362 | 178 | 51 | 1 |
| tabular | 1024 | 23486 | 11208 | 3075 | 1 |

Independent model traversal defines ordered structural and jq stream records.
A literal 1-item oracle checks the stream-record traversal. All decoder output
is compared to these expectations before timing. Structural comparisons normalize
source spans, key quoting, and declared array lengths, which differ by format;
they preserve keys, values, boundaries, order, and observed array counts.

The query emits exactly `[["tail"],7]`. Literal expected JSON and TOON bytes check
encoding and runner output. The runner explicitly selects compact JSON or unframed TOON.
Unframed TOON is valid because there is 1 result.
The encoding group has 1 case per output format, since its selected result is
identical across all input sizes. Accepted-only execution cases likewise contain
1 event; their size suffix identifies the source fixture, not event volume.

Byte throughput uses the actual representation size. Event throughput counts
every leaf and container-end record. Criterion measures time per whole case
iteration, not per event; use the reported throughput for event rates.

## Dependency and build policy

Criterion 0.8.2 is pinned as a development dependency. Its declared MSRV is 1.86,
below tq's 1.95 minimum. Only `cargo_bench_support` is enabled; Criterion's default
Rayon and Plotters features are disabled. Statistical/reporting dependencies remain
development-only. The production dependency graph does not include Criterion.

Explicit targets use `harness = false`. Release and bench builds use Cargo's default
codegen-unit and LTO settings. Bench builds retain line-table debug information.
The existing test-support crate has no new benchmark group or production dependency.
Fixture modules live under `benchmarks/micro` and are compiled only by benchmark
targets; they do not create a new workspace crate or release API.

## Historical comparison

Compare equivalent source in isolated baseline and candidate trees with separate
target directories. Run both on the same quiet host, compiler, target, profile,
and sampling settings. Keep each revision's production dependency resolutions.
Record any development lockfile additions and all production dependency differences.

The required public boundaries exist on v0.3.0. Its VM has no effect sink, so the
historical adapter removes only the 2 `take_effects` calls from the benchmark.
This represents each revision's production lifecycle; it does not isolate effect
draining as the cause of a difference. No production source patch is allowed.
Record any further adapter and mark mismatched boundaries non-comparable.

Retain the benchmark source and adapter patch, fixture identity, source revisions,
dirty diff, lockfiles, compiler/target/profile, OS/CPU, commands, sample settings,
raw Criterion artifacts, dispersion and confidence intervals. Use explicit
baseline names and preserve output directories before subsequent runs.
Record at least 2 runs per revision to expose between-run variation.

Criterion's baseline command expects both datasets in its output directory.
For isolated trees, compare retained raw artifacts by stable ID or copy the saved
baseline directories into the candidate output tree without overwriting current
samples. Never compare cases with different fixture or boundary identities.

Nanosecond display does not establish nanosecond accuracy. Results from another
host do not establish a regression. Report measured stage changes separately from
root-cause hypotheses. Keep developer evidence outside cross-tool comparison pages.

## Add a case

1. Choose an existing public boundary and define exactly what one iteration includes.
2. Add a deterministic bounded fixture and an independent ordered-result oracle.
3. Check it before timing and consume timed outputs without unbounded retention.
4. Assign a stable ID and units, then run smoke, formatting, and Clippy.
5. Invalidate old baselines if the fixture or measured boundary changes.

## Native follow-up

Completing this suite does not resolve issue #48. A later optimization needs
repeated native JSON and TOON campaigns against v0.3.0 on the same frozen corpus
with comparable explicit output framing. Keep wall-time and peak-RSS deltas
separate and apply the existing disclosure and acceptance thresholds.

## References

1. [Contributor checks](../contributor-checks.md)
2. [Native benchmark harness](benchmark-harness.md)
3. [Criterion issue #32](https://github.com/commandzero/tq/issues/32)
4. [Event-stream regression #48](https://github.com/commandzero/tq/issues/48)
5. [Criterion timing loops](https://docs.rs/criterion/0.8.2/criterion/struct.Bencher.html)
