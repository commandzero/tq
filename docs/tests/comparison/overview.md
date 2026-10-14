---
type: Report
title: jq, yq, and tq benchmark comparisons
description: Recorded benchmark comparisons across jq, yq, and tq.
generated:
  by: codex
  at: 2026-10-08T15:38:02.825Z
benchmark_runs:
- campaign_id: null
  identity_status: not-recorded
  provenance: The workload tables identify tq 0.1.0 build a4b4d916-worktree, jq 1.8.1, and yq 4.53.2; no retained campaign ID or binary hashes are recorded in this index.
  binaries:
    tq:
      version: tq 0.1.0 (TOON v3; jq target 1.8.x; revision a4b4d916-worktree)
      sha256: null
    jq:
      version: jq-1.8.1
      sha256: null
    yq:
      version: yq (https://github.com/mikefarah/yq/) version v4.53.2
      sha256: null
- campaign_id: 2026-09-26T19:27:09.746840362Z
  identity_status: not-recorded
  run_scope: system-allocator
  provenance: Extended Linux diagnostic; exact tq, jq, and yq versions and executable hashes retained in system.json; tq revision unknown.
  binaries:
    tq:
      version: tq 0.4.0 (TOON v3; jq target 1.8.x; revision unknown)
      sha256: 221c680db8481f965b8579d036183766e180473185799590c72247be7fb107c4
      identity_status: not-recorded
      allocator: system
    jq:
      version: jq-1.8.1
      sha256: 136748786226819bf582738e8be963638c9d721aa0c5d1d650b506a2a52ddb97
    yq:
      version: yq (https://github.com/mikefarah/yq/) version v4.53.2
      sha256: d56bf5c6819e8e696340c312bd70f849dc1678a7cda9c2ad63eebd906371d56b
- campaign_id: 2026-09-26T19:45:18.07883121Z
  identity_status: not-recorded
  run_scope: default-mimalloc
  provenance: Extended Linux diagnostic; exact tq, jq, and yq versions and executable hashes retained in mimalloc.json; tq revision unknown.
  binaries:
    tq:
      version: tq 0.4.0 (TOON v3; jq target 1.8.x; revision unknown)
      sha256: 7068a178ce8f62f6cbfaaaf588049882dd5500d8213a7711a121043db6a0d338
      identity_status: not-recorded
    jq:
      version: jq-1.8.1
      sha256: 136748786226819bf582738e8be963638c9d721aa0c5d1d650b506a2a52ddb97
    yq:
      version: yq (https://github.com/mikefarah/yq/) version v4.53.2
      sha256: d56bf5c6819e8e696340c312bd70f849dc1678a7cda9c2ad63eebd906371d56b
- campaign_id: null
  identity_status: not-recorded
  run_scope: release-validation
  provenance: Helper-only release-validation controls; exact campaign IDs, helper version strings, and executable hashes were not recorded. Collector source SHA-256 is retained.
  binaries:
    tq-bench-worker:
      version: null
      sha256: null
      identity_status: not-recorded
      protocol: tq-bench-worker-protocol-v3
      collector_source_sha256: 9527c5326c87782e237f448ae91eb93ee479c0e4ebcb6f039ad04cdd51e03bf5
- campaign_id: null
  identity_status: not-recorded
  run_scope: worker-isolation
  provenance: Helper-only worker-isolation controls; exact campaign IDs, helper version strings, and executable hashes were not recorded. Collector source SHA-256 is retained.
  binaries:
    tq-bench-worker:
      version: null
      sha256: null
      identity_status: not-recorded
      protocol: tq-bench-worker-protocol-v3
      collector_source_sha256: 9527c5326c87782e237f448ae91eb93ee479c0e4ebcb6f039ad04cdd51e03bf5
- campaign_id: 2026-10-08T15:10:27.867608118Z
  binaries:
    jq:
      version: jq-1.8.1
      sha256: 020468de7539ce70ef1bceaf7cde2e8c4f2ca6c3afb84642aabc5c97d9fc2a0d
    tq:
      version: tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)
      sha256: 474625b11f0ec3a1fa125bbeb4e55008efa50ec713b0c398822ada48f0289b91
    yq:
      version: yq (https://github.com/mikefarah/yq/) version v4.53.2
      sha256: d56bf5c6819e8e696340c312bd70f849dc1678a7cda9c2ad63eebd906371d56b
---
# jq, yq, and tq benchmark comparisons

Compare jq, yq, and tq on common data-processing tasks. Each page explains
what the query does and preserves the recorded results for that workload.

Only outputs that pass the correctness check are timed. Failed, unsupported,
and unmeasured cases remain visible. The current Linux campaign measures
jq 1.8.1, yq 4.53.2 and the frozen tq 0.5.0 / TOON 4.1 candidate with
fresh native calibration and worker-isolation evidence. Captured versions and
digests are in frontmatter; earlier capture identities remain historical.
This cross-tool comparison is not a tq self-regression baseline.

The [Linux worker-validation results](worker-validation.md) cover the new
native accounting path separately. They are helper controls, not jq/yq/tq
workload measurements. Every workload table below comes from the full native
rerun; no wrapper-based measurements are included.

## Native jq compatibility closeout evidence

The [jq manual platform results](../jq-manual/coverage.md#native-platform-results)
and [durable closeout record](../../../tests/compatibility/reviews/parity-closeout.toon)
identify the latest 2026-10-06 macOS **newhelp-final** and Linux **help-final**
source-bound release runs (884-file snapshots, base `3e0dedb` plus uncommitted
fixes; pinned jq 1.8.2). Final `args.rs` help wording/regression clarifies CLI
ambient access versus embedded admission without changing policy logic.
Earlier artifacts remain checkpoints, not same-source/binary proof.
macOS records 943/952 primary, 919/921 compact, and
921/921 TOON matches; Linux records 937/952, 913/921, and 921/921. Both preserve
518 original cases and 303 protected exact contracts; strict manual exit 1
remains. Corrected full campaigns cover 1,220 cases and 4,804 executed
observations with zero harness errors/declared-contract failures, but remain
`observed-differences`, not exact acceptance. macOS reuses an independently
source-bound `embedded-denial-p2-closeout-3e0dedb` campaign that predates the help
fix: equal source applied only to late-p2, not final-help. The only crate-source
delta is help wording/test; diagnostic relevance is not execution of final-help
binaries, whose CLI/helper/embedded hashes differ. Linux reruns the full campaign
with final-help binaries. P2 fakehost regressions independently enforce denial
and JSON companion contracts without disclosure. Linux's final workspace records
1,891 passed, 0 failed and 11 ignored; four reference/relocation entries explicitly
pass separately. macOS final full preflight/all-feature workspace pass. Ignored
accounting tests and these correctness runs establish no calibration.

Windows now has native release/default renewal in
`target/closeout/windows/final-286b5f6/native`: **912/952** primary,
**879/921** compact, **921/921** TOON and **60 unique differences** under #69.
All 303 protected IDs are present, but only **291 primary** and **289
primary-plus-compact** match—not 303 exact as on macOS/Linux. Its full campaign
covers 1,220 cases/4,804 executed observations with zero harness/contract failures
and **146 difference cases**. All 297 CLI composition cases execute, retaining
8 math/reference-availability differences; 297 embedded/resource witnesses pass.
Final all-feature workspace, strict Clippy and formatting pass; 10 entries remain
ignored, with five reference tests explicitly passed separately. No unknown
workspace pass total or calibrated performance result is implied.

Windows source is `286b5f6` plus one test-only catalog-reference overlay, with
exact source/profile/frozen binary mapping—not a full release rebuild or campaign
rerun after the test fix. Product/helper code remains unchanged, including relative
to macOS/Linux final-help sources. Actual system SSH/SCP succeeded and execution
was native PowerShell; earlier macOS-client failures are historical. No security
changes. Windows v6 raw evidence remains retained and separately hash-scoped.
Live #69/#70/#31 scope-transfer bodies were verified at the preceding 2026-10-06
checkpoint. The user-approved
scope transfer leaves the enumerated Windows 60 under [#69](https://github.com/commandzero/tq/issues/69)
and Linux 15 under [#70](https://github.com/commandzero/tq/issues/70), unresolved,
with exact witnesses retained for reconsideration after platform/reference fixes.
It does not renew approvals or establish an all-platform pass.

The correctness closeouts above do not establish calibration. Fresh Linux
controls captured on 2026-10-08 now validate the workload campaign below.
They do not renew macOS or Windows calibration, allocator diagnostics, or
other platform acceptance. [#31](https://github.com/commandzero/tq/issues/31)
and platform follow-ups remain separate.

## Current allocator diagnostic

The [mimalloc versus system allocator summary](mimalloc-vs-default.md) records
the 2026-09-26 extended comparison with jq 1.8.1 and the default mimalloc
build of tq 0.4.0. Its timing and memory measurements are diagnostic: the
campaigns lack linked native timing-calibration and launch-isolation evidence.
The publication renderer rejects them, so they do not replace the historical
workload tables below. The allocator summary includes the measured benefits,
memory costs, unchanged failures, and retained raw-report identities.

## Findings

Campaign `2026-10-08T15:10:27.867608118Z` completed all 858 planned adapter
observations across 39 workloads: 707 timed, 134 unsupported, 8 incorrect,
and 9 resource-limit rows. Each timed row has one warmup and three measured
samples: 2,121 valid timed samples plus 9 retained failed resource attempts.
All recorded samples have positive native peak RSS. No rows were filtered.

The eight incorrect rows are yq JSON/YAML CSV and TSV formatting on the week
and month snapshots; ordered-result checks failed before timing. The nine
resource-limit rows are tq JSON/YAML/TOON object construction on the month
snapshot and string reduction on the week/month snapshots, all classified
resource exits (status 5). Unsupported and failed rows support no speed ranking.
The complete campaign therefore exits 1 with `observed-failures`, not a pass.

The four frozen USGS snapshots retain their original input identities.
Native CSV, TSV and JSON-sequence workloads additionally cover deterministic
8-record and 131,072-record fixtures. Their results are separate from the
month-dataset overview and from accounting calibration controls.

On the month snapshot, identity re-encoding has median wall time / MAD of
267.698 / 0.313 ms for jq JSON, 189.093 / 1.703 ms for tq JSON, and
211.167 / 0.138 ms for tq TOON. Peak RSS is respectively 61.1, 67.0 and
66.4 MiB. These are workload-specific tradeoffs, not an overall winner.

The identity commands emit pretty JSON from jq, compact JSON from yq and
default TOON 4.1 from tq. Their input may be the same JSON snapshot, but
output encoding and byte counts differ; this is not a same-output serializer
comparison. The command/output concessions are retained rather than silently
changing the measured matrix.

Native-format throughput has different tradeoffs. For 131,072 records, tq
CSV takes 5,049.766 ms versus yq's 2,257.305 ms, but uses 15.3 versus
598.5 MiB peak RSS. JSON sequences take 5,242.910 ms for tq versus
96.992 ms for jq, with 14.8 versus 2.7 MiB peak RSS. Dispersion and first
output timings remain in the individual workload tables; no overhead is
subtracted from measured samples.

No self-regression baseline was supplied, so the new report's regression
gate is explicitly not evaluated. The historical
[Linux self-regression review](../../../openspec/changes/archive/2026-09-11-native-benchmark-process-accounting/linux-regression-review.md)
evaluated 410 tq rows under its original identities and remains historical.
Likewise, the focused migration guard's existing +20% hard / +10% advisory
evidence is not renewed or relabelled by this cross-tool campaign.

## Results

<!-- benchmark-results:start -->
Last updated: 2026-10-08
Suite: `natural-corpus` | Profile: `standard` | Campaign status: `observed-failures`

858 adapter observations across 39 workloads. Only correctness-checked outputs are timed; failed rows cannot support a speed ranking.

Environment: `linux` / `x86_64`, AMD Ryzen 7 7700 8-Core Processor, 16 logical CPUs, 61.9 GiB RAM; kernel `Linux 7.2.4-ogc3.1.fc44.x86_64 #1 SMP PREEMPT_DYNAMIC Sun Sep 13 01:41:06 UTC 2026`; compiler profile `release-benchmark`

RSS collector provenance (outside measurement tables): `linux-wait4`.
Measurement method (outside measurement tables): `tq-bench` native measurement: RSS scope `wait4 child lifetime including pre exec waited descendants and threads; sampled worker process group`; residual RSS floor `2.6 MiB` retained, not subtracted; observed control excess `0.9 ms`; primary timing includes process-group RSS sampling; `tq-bench` native measurement: RSS scope `wait4 child lifetime including pre exec waited descendants and threads`; residual RSS floor `2.6 MiB` retained, not subtracted; observed control excess `0.9 ms`; primary timing is sampler-free.

### Campaign coverage

| Outcome | jq JSON | jq JSON-SEQ | yq JSON | yq YAML | yq CSV | yq TSV | tq JSON | tq YAML | tq TOON | tq JSON-SEQ | tq CSV | tq TSV |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| timed | 141 | 2 | 72 | 72 | 2 | 2 | 138 | 134 | 138 | 2 | 2 | 2 |
| incorrect | 0 | 0 | 4 | 4 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| unsupported | 0 | 0 | 65 | 65 | 0 | 0 | 0 | 4 | 0 | 0 | 0 | 0 |
| timeout | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| resource-limit | 0 | 0 | 0 | 0 | 0 | 0 | 3 | 3 | 3 | 0 | 0 | 0 |
| oom-or-signal | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

### usgs-all-month

Largest recorded JSON input: 7665096 bytes, 10792 logical records. Each row compares the same workload across tools. Wall time cells use milliseconds and peak RSS cells use MiB, with one decimal place. Lower is better. A `-` cell means no valid comparable measurement, not zero; compare matching formats and check each workload page for outcomes and sample counts.

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `ordered result sequence`, `process exited with classified error Resource (exit status 5)`, `tq stream mode requires TOON or JSON event input; YAML is document-at-a-time.`, `yq rejects the catalog jq @html/@uri expression.`, `yq rejects the catalog jq @sh expression for this array.`, `yq rejects the catalog jq any predicate expression.`, `yq rejects the catalog jq def expression used by this adapter.`, `yq rejects the catalog jq format-string expression.`, `yq rejects the catalog jq label/break expression.`, `yq rejects the catalog jq paths expression.`, `yq rejects the catalog jq recurse expression.`, `yq rejects the catalog jq recursive scalar expression.`, `yq rejects the catalog jq scalar-utility expression.`, `yq rejects the catalog jq stream-event invocation.`, `yq rejects the catalog jq strings/test expression.`, `yq rejects the catalog jq walk expression.`.

| Workload | Metric | jq JSON | jq JSON-SEQ | yq JSON | yq YAML | yq CSV | yq TSV | tq JSON | tq YAML | tq TOON | tq JSON-SEQ | tq CSV | tq TSV |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| array-construction | Wall time | 125.5 ms | - | 552.4 ms | 1085.5 ms | - | - | 91.4 ms | 131.8 ms | 143.6 ms | - | - | - |
| array-construction | Peak RSS | 60.8 MiB | - | 454.9 MiB | 1040.3 MiB | - | - | 78.8 MiB | 117.7 MiB | 99.5 MiB | - | - | - |
| blocking-sort | Wall time | 120.1 ms | - | 313.9 ms | 898.3 ms | - | - | 115.2 ms | 131.8 ms | 143.3 ms | - | - | - |
| blocking-sort | Peak RSS | 57.3 MiB | - | 322.4 MiB | 1032.3 MiB | - | - | 27.8 MiB | 87.8 MiB | 21.9 MiB | - | - | - |
| comma-generator-sort | Wall time | 300.9 ms | - | 984.9 ms | 1476.3 ms | - | - | 190.1 ms | 232.6 ms | 239.8 ms | - | - | - |
| comma-generator-sort | Peak RSS | 61.1 MiB | - | 699.4 MiB | 1238.4 MiB | - | - | 83.2 MiB | 117.6 MiB | 99.3 MiB | - | - | - |
| dead-sort-length | Wall time | 109.4 ms | - | 310.3 ms | 856.6 ms | - | - | 101.9 ms | 121.1 ms | 133.9 ms | - | - | - |
| dead-sort-length | Peak RSS | 56.9 MiB | - | 325.7 MiB | 954.2 MiB | - | - | 23.6 MiB | 87.7 MiB | 22.0 MiB | - | - | - |
| event-stream | Wall time | 240.6 ms | - | - | - | - | - | 557.6 ms | - | 600.8 ms | - | - | - |
| event-stream | Peak RSS | 2.7 MiB | - | - | - | - | - | 15.3 MiB | - | 15.5 MiB | - | - | - |
| format-base64-roundtrip | Wall time | 100.5 ms | - | 466.6 ms | 1028.8 ms | - | - | 272.4 ms | 189.9 ms | 207.1 ms | - | - | - |
| format-base64-roundtrip | Peak RSS | 56.6 MiB | - | 437.3 MiB | 1170.2 MiB | - | - | 30.2 MiB | 117.7 MiB | 16.3 MiB | - | - | - |
| format-csv | Wall time | 112.8 ms | - | - | - | - | - | 158.5 ms | 200.5 ms | 207.1 ms | - | - | - |
| format-csv | Peak RSS | 56.8 MiB | - | - | - | - | - | 73.1 MiB | 117.8 MiB | 97.0 MiB | - | - | - |
| format-json | Wall time | 246.4 ms | - | 1007.5 ms | 1458.1 ms | - | - | 229.4 ms | 272.0 ms | 281.9 ms | - | - | - |
| format-json | Peak RSS | 59.8 MiB | - | 798.3 MiB | 1270.3 MiB | - | - | 73.0 MiB | 117.8 MiB | 97.2 MiB | - | - | - |
| format-shell | Wall time | 110.9 ms | - | - | - | - | - | 159.4 ms | 199.4 ms | 206.4 ms | - | - | - |
| format-shell | Peak RSS | 56.8 MiB | - | - | - | - | - | 73.3 MiB | 117.7 MiB | 97.4 MiB | - | - | - |
| format-template | Wall time | 113.1 ms | - | - | - | - | - | 276.2 ms | 198.4 ms | 208.2 ms | - | - | - |
| format-template | Peak RSS | 56.6 MiB | - | - | - | - | - | 30.0 MiB | 87.8 MiB | 15.9 MiB | - | - | - |
| format-tsv | Wall time | 112.0 ms | - | - | - | - | - | 156.8 ms | 197.9 ms | 207.1 ms | - | - | - |
| format-tsv | Peak RSS | 56.8 MiB | - | - | - | - | - | 73.2 MiB | 117.8 MiB | 97.3 MiB | - | - | - |
| format-uri-html | Wall time | 109.6 ms | - | - | - | - | - | 274.1 ms | 193.9 ms | 207.1 ms | - | - | - |
| format-uri-html | Peak RSS | 56.6 MiB | - | - | - | - | - | 30.3 MiB | 117.5 MiB | 15.9 MiB | - | - | - |
| identity-reencode | Wall time | 267.7 ms | - | 721.9 ms | 1260.7 ms | - | - | 189.1 ms | 203.5 ms | 211.2 ms | - | - | - |
| identity-reencode | Peak RSS | 61.1 MiB | - | 460.1 MiB | 1042.6 MiB | - | - | 67.0 MiB | 117.0 MiB | 66.4 MiB | - | - | - |
| issue5-collection | Wall time | 122.8 ms | - | 572.8 ms | 1075.0 ms | - | - | 90.4 ms | 133.1 ms | 140.5 ms | - | - | - |
| issue5-collection | Peak RSS | 57.7 MiB | - | 599.1 MiB | 1185.2 MiB | - | - | 81.5 MiB | 117.7 MiB | 99.6 MiB | - | - | - |
| issue5-json-conversion | Wall time | 107.7 ms | - | 252.8 ms | 864.9 ms | - | - | 75.4 ms | 116.0 ms | 124.3 ms | - | - | - |
| issue5-json-conversion | Peak RSS | 56.6 MiB | - | 281.1 MiB | 1030.9 MiB | - | - | 73.3 MiB | 117.9 MiB | 97.9 MiB | - | - | - |
| issue5-paths | Wall time | 105.8 ms | - | - | - | - | - | 73.5 ms | 117.6 ms | 123.2 ms | - | - | - |
| issue5-paths | Peak RSS | 56.6 MiB | - | - | - | - | - | 73.2 MiB | 117.8 MiB | 97.4 MiB | - | - | - |
| issue5-predicate | Wall time | 88.5 ms | - | - | - | - | - | 75.0 ms | 115.1 ms | 122.7 ms | - | - | - |
| issue5-predicate | Peak RSS | 56.6 MiB | - | - | - | - | - | 73.2 MiB | 117.6 MiB | 97.6 MiB | - | - | - |
| issue5-scalar-utilities | Wall time | 209.7 ms | - | - | - | - | - | 91.0 ms | 132.7 ms | 142.3 ms | - | - | - |
| issue5-scalar-utilities | Peak RSS | 57.1 MiB | - | - | - | - | - | 79.3 MiB | 117.5 MiB | 99.7 MiB | - | - | - |
| label-early-break | Wall time | 88.7 ms | - | - | - | - | - | 73.5 ms | 115.8 ms | 124.1 ms | - | - | - |
| label-early-break | Peak RSS | 56.6 MiB | - | - | - | - | - | 73.3 MiB | 117.7 MiB | 97.3 MiB | - | - | - |
| multi-result-projection | Wall time | 95.9 ms | - | 351.2 ms | 922.8 ms | - | - | 111.3 ms | 182.8 ms | 140.5 ms | - | - | - |
| multi-result-projection | Peak RSS | 56.9 MiB | - | 327.2 MiB | 1015.0 MiB | - | - | 17.1 MiB | 117.7 MiB | 15.1 MiB | - | - | - |
| numeric-reduction | Wall time | 111.2 ms | - | 328.4 ms | 885.6 ms | - | - | 84.0 ms | 126.3 ms | 135.5 ms | - | - | - |
| numeric-reduction | Peak RSS | 56.6 MiB | - | 323.0 MiB | 1032.7 MiB | - | - | 72.9 MiB | 117.7 MiB | 97.6 MiB | - | - | - |
| object-construction | Wall time | 124.2 ms | - | 21428.0 ms | 20881.0 ms | - | - | - | - | - | - | - | - |
| object-construction | Peak RSS | 62.1 MiB | - | 470.4 MiB | 1310.8 MiB | - | - | - | - | - | - | - | - |
| parse-discard | Wall time | 88.1 ms | - | 257.6 ms | 833.5 ms | - | - | 73.5 ms | 116.6 ms | 123.0 ms | - | - | - |
| parse-discard | Peak RSS | 56.6 MiB | - | 280.7 MiB | 1027.7 MiB | - | - | 72.3 MiB | 116.9 MiB | 96.6 MiB | - | - | - |
| path-update | Wall time | 268.2 ms | - | 722.1 ms | 1220.2 ms | - | - | 159.2 ms | 202.5 ms | 210.3 ms | - | - | - |
| path-update | Peak RSS | 60.9 MiB | - | 461.0 MiB | 964.1 MiB | - | - | 73.1 MiB | 117.9 MiB | 97.4 MiB | - | - | - |
| recurse-bounded | Wall time | 432.8 ms | - | - | - | - | - | 262.5 ms | 305.9 ms | 312.5 ms | - | - | - |
| recurse-bounded | Peak RSS | 61.1 MiB | - | - | - | - | - | 73.3 MiB | 117.3 MiB | 97.4 MiB | - | - | - |
| recursive-scalars | Wall time | 315.1 ms | - | - | - | - | - | 2054.1 ms | 2144.2 ms | 2115.5 ms | - | - | - |
| recursive-scalars | Peak RSS | 60.9 MiB | - | - | - | - | - | 72.9 MiB | 117.4 MiB | 96.8 MiB | - | - | - |
| regex-test | Wall time | 122.3 ms | - | - | - | - | - | 136.9 ms | 179.4 ms | 187.0 ms | - | - | - |
| regex-test | Peak RSS | 57.1 MiB | - | - | - | - | - | 80.3 MiB | 118.7 MiB | 100.5 MiB | - | - | - |
| scalar-extraction | Wall time | 105.3 ms | - | 255.4 ms | 865.9 ms | - | - | 73.5 ms | 116.0 ms | 123.1 ms | - | - | - |
| scalar-extraction | Peak RSS | 56.6 MiB | - | 279.6 MiB | 1031.4 MiB | - | - | 72.8 MiB | 117.4 MiB | 97.1 MiB | - | - | - |
| selective-filter | Wall time | 113.1 ms | - | 350.1 ms | 943.5 ms | - | - | 218.9 ms | 146.8 ms | 200.4 ms | - | - | - |
| selective-filter | Peak RSS | 56.6 MiB | - | 327.9 MiB | 1033.9 MiB | - | - | 18.3 MiB | 117.8 MiB | 15.9 MiB | - | - | - |
| string-reduction | Wall time | 115.8 ms | - | 311.0 ms | 893.5 ms | - | - | - | - | - | - | - | - |
| string-reduction | Peak RSS | 56.9 MiB | - | 324.4 MiB | 1029.7 MiB | - | - | - | - | - | - | - | - |
| user-filter-call | Wall time | 95.2 ms | - | - | - | - | - | 148.4 ms | 189.2 ms | 197.8 ms | - | - | - |
| user-filter-call | Peak RSS | 56.9 MiB | - | - | - | - | - | 73.0 MiB | 117.6 MiB | 97.2 MiB | - | - | - |
| user-filter-map | Wall time | 112.5 ms | - | - | - | - | - | 84.9 ms | 127.4 ms | 135.7 ms | - | - | - |
| user-filter-map | Peak RSS | 56.9 MiB | - | - | - | - | - | 77.1 MiB | 117.8 MiB | 99.4 MiB | - | - | - |
| user-filter-select | Wall time | 111.9 ms | - | - | - | - | - | 89.5 ms | 132.5 ms | 140.4 ms | - | - | - |
| user-filter-select | Peak RSS | 56.8 MiB | - | - | - | - | - | 77.3 MiB | 117.8 MiB | 97.2 MiB | - | - | - |
| user-filter-sort-by | Wall time | 131.7 ms | - | - | - | - | - | 98.0 ms | 137.7 ms | 147.4 ms | - | - | - |
| user-filter-sort-by | Peak RSS | 57.7 MiB | - | - | - | - | - | 89.3 MiB | 120.0 MiB | 99.6 MiB | - | - | - |
| walk-structural | Wall time | 919.9 ms | - | - | - | - | - | 382.0 ms | 426.6 ms | 440.3 ms | - | - | - |
| walk-structural | Peak RSS | 81.7 MiB | - | - | - | - | - | 109.3 MiB | 117.4 MiB | 131.5 MiB | - | - | - |
<!-- benchmark-results:end -->

## Method

`tq-bench` checks correctness, then measures each executable from launch to
exit observation. Input preparation, worker startup and cleanup are excluded.
Native `wait4` records CPU time and lifetime peak RSS, including launch,
threads and waited descendants. The launch RSS floor is not subtracted.
First-output latency uses the earliest observed output, or completion if output
was only visible at exit.

Primary measurements run without RSS sampling; separate passes enforce RSS
limits. Compare tools on the same host and OS using repeated samples and
dispersion. The observed 1.1 ms control excess is not a timer-error guarantee.
See the [measurement details](../benchmark-harness.md).

## Workloads

1. [Startup identity](startup.md)
2. [Discarding a stream](parse-discard.md)
3. [Scalar field extraction](scalar-extraction.md)
4. [Projecting many results](multi-result-projection.md)
5. [Selective filtering](selective-filter.md)
6. [Numeric reduction](numeric-reduction.md)
7. [String reduction](string-reduction.md)
8. [Array construction](array-construction.md)
9. [Object construction](object-construction.md)
10. [Deep object merge](object-deep-merge.md)
11. [Path update](path-update.md)
12. [Blocking sort](blocking-sort.md)
13. [Sort by multiple keys](comma-generator-sort.md)
14. [Identity re-encoding](identity-reencode.md)
15. [Event stream filtering](event-stream.md)
16. [Recursive scalar traversal](recursive-scalars.md)
17. [User function calls](user-filter-call.md)
18. [User function mapping](user-filter-map.md)
19. [User function selection](user-filter-select.md)
20. [User function sorting](user-filter-sort-by.md)
21. [Regular-expression testing](regex-test.md)
22. [Sort before counting](dead-sort-length.md)
23. [Grouping a collection](issue5-collection.md)
24. [Enumerating paths](issue5-paths.md)
25. [JSON round trip](issue5-json-conversion.md)
26. [Any-match predicate](issue5-predicate.md)
27. [String and scalar utilities](issue5-scalar-utilities.md)
28. [Reading additional inputs](issue5-inputs.md)
29. [Base64 startup formatting](format-base64-startup.md)
30. [JSON text formatting](format-json.md)
31. [HTML and URI escaping](format-uri-html.md)
32. [CSV formatting](format-csv.md)
33. [TSV formatting](format-tsv.md)
34. [Shell quoting](format-shell.md)
35. [Base64 round trip](format-base64-roundtrip.md)
36. [Templated URI output](format-template.md)
37. [Bounded recursion](recurse-bounded.md)
38. [Structural walking](walk-structural.md)
39. [Early break with a label](label-early-break.md)
40. [Native CSV record parsing](native-csv.md)
41. [Native TSV record parsing](native-tsv.md)
42. [Native JSON sequence parsing](native-json-seq.md)

## TOON 4.1 campaign evidence

The standard/exhaustive/compare campaign completed in 717.291 seconds with
all four frozen hour/day/week/month manifests and no case or adapter filters.
The release harness was built from
`3c025a842f23a76a3b444ed85269ac21b0304a0c` before host admission.

Each measurement phase passed a 30-second idle window. The comparison's
pre-run CPU idle minimum was 99.57%; all 150 host observations recorded no
swap activity, no unrelated builds/tests, at most 0.60% external host CPU
and at most 0.30% I/O wait. The monitor excludes owned benchmark descendants
from external CPU and interrupts the owned coordinator on admission violations.
No unrelated workload was stopped.

Fresh controls contain 420 accounting records and 120 worker-isolation
records with no validation failures. The worker uses
`tq-bench-worker-protocol-v4`; its SHA-256 is
`35263fb3f434f7a2e8be8d9b3f4bde6784da5bf075d7316880ade03d98e548de`.
The linked calibration summary SHA-256 is
`1054b037c39bcc36cd967aa54d60a4131e13c851f4008b2eff549aa18bf04eed`;
the raw workload report SHA-256 is
`9b901f72318284cc9421a9bb44cbf20a7d434c37360250a4a58437aea6447053`.
Raw records, host observations and commands remain in the benchmark archive,
not this documentation bundle. See [worker validation](worker-validation.md)
for the control scope and limits.

