# Native worker proof results

## Published Linux campaign linkage

The published Linux workload evidence remains the frozen issue #30 campaign,
with collector-source SHA-256
`9527c5326c87782e237f448ae91eb93ee479c0e4ebcb6f039ad04cdd51e03bf5`.
Its self-regression gate accepted 410 comparable standard pairs with no
disclosures or blocking increases. The separate supplemental smoke review
accepted nine pairs. Those workload results remain linked to their retained
controls; the poll-fix controls below use a different collector source and
must not be substituted into those published rows. See the
[comparison index](../../../../docs/tests/comparison/index.md),
[Linux regression review](linux-regression-review.md) and
[smoke regression review](smoke-regression-review.md).

The retained Linux controls supporting the published campaign are:

- `native-verification-20260911/linux/controls/1789163444127543-2665846/`,
  summary SHA-256
  `d3048417f44817ca38f6ae2b0f818f8a0c926afc8c9f1dc123d13e54e3efcad6`.
- `native-verification-20260911/linux/controls/worker-isolation-1789163506430209-2665846/`,
  summary SHA-256
  `0025f59251ec4bce833a0d54dfd89aa4f55ad0743efd9c9327e3aaf31c3394e7`.

## Poll-fix control result

The post-poll-fix release controls pass on both native hosts. Each main
validation bundle contains 420 rows: 21 control cases at 20 repetitions each.
Each paired worker-isolation bundle contains 120 rows: 60 noop, 20 prepared
stdin, 20 high-allocation, and 20 low-allocation request-sequence rows.

Every main row has one RSS comparison and two CPU metric comparisons. Every
worker-isolation row has the same three checks. All RSS checks passed and all
CPU checks were automatically green, with no informational, warning or blocking
results. All four bundles report no validation failures.

| Host | Main rows | Main RSS / CPU metrics | Worker-isolation rows | Worker RSS / CPU metrics |
| --- | ---: | ---: | ---: | ---: |
| Linux x86_64 | 420 | 420/420; 840/840 | 120 | 120/120; 240/240 |
| macOS aarch64 | 420 | 420/420; 840/840 | 120 | 120/120; 240/240 |

The worker-isolation records independently retain one worker identity,
`tq-bench-worker-protocol-v3`, and one collector-source identity per host.
Linux records use `linux-wait4` RSS provenance; macOS records use
`darwin-wait4`. The residual floors are retained, not subtracted.

The 22 worker integration tests and two actual lifecycle failure-injection
tests pass on both native hosts. They cover delayed startup and communication,
malformed replies, non-reading workers, coordinator loss, worker loss,
descendant cleanup and bounded termination. The lifecycle suite also runs
three helper entries.

## Host specifications and timing controls

| Host | CPU / logical CPUs | Memory | Page size | Kernel | Residual floor |
| --- | --- | ---: | ---: | --- | ---: |
| Linux x86_64 | AMD Ryzen 7 7700 8-Core Processor / 16 | 61.9 GiB | 4.0 KiB | `7.2.0-ogc4.1.fc44.x86_64` | 2.5 MiB |
| macOS aarch64 | Apple M1 Max / 10 | 32.0 GiB | 16.0 KiB | `Darwin 25.6.0`, `RELEASE_ARM64_T6000` | 5.8 MiB |

All controls used 20 repetitions and release helpers built with the pinned
nightly-2026-08-30 toolchain. The macOS version was 26.6.2, build 25G83.
Known-duration controls were run at 20, 100 and 250 ms. The largest observed
control excess durations were:

| Host | No sampler | Explicit process-group sampler | Child deadline overrun, no sampler / sampler | Median native-wall minus child elapsed, no sampler / sampler |
| --- | ---: | ---: | ---: | ---: |
| Linux x86_64 | 1.0 ms | 1.0 ms | 3.0 µs / 3.0 µs | 0.8 ms / 0.8 ms |
| macOS aarch64 | 23.5 ms | 30.4 ms | 2.0 µs / 4.0 µs | 7.6 ms / 9.4 ms |

