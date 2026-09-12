# Verification report

Change: `native-benchmark-process-accounting`
Date: 2026-09-12 UTC
Scope: current code, all planning artifacts, issue #30 acceptance criteria,
local preflight, and fresh native macOS/Linux controls.

## Summary

| Dimension | Status |
| --- | --- |
| Completeness | 24/24 tasks complete, including both native campaigns |
| Correctness | All seven requirements covered; both native matrices and the Linux regression gate verified |
| Coherence | Worker ownership and same-host measurement policy follow the design |

The worker-delay and malformed-control coverage findings are resolved. Review
also found and fixed a request-thread cleanup bug and the macOS polling crash.
The user approved same-host, same-OS comparisons without a universal 1 ms
accuracy guarantee. CPU/RSS and self-regression tolerances are unchanged.

## Resolved findings

- Worker requests now use an anonymous prepared file containing bounded JSON.
  The former pipe writer could remain detached, or block a join, after a
  termination failure. Removing the thread removes both failure paths.
  Request preparation remains outside target timing.
- All 22 `benchmark_worker.rs` tests pass on both native hosts. Delay tests
  execute a real target through the real worker. Fault tests cover empty,
  truncated, malformed, oversized and incompatible replies, non-reading
  workers, retained diagnostics, no valid outcome on failure, and PID cleanup.
  Linux liveness checks distinguish zombies from live processes.
- Both actual process-loss injections pass on both hosts: coordinator loss
  and worker loss. The ignored lifecycle suite also runs three helper entries.
- Fresh controls cover allocation sizes, short released bursts, threads,
  independent children, 0/32/128 MiB coordinator allocations, prepared 32 MiB
  stdin and high/low request sequences. Each host retains 420 main pairs plus
  120 worker-isolation pairs. All CPU/RSS comparisons pass against independent
  native time controls. See [worker-proof.md](worker-proof.md).
- Worker/source identities, residual-floor evidence, calibration linkage and
  stale-worker rejection are implemented and tested. Old reports were not
  relabeled with the new collector identity.
- Smoke JSON corpus identities now record the actual launch path. A focused
  regression test covers relative seed metadata. The original smoke reports
  retain three automated unavailable rows; the
  [supplemental smoke review](smoke-regression-review.md) verifies matching
  inputs and contracts and evaluates all nine tq rows without changing samples.
  No smoke wall-time or RSS increase exceeds 20%.
- Final standards review found no hard violations and two non-blocking
  maintenance suggestions. Final spec review found no remaining implementation
  blocker. Issue #30 acceptance commands explicitly pin both regression
  thresholds at 50%; the generic evaluator retains its configurable thresholds.
- The Linux standard gate evaluated 410 comparable tq rows with no disclosures
  above 20% and no blocking increases above 50%. The largest increases were
  13.2% in median wall time and 3.0% in maximum observed peak RSS. See the
  [Linux regression review](linux-regression-review.md).
- All 40 published Linux Results blocks match a fresh render from the retained
  standard and smoke reports. Authored monthly examples were checked against
  raw summaries. Public comparison tables remain Linux-only.

## Native campaign interruption

Two macOS standard campaigns aborted in Rust's `thread::sleep` after
`nanosleep` returned EINVAL rather than EINTR. The assertion is in the pinned
standard library at `sys/thread/unix.rs:617`. The coordinator produced no
standard JSON report, so none of that partial run is used for publication.
Their complete logs remain under the external archive's
`.work/native-verification-20260911/macos/` directory as
`campaign-first/standard.log` and `standard-repeat.log`.

A focused 30-sample parse-discard rerun passed unchanged, but the full repeat
reproduced the assertion. The macOS polling path now uses safe thread parking
against a monotonic deadline, with tests for zero delay and early unparks.
Polling intervals, timeout gates and Linux's sleep implementation are unchanged.
Targeted standards and spec reviews found no critical or warning issues.
Fresh controls and lifecycle tests pass on both hosts with collector source
`51e062e1eca0d34baa4f43ef1fc7e667adde20378e2c9309d077a64e6b93fbdf`.
The replacement macOS campaign completed all 846 observations without a
coordinator crash. The interrupted campaigns remain retained and contribute
no samples to the replacement report.

