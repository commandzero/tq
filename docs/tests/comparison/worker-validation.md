---
type: Report
title: "Linux worker-validation results"
description: "Native Linux CPU and RSS controls supporting the published benchmark campaign."
generated: { by: codex, at: 2026-09-11T20:18:00Z }
---

# Linux worker-validation results

## What this measures

These controls validate native target CPU and peak RSS collection through
the isolated Rust worker. They do not measure jq, yq, or tq query performance.
The release validation run contains 420 records: 21 controls, each repeated
20 times. The separate worker-isolation run contains 120 records: 60
coordinator-memory controls, 20 prepared-stdin controls, and 40 high/low
request-sequence controls.

## Host specifications

The recorded host is Linux x86_64 with an AMD Ryzen 7 7700 8-Core Processor,
16 logical CPUs, 61.9 GiB RAM, and 4.0 KiB pages. The recorded kernel was
`Linux 7.2.0-ogc4.1.fc44.x86_64 #1 SMP PREEMPT_DYNAMIC Thu Aug 20 16:15:37 UTC 2026`.
The release helpers used `rustc 1.100.0-nightly (fd7ed57df 2026-08-29)`.
Both runs used 20 repetitions and were captured on 2026-09-11 under
elevated permissions.

## Results

All 420 main validation records passed RSS and CPU comparison gates. All 120
worker-isolation records passed RSS comparison. Their 240 CPU metric
comparisons (user and system for each record) were automatically green. No
comparison required an informational notice, approval, or investigation.

| Validation group | Cases | Records | RSS passed | CPU records green |
| --- | --- | ---: | ---: | ---: |
| No-op and ordering | `noop`, `noop-after` | 40 | 40 | 40 |
| Allocation and concurrency | 6 allocation controls | 120 | 120 | 120 |
| Known duration, no sampler | 20/100/250 ms | 60 | 60 | 60 |
| Sampler variant | no-op plus 20/100/250 ms | 80 | 80 | 80 |
| Busy-deadline controls, no sampler | 20/100/250 ms | 60 | 60 | 60 |
| Busy-deadline controls, sampler | 20/100/250 ms | 60 | 60 | 60 |
| Main validation total | 21 controls | 420 | 420 | 420 |

| Worker-isolation control | Records | RSS passed | CPU records green |
| --- | ---: | ---: | ---: |
| Coordinator no-op, 0 MiB | 20 | 20 | 20 |
| Coordinator no-op, 32 MiB | 20 | 20 | 20 |
| Coordinator no-op, 128 MiB | 20 | 20 | 20 |
| Prepared stdin, 32 MiB | 20 | 20 | 20 |
| High request | 20 | 20 | 20 |
| Low request after high | 20 | 20 | 20 |
| Worker-isolation total | 120 | 120 | 120 |

## Method and limits

The worker owns target launch, target exact-child accounting, and cleanup.
Worker startup, communication, and teardown are outside the native target wall
interval. Ordinary controls use the worker without sampled RSS enforcement.
Sampler controls use an explicit worker process-group RSS sampler and have a
separate calibration linkage.

Independent GNU `/usr/bin/time -v` is used only as a paired Linux validation
control for CPU and RSS scope. It is not used to measure the benchmark
workload, and its output is not substituted into the published measurement
rows. Prepared input and output captures use files; target payload checks occur
outside the measured interval.

CPU differences are divided by native target wall duration. At most 10% is
automatic green; above 10% through 20% is green with info; above 20% but below
50% requires approval; 50% or more blocks acceptance and requires
investigation. Below 500 ms, a difference strictly below the greater of 20 ms
and 10% of runtime overrides these bands to automatic green. Decisions use
unrounded values.

RSS comparisons use the greater of four host pages and 25% of the larger
paired value. Coordinator-isolation tolerances use the greater of eight pages
and 25% of the larger median. The retained 0/32/128 MiB controls show no
material coordinator-memory effect within those bounds; their residual
worker launch RSS floor was 2.5 MiB. Lifetime RSS includes
launch effects; it is not a pure post-exec memory measurement.

The largest observed control excess durations were 1.1 ms without sampling
and 1.0 ms with sampling. These subtract the requested control interval from
full process runtime, which also includes startup and scheduling. They are not
timer-error bounds. Busy-deadline controls had a median full-runtime/child
interval difference of 0.8 ms in each mode. No estimated overhead is subtracted
from benchmark samples. Tool comparisons use the same host, OS, harness and
disclosed concessions; results are not compared across operating systems.

The native worker used launch protocol `tq-bench-worker-protocol-v3` and
collector-source SHA-256
`9527c5326c87782e237f448ae91eb93ee479c0e4ebcb6f039ad04cdd51e03bf5`.
The calibration summary SHA-256 is
`d3048417f44817ca38f6ae2b0f818f8a0c926afc8c9f1dc123d13e54e3efcad6`.
These identities match the published Linux campaigns. Later platform fixes
and their controls do not relabel these measurements. Raw records and
calibration artifacts remain retained in the external benchmark archive.

The [comparison index](index.md) reports Linux workload measurements. These
validation controls remain separate from workload samples.
