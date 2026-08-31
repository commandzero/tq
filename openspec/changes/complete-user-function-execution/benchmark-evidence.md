# Benchmark evidence

The focused `standard` campaign ran on 2026-08-31 against the frozen
`usgs-all-month` JSON corpus (7,877,699 bytes), using release builds and ten
samples per tool and case. The raw report is
`benchmarks/.work/is8-standard.json` and is intentionally ignored as local
campaign output.

| Case | jq median | tq median | tq / jq | Goal |
| --- | ---: | ---: | ---: | ---: |
| Direct call | 94,168 us | 179,466 us | 1.906 | <= 2.0 |
| `map` callback | 93,265 us | 124,976 us | 1.340 | <= 2.0 |
| `select` callback | 116,149 us | 135,406.5 us | 1.166 | <= 2.0 |
| `sort_by` callback | 119,132.5 us | 150,397 us | 1.262 | <= 2.0 |

All measured wall-time goals passed. Peak RSS was unavailable (`null`) for
both jq and tq in every sample on this macOS host, so the 1.5-times memory goal
is unmeasured rather than passing. No measured soft target missed, and no
profiling follow-up is required from this campaign.