Published Linux campaigns retain collector source
`9527c5326c87782e237f448ae91eb93ee479c0e4ebcb6f039ad04cdd51e03bf5`
and their matching controls. They are neither relabeled nor pooled with the
new macOS campaign. The triggering cause of the platform `nanosleep` error
is not established; the fix avoids that failing path.

## Native workload acceptance

Both native hosts completed the standard, rapid and three-workload smoke
campaigns. The table reports coverage, not a cross-OS performance comparison.
Every primary sample has positive native RSS. Instrumented limit checks are
separate from primary timing and RSS summaries.

| Host / profile | Timed rows | Unsupported rows | Resource-limit rows | Primary samples | Instrumented samples |
| --- | ---: | ---: | ---: | ---: | ---: |
| Linux standard | 703 | 134 | 9 | 10,579 | 180 |
| Linux rapid | 27 | 3 | 0 | 27 | 3 |
| Linux smoke | 18 | 0 | 0 | 540 | 0 |
| macOS standard | 703 | 134 | 9 | 10,579 | 180 |
| macOS rapid | 27 | 3 | 0 | 27 | 3 |
| macOS smoke | 18 | 0 | 0 | 540 | 0 |

No campaign contains an incorrect result. Each standard report includes
10,570 valid timed samples and nine failed resource-limit attempts. The
resource-limit rows report tq's classified resource exit, status 5, for monthly
object construction and monthly/weekly string reduction in three formats.
They remain visible and are excluded from speed rankings. Their presence
explains the standard command's exit status 1 and `observed-failures` status;
it is not an accounting or coordinator failure.

Each standard report has 30 timed rows with empty output and 673 with
nonempty output. Empty-output samples have no first-output latency;
nonempty-output samples have valid latency records. No sample lacks its
measurement protocol. Tool versions were jq 1.8.1 and pinned yq 4.53.2.
Both hosts used the frozen hour/day/week/month corpus and `release-benchmark`
builds. Host specifications and collector identities are in
[worker-proof.md](worker-proof.md).

The Linux baseline/candidate gate and supplemental smoke review accept 419
comparable tq pairs with no wall-time or peak-RSS disclosure above 20%.
The 13 unsupported/resource-limited standard rows remain unavailable, not
passing regression comparisons. Public comparison Results remain Linux-only.
macOS supplies native verification evidence, not a cross-OS ranking or a
separate tq self-regression claim.

### Retained macOS campaign identity

Reports are under the external archive's
`.work/native-verification-20260911/macos/poll-fix/campaign/` directory.
Their SHA-256 values are:

| Report | SHA-256 |
| --- | --- |
| `standard.json` | `3f1e97a6c8de63f4bbe7ca9ccc55ebe0967a03a6b6efbcae32bde21ac8a3a4d1` |
| `rapid.json` | `51369f43a5f820cbc6afdaf4064075cf2f154c377853d50a1a7328669adad174` |
| `smoke.json` | `87350aed5583e33dcb182d9b601d9aa3aa31e134a9bb675fafc072d1a4b2893d` |

Every macOS sample links to final collector source `51e062e1...`, worker
`afa23745...` and calibration summary `3cccca22...`. Full identities are in
the worker proof and raw reports. The retained runner is
`run-macos-poll-fix.sh`; it records build/test commands, frozen manifests,
calibration input, profiles and exit statuses. The Linux report identities
are recorded in the linked standard and smoke regression reviews.

### Accepted timing interpretation

The busy-deadline control reports its own elapsed interval after controlled
work. Native wall duration also includes legitimate target startup and exit
observation. Their difference is not solely collector error, and no estimated
startup cost is subtracted from workload measurements.

| Native host | Primary observed control excess | Sampled control excess | Primary median native/child difference | Child overrun, primary/sampled |
| --- | ---: | ---: | ---: | ---: |
| Linux, Ryzen 7 7700 | 1.0 ms | 1.0 ms | 0.8 ms | 3.0 µs / 3.0 µs |
| macOS, M1 Max | 23.5 ms | 30.4 ms | 7.6 ms | 2.0 µs / 4.0 µs |

This table describes final-source controls. The published Linux workload
reports retain their original matching controls and identities, detailed in
[worker-proof.md](worker-proof.md).

