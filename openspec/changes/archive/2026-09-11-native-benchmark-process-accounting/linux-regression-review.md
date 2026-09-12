# Linux tq self-regression review

Date: 2026-09-11

Scope: standard native Linux campaign, baseline `6e357f7` versus the candidate
release-benchmark build. This is a same-host, same-OS tq self-regression
review; it is not a cross-OS comparison.

## Evidence identity

| Evidence | Identity |
| --- | --- |
| Baseline report | `baseline-standard.json`, SHA-256 `c479b2aa268a0f346b04a33f2ad3b48a3c77de9f85b3b8b388677d12595573a1` |
| Candidate report and full gate | `candidate-standard.json`, SHA-256 `5d1759dfb7bf70bf9d653270a8c6b361051f12ab34faa29d9b729e9a083e7634` |
| Native collector source | `9527c5326c87782e237f448ae91eb93ee479c0e4ebcb6f039ad04cdd51e03bf5` |
| Calibration/control evidence | `d3048417f44817ca38f6ae2b0f818f8a0c926afc8c9f1dc123d13e54e3efcad6` |
| Worker protocol | `tq-bench-worker-protocol-v3`; worker SHA-256 `2e77024616bc265b9f044eef803435a00d6dd21d1bd738e6a0d6e66d52e8bb73` |
| Corpus | Frozen campaign `2026-09-05T23-11-16.716437607Z`; `usgs-all-month` source manifest SHA-256 `406988754869a8dd9a1fe3802be33a200b6508f6c873a927e3a7e1365014c895`, source artifact SHA-256 `459af9ee1e8b067d8e3602454ff2c7f47baf60f3575d3915143855c333f923d2` |

Both reports use an AMD Ryzen 7 7700 with 8 physical cores, 16 logical CPUs,
61.9 GiB RAM and x86_64 Linux kernel `7.2.0-ogc4.1.fc44.x86_64`.
Both builds use the pinned nightly-2026-08-30 compiler and
`release-benchmark` profile. macOS evidence is separate and is not pooled or
ranked here.

## Gate result

The issue #30 gate uses median wall time and the maximum observed peak RSS
per row, calculated independently from unrounded measurements. With at least
five samples and a 50% threshold for each metric, it evaluated 410 comparable
standard tq rows. No increase exceeded the 20% disclosure threshold or the
50% blocking threshold.

The 13 unavailable rows were retained and excluded from the denominator:

- 4 unsupported `tq-yaml` event-stream rows (one per frozen source); and
- 9 resource-limit rows: 3 object-construction formats on `usgs-all-month`,
  plus 3 string-reduction formats on each of `usgs-all-month` and
  `usgs-all-week`.

## Largest comparable changes

The largest wall-time increase was 13.2% for
`benchmark.format-base64-roundtrip` / `tq-toon` / `usgs-all-hour`:
baseline median `1,176.5` µs versus candidate `1,331.5` µs. Both had 30
samples; median absolute deviation was `9.5` µs versus `29.5` µs. The
baseline p95 was 1,511.0 µs and candidate p95 was 1,492.0 µs. Baseline
samples ranged from 1,151.0 µs to 1,632.0 µs; candidate samples ranged from
1,159.0 µs to 1,625.0 µs.

The largest peak-RSS increase was 3.0% for
`benchmark.multi-result-projection` / `tq-json` / `usgs-all-week`.
Maximum observed peak RSS increased from 7.6 MiB to 7.9 MiB across 10
samples per build. Raw byte counts remain in the reports; rounded display
values do not determine acceptance.

These results support acceptance of the Linux tq self-regression gate only.
They do not claim cross-OS performance equivalence or eliminate the separate
independent native `time` validation controls.
