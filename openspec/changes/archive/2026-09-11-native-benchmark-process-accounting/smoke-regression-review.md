# Linux smoke regression review

## Scope and decision

This is a supplemental acceptance evaluation for the Linux issue #30 smoke
campaign. It covers the nine `tq` baseline/candidate pairs in the three
workloads below. The separate [standard campaign review](linux-regression-review.md)
covers the other 410 comparable tq rows.

The raw reports are retained unchanged in the external benchmark archive. Each
row has 30 baseline samples and 30 candidate samples. The table displays wall
medians, report peak RSS maxima, and RSS sample median absolute deviations in
one-decimal units. The threshold decision uses the unrounded values from the
retained reports. Percentages are shown to one decimal. A positive change
means the candidate is slower or uses more memory.

| Workload / adapter | Samples (baseline / candidate) | Wall median (MAD), baseline | Wall median (MAD), candidate | Wall change | Peak RSS max (sample MAD), baseline | Peak RSS max (sample MAD), candidate | RSS change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `format-base64-startup` / `tq-json` | 30 / 30 | 1.2 ms (MAD 0.0 ms) | 1.2 ms (MAD 0.0 ms) | -0.3% | 7.9 MiB (sample MAD 0.1 MiB) | 7.8 MiB (sample MAD 0.0 MiB) | -0.5% |
| `format-base64-startup` / `tq-yaml` | 30 / 30 | 1.2 ms (MAD 0.0 ms) | 1.2 ms (MAD 0.0 ms) | +0.9% | 8.0 MiB (sample MAD 0.1 MiB) | 8.0 MiB (sample MAD 0.1 MiB) | -0.1% |
| `format-base64-startup` / `tq-toon` | 30 / 30 | 1.2 ms (MAD 0.0 ms) | 1.2 ms (MAD 0.0 ms) | +0.1% | 7.8 MiB (sample MAD 0.1 MiB) | 7.7 MiB (sample MAD 0.1 MiB) | -0.5% |
| `object-deep-merge` / `tq-json` | 30 / 30 | 48.5 ms (MAD 0.9 ms) | 48.4 ms (MAD 1.1 ms) | -0.2% | 36.8 MiB (sample MAD 0.1 MiB) | 36.8 MiB (sample MAD 0.1 MiB) | +0.2% |
| `object-deep-merge` / `tq-yaml` | 30 / 30 | 87.8 ms (MAD 0.7 ms) | 88.1 ms (MAD 0.3 ms) | +0.4% | 56.5 MiB (sample MAD 0.1 MiB) | 56.5 MiB (sample MAD 0.1 MiB) | +0.0% |
| `object-deep-merge` / `tq-toon` | 30 / 30 | 48.7 ms (MAD 1.3 ms) | 48.4 ms (MAD 0.6 ms) | -0.5% | 38.4 MiB (sample MAD 0.1 MiB) | 38.3 MiB (sample MAD 0.1 MiB) | -0.1% |
| `startup` / `tq-json` | 30 / 30 | 1.0 ms (MAD 0.0 ms) | 1.0 ms (MAD 0.0 ms) | +1.0% | 7.1 MiB (sample MAD 0.1 MiB) | 7.0 MiB (sample MAD 0.1 MiB) | -1.1% |
| `startup` / `tq-yaml` | 30 / 30 | 1.0 ms (MAD 0.0 ms) | 1.0 ms (MAD 0.0 ms) | +0.6% | 7.3 MiB (sample MAD 0.1 MiB) | 7.4 MiB (sample MAD 0.1 MiB) | +0.6% |
| `startup` / `tq-toon` | 30 / 30 | 1.0 ms (MAD 0.0 ms) | 1.0 ms (MAD 0.0 ms) | -1.0% | 7.0 MiB (sample MAD 0.1 MiB) | 7.0 MiB (sample MAD 0.1 MiB) | -0.9% |

No row reaches the 20% disclosure threshold or the 50% blocking threshold.
All nine rows are therefore accepted by the issue #30 smoke regression policy.
RSS acceptance uses the report peak RSS maximum; the sample MAD values provide
dispersion context and are not substituted for that gate metric. No sample was
removed, rewritten, substituted, or rerun for this evaluation.

## Host and measurement identity

Both reports were collected on the same Linux host identity and have matching
host metadata:

- OS: Linux; kernel `7.2.0-ogc4.1.fc44.x86_64`.
- Architecture: `x86_64`.
- CPU: AMD Ryzen 7 7700 8-Core Processor; 16 logical CPUs.
- Memory: 61.9 GiB (66,457,382,912 bytes).
- Build: `release-benchmark`, pinned nightly-2026-08-30 compiler.

The reports use the same jq 1.8.1 and pinned yq 4.53.2 identities, corpus
bytes, corpus hashes, logical-record counts, corpus manifest, worker protocol,
collector source, measurement contract, and calibration linkage. The tq
executable identity differs as required for a baseline/candidate comparison.
Measurements use the native `tq-bench` worker and Linux `wait4` accounting;
there are no time-command measurements in these reports.

## Three unavailable JSON rows

The generated candidate report correctly preserves the automated gate result:
the `tq-json` rows for `startup`, `format-base64-startup`, and
`object-deep-merge` are listed as unavailable because command normalization did
not remove their relocated absolute input paths. Their corpus entries recorded
relative artifact paths (`startup.json` and `deep-merge.json`), while the
commands contain absolute temporary paths. The YAML and TOON entries recorded
absolute paths, so their equivalent relocation was normalized successfully.

This is a report-generation bookkeeping defect, not a measurement failure. For
the three JSON rows, the baseline and candidate reports have identical corpus
artifact bytes and SHA-256 values, identical logical-record metadata and
manifest identity, and commands equivalent apart from the expected executable
identity and temporary-root relocation. Their worker, collector, host, and
measurement identities also match. The retained raw samples independently
provide the values in the table above.

Accordingly, this document is a legitimate supplemental evaluation of those
three measured rows while the original automated `unavailable` statuses remain
unchanged in the retained raw reports. The generator path-normalization fix is
now implemented for future reports; it does not alter these immutable raw
artifacts. This supplemental evaluation does not broaden acceptance to the
full standard campaign.

## Retained raw evidence

Paths are relative to the external archive's `.work/` directory. The SHA-256
values identify the complete, unmodified JSON reports:

1. `native-verification-20260911/linux/campaign/baseline-smoke.json`:
   `c800f283bd9af03ed573f53c466afc2a55a9133396d2425fed78fb543ae7ef9f`.
2. `native-verification-20260911/linux/campaign/candidate-smoke.json`:
   `eabe3b11f721aa4cefd17c2110e79c325f374c06719cf82efbfc29e7115ac4fd`.