Observed excess retains the sleep-based end-to-end controls. Busy controls
distinguish sleep scheduling from the controlled interval, but do not establish
universal 1 ms accuracy. The accepted policy compares tools within the same
host and OS using equivalent measurement settings and concessions. Repetitions
and dispersion remain necessary; scheduling noise does not cancel exactly.
Human-facing labels now say observed control excess. Legacy serialized field
names remain compatible and explicitly document this meaning. Regression
comparability rejects different operating systems, with a dedicated test.

## Requirement coverage

All 35 scenarios were reviewed. Implementation coverage is distinct from
completed native acceptance evidence.

| Requirement | Evidence |
| --- | --- |
| Resource and latency metrics | `measure.rs`, native preflight, conversion tests, positive-RSS publication gates, both-host allocation/thread controls. |
| tq performance regression policy | `report.rs` and 31 passing reporting tests cover independent metrics, exact 20%/50% boundaries, unavailable baselines, different operating systems and incompatible protocols. Linux standard and supplemental smoke evaluations pass. |
| Direct invocation and timing boundary | Prepared files, literal arguments, working directory, EOF, real worker-delay tests and frozen exit-observation timing. Precision has the limitation above. |
| Isolated measurement worker | Bounded requests/replies, 22 worker tests, both process-loss injections, coordinator-memory controls and 15 passing calibration tests. |
| Single-owner lifecycle | `native_process.rs` owns WNOWAIT observation, cleanup and consuming wait4. Exit, signal, deadline, cancellation, cleanup and repeated-child tests pass. |
| Native accounting validation | Audited wait4 0.2.0, Rust 1.95 minimum, checked first-party conversions and documented valid-OS-counter assumption; 540 passing pairs per host. No first-party unsafe bridge. Windows remains deferred to issue #31. |
| Evidence publication | Renderer tests cover units, one decimal, placeholders, preserved authored text, source-free tables and stale-calibration rejection. All 40 Linux Results blocks match a fresh render; macOS verification is retained separately. |

The nine issue #30 acceptance criteria map to these requirements. Native
interface, allocation bursts, units, independent children/threads, lifecycle
and independent counters are demonstrated. Timing precision has the stated
limitation. Both-host matrix acceptance and the Linux self-regression review
are complete.
No verification dimension was skipped.

## Checks

- Workspace/all-target typechecking, strict all-feature Clippy, formatting and
  diff checks pass.
- Final post-fix `RUST_TEST_THREADS=1 ./scripts/preflight.sh` completed with
  exit status 0. Its log is `final-preflight-macos-poll-fix.log` in the external
  evidence directory. Documentation checks passed and strict OpenSpec validation
  reported 20 passing items.
- Full workspace tests pass with elevated process-inspection permissions.
  The earlier sandboxed attempt failed two RSS-enforcement cases with EPERM;
  collection-failure gates were not weakened.
- A later parallel preflight reproduced a macOS sampled-RSS inspection timeout.
  Under concurrent test load, `ps` exceeded its 250 ms deadline. The complete
  20-test benchmark-gate file passed serially, and standalone process inspection
  succeeded. Final workspace verification uses serial tests; the failed
  preflight and focused reproductions remain retained. No sampler timeout or
  collection-failure gate was weakened. This does not affect sampler-free
  primary workload timing.
- Strict all-change OpenSpec validation passes with telemetry disabled.
- Documentation validation reports zero errors/warnings and six informational
  links to intentional non-concept artifacts.
- Native worker/lifecycle tests and 20 repetitions of every accounting control
  pass on both hosts outside restricted sandboxes.

Logs and raw evidence are in the external archive under
`.work/native-verification-20260911/`. The Linux baseline is an isolated
release build of `6e357f7`, using the candidate's pinned compiler and release
settings. Both tq binaries use the same frozen Linux collector and corpus.
The completed campaigns and self-regression decisions are recorded in the
linked Linux reviews.

## Assessment

Implementation, lifecycle tests, both-host controls and matrices, Linux
self-regression review, publication and repository preflight are complete.
There are no outstanding critical or warning findings. Two non-blocking
maintenance suggestions remain from the standards review: consider splitting
the large native-validation test module in future work, and rename the
lifecycle fixture's PID artifact for clarity. Neither changes acceptance.
The change is ready for sync and archive.
