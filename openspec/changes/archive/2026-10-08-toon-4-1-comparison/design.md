# Comparison campaign contract

The fresh comparison is a diagnostic work item, not historical acceptance. It is pending because no candidate host is admitted as idle. The prior ten-workload C8 report, including its accepted 14.32% advisory result, remains the old report and is not replaced, retagged, or presented as a fresh comparison.

## Admission and status

Require an immediately-before-run and through-run observation of idle CPU and I/O. Current readiness observations: Ironhide load 4.66 with CPU idle 42–49% and heavy swap/I/O; Sideswipe load 3.87–3.96 with CPU idle 70–78%; localJetfire load 1.97–2.05 with CPU idle 90–93%. None is admitted. Do not stop unrelated services or relax admission rules. Status remains `pending-observably-idle-host`; no fresh benchmark has been executed or promoted. A different host requires distinct campaign metadata and fresh readiness evidence; none is currently admitted.

## Reproduction when admitted

Use the frozen tq 0.5 C9 Linux candidate at `/tmp/tq-toon-4-1-candidate-c9-alpha-20261007/source`, the existing comparison helper, original jq 1.8.1 and yq 4.53.2, and all four retained Linux corpus manifests:

- `/var/tmp/tq-performance-20260924.UbhxO1/corpus/campaigns/2026-09-24T22-27-25.718567059Z/usgs-all-hour/manifest.json`
- `/var/tmp/tq-performance-20260924.UbhxO1/corpus/campaigns/2026-09-24T22-27-25.718567059Z/usgs-all-day/manifest.json`
- `/var/tmp/tq-performance-20260924.UbhxO1/corpus/campaigns/2026-09-24T22-27-25.718567059Z/usgs-all-week/manifest.json`
- `/var/tmp/tq-performance-20260924.UbhxO1/corpus/campaigns/2026-09-24T22-27-25.718567059Z/usgs-all-month/manifest.json`

Run the helper/native campaign with `--suite natural-corpus --profile standard --mode exhaustive --sampling compare`, all four manifests, no `--case` or `--adapter` filters, and an explicit new report output path. Standard plus compare is one warmup and three measured samples. Do not use `--sampling catalog`: it is not the requested repetition policy. Retain all outcomes, rows, campaign metadata, and tool input hashes.

Do not use `--markdown-dir` without fresh matching calibration and launch-isolation evidence. Do not run a quick baseline or promote this comparison into publication evidence. Do not overlap with builds or tests. Observe and record idle CPU and I/O immediately before and continuously through the run; abort admission if that condition ceases. Do not stop services or relax any guard. Report the actual status and evidence; never create a placeholder report or mark the pending run complete.

The four frozen Linux manifests and historical binary identities are inputs only. A run on any other host needs distinct campaign metadata and separately measured tool identities; it cannot inherit historical host/readiness assertions.