The observed excess includes target launch, sleep scheduling, target shutdown
and exit-observation delay. Worker cleanup remains outside the measured
interval. These observations are not isolated timer error or a guaranteed
error bound. No startup constant is subtracted from workload samples.
The busy-deadline child measures its own controlled interval separately.
Median wall-minus-child values above use the retained aggregate across the
three busy-deadline durations, not the largest individual case median.
No-op wall medians and MADs were 728.0 µs and 13.0 µs on Linux, and
6,989.0 µs and 1,737.0 µs on macOS.

Comparisons use the same host and OS with equivalent measurement settings and
concessions. No universal 1 ms guarantee is required. Repetitions and
dispersion remain necessary because scheduling noise does not cancel exactly.

## Poll-fix identities and retained evidence

The poll-fix controls use collector-source SHA-256
`51e062e1eca0d34baa4f43ef1fc7e667adde20378e2c9309d077a64e6b93fbdf`.

### Linux

- Main controls: `native-verification-20260911/linux/poll-fix/controls/1789169197544821-2888425/`.
  Summary SHA-256 `193df1b52577c5b1676258f77da72c1366180a04cff3cc90b23fd7e36ad15072`.
- Worker-isolation controls: `native-verification-20260911/linux/poll-fix/controls/worker-isolation-1789169259615389-2888425/`.
  Summary SHA-256 `55334df330990339e1e87f197d05cce32506fe5e34149d3122529d53ba88f71c`.

| Executable | SHA-256 |
| --- | --- |
| Worker | `7d6974c8cd4a450d767e02c9b49d8cfc1e92d0d7bc6e5f1dc4aeb703198520d4` |
| Probe | `2d048a19a11b7d9611a8cab4de17780544b95b715ce516c166de8e4669884feb` |
| Validation runner | `84ca472ba09f13d38bc3ed4747196663852e0c94aed73a2b548d381bf186811d` |

### macOS

- Main controls: `native-verification-20260911/macos/poll-fix/controls/1789169059884976-17799/`.
  Summary SHA-256 `3cccca22fbc13d10489de21aba342e76e0a73369aadc54806b69813fda70e7d9`.
- Worker-isolation controls: `native-verification-20260911/macos/poll-fix/controls/worker-isolation-1789169136941087-17799/`.
  Summary SHA-256 `f8af2f9cff505d4d3f0e54e8691045635de98d21b85bf510468efd205568a163`.

| Executable | SHA-256 |
| --- | --- |
| Worker | `afa237457bd85af0e5907f855fb0d7eae0562cb4fb4b3f1f8a6853ea09c4e1e0` |
| Probe | `04c373c3067dd57814aa21c04e49adbc3f7036a92bd1cd523ce37784d2feda38` |
| Validation runner | `c99567f44cd414d4e579cc786542342be6670b411159d3e8dcef75c41400af61` |

The paths above are relative to the external benchmark archive's
`.work/` directory. The raw records, summaries, and independent control
records remain retained there. Independent GNU `/usr/bin/time -v` on Linux
and BSD `/usr/bin/time -l` on macOS are controls only; they are not the
benchmark measurement path. Workload timing and RSS results are produced by
the native `tq-bench` measurement path.

## Measurement policy

CPU differences are divided by native target wall duration. Differences up to
10% are automatic green; differences above 10% through 20% are green with an
informational notice; differences above 20% and below 50% require approval;
differences at least 50% block acceptance and require investigation. Below
500 ms, a difference strictly below `max(20 ms, 10% of runtime)` is automatic
green. Exactly 500 ms uses the percentage bands. RSS tolerances are unchanged
and page-aware.

The native worker owns target launch, exact-child accounting, and cleanup.
Worker startup, request transfer, reply transfer, and worker teardown are
outside the target interval. Ordinary controls use the worker without sampled
RSS enforcement; sampler controls use the explicit worker process-group RSS
sampler and are calibrated separately.

The replacement macOS smoke, rapid and standard campaigns completed with this
collector. Standard retained 846 observations: 703 timed, 134 unsupported and
9 resource-limit rows, with no incorrect results. All 10,579 primary samples
had positive native RSS. See the [verification report](verification-report.md)
for campaign identities and acceptance details. Public comparison tables and
tq baseline/candidate regression decisions remain Linux-only; no cross-OS
performance comparison is made.
